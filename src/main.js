const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { getCurrentWindow, LogicalSize } = window.__TAURI__.window;

const appWindow = getCurrentWindow();

const MIN_W = 360;
const MAX_H = 420; // cap; beyond this the output area scrolls

const PLACEHOLDER = "译文…";

// Compact labels for the direction chip (keys match settings.js LANGS values).
const LANG_LABEL = {
  auto: "自动",
  English: "EN",
  Chinese: "中文",
  Japanese: "日本語",
  Korean: "한국어",
  French: "FR",
  German: "DE",
  Spanish: "ES",
  Russian: "RU",
  Italian: "IT",
  Portuguese: "PT",
  Arabic: "AR",
};
const langLabel = (code) => LANG_LABEL[code] || code || "自动";

let inputEl;
let outputEl;
let statusEl;
let copyBtn;
let providerEl;
let langChipEl;
let resultPanelEl;
let debounceTimer = null;
let reqSeq = 0; // guards against out-of-order responses
let statusTimer = null;
let focusedSinceSummon = false;
let suppressAutoHideUntil = 0;

function providerBadgeText(settings) {
  const provider = settings && settings.provider;
  const model = (settings && settings.model) || "";
  if (provider === "anthropic") {
    if (model.toLowerCase().startsWith("mimo")) return "Mimo";
    return "Anthropic";
  }
  if (model.toLowerCase().includes("deepseek")) return "DeepSeek";
  return "OpenAI";
}

function flashStatus(text, state, ttl = 900) {
  clearTimeout(statusTimer);
  statusEl.textContent = text;
  statusEl.dataset.state = state || "";
  if (!text) return;
  statusTimer = setTimeout(() => {
    statusEl.textContent = "";
    statusEl.dataset.state = "";
  }, ttl);
}

// Grow the textarea to fit its text, then resize the window to fit the card.
function autosize() {
  inputEl.style.height = "auto";
  inputEl.style.height = inputEl.scrollHeight + "px";
  const wantH = Math.min(MAX_H, Math.ceil(document.documentElement.scrollHeight));
  const wantW = Math.max(MIN_W, Math.ceil(document.documentElement.clientWidth || window.innerWidth || MIN_W));
  appWindow.setSize(new LogicalSize(wantW, wantH)).catch(() => {});
}

function setOutput(text, cls) {
  outputEl.className = "output" + (cls ? " " + cls : "");
  outputEl.textContent = text;
  if (resultPanelEl) {
    resultPanelEl.classList.toggle("is-empty", cls === "empty");
    resultPanelEl.classList.toggle("is-error", cls === "error");
  }
  outputEl.classList.remove("revealed");
  requestAnimationFrame(() => {
    requestAnimationFrame(() => outputEl.classList.add("revealed"));
  });
  // Copy only makes sense for a real translation.
  const hasTranslation = cls === "";
  copyBtn.disabled = !hasTranslation;
  autosize();
}

async function applySettings(s) {
  const fs = (s && s.font_size) || 15;
  document.documentElement.style.setProperty("--fs", fs + "px");
  if (providerEl) providerEl.textContent = providerBadgeText(s);
  if (langChipEl) {
    const src = (s && s.source_lang) || "auto";
    const tgt = (s && s.target_lang) || "English";
    langChipEl.textContent = `${langLabel(src)} → ${langLabel(tgt)}`;
  }
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
    flashStatus("", "");
    return;
  }
  const myReq = ++reqSeq;
  clearTimeout(statusTimer);
  statusEl.textContent = "翻译中";
  statusEl.dataset.state = "busy";
  try {
    const result = await invoke("translate", { text: value });
    if (myReq !== reqSeq) return; // a newer keystroke superseded this one
    setOutput(result || PLACEHOLDER, result ? "" : "empty");
    flashStatus("", "");
  } catch (err) {
    if (myReq !== reqSeq) return;
    setOutput(String(err), "error");
    flashStatus("未完成", "error", 1400);
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

function bindWindowGestures() {
  const dragHandle = document.querySelector("#drag-handle");
  const resizeHandle = document.querySelector("#resize");

  dragHandle.addEventListener("mousedown", (e) => {
    if (e.button !== 0 || e.target.closest("button")) return;
    appWindow.startDragging().catch(() => {});
  });

  resizeHandle.addEventListener("mousedown", (e) => {
    if (e.button !== 0) return;
    e.preventDefault();
    appWindow.startResizeDragging("SouthEast").catch(() => {});
  });
}

window.addEventListener("DOMContentLoaded", () => {
  inputEl = document.querySelector("#input");
  outputEl = document.querySelector("#output");
  statusEl = document.querySelector("#status");
  copyBtn = document.querySelector("#copy");
  providerEl = document.querySelector("#provider");
  langChipEl = document.querySelector("#lang-chip");
  resultPanelEl = document.querySelector(".result-panel");

  inputEl.addEventListener("input", onInput);
  copyBtn.addEventListener("click", copyTranslation);
  document.querySelector("#gear").addEventListener("click", () => {
    invoke("open_settings");
  });
  bindWindowGestures();

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
