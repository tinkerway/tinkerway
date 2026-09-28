const MODAL_MS = 240;

const el = {
  compose: document.getElementById("compose"),
  feed: document.getElementById("feed"),
  feedScroll: document.getElementById("feed-scroll"),
  btnSend: document.getElementById("btn-send"),
  btnHome: document.getElementById("btn-home"),
  modal: document.getElementById("modal"),
  modalBackdrop: document.getElementById("modal-backdrop"),
  modalTitle: document.getElementById("modal-title"),
  modalBody: document.getElementById("modal-body"),
  btnModalClose: document.getElementById("btn-modal-close"),
};

/** @type {string | null} */
let animateId = null;
let modalClosing = false;

function titleFromBody(body) {
  const firstLine = String(body || "")
    .split(/\r?\n/)[0]
    .trim();
  if (!firstLine) return "Untitled";
  const words = firstLine.split(/\s+/).filter(Boolean);
  let title = words.slice(0, 8).join(" ");
  if ([...title].length > 60) {
    title = [...title].slice(0, 57).join("") + "…";
  }
  if (words.length > 8) title += "…";
  return title;
}

const mockVault = (() => {
  /** @type {{ id: string, title: string, body: string, updatedUnix: number }[]} */
  let notes = [];
  let seq = 0;
  return {
    async list_notes() {
      return [...notes].sort((a, b) => b.updatedUnix - a.updatedUnix);
    },
    async read_note({ id }) {
      const n = notes.find((x) => x.id === id);
      if (!n) throw new Error("not found");
      return { ...n };
    },
    async create_note({ body }) {
      const id = `mock-${++seq}`;
      notes.unshift({
        id,
        title: titleFromBody(body),
        body,
        updatedUnix: Math.floor(Date.now() / 1000),
      });
      return id;
    },
    async update_note({ id, body }) {
      const n = notes.find((x) => x.id === id);
      if (!n) throw new Error("not found");
      n.body = body;
      n.title = titleFromBody(body);
      n.updatedUnix = Math.floor(Date.now() / 1000);
    },
  };
})();

function invoke(cmd, args = {}) {
  const core = window.__TAURI__?.core;
  if (core?.invoke) return core.invoke(cmd, args);
  const fn = mockVault[cmd];
  if (!fn) return Promise.reject(new Error(`unknown command: ${cmd}`));
  return fn(args);
}

function whenLabel(updatedUnix) {
  const now = Math.floor(Date.now() / 1000);
  const age = Math.max(0, now - (updatedUnix || now));
  if (age < 120) return "Now";
  if (age < 18 * 3600) return "Today";
  if (age < 42 * 3600) return "Yesterday";
  // Unix day 0 was a Thursday.
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

function chronologicalForFeed(notes) {
  return [...notes].sort((a, b) => {
    if (a.updatedUnix !== b.updatedUnix) return a.updatedUnix - b.updatedUnix;
    return String(a.id).localeCompare(String(b.id));
  });
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

function openModal(body, updatedUnix) {
  modalClosing = false;
  el.modalTitle.textContent = whenLabel(updatedUnix);
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
    el.modalTitle.textContent = "";
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
  preview.textContent = note.body || note.title || "";

  const meta = document.createElement("div");
  meta.className = "feed-meta";
  meta.textContent = whenLabel(note.updatedUnix);

  btn.append(preview, meta);
  btn.addEventListener("click", () => onOpenNote(note));
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

async function refreshFeed() {
  const notes = chronologicalForFeed(await invoke("list_notes"));
  const entering = animateId;
  animateId = null;
  el.feed.replaceChildren();
  for (const note of notes) {
    el.feed.append(makeRow(note, { animate: note.id === entering }));
  }
  scrollFeedToEnd();
}

function onOpenNote(note) {
  openModal(note.body || note.title || "", note.updatedUnix);
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
  const id = await invoke("create_note", { body });
  animateId = id;
  el.compose.value = "";
  resizeCompose();
  updateSendEnabled();
  await refreshFeed();
  el.compose.focus();
}

function onComposeInput() {
  updateSendEnabled();
  resizeCompose();
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
