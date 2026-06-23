const { invoke } = window.__TAURI__.core;
const { getCurrentWindow } = window.__TAURI__.window;

// value = English name used in the prompt; label = friendly display.
const LANGS = [
  ["English", "English"],
  ["Chinese", "中文"],
  ["Japanese", "日本語"],
  ["Korean", "한국어"],
  ["French", "Français"],
  ["German", "Deutsch"],
  ["Spanish", "Español"],
  ["Russian", "Русский"],
  ["Italian", "Italiano"],
  ["Portuguese", "Português"],
  ["Arabic", "العربية"],
];

let els = {};

function fillSelect(sel, includeAuto) {
  const opts = [];
  if (includeAuto) opts.push(["auto", "自动检测"]);
  for (const [v, l] of LANGS) opts.push([v, l]);
  sel.innerHTML = opts
    .map(([v, l]) => `<option value="${v}">${l}</option>`)
    .join("");
}

// Build a Tauri accelerator string (e.g. "CommandOrControl+Shift+T") from a keydown event.
function accelFromEvent(e) {
  const mods = [];
  if (e.metaKey) mods.push("CommandOrControl");
  if (e.ctrlKey) mods.push("Control");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");

  const code = e.code;
  let key = null;
  if (/^Key[A-Z]$/.test(code)) key = code.slice(3);
  else if (/^Digit[0-9]$/.test(code)) key = code.slice(5);
  else if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) key = code;
  else if (code === "Space") key = "Space";
  else if (code === "ArrowUp") key = "Up";
  else if (code === "ArrowDown") key = "Down";
  else if (code === "ArrowLeft") key = "Left";
  else if (code === "ArrowRight") key = "Right";
  else if (code === "Comma") key = ",";
  else if (code === "Period") key = ".";
  else if (code === "Slash") key = "/";
  else if (code === "Backquote") key = "`";

  if (!key || mods.length === 0) return null; // need a modifier + a real key
  return [...mods, key].join("+");
}

// Pretty display of an accelerator for macOS.
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

function flash(text, isError) {
  els.msg.textContent = text;
  els.msg.className = "msg" + (isError ? " error" : " ok");
}

async function load() {
  const s = await invoke("get_settings");
  els.api_key.value = s.api_key || "";
  els.base_url.value = s.base_url || "";
  els.source_lang.value = s.source_lang || "auto";
  els.target_lang.value = s.target_lang || "English";
  els.hotkey.dataset.accel = s.hotkey || "";
  els.hotkey.value = prettyAccel(s.hotkey);
}

async function save() {
  const settings = {
    api_key: els.api_key.value.trim(),
    base_url: (els.base_url.value.trim() || "https://api.deepseek.com/v1"),
    source_lang: els.source_lang.value,
    target_lang: els.target_lang.value,
    hotkey: els.hotkey.dataset.accel || "CommandOrControl+Shift+T",
  };
  try {
    await invoke("save_settings", { settings });
    flash("已保存 ✓", false);
    // Briefly show the confirmation, then close the settings window.
    setTimeout(() => getCurrentWindow().hide(), 600);
  } catch (err) {
    flash(String(err), true);
  }
}

window.addEventListener("DOMContentLoaded", () => {
  els = {
    api_key: document.querySelector("#api_key"),
    base_url: document.querySelector("#base_url"),
    hotkey: document.querySelector("#hotkey"),
    source_lang: document.querySelector("#source_lang"),
    target_lang: document.querySelector("#target_lang"),
    save: document.querySelector("#save"),
    msg: document.querySelector("#msg"),
  };

  fillSelect(els.source_lang, true);
  fillSelect(els.target_lang, false);

  els.hotkey.addEventListener("keydown", (e) => {
    e.preventDefault();
    if (e.key === "Escape") {
      els.hotkey.blur();
      return;
    }
    const accel = accelFromEvent(e);
    if (accel) {
      els.hotkey.dataset.accel = accel;
      els.hotkey.value = prettyAccel(accel);
      flash("", false);
    }
  });

  els.save.addEventListener("click", save);
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && document.activeElement !== els.hotkey) {
      getCurrentWindow().hide();
    }
  });

  load();
});
