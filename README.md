# translate-me

一个 macOS 小工具：用全局快捷键唤起，在鼠标光标下方弹出一个悬浮输入框，
随着输入实时调用 **DeepSeek** API 翻译成英文。基于 **Rust + Tauri v2**。

## 功能

- 全局快捷键 **⌘⇧T**（Cmd+Shift+T）唤起 / 收起输入框
- 输入框出现在当前鼠标光标位置
- 边输入边翻译（350ms 防抖，自动丢弃过期结果）
- 无边框、半透明、置顶、不占 Dock（Accessory 应用）
- **Esc** 或点击别处自动收起
- 翻译引擎：DeepSeek `deepseek-chat`

## 配置

密钥从项目根目录的 `.env` 读取（dotenv 会向上层目录查找）：

```
DEEPSEEK_API_KEY=sk-xxxxxxxx
DEEPSEEK_BASE_URL=https://api.deepseek.com/v1
```

`.env` 已加入 `.gitignore`，不会被提交。

## 运行

```bash
npm install          # 安装 @tauri-apps/cli
npm run tauri dev    # 开发运行
npm run tauri build  # 打包成 .app / .dmg
```

首次运行后用 ⌘⇧T 唤起即可。

## 自定义

- **改快捷键**：编辑 `src-tauri/src/lib.rs` 中的 `"CommandOrControl+Shift+T"`。
- **改目标语言**：`translate` 命令默认译为英文（`target` 参数，默认 `"English"`）。
- **改窗口大小/外观**：`src-tauri/tauri.conf.json` 与 `src/styles.css`。

## 结构

| 文件 | 作用 |
| --- | --- |
| `src-tauri/src/lib.rs` | 全局快捷键、光标定位、`translate` / `hide_window` 命令 |
| `src/index.html` `src/main.js` `src/styles.css` | 悬浮输入框 UI 与实时翻译逻辑 |
| `src-tauri/tauri.conf.json` | 窗口（无边框/透明/置顶/隐藏启动）配置 |
| `src-tauri/capabilities/default.json` | 权限（global-shortcut、window show/hide/focus/position） |
