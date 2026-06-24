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

- **⌥Space** to summon / dismiss — the input box appears at your cursor
- Translates live as you type (OpenAI-compatible or Anthropic-compatible APIs, 350ms debounce)
- Auto-growing box, one-click **copy**, **Esc** to dismiss
- Frameless · translucent · always-on-top · no Dock icon (menubar tray app)
- Settings panel: API key, hotkey, source / target language, font size

## Run

```bash
npm install
npm run tauri dev      # develop
npm run tauri build    # bundle .app / .dmg
./scripts/install-local.sh
```

For local builds, use `./scripts/install-local.sh` instead of manually copying
the `.app`. It builds, installs to `/Applications`, removes quarantine metadata,
and applies a stable local code-signing requirement.

Or you can install `translate-me` by claude code or codex:

```
claude --dangerously-skip-permissions "Install translate-me from source: https://github.com/duinodu/translate-me"
```

```
codex --dangerously-bypass-approvals-and-sandbox "Install translate-me from source: https://github.com/duinodu/translate-me"
```

## Configure

Open **Settings** from the menubar tray icon (or the ⚙ button) to set the API token,
provider, model, hotkey, languages and font size. Settings persist to
`~/Library/Application Support/com.duino.translateme/settings.json`.
