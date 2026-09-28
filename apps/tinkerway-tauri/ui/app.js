/* Demo v1 capture (Tauri). Draft stays off the list until Send.
   Master key never crosses IPC. */

const AUTOSAVE_MS = 400;

const COMPOSE_LINE = 30;
const COMPOSE_MAX = 240;

const el = {
  compose: document.getElementById("compose"),
  thread: document.getElementById("thread"),
  list: document.getElementById("note-list"),
  home: document.getElementById("home"),
  shortcuts: document.getElementById("shortcuts"),
  breadcrumb: document.getElementById("breadcrumb"),
  modal: document.getElementById("note-modal"),
  modalBody: document.getElementById("modal-body"),
  btnSend: document.getElementById("btn-send"),
  btnHome: document.getElementById("btn-home"),
  btnShortcuts: document.getElementById("btn-shortcuts"),
  btnCloseShortcuts: document.getElementById("btn-close-shortcuts"),
  btnCloseNote: document.getElementById("btn-close-note"),
  btnModalBackdrop: document.getElementById("btn-modal-backdrop"),
  sendChord: document.getElementById("send-chord"),
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
}

function setKeysVisible(show) {
  if (keysVisible === show) return;
  keysVisible = show;
  el.sendChord.classList.toggle("hidden", !show);
}

function noteModalOpen() {
  return !el.modal.classList.contains("hidden");
}

function resizeCompose() {
  const field = el.compose;
  field.style.height = "0px";
  const next = Math.max(COMPOSE_LINE, Math.min(field.scrollHeight, COMPOSE_MAX));
  field.style.height = `${next}px`;
  field.style.overflowY = field.scrollHeight > COMPOSE_MAX ? "auto" : "hidden";
}

function scrollThreadToLatest() {
  el.thread.scrollTop = el.thread.scrollHeight;
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

function closeNote() {
  selectedId = null;
  el.modal.classList.add("hidden");
  renderListActive();
  el.compose.focus();
}

function openNote(note) {
  selectedId = note.id;
  el.modalBody.textContent = note.body;
  el.modal.classList.remove("hidden");
  renderListActive();
  el.btnCloseNote.focus();
}

function goHome() {
  hideShortcuts();
  closeNote();
}

async function refreshList() {
  const notes = await invoke("list_notes");
  const visible = [];
  for (const note of notes) {
    if (draftId && note.id === draftId) continue;
    visible.push(note);
  }
  // Vault returns newest first. The thread reads downward, newest just above the field.
  visible.reverse();
  el.list.replaceChildren();
  for (const note of visible) {
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
  scrollThreadToLatest();
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
  hideShortcuts();
  openNote(note);
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
  selectedId = null;
  el.modal.classList.add("hidden");
  el.compose.value = "";
  resizeCompose();
  updateSendEnabled();
  el.compose.focus();
  await refreshList();
}

function onComposeInput() {
  resizeCompose();
  updateSendEnabled();
  scheduleSave();
}

function closeShortcutsOrBlur() {
  if (noteModalOpen()) {
    closeNote();
    return;
  }
  if (onShortcutsScreen()) {
    hideShortcuts();
    el.compose.focus();
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

  if (noteModalOpen() || onShortcutsScreen()) {
    return;
  }

  if (mod && e.key === "Enter") {
    e.preventDefault();
    send().catch(console.error);
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
el.btnHome.addEventListener("click", goHome);
el.btnShortcuts.addEventListener("click", showShortcuts);
el.btnCloseShortcuts.addEventListener("click", () => {
  hideShortcuts();
  el.compose.focus();
});
el.btnCloseNote.addEventListener("click", closeNote);
el.btnModalBackdrop.addEventListener("click", closeNote);
window.addEventListener("keydown", onKeyDown);
window.addEventListener("keyup", onKeyUp);
window.addEventListener("blur", onBlurWindow);

updateModLabels();
updateSendEnabled();
resizeCompose();
refreshList()
  .then(() => el.compose.focus())
  .catch((err) => {
    console.error(err);
    el.compose.focus();
  });
