const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { getCurrentWindow, LogicalSize } = window.__TAURI__.window;

const appWindow = getCurrentWindow();

const WIDTH = 480; // fixed window width (logical px); height tracks content
const MAX_H = 420; // cap; beyond this the output area scrolls

const PLACEHOLDER = "译文…";

let inputEl;
let outputEl;
let statusEl;
let hkEl;
let copyBtn;
let debounceTimer = null;
let reqSeq = 0; // guards against out-of-order responses
let focusedSinceSummon = false;
let suppressAutoHideUntil = 0;

function prettyAccel(accel) {
  if (!accel) return "";
  return accel
    .replace("CommandOrControl", "⌘")
    .replace("Command", "⌘")
    .replace("Control", "⌃")
    .replace("Alt", "⌥")
    .replace("Shift", "⇧")
    .replace("Super", "⌘")
    .replaceAll("+", "");
}

// Grow the textarea to fit its text, then resize the window to fit the card.
function autosize() {
  inputEl.style.height = "auto";
  inputEl.style.height = inputEl.scrollHeight + "px";
  const want = Math.min(MAX_H, Math.ceil(document.documentElement.scrollHeight));
  appWindow.setSize(new LogicalSize(WIDTH, want)).catch(() => {});
}

function setOutput(text, cls) {
  outputEl.className = "output" + (cls ? " " + cls : "");
  outputEl.textContent = text;
  // Copy only makes sense for a real translation.
  const hasTranslation = cls === "";
  copyBtn.disabled = !hasTranslation;
  autosize();
}

async function applySettings(s) {
  const fs = (s && s.font_size) || 15;
  document.documentElement.style.setProperty("--fs", fs + "px");
  if (hkEl) hkEl.textContent = prettyAccel(s && s.hotkey) || "⌥Space";
  autosize();
}

async function refreshSettings() {
  try {
    await applySettings(await invoke("get_settings"));
  } catch (_) {}
}

async function runTranslate(text) {
  const value = text.trim();
  if (!value) {
    setOutput(PLACEHOLDER, "empty");
    statusEl.textContent = "";
    return;
  }
  const myReq = ++reqSeq;
  statusEl.textContent = "翻译中…";
  try {
    const result = await invoke("translate", { text: value });
    if (myReq !== reqSeq) return; // a newer keystroke superseded this one
    statusEl.textContent = "";
    setOutput(result || PLACEHOLDER, result ? "" : "empty");
  } catch (err) {
    if (myReq !== reqSeq) return;
    statusEl.textContent = "";
    setOutput(String(err), "error");
  }
}

function onInput() {
  autosize();
  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => runTranslate(inputEl.value), 350);
}

async function copyTranslation() {
  const text = outputEl.textContent;
  if (copyBtn.disabled || !text) return;
  try {
    await navigator.clipboard.writeText(text);
  } catch (_) {
    // Fallback for environments where the async clipboard API is blocked.
    const ta = document.createElement("textarea");
    ta.value = text;
    document.body.appendChild(ta);
    ta.select();
    document.execCommand("copy");
    document.body.removeChild(ta);
  }
  await hide();
}

async function hide() {
  if (debounceTimer) clearTimeout(debounceTimer);
  focusedSinceSummon = false;
  suppressAutoHideUntil = 0;
  reqSeq++; // cancel any in-flight render
  try {
    await invoke("hide_window");
  } catch (_) {}
}

function focusInputSoon() {
  inputEl.focus();
  requestAnimationFrame(() => inputEl.focus());
  setTimeout(() => inputEl.focus(), 60);
  setTimeout(() => inputEl.focus(), 180);
}

window.addEventListener("DOMContentLoaded", () => {
  inputEl = document.querySelector("#input");
  outputEl = document.querySelector("#output");
  statusEl = document.querySelector("#status");
  hkEl = document.querySelector("#hk");
  copyBtn = document.querySelector("#copy");

  inputEl.addEventListener("input", onInput);
  copyBtn.addEventListener("click", copyTranslation);
  document.querySelector("#gear").addEventListener("click", () => {
    invoke("open_settings");
  });

  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      e.preventDefault();
      hide();
    }
  });

  // The hotkey handler in Rust emits "summon" each time the window is shown.
  listen("summon", () => {
    focusedSinceSummon = false;
    suppressAutoHideUntil = Date.now() + 900;
    inputEl.value = "";
    setOutput(PLACEHOLDER, "empty");
    statusEl.textContent = "";
    focusInputSoon();
    refreshSettings();
  });

  // Auto-hide when the window loses focus (click elsewhere).
  appWindow.onFocusChanged(({ payload: focused }) => {
    if (focused) {
      focusedSinceSummon = true;
      return;
    }
    if (!focusedSinceSummon || Date.now() < suppressAutoHideUntil) return;
    hide();
  });

  setOutput(PLACEHOLDER, "empty");
  inputEl.focus();
  refreshSettings();
});
