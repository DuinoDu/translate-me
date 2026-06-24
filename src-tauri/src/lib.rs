use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

const DEFAULT_HOTKEY: &str = "CommandOrControl+Shift+T";
const DEFAULT_BASE_URL: &str = "https://api.deepseek.com/v1";
const DEFAULT_PROVIDER: &str = "openai";
const DEFAULT_MODEL: &str = "deepseek-chat";
const DEFAULT_ANTHROPIC_VERSION: &str = "2023-06-01";

/// User-configurable settings, persisted to `settings.json` in the app config dir.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Settings {
    #[serde(default)]
    api_key: String,
    #[serde(default = "default_base_url")]
    base_url: String,
    #[serde(default = "default_provider")]
    provider: String,
    #[serde(default = "default_model")]
    model: String,
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
    std::env::var("ANTHROPIC_BASE_URL")
        .or_else(|_| std::env::var("DEEPSEEK_BASE_URL"))
        .unwrap_or_else(|_| DEFAULT_BASE_URL.to_string())
}
fn default_provider() -> String {
    if std::env::var("ANTHROPIC_AUTH_TOKEN").is_ok() || std::env::var("ANTHROPIC_BASE_URL").is_ok() {
        "anthropic".to_string()
    } else {
        DEFAULT_PROVIDER.to_string()
    }
}
fn default_model() -> String {
    std::env::var("ANTHROPIC_SMALL_FAST_MODEL")
        .or_else(|_| std::env::var("ANTHROPIC_DEFAULT_HAIKU_MODEL"))
        .or_else(|_| std::env::var("ANTHROPIC_DEFAULT_SONNET_MODEL"))
        .or_else(|_| std::env::var("ANTHROPIC_DEFAULT_OPUS_MODEL"))
        .unwrap_or_else(|_| DEFAULT_MODEL.to_string())
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
        let provider = default_provider();
        Settings {
            api_key: std::env::var("ANTHROPIC_AUTH_TOKEN")
                .or_else(|_| std::env::var("DEEPSEEK_API_KEY"))
                .unwrap_or_default(),
            base_url: default_base_url(),
            provider,
            model: default_model(),
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
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let json = serde_json::to_string_pretty(&guard.settings).map_err(|e| e.to_string())?;
        fs::write(&path, json).map_err(|e| format!("写入配置失败: {e}"))?;
    }
    Ok(())
}

/// Translate `text` using the configured provider and the saved settings.
#[tauri::command]
async fn translate(text: String, state: State<'_, SharedState>) -> Result<String, String> {
    let text = text.trim().to_string();
    if text.is_empty() {
        return Ok(String::new());
    }

    // Snapshot the settings, then drop the lock before any await.
    let (api_key, base, provider, model, source, target) = {
        let g = state.lock().unwrap();
        let s = &g.settings;
        (
            s.api_key.clone(),
            s.base_url.clone(),
            s.provider.clone(),
            s.model.clone(),
            s.source_lang.clone(),
            s.target_lang.clone(),
        )
    };

    if api_key.trim().is_empty() {
        return Err("尚未配置 API Key（请在设置中填写）".to_string());
    }

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

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?;

    if provider.eq_ignore_ascii_case("anthropic") {
        let url = format!("{}/v1/messages", base.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": model,
            "max_tokens": 512,
            "thinking": { "type": "disabled" },
            "system": instruction,
            "messages": [
                { "role": "user", "content": text }
            ]
        });

        let resp = client
            .post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", DEFAULT_ANTHROPIC_VERSION)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let detail = resp.text().await.unwrap_or_default();
            return Err(format!("Anthropic API error {status}: {detail}"));
        }

        let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let out = v["content"]
            .as_array()
            .map(|blocks| {
                blocks
                    .iter()
                    .filter_map(|block| {
                        if block["type"].as_str() == Some("text") {
                            block["text"].as_str().map(str::trim)
                        } else {
                            None
                        }
                    })
                    .filter(|text| !text.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();
        return Ok(out);
    }

    let url = format!("{}/chat/completions", base.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": model,
        "messages": [
            { "role": "system", "content": instruction },
            { "role": "user", "content": text }
        ],
        "temperature": 0,
        "stream": false
    });

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
        return Err(format!("OpenAI-compatible API error {status}: {detail}"));
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

/// Toggle the floating input: if visible, hide it; otherwise move it just below
/// the mouse cursor, show it and focus the input field.
fn toggle_at_cursor(app: &AppHandle) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    if win.is_visible().unwrap_or(false) {
        let _ = win.hide();
        return;
    }
    if let Ok(pos) = app.cursor_position() {
        let _ = win.set_position(PhysicalPosition::new(pos.x as i32, pos.y as i32 + 18));
    }
    let _ = win.show();
    let _ = win.set_focus();
    let _ = app.emit("summon", ());
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
                        toggle_at_cursor(app);
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

            // Register the configured hotkey (fall back to default if invalid).
            let shortcut: Shortcut = settings
                .hotkey
                .parse()
                .unwrap_or_else(|_| DEFAULT_HOTKEY.parse().unwrap());
            app.global_shortcut().register(shortcut)?;

            // System tray (the only persistent entry point for an Accessory app).
            // "翻译" shows the current hotkey as its accelerator (rendered ⌘⇧T on macOS).
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
                    "summon" => toggle_at_cursor(app),
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
