use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

const DEFAULT_HOTKEY: &str = "Alt+Space";
const LEGACY_DEFAULT_HOTKEY: &str = "CommandOrControl+Shift+T";
const DEFAULT_BASE_URL: &str = "https://api.deepseek.com/v1";

/// User-configurable settings, persisted to `settings.json` in the app config dir.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Settings {
    #[serde(default)]
    api_key: String,
    #[serde(default = "default_base_url")]
    base_url: String,
    /// Source language, or "auto" to let the model detect it.
    #[serde(default = "default_source")]
    source_lang: String,
    #[serde(default = "default_target")]
    target_lang: String,
    #[serde(default = "default_hotkey")]
    hotkey: String,
    /// Font size (px) for the floating input / translation text.
    #[serde(default = "default_font_size")]
    font_size: u32,
}

fn default_base_url() -> String {
    std::env::var("DEEPSEEK_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string())
}
fn default_source() -> String {
    "auto".to_string()
}
fn default_target() -> String {
    "English".to_string()
}
fn default_hotkey() -> String {
    DEFAULT_HOTKEY.to_string()
}
fn default_font_size() -> u32 {
    15
}

impl Settings {
    /// Initial settings seeded from environment / .env (first run only).
    fn from_env() -> Self {
        Settings {
            api_key: std::env::var("DEEPSEEK_API_KEY").unwrap_or_default(),
            base_url: default_base_url(),
            source_lang: default_source(),
            target_lang: default_target(),
            hotkey: default_hotkey(),
            font_size: default_font_size(),
        }
    }
}

struct AppState {
    settings: Settings,
    config_path: Option<PathBuf>,
    translate_item: Option<MenuItem<tauri::Wry>>,
}

type SharedState = Mutex<AppState>;

fn write_settings_file(path: &PathBuf, settings: &Settings) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| format!("写入配置失败: {e}"))
}

/// Return the current settings to the frontend.
#[tauri::command]
fn get_settings(state: State<'_, SharedState>) -> Settings {
    state.lock().unwrap().settings.clone()
}

/// Persist new settings, re-register the global hotkey, and write to disk.
#[tauri::command]
fn save_settings(
    app: AppHandle,
    state: State<'_, SharedState>,
    settings: Settings,
) -> Result<(), String> {
    // Validate the hotkey before committing anything.
    let shortcut: Shortcut = settings
        .hotkey
        .parse()
        .map_err(|_| format!("无法识别的快捷键: {}", settings.hotkey))?;

    // Re-register the global shortcut (we only ever keep one registered).
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    gs.register(shortcut)
        .map_err(|e| format!("快捷键注册失败（可能被占用）: {e}"))?;

    let mut guard = state.lock().unwrap();
    // Keep the tray menu's displayed shortcut in sync with the new hotkey.
    if let Some(item) = &guard.translate_item {
        let _ = item.set_accelerator(Some(settings.hotkey.as_str()));
    }
    guard.settings = settings;
    if let Some(path) = guard.config_path.clone() {
        write_settings_file(&path, &guard.settings)?;
    }
    Ok(())
}

/// Translate `text` using the DeepSeek chat API and the saved settings.
#[tauri::command]
async fn translate(text: String, state: State<'_, SharedState>) -> Result<String, String> {
    let text = text.trim().to_string();
    if text.is_empty() {
        return Ok(String::new());
    }

    // Snapshot the settings, then drop the lock before any await.
    let (api_key, base, source, target) = {
        let g = state.lock().unwrap();
        let s = &g.settings;
        (
            s.api_key.clone(),
            s.base_url.clone(),
            s.source_lang.clone(),
            s.target_lang.clone(),
        )
    };

    if api_key.trim().is_empty() {
        return Err("尚未配置 API Key（请在设置中填写）".to_string());
    }

    let url = format!("{}/chat/completions", base.trim_end_matches('/'));
    let instruction = if source.eq_ignore_ascii_case("auto") || source.is_empty() {
        format!(
            "You are a translation engine. Translate the user's text into {target}. \
             Output ONLY the translation itself — no quotes, no explanations, no extra text."
        )
    } else {
        format!(
            "You are a translation engine. Translate the user's text from {source} into {target}. \
             Output ONLY the translation itself — no quotes, no explanations, no extra text."
        )
    };

    let body = serde_json::json!({
        "model": "deepseek-chat",
        "messages": [
            { "role": "system", "content": instruction },
            { "role": "user", "content": text }
        ],
        "temperature": 0,
        "stream": false
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .post(&url)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let detail = resp.text().await.unwrap_or_default();
        return Err(format!("DeepSeek API error {status}: {detail}"));
    }

    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let out = v["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or_default()
        .trim()
        .to_string();
    Ok(out)
}

/// Hide the floating window (called from the frontend on Esc / blur).
#[tauri::command]
fn hide_window(window: tauri::WebviewWindow) {
    let _ = window.hide();
}

/// Show the settings window (called from the ⚙ button and the tray menu).
#[tauri::command]
fn open_settings(app: AppHandle) {
    show_settings(&app);
}

fn show_settings(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

#[cfg(target_os = "macos")]
fn prepare_overlay_window(win: &tauri::WebviewWindow) {
    use objc2_app_kit::{NSPopUpMenuWindowLevel, NSWindow, NSWindowCollectionBehavior};

    let _ = win.set_always_on_top(true);
    let _ = win.set_focusable(true);
    let _ = win.set_visible_on_all_workspaces(false);

    if let Ok(raw) = win.ns_window() {
        let ns_window: &NSWindow = unsafe { &*raw.cast::<NSWindow>() };
        let mut behavior = ns_window.collectionBehavior();
        behavior &= !(NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenPrimary
            | NSWindowCollectionBehavior::FullScreenNone
            | NSWindowCollectionBehavior::Managed
            | NSWindowCollectionBehavior::Primary
            | NSWindowCollectionBehavior::Auxiliary
            | NSWindowCollectionBehavior::Stationary);
        behavior |= NSWindowCollectionBehavior::MoveToActiveSpace
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::CanJoinAllApplications
            | NSWindowCollectionBehavior::Transient;

        ns_window.setCollectionBehavior(behavior);
        ns_window.setLevel(NSPopUpMenuWindowLevel);
        ns_window.setCanHide(false);
        ns_window.setHidesOnDeactivate(false);
    }
}

#[cfg(not(target_os = "macos"))]
fn prepare_overlay_window(_win: &tauri::WebviewWindow) {}

#[cfg(target_os = "macos")]
fn focus_overlay_window(win: &tauri::WebviewWindow) {
    use objc2_app_kit::NSWindow;

    if let Ok(raw) = win.ns_window() {
        let ns_window: &NSWindow = unsafe { &*raw.cast::<NSWindow>() };
        ns_window.makeKeyAndOrderFront(None);
        ns_window.orderFrontRegardless();
    }
}

#[cfg(not(target_os = "macos"))]
fn focus_overlay_window(_win: &tauri::WebviewWindow) {}

#[cfg(target_os = "macos")]
fn overlay_is_frontmost(win: &tauri::WebviewWindow) -> bool {
    use objc2_app_kit::NSWindow;

    if let Ok(raw) = win.ns_window() {
        let ns_window: &NSWindow = unsafe { &*raw.cast::<NSWindow>() };
        ns_window.isVisible() && ns_window.isKeyWindow()
    } else {
        win.is_visible().unwrap_or(false) && win.is_focused().unwrap_or(false)
    }
}

#[cfg(not(target_os = "macos"))]
fn overlay_is_frontmost(win: &tauri::WebviewWindow) -> bool {
    win.is_visible().unwrap_or(false) && win.is_focused().unwrap_or(false)
}

fn position_near_cursor(app: &AppHandle, win: &tauri::WebviewWindow) {
    let Ok(cursor) = app.cursor_position() else {
        return;
    };

    let Some(monitor) = app
        .monitor_from_point(cursor.x, cursor.y)
        .ok()
        .flatten()
        .or_else(|| win.current_monitor().ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten())
    else {
        let _ = win.set_position(PhysicalPosition::new(
            cursor.x.round() as i32,
            cursor.y.round() as i32 + 18,
        ));
        return;
    };

    let work_area = monitor.work_area();
    let size = win.outer_size().ok();
    let width = size.map(|s| s.width as i32).unwrap_or(480);
    let height = size.map(|s| s.height as i32).unwrap_or(180);
    let margin = 8;

    let min_x = work_area.position.x + margin;
    let min_y = work_area.position.y + margin;
    let max_x = work_area.position.x + work_area.size.width as i32 - width - margin;
    let max_y = work_area.position.y + work_area.size.height as i32 - height - margin;
    let x = (cursor.x.round() as i32).clamp(min_x, max_x.max(min_x));
    let y = (cursor.y.round() as i32 + 18).clamp(min_y, max_y.max(min_y));

    let _ = win.set_position(PhysicalPosition::new(x, y));
}

/// Force the floating input to show near the cursor and become the frontmost
/// window. This path is used by both tray menu and hotkey summon.
fn show_at_cursor(app: &AppHandle) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };

    if win.is_visible().unwrap_or(false) && !overlay_is_frontmost(&win) {
        let _ = win.hide();
    }
    prepare_overlay_window(&win);
    let _ = win.unminimize();
    position_near_cursor(app, &win);
    let _ = win.show();
    focus_overlay_window(&win);
    let _ = win.set_focus();
    focus_overlay_window(&win);
    let _ = app.emit("summon", ());
}

/// Toggle the floating input: if visible, hide it; otherwise move it just below
/// the mouse cursor, show it and focus the input field.
fn toggle_at_cursor(app: &AppHandle) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    if overlay_is_frontmost(&win) {
        let _ = win.hide();
        return;
    }
    show_at_cursor(app);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load DEEPSEEK_* keys from .env (searches cwd and parent dirs) for first run.
    let _ = dotenvy::dotenv();

    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, _shortcut, event| {
                    // Only one shortcut is ever registered, so any press = summon.
                    if event.state() == ShortcutState::Pressed {
                        let app_handle = app.clone();
                        let _ = app.run_on_main_thread(move || toggle_at_cursor(&app_handle));
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            translate,
            hide_window,
            get_settings,
            save_settings,
            open_settings
        ])
        .setup(move |app| {
            // No Dock icon — behaves like a lightweight menubar/utility app.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            if let Some(win) = app.get_webview_window("main") {
                prepare_overlay_window(&win);
            }

            // Load persisted settings (fall back to env-seeded defaults).
            let config_path = app
                .path()
                .app_config_dir()
                .ok()
                .map(|d| d.join("settings.json"));
            let mut settings = Settings::from_env();
            if let Some(p) = &config_path {
                if let Ok(raw) = fs::read_to_string(p) {
                    if let Ok(parsed) = serde_json::from_str::<Settings>(&raw) {
                        settings = parsed;
                    }
                }
            }
            if settings.hotkey == LEGACY_DEFAULT_HOTKEY {
                settings.hotkey = default_hotkey();
                if let Some(path) = &config_path {
                    let _ = write_settings_file(path, &settings);
                }
            }

            // Register the configured hotkey (fall back to default if invalid).
            let shortcut: Shortcut = settings
                .hotkey
                .parse()
                .unwrap_or_else(|_| DEFAULT_HOTKEY.parse().unwrap());
            app.global_shortcut().register(shortcut)?;

            // System tray (the only persistent entry point for an Accessory app).
            // "翻译" shows the current hotkey as its accelerator (rendered ⌥Space on macOS).
            let summon_i =
                MenuItem::with_id(app, "summon", "翻译", true, Some(settings.hotkey.as_str()))?;
            let settings_i = MenuItem::with_id(app, "settings", "设置…", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&summon_i, &settings_i, &quit_i])?;

            app.manage(Mutex::new(AppState {
                settings,
                config_path,
                translate_item: Some(summon_i.clone()),
            }));

            let mut tray = TrayIconBuilder::new()
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "summon" => show_at_cursor(app),
                    "settings" => show_settings(app),
                    "quit" => app.exit(0),
                    _ => {}
                });
            // Use the (newly replaced) app logo as the menubar icon. Embedding the
            // file at build time guarantees the tray reflects the current icon.
            if let Ok(img) = tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png")) {
                tray = tray.icon(img);
            } else if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
