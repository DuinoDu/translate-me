const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { getCurrentWindow } = window.__TAURI__.window;

const appWindow = getCurrentWindow();

let inputEl;
let outputEl;
let statusEl;
let debounceTimer = null;
let reqSeq = 0; // guards against out-of-order responses

function setOutput(text, cls) {
  outputEl.className = "output" + (cls ? " " + cls : "");
  outputEl.textContent = text;
}

async function runTranslate(text) {
  const value = text.trim();
  if (!value) {
    setOutput("译文将实时显示在这里", "empty");
    statusEl.textContent = "";
    return;
  }
  const myReq = ++reqSeq;
  statusEl.textContent = "翻译中…";
  try {
    const result = await invoke("translate", { text: value });
    if (myReq !== reqSeq) return; // a newer keystroke superseded this one
    statusEl.textContent = "";
    if (result) {
      setOutput(result, "");
    } else {
      setOutput("译文将实时显示在这里", "empty");
    }
  } catch (err) {
    if (myReq !== reqSeq) return;
    statusEl.textContent = "";
    setOutput(String(err), "error");
  }
}

function onInput() {
  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => runTranslate(inputEl.value), 350);
}

async function hide() {
  if (debounceTimer) clearTimeout(debounceTimer);
  reqSeq++; // cancel any in-flight render
  try {
    await invoke("hide_window");
  } catch (_) {}
}

window.addEventListener("DOMContentLoaded", () => {
  inputEl = document.querySelector("#input");
  outputEl = document.querySelector("#output");
  statusEl = document.querySelector("#status");

  inputEl.addEventListener("input", onInput);

  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      e.preventDefault();
      hide();
    }
  });

  // The hotkey handler in Rust emits "summon" each time the window is shown.
  listen("summon", () => {
    inputEl.value = "";
    setOutput("译文将实时显示在这里", "empty");
    statusEl.textContent = "";
    inputEl.focus();
    inputEl.select();
  });

  // Auto-hide when the window loses focus (click elsewhere).
  appWindow.onFocusChanged(({ payload: focused }) => {
    if (!focused) hide();
  });

  inputEl.focus();
});
