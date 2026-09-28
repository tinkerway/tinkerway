/* Demo v1 capture (Tauri). Messaging-shaped feed; capture-only (no replies).
   Draft stays off the feed until Send. Master key never crosses IPC.
   Enter sends; Shift+Enter newline. */

const AUTOSAVE_MS = 400;
const MODAL_MS = 240;

const el = {
  compose: document.getElementById("compose"),
  feed: document.getElementById("feed"),
  feedScroll: document.getElementById("feed-scroll"),
  btnSend: document.getElementById("btn-send"),
  btnHome: document.getElementById("btn-home"),
  modal: document.getElementById("modal"),
  modalBackdrop: document.getElementById("modal-backdrop"),
  modalBody: document.getElementById("modal-body"),
  btnModalClose: document.getElementById("btn-modal-close"),
};

/** @type {string | null} */
let draftId = null;
let saveTimer = null;
let draftDirty = false;
/** @type {string | null} */
let animateId = null;
/** @type {Map<string, string>} */
const bodyCache = new Map();
let modalClosing = false;

function invoke(cmd, args = {}) {
  const core = window.__TAURI__?.core;
  if (!core?.invoke) {
    return Promise.reject(new Error("Tauri IPC unavailable"));
  }
  return core.invoke(cmd, args);
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

function updateSendEnabled() {
  el.btnSend.disabled = el.compose.value.trim().length === 0;
}

function resizeCompose() {
  const node = el.compose;
  node.style.height = "auto";
  const maxPx = parseFloat(getComputedStyle(node).maxHeight) || 152;
  node.style.height = `${Math.min(node.scrollHeight, maxPx)}px`;
}

function isModalOpen() {
  return el.modal.classList.contains("is-open") && !el.modal.classList.contains("hidden");
}

function openModal(body) {
  modalClosing = false;
  el.modalBody.textContent = body || "";
  el.modal.classList.remove("hidden", "is-closing");
  el.modal.removeAttribute("hidden");
  // Next frame so CSS transitions run from the closed state.
  requestAnimationFrame(() => {
    el.modal.classList.add("is-open");
  });
  el.btnModalClose.focus();
}

function closeModal() {
  if (!isModalOpen() || modalClosing) return;
  modalClosing = true;
  el.modal.classList.add("is-closing");
  el.modal.classList.remove("is-open");
  window.setTimeout(() => {
    el.modal.classList.add("hidden");
    el.modal.classList.remove("is-closing");
    el.modal.setAttribute("hidden", "");
    el.modalBody.textContent = "";
    modalClosing = false;
    el.compose.focus();
  }, MODAL_MS);
}

function scrollFeedToEnd() {
  el.feedScroll.scrollTop = el.feedScroll.scrollHeight;
}

function makeRow(note, { animate } = {}) {
  const li = document.createElement("li");
  const btn = document.createElement("button");
  btn.type = "button";
  btn.className = "feed-row" + (animate ? " is-entering" : "");
  btn.dataset.id = note.id;
  btn.setAttribute("aria-label", "Open capture");

  const preview = document.createElement("p");
  preview.className = "feed-preview";
  preview.textContent = bodyCache.get(note.id) || note.title || "";

  const meta = document.createElement("div");
  meta.className = "feed-meta";
  meta.textContent = whenLabel(note.updatedUnix);

  btn.append(preview, meta);
  btn.addEventListener("click", () => onOpenNote(note.id));
  if (animate) {
    btn.addEventListener(
      "animationend",
      () => btn.classList.remove("is-entering"),
      { once: true },
    );
  }
  li.append(btn);
  return li;
}

async function ensureBody(id) {
  if (bodyCache.has(id)) return bodyCache.get(id);
  const note = await invoke("read_note", { id });
  bodyCache.set(id, note.body || "");
  return note.body || "";
}

async function refreshFeed() {
  const notes = await invoke("list_notes");
  // Vault returns newest-first; reverse so oldest→newest and newest sits above compose.
  const visible = notes.filter((n) => !(draftId && n.id === draftId)).reverse();

  await Promise.all(
    visible.map(async (n) => {
      if (!bodyCache.has(n.id)) {
        try {
          await ensureBody(n.id);
        } catch (err) {
          console.error("read preview", err);
          bodyCache.set(n.id, n.title || "");
        }
      }
    }),
  );

  const entering = animateId;
  animateId = null;
  el.feed.replaceChildren();
  for (const note of visible) {
    el.feed.append(makeRow(note, { animate: note.id === entering }));
  }
  scrollFeedToEnd();
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

async function onOpenNote(id) {
  try {
    const body = await ensureBody(id);
    openModal(body);
  } catch (err) {
    console.error(err);
  }
}

function pulseSend() {
  el.btnSend.classList.remove("is-sending");
  // Retrigger CSS animation.
  void el.btnSend.offsetWidth;
  el.btnSend.classList.add("is-sending");
  window.setTimeout(() => el.btnSend.classList.remove("is-sending"), 340);
}

async function send() {
  const body = el.compose.value;
  if (!body.trim()) return;
  pulseSend();
  clearTimeout(saveTimer);
  let id = draftId;
  if (id) {
    await invoke("update_note", { id, body });
  } else {
    id = await invoke("create_note", { body });
  }
  bodyCache.set(id, body);
  animateId = id;
  draftId = null;
  draftDirty = false;
  el.compose.value = "";
  resizeCompose();
  updateSendEnabled();
  await refreshFeed();
  el.compose.focus();
}

function onComposeInput() {
  updateSendEnabled();
  resizeCompose();
  scheduleSave();
}

function onComposeKeyDown(e) {
  if (e.key !== "Enter") return;
  if (e.shiftKey) return;
  if (e.isComposing || e.keyCode === 229) return;
  e.preventDefault();
  if (!el.btnSend.disabled) send().catch(console.error);
}

function onKeyDown(e) {
  if (e.key === "Escape") {
    if (isModalOpen()) {
      e.preventDefault();
      closeModal();
      return;
    }
    if (document.activeElement === el.compose) {
      e.preventDefault();
      el.compose.blur();
    }
  }
}

function goHome() {
  if (isModalOpen()) closeModal();
  else el.compose.focus();
}

el.compose.addEventListener("input", onComposeInput);
el.compose.addEventListener("keydown", onComposeKeyDown);
el.btnSend.addEventListener("click", () => {
  if (!el.btnSend.disabled) send().catch(console.error);
});
el.btnHome.addEventListener("click", goHome);
el.btnModalClose.addEventListener("click", closeModal);
el.modalBackdrop.addEventListener("click", closeModal);
window.addEventListener("keydown", onKeyDown);

updateSendEnabled();
resizeCompose();
refreshFeed()
  .then(() => el.compose.focus())
  .catch((err) => {
    console.error(err);
    el.compose.focus();
  });
