/* Demo v1 capture (Tauri explore). Draft stays off the list until Send.
   Master key never crosses IPC — only note DTOs via invoke. */

const AUTOSAVE_MS = 400;

const el = {
  compose: document.getElementById("compose"),
  writeCard: document.getElementById("write-card"),
  readCard: document.getElementById("read-card"),
  readBody: document.getElementById("read-body"),
  list: document.getElementById("note-list"),
  home: document.getElementById("home"),
  shortcuts: document.getElementById("shortcuts"),
  btnSend: document.getElementById("btn-send"),
  btnNew: document.getElementById("btn-new"),
  btnHome: document.getElementById("btn-home"),
  btnShortcuts: document.getElementById("btn-shortcuts"),
  btnCloseShortcuts: document.getElementById("btn-close-shortcuts"),
};

/** @type {string | null} */
let draftId = null;
/** @type {string | null} */
let selectedId = null;
let saveTimer = null;
let dirty = false;
let modHeld = false;

function invoke(cmd, args = {}) {
  const core = window.__TAURI__?.core;
  if (!core?.invoke) {
    return Promise.reject(new Error("Tauri IPC unavailable (open via cargo run -p tinkerway-tauri)"));
  }
  return core.invoke(cmd, args);
}

function isMac() {
  return /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);
}

function formatTime(unix) {
  if (!unix) return "";
  try {
    return new Intl.DateTimeFormat(undefined, {
      month: "short",
      day: "numeric",
      hour: "numeric",
      minute: "2-digit",
    }).format(new Date(unix * 1000));
  } catch {
    return "";
  }
}

function showCompose() {
  selectedId = null;
  el.readCard.classList.add("hidden");
  el.writeCard.classList.remove("hidden");
  el.compose.focus();
  renderListActive();
}

function showRead(body) {
  el.writeCard.classList.add("hidden");
  el.readCard.classList.remove("hidden");
  el.readBody.textContent = body;
  el.compose.blur();
  renderListActive();
}

function openShortcuts() {
  el.shortcuts.classList.remove("hidden");
  el.home.classList.add("hidden");
}

function closeShortcuts() {
  el.shortcuts.classList.add("hidden");
  el.home.classList.remove("hidden");
  if (!selectedId) el.compose.focus();
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
    const title = document.createElement("div");
    title.className = "note-title";
    title.textContent = note.title || "Untitled";
    const time = document.createElement("div");
    time.className = "note-time";
    time.textContent = formatTime(note.updatedUnix);
    btn.append(title, time);
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

async function onSelectNote(id) {
  if (selectedId === id) {
    showCompose();
    return;
  }
  const note = await invoke("read_note", { id });
  selectedId = id;
  showRead(note.body);
}

async function flushDraft() {
  const body = el.compose.value;
  if (!body.trim()) return;
  if (draftId) {
    await invoke("update_note", { id: draftId, body });
  } else {
    draftId = await invoke("create_note", { body });
  }
  dirty = false;
}

function scheduleSave() {
  dirty = true;
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    flushDraft().catch((err) => console.error("autosave", err));
  }, AUTOSAVE_MS);
}

async function send() {
  const body = el.compose.value;
  if (!body.trim()) return;
  clearTimeout(saveTimer);
  await flushDraft();
  draftId = null;
  el.compose.value = "";
  dirty = false;
  showCompose();
  await refreshList();
}

async function newCapture() {
  clearTimeout(saveTimer);
  if (el.compose.value.trim()) {
    await flushDraft();
    draftId = null;
    el.compose.value = "";
    dirty = false;
    await refreshList();
  }
  showCompose();
}

function onComposeInput() {
  if (selectedId) return;
  scheduleSave();
}

function updateModLabels() {
  const glyph = isMac() ? "⌘" : "Ctrl";
  for (const k of document.querySelectorAll("kbd[data-mod]")) {
    k.textContent = glyph;
  }
}

function onKeyDown(e) {
  const mod = e.metaKey || e.ctrlKey;
  if (e.key === "Meta" || e.key === "Control") {
    modHeld = true;
  }

  if (e.key === "Escape") {
    if (!el.shortcuts.classList.contains("hidden")) {
      closeShortcuts();
      e.preventDefault();
      return;
    }
    if (document.activeElement === el.compose) {
      el.compose.blur();
      e.preventDefault();
    }
    return;
  }

  if (!el.shortcuts.classList.contains("hidden")) {
    return;
  }

  if (mod && e.key === "Enter") {
    e.preventDefault();
    send().catch(console.error);
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
    openShortcuts();
  }
}

function onKeyUp(e) {
  if (e.key === "Meta" || e.key === "Control") {
    modHeld = false;
  }
}

el.compose.addEventListener("input", onComposeInput);
el.btnSend.addEventListener("click", () => send().catch(console.error));
el.btnNew.addEventListener("click", () => newCapture().catch(console.error));
el.btnHome.addEventListener("click", () => {
  closeShortcuts();
  showCompose();
});
el.btnShortcuts.addEventListener("click", openShortcuts);
el.btnCloseShortcuts.addEventListener("click", closeShortcuts);
window.addEventListener("keydown", onKeyDown);
window.addEventListener("keyup", onKeyUp);

updateModLabels();
refreshList()
  .then(() => showCompose())
  .catch((err) => {
    console.error(err);
    el.compose.placeholder = "Vault unavailable — see apps/tinkerway-tauri/README.md";
  });

// silence unused in strict tooling if any
void modHeld;
