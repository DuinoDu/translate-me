use tauri::{Emitter, Manager, PhysicalPosition};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

/// Translate `text` into `target` (default English) using the DeepSeek chat API.
/// Returns the translated string, or an error message shown in the UI.
#[tauri::command]
async fn translate(text: String, target: Option<String>) -> Result<String, String> {
    let text = text.trim().to_string();
    if text.is_empty() {
        return Ok(String::new());
    }
    let target = target.unwrap_or_else(|| "English".to_string());

    let api_key = std::env::var("DEEPSEEK_API_KEY")
        .map_err(|_| "DEEPSEEK_API_KEY is not set (check your .env)".to_string())?;
    let base = std::env::var("DEEPSEEK_BASE_URL")
        .unwrap_or_else(|_| "https://api.deepseek.com/v1".to_string());
    let url = format!("{}/chat/completions", base.trim_end_matches('/'));

    let body = serde_json::json!({
        "model": "deepseek-chat",
        "messages": [
            {
                "role": "system",
                "content": format!(
                    "You are a translation engine. Translate the user's text into {}. \
                     Output ONLY the translation itself — no quotes, no explanations, no extra text.",
                    target
                )
            },
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

/// Toggle the floating input: if visible, hide it; otherwise move it just below
/// the mouse cursor, show it and focus the input field.
fn toggle_at_cursor(app: &tauri::AppHandle) {
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
    // tell the frontend to clear + focus the input
    let _ = app.emit("summon", ());
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load DEEPSEEK_* keys from .env (searches cwd and parent dirs).
    let _ = dotenvy::dotenv();

    // Default hotkey: Cmd+Shift+T. Change the string below to rebind.
    let toggle: Shortcut = "CommandOrControl+Shift+T"
        .parse()
        .expect("invalid global shortcut definition");
    let toggle_for_handler = toggle.clone();

    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if shortcut == &toggle_for_handler && event.state() == ShortcutState::Pressed {
                        toggle_at_cursor(app);
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![translate, hide_window])
        .setup(move |app| {
            // No Dock icon — behaves like a lightweight menubar/utility app.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            app.global_shortcut().register(toggle)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
