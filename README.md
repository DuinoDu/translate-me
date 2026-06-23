<p align="center">
  <img src="src-tauri/icons/icon.png" width="120" alt="translate-me logo" />
</p>

<h1 align="center">translate-me</h1>

<p align="center">A floating macOS translator — summon with a hotkey, translate as you type. Rust + Tauri v2.</p>

<p align="center">
  <a href="https://x.com/LovemoonRobot/status/2069400393722007735">𝕏 / Twitter</a>
  &nbsp;·&nbsp;
  <a href="http://xhslink.com/o/HbALduehJj">小红书 / Xiaohongshu</a>
</p>

## Demo

<p align="center">
  <img src="media/translate-me-usage.gif" width="640" alt="translate-me demo" />
</p>

<p align="center"><em><a href="https://github.com/DuinoDu/translate-me/releases/download/v0.1.0/translate-me-usage.mp4">▶ Full video (with audio)</a></em></p>

## Features

- **⌘⇧T** to summon / dismiss — the input box appears at your cursor
- Translates live as you type (DeepSeek `deepseek-chat`, 350ms debounce)
- Auto-growing box, one-click **copy**, **Esc** to dismiss
- Frameless · translucent · always-on-top · no Dock icon (menubar tray app)
- Settings panel: API key, hotkey, source / target language, font size

## Run

```bash
npm install
npm run tauri dev      # develop
npm run tauri build    # bundle .app / .dmg
```

## Configure

Open **Settings** from the menubar tray icon (or the ⚙ button) to set the API key,
hotkey, languages and font size. Settings persist to
`~/Library/Application Support/com.duino.translateme/settings.json`.
