/* Demo v1 capture (Tauri explore) — UI/behavior parity with GPUI app/src/home.rs.
   Draft stays off the list until Send. Master key never crosses IPC. */

const AUTOSAVE_MS = 400;

const el = {
  compose: document.getElementById("compose"),
  writeCard: document.getElementById("write-card"),
  readCard: document.getElementById("read-card"),
  readBody: document.getElementById("read-body"),
  list: document.getElementById("note-list"),
  home: document.getElementById("home"),
  shortcuts: document.getElementById("shortcuts"),
  breadcrumb: document.getElementById("breadcrumb"),
  btnSend: document.getElementById("btn-send"),
  btnNew: document.getElementById("btn-new"),
  btnHome: document.getElementById("btn-home"),
  btnShortcuts: document.getElementById("btn-shortcuts"),
  btnCloseShortcuts: document.getElementById("btn-close-shortcuts"),
  sendChord: document.getElementById("send-chord"),
  newChord: document.getElementById("new-chord"),
};

/** @type {string | null} */
let draftId = null;
/** @type {string | null} */
let selectedId = null;
let saveTimer = null;
let draftDirty = false;
let keysVisible = false;

function invoke(cmd, args = {}) {
  const core = window.__TAURI__?.core;
  if (!core?.invoke) {
    return Promise.reject(new Error("Tauri IPC unavailable"));
  }
  return core.invoke(cmd, args);
}

function isMac() {
  return /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);
}

function modName() {
  return isMac() ? "Cmd" : "Ctrl";
}

function whenLabel(updatedUnix) {
  const now = Math.floor(Date.now() / 1000);
  const age = Math.max(0, now - (updatedUnix || now));
  if (age < 120) return "Now";
  if (age < 18 * 3600) return "Today";
  if (age < 42 * 3600) return "Yesterday";
  const days = [
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
  ];
  return days[Math.floor((updatedUnix || 0) / 86400) % 7];
}

function updateModLabels() {
  const name = modName();
  for (const node of document.querySelectorAll("[data-mod-name]")) {
    node.textContent = name;
  }
  el.sendChord.textContent = `${name} + Enter`;
  el.newChord.textContent = `${name} + N`;
}

function setKeysVisible(show) {
  if (keysVisible === show) return;
  keysVisible = show;
  el.sendChord.classList.toggle("hidden", !show);
  el.newChord.classList.toggle("hidden", !show);
}

function updateSendEnabled() {
  const can = el.compose.value.trim().length > 0;
  el.btnSend.disabled = !can;
  el.btnSend.classList.toggle("is-dim", !can);
}

function onShortcutsScreen() {
  return !el.shortcuts.classList.contains("hidden");
}

function showShortcuts() {
  el.shortcuts.classList.remove("hidden");
  el.home.classList.add("hidden");
  el.breadcrumb.classList.remove("hidden");
  el.btnCloseShortcuts.classList.remove("hidden");
}

function hideShortcuts() {
  el.shortcuts.classList.add("hidden");
  el.home.classList.remove("hidden");
  el.breadcrumb.classList.add("hidden");
  el.btnCloseShortcuts.classList.add("hidden");
}

function showCompose() {
  selectedId = null;
  el.readCard.classList.add("hidden");
  el.writeCard.classList.remove("hidden");
  el.compose.focus();
  updateSendEnabled();
  renderListActive();
}

function showRead(body) {
  el.writeCard.classList.add("hidden");
  el.readCard.classList.remove("hidden");
  el.readBody.textContent = body;
  el.compose.blur();
  renderListActive();
}

function goHome() {
  hideShortcuts();
  showCompose();
}

async function refreshList() {
  const notes = await invoke("list_notes");
  el.list.replaceChildren();
  for (const note of notes) {
    if (draftId && note.id === draftId) continue;
    const li = document.createElement("li");
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "note-row" + (selectedId === note.id ? " active" : "");
    btn.dataset.id = note.id;

    const mark = document.createElement("span");
    mark.className = "note-mark";
    mark.setAttribute("aria-hidden", "true");

    const title = document.createElement("span");
    title.className = "note-title";
    title.textContent = note.title || "Untitled";

    const when = document.createElement("span");
    when.className = "note-when";
    when.textContent = whenLabel(note.updatedUnix);

    btn.append(mark, title, when);
    btn.addEventListener("click", () => onSelectNote(note.id));
    li.append(btn);
    el.list.append(li);
  }
}

function renderListActive() {
  for (const row of el.list.querySelectorAll(".note-row")) {
    row.classList.toggle("active", row.dataset.id === selectedId);
  }
}

async function flushDraft() {
  if (!draftDirty) return;
  const body = el.compose.value;
  if (!body.trim()) {
    draftDirty = false;
    return;
  }
  if (draftId) {
    await invoke("update_note", { id: draftId, body });
  } else {
    draftId = await invoke("create_note", { body });
  }
  draftDirty = false;
}

function scheduleSave() {
  draftDirty = true;
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    flushDraft().catch((err) => console.error("autosave", err));
  }, AUTOSAVE_MS);
}

async function onSelectNote(id) {
  if (selectedId === id) {
    goHome();
    return;
  }
  clearTimeout(saveTimer);
  await flushDraft();
  const note = await invoke("read_note", { id });
  selectedId = id;
  hideShortcuts();
  showRead(note.body);
}

async function send() {
  const body = el.compose.value;
  if (!body.trim()) return;
  clearTimeout(saveTimer);
  if (draftId) {
    await invoke("update_note", { id: draftId, body });
  } else {
    await invoke("create_note", { body });
  }
  draftId = null;
  draftDirty = false;
  el.compose.value = "";
  updateSendEnabled();
  showCompose();
  await refreshList();
}

async function newCapture() {
  clearTimeout(saveTimer);
  // Match GPUI start_new: save compose content if any, else leave open capture / shortcuts.
  if (el.compose.value.trim()) {
    await send();
    return;
  }
  goHome();
}

function onComposeInput() {
  if (selectedId) return;
  updateSendEnabled();
  scheduleSave();
}

function closeShortcutsOrBlur() {
  if (onShortcutsScreen()) {
    hideShortcuts();
    if (!selectedId) el.compose.focus();
    return;
  }
  if (document.activeElement === el.compose) {
    el.compose.blur();
  }
}

function onKeyDown(e) {
  const mod = e.metaKey || e.ctrlKey;
  if (e.key === "Meta" || e.key === "Control") {
    setKeysVisible(true);
  }

  if (e.key === "Escape") {
    closeShortcutsOrBlur();
    e.preventDefault();
    return;
  }

  if (onShortcutsScreen()) {
    return;
  }

  if (mod && e.key === "Enter") {
    e.preventDefault();
    if (!selectedId) send().catch(console.error);
    return;
  }

  if (mod && (e.key === "n" || e.key === "N")) {
    e.preventDefault();
    newCapture().catch(console.error);
    return;
  }

  const inField = document.activeElement === el.compose;
  if (!inField && (e.key === "?" || (e.key === "/" && e.shiftKey))) {
    e.preventDefault();
    showShortcuts();
  }
}

function onKeyUp(e) {
  if (e.key === "Meta" || e.key === "Control") {
    // Secondary still held (Cmd+Ctrl edge) — only hide when neither remains.
    if (!e.metaKey && !e.ctrlKey) setKeysVisible(false);
  }
}

function onBlurWindow() {
  setKeysVisible(false);
}

el.compose.addEventListener("input", onComposeInput);
el.btnSend.addEventListener("click", () => {
  if (!el.btnSend.disabled) send().catch(console.error);
});
el.btnNew.addEventListener("click", () => newCapture().catch(console.error));
el.btnHome.addEventListener("click", goHome);
el.btnShortcuts.addEventListener("click", showShortcuts);
el.btnCloseShortcuts.addEventListener("click", () => {
  hideShortcuts();
  if (!selectedId) el.compose.focus();
});
window.addEventListener("keydown", onKeyDown);
window.addEventListener("keyup", onKeyUp);
window.addEventListener("blur", onBlurWindow);

updateModLabels();
updateSendEnabled();
refreshList()
  .then(() => showCompose())
  .catch((err) => {
    console.error(err);
    showCompose();
  });
