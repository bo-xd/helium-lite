"use strict";

const DEFAULT_SHORTCUTS = [
  { title: "YouTube", url: "https://www.youtube.com" },
  { title: "GitHub", url: "https://github.com" },
  { title: "ArchWiki", url: "https://wiki.archlinux.org" },
  { title: "DuckDuckGo", url: "https://duckduckgo.com" },
  { title: "Proton", url: "https://proton.me" },
  { title: "Wikipedia", url: "https://www.wikipedia.org" }
];

const STORAGE_DEFAULTS = {
  shortcuts: DEFAULT_SHORTCUTS,
  showShortcuts: true,
  appearanceMode: "matte",
  activeSwatch: "kestrel",
  customBg: ""
};

// Mirrors the shortcut list in localStorage so tiles render on first paint.
const LIST_CACHE_KEY = "kestrel-ntp-shortcuts";
const MAX_BG_EDGE = 2560;
const SVG_NS = "http://www.w3.org/2000/svg";

// Zero-network glyphs for common sites; color null follows the theme ink.
const DOMAIN_ICONS = {
  "youtube.com": {
    color: "#ff4e45",
    path: "M21.6 7.2a2.7 2.7 0 0 0-1.9-1.9C18 5 12 5 12 5s-6 0-7.7.3a2.7 2.7 0 0 0-1.9 1.9C2 9 2 12 2 12s0 3 .4 4.8a2.7 2.7 0 0 0 1.9 1.9C6 19 12 19 12 19s6 0 7.7-.3a2.7 2.7 0 0 0 1.9-1.9C22 15 22 12 22 12s0-3-.4-4.8ZM10 15V9l5.2 3-5.2 3Z"
  },
  "github.com": {
    color: null,
    path: "M12 2a10 10 0 0 0-3.16 19.49c.5.09.68-.22.68-.48v-1.7c-2.78.6-3.37-1.34-3.37-1.34-.45-1.15-1.11-1.46-1.11-1.46-.91-.62.07-.6.07-.6 1 .07 1.53 1.03 1.53 1.03.89 1.52 2.34 1.08 2.91.83.09-.65.35-1.09.63-1.34-2.22-.25-4.56-1.11-4.56-4.94 0-1.09.39-1.98 1.03-2.68-.1-.25-.45-1.27.1-2.64 0 0 .84-.27 2.75 1.02a9.58 9.58 0 0 1 5 0c1.91-1.29 2.75-1.02 2.75-1.02.55 1.37.2 2.39.1 2.64.64.7 1.03 1.59 1.03 2.68 0 3.84-2.34 4.68-4.57 4.93.36.31.68.92.68 1.85v2.75c0 .27.18.58.69.48A10 10 0 0 0 12 2Z"
  },
  "archlinux.org": {
    color: "#1793d1",
    path: "M12 2 9.8 7.4c.7.7 1.5 1.5 2.8 2.4-1.4-.6-2.4-1.2-3.1-1.8L4 21.5c2.4-1.4 4.3-2.3 6.1-2.6-.1-.4-.1-.9-.1-1.3 0-1.8 1-3.2 2.1-3.2s2 1.4 2 3.2c0 .4 0 .9-.1 1.3 1.8.3 3.7 1.2 6.1 2.6l-1.3-3.2c-.9-.7-1.9-1.6-3.7-2.6 1.3.3 2.3.8 3.1 1.3L12 2Z"
  },
  "duckduckgo.com": {
    color: "#de5833",
    path: "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20Zm1.2 5.4c1.6 0 2.7 1.2 2.7 2.8 0 .6-.2 1.1-.4 1.6l2.3.9-2.5 1.4c-.3 1.8-1.4 3.3-3.1 3.3-1.9 0-3.4-1.6-3.4-4.2 0-3.4 1.9-5.8 4.4-5.8Z"
  },
  "proton.me": {
    color: "#8b6ef6",
    path: "M4 6.5A2.5 2.5 0 0 1 6.5 4h11A2.5 2.5 0 0 1 20 6.5v11a2.5 2.5 0 0 1-2.5 2.5h-11A2.5 2.5 0 0 1 4 17.5v-11Zm2.5.8 5.5 4.4 5.5-4.4v-.8l-5.5 4.3L6.5 6.5v.8Z"
  },
  "wikipedia.org": {
    color: null,
    path: "M3 6h3.5v1.2h-.9l2.8 7.8 2.2-5.6-1.1-2.2h-.8V6h3.8v1.2h-.8l1.8 4.4 2.1-4.4h-.9V6H18v1.2h-.8L14 14.3l1.2 2.7 3.2-9.8h-.9V6H21v1.2h-.9L15.4 19h-1.3l-2-4.6L10 19H8.7L4 7.2H3V6Z"
  }
};

const ICON_PATHS = {
  edit: ["M12 20h9", "M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4Z"],
  remove: ["M4 7h16", "M10 11v6", "M14 11v6", "M6 7l1 12a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2l1-12", "M9 7V4h6v3"]
};

const Theme = window.KestrelTheme;
const store = typeof chrome !== "undefined" && chrome.storage && chrome.storage.local
  ? chrome.storage.local
  : null;

const $ = (id) => document.getElementById(id);
const ui = {
  tiles: $("tilesGrid"),
  empty: $("emptyState"),
  emptyAdd: $("emptyAdd"),
  toggle: $("editToggle"),
  panel: $("editorPanel"),
  close: $("closePanel"),
  modes: Array.from(document.querySelectorAll(".seg[data-mode]")),
  swatches: $("swatches"),
  bgPick: $("bgPick"),
  bgPickLabel: $("bgPickLabel"),
  bgInput: $("bgInput"),
  bgRemove: $("bgRemove"),
  bgMessage: $("bgMessage"),
  showShortcuts: $("showShortcuts"),
  manager: $("shortcutManager"),
  list: $("shortcutList"),
  addTrigger: $("addTrigger"),
  form: $("shortcutForm"),
  name: $("scName"),
  url: $("scUrl"),
  urlError: $("scUrlError"),
  save: $("scSave"),
  cancel: $("scCancel"),
  settings: $("openSettings")
};

const isShortcut = (item) => item && typeof item.title === "string" && typeof item.url === "string";

function readListCache() {
  try {
    const data = JSON.parse(localStorage.getItem(LIST_CACHE_KEY));
    return data && Array.isArray(data.shortcuts) ? data : null;
  } catch {
    return null;
  }
}

function writeListCache() {
  try {
    localStorage.setItem(
      LIST_CACHE_KEY,
      JSON.stringify({ shortcuts: state.shortcuts, showShortcuts: state.showShortcuts })
    );
  } catch {
    // Cache only; chrome.storage remains the source of truth.
  }
}

const cachedTheme = Theme.cached();
const cachedList = readListCache();
const state = {
  shortcuts: cachedList ? cachedList.shortcuts.filter(isShortcut) : DEFAULT_SHORTCUTS.slice(),
  showShortcuts: cachedList ? cachedList.showShortcuts !== false : true,
  mode: cachedTheme.mode || "matte",
  swatch: cachedTheme.swatch || "kestrel",
  customBg: "",
  editIndex: -1
};

function persist(patch) {
  if ("shortcuts" in patch || "showShortcuts" in patch) writeListCache();
  if ("appearanceMode" in patch || "activeSwatch" in patch) Theme.cache(state.mode, state.swatch);
  return store ? store.set(patch) : Promise.resolve();
}

function hostOf(url) {
  try {
    return new URL(url).hostname.toLowerCase().replace(/^www\./, "");
  } catch {
    return "";
  }
}

function toWebUrl(raw) {
  const text = raw.trim();
  if (!text) return null;
  const candidate = /^[a-z][a-z0-9+.-]*:\/\//i.test(text) ? text : `https://${text}`;
  try {
    const url = new URL(candidate);
    if (url.protocol !== "http:" && url.protocol !== "https:") return null;
    if (!url.hostname.includes(".") && url.hostname !== "localhost") return null;
    return url.href;
  } catch {
    return null;
  }
}

function strokeIcon(paths, size) {
  const svg = document.createElementNS(SVG_NS, "svg");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("width", String(size));
  svg.setAttribute("height", String(size));
  svg.setAttribute("fill", "none");
  svg.setAttribute("stroke", "currentColor");
  svg.setAttribute("stroke-width", "1.8");
  svg.setAttribute("stroke-linecap", "round");
  svg.setAttribute("stroke-linejoin", "round");
  svg.setAttribute("aria-hidden", "true");
  for (const d of paths) {
    const path = document.createElementNS(SVG_NS, "path");
    path.setAttribute("d", d);
    svg.appendChild(path);
  }
  return svg;
}

function siteIcon(item) {
  const host = hostOf(item.url);
  const match = host && Object.keys(DOMAIN_ICONS).find((d) => host === d || host.endsWith(`.${d}`));
  if (match) {
    const spec = DOMAIN_ICONS[match];
    const svg = document.createElementNS(SVG_NS, "svg");
    svg.setAttribute("viewBox", "0 0 24 24");
    svg.setAttribute("class", "glyph");
    svg.setAttribute("aria-hidden", "true");
    const path = document.createElementNS(SVG_NS, "path");
    path.setAttribute("fill", spec.color || "currentColor");
    path.setAttribute("d", spec.path);
    svg.appendChild(path);
    return svg;
  }
  const mono = document.createElement("span");
  mono.className = "mono";
  mono.setAttribute("aria-hidden", "true");
  mono.textContent = (item.title.trim() || host || "?").charAt(0).toUpperCase();
  return mono;
}

function iconButton(kind, label, onClick) {
  const btn = document.createElement("button");
  btn.type = "button";
  btn.className = kind === "remove" ? "icon-btn danger" : "icon-btn";
  btn.setAttribute("aria-label", label);
  btn.title = label;
  btn.appendChild(strokeIcon(ICON_PATHS[kind], 15));
  btn.addEventListener("click", onClick);
  return btn;
}

/* Rendering */

function renderTiles() {
  ui.tiles.textContent = "";
  const hasItems = state.shortcuts.length > 0;
  ui.tiles.hidden = !state.showShortcuts || !hasItems;
  ui.empty.hidden = !state.showShortcuts || hasItems;
  if (ui.tiles.hidden) return;

  const frag = document.createDocumentFragment();
  for (const item of state.shortcuts) {
    const tile = document.createElement("a");
    tile.className = "tile";
    tile.href = item.url;

    const face = document.createElement("span");
    face.className = "tile-face";
    face.appendChild(siteIcon(item));

    const label = document.createElement("span");
    label.className = "tile-label";
    label.textContent = item.title;

    tile.append(face, label);
    frag.appendChild(tile);
  }
  ui.tiles.appendChild(frag);
}

function renderList() {
  ui.list.textContent = "";
  const frag = document.createDocumentFragment();
  state.shortcuts.forEach((item, index) => {
    const row = document.createElement("li");
    row.className = "sc-row";

    const icon = document.createElement("span");
    icon.className = "sc-icon";
    icon.appendChild(siteIcon(item));

    const text = document.createElement("div");
    text.className = "sc-text";
    const name = document.createElement("span");
    name.className = "sc-name";
    name.textContent = item.title;
    const host = document.createElement("span");
    host.className = "sc-host";
    host.textContent = hostOf(item.url);
    text.append(name, host);

    const actions = document.createElement("div");
    actions.className = "sc-actions";
    actions.append(
      iconButton("edit", `Edit ${item.title}`, () => openForm(index)),
      iconButton("remove", `Remove ${item.title}`, () => removeShortcut(index))
    );

    row.append(icon, text, actions);
    frag.appendChild(row);
  });
  ui.list.appendChild(frag);
}

function renderAppearance() {
  Theme.apply(state.mode, state.swatch);
  ui.modes.forEach((btn) => {
    btn.setAttribute("aria-pressed", String(btn.dataset.mode === state.mode));
  });
  ui.swatches.querySelectorAll(".swatch").forEach((btn) => {
    btn.setAttribute("aria-pressed", String(btn.dataset.swatch === state.swatch));
  });
  const hasBg = state.customBg !== "";
  document.body.classList.toggle("has-bg", hasBg);
  document.body.style.backgroundImage = hasBg ? `url("${state.customBg}")` : "";
  ui.bgRemove.hidden = !hasBg;
  ui.bgPickLabel.textContent = hasBg ? "Change background" : "Upload background";
}

function renderAll() {
  renderAppearance();
  renderTiles();
  renderList();
  ui.showShortcuts.checked = state.showShortcuts;
  ui.manager.hidden = !state.showShortcuts;
}

function buildSwatches() {
  const frag = document.createDocumentFragment();
  for (const swatch of Theme.SWATCHES) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "swatch";
    btn.dataset.swatch = swatch.id;
    btn.title = swatch.name;
    btn.setAttribute("aria-label", swatch.name);
    btn.style.setProperty("--sw-tile", swatch.tile);
    btn.style.setProperty("--sw-accent", swatch.accent);
    btn.addEventListener("click", () => {
      state.swatch = swatch.id;
      persist({ activeSwatch: swatch.id });
      renderAppearance();
    });
    frag.appendChild(btn);
  }
  ui.swatches.appendChild(frag);
}

/* Drawer */

const isOpen = () => ui.panel.classList.contains("open");

function setPanel(open, { restoreFocus = true } = {}) {
  ui.panel.classList.toggle("open", open);
  ui.panel.inert = !open;
  ui.toggle.setAttribute("aria-expanded", String(open));
  if (open) {
    ui.close.focus();
  } else {
    closeForm();
    if (restoreFocus) ui.toggle.focus();
  }
}

/* Shortcut form */

function setUrlError(visible) {
  ui.urlError.hidden = !visible;
  ui.url.setAttribute("aria-invalid", String(visible));
}

function openForm(index = -1) {
  const item = state.shortcuts[index];
  state.editIndex = item ? index : -1;
  ui.name.value = item ? item.title : "";
  ui.url.value = item ? item.url : "";
  ui.save.textContent = item ? "Save" : "Add";
  setUrlError(false);
  ui.form.hidden = false;
  ui.addTrigger.hidden = true;
  ui.name.focus();
}

function closeForm() {
  state.editIndex = -1;
  ui.form.reset();
  ui.form.hidden = true;
  ui.addTrigger.hidden = false;
  setUrlError(false);
}

function commitShortcuts() {
  persist({ shortcuts: state.shortcuts });
  renderTiles();
  renderList();
}

function removeShortcut(index) {
  state.shortcuts.splice(index, 1);
  if (state.editIndex === index) closeForm();
  commitShortcuts();
  ui.addTrigger.focus();
}

ui.form.addEventListener("submit", (event) => {
  event.preventDefault();
  const url = toWebUrl(ui.url.value);
  if (!url) {
    setUrlError(true);
    ui.url.focus();
    return;
  }
  const entry = { title: ui.name.value.trim() || hostOf(url), url };
  if (state.editIndex >= 0) {
    state.shortcuts[state.editIndex] = entry;
  } else {
    state.shortcuts.push(entry);
  }
  closeForm();
  commitShortcuts();
  ui.addTrigger.focus();
});

ui.url.addEventListener("input", () => {
  if (!ui.urlError.hidden) setUrlError(false);
});

ui.cancel.addEventListener("click", () => {
  closeForm();
  ui.addTrigger.focus();
});

ui.addTrigger.addEventListener("click", () => openForm(-1));

ui.emptyAdd.addEventListener("click", () => {
  setPanel(true);
  openForm(-1);
});

/* Appearance controls */

ui.modes.forEach((btn) => {
  btn.addEventListener("click", () => {
    state.mode = btn.dataset.mode;
    persist({ appearanceMode: state.mode });
    renderAppearance();
  });
});

Theme.media.addEventListener("change", () => {
  if (state.mode === "system") renderAppearance();
});

function setBgMessage(text) {
  ui.bgMessage.textContent = text;
  ui.bgMessage.hidden = text === "";
}

// Downscale before saving so the stored data URL stays small and quick to paint.
async function imageToDataUrl(file) {
  const bitmap = await createImageBitmap(file);
  const scale = Math.min(1, MAX_BG_EDGE / Math.max(bitmap.width, bitmap.height));
  const canvas = document.createElement("canvas");
  canvas.width = Math.round(bitmap.width * scale);
  canvas.height = Math.round(bitmap.height * scale);
  canvas.getContext("2d").drawImage(bitmap, 0, 0, canvas.width, canvas.height);
  bitmap.close();
  return canvas.toDataURL("image/jpeg", 0.86);
}

ui.bgPick.addEventListener("click", () => ui.bgInput.click());

ui.bgInput.addEventListener("change", async () => {
  const file = ui.bgInput.files && ui.bgInput.files[0];
  ui.bgInput.value = "";
  if (!file) return;
  setBgMessage("");

  let dataUrl;
  try {
    dataUrl = await imageToDataUrl(file);
  } catch {
    setBgMessage("That file can't be used as a background. Try a JPEG or PNG.");
    return;
  }

  try {
    await persist({ customBg: dataUrl });
  } catch {
    setBgMessage("That image is too large to save. Try a smaller one.");
    return;
  }
  state.customBg = dataUrl;
  renderAppearance();
});

ui.bgRemove.addEventListener("click", () => {
  state.customBg = "";
  persist({ customBg: "" });
  setBgMessage("");
  renderAppearance();
  ui.bgPick.focus();
});

ui.showShortcuts.addEventListener("change", () => {
  state.showShortcuts = ui.showShortcuts.checked;
  persist({ showShortcuts: state.showShortcuts });
  if (!state.showShortcuts) closeForm();
  renderTiles();
  ui.manager.hidden = !state.showShortcuts;
});

ui.settings.addEventListener("click", () => {
  if (typeof chrome !== "undefined" && chrome.tabs) {
    chrome.tabs.update({ url: "chrome://settings" });
  }
});

/* Drawer wiring */

ui.toggle.addEventListener("click", () => setPanel(!isOpen()));
ui.close.addEventListener("click", () => setPanel(false));

document.addEventListener("pointerdown", (event) => {
  if (!isOpen()) return;
  if (ui.panel.contains(event.target) || ui.toggle.contains(event.target)) return;
  setPanel(false, { restoreFocus: false });
});

document.addEventListener("keydown", (event) => {
  if (event.key !== "Escape" || !isOpen()) return;
  if (!ui.form.hidden) {
    closeForm();
    ui.addTrigger.focus();
    return;
  }
  setPanel(false);
});

/* Boot */

function applyStored(data) {
  if (Array.isArray(data.shortcuts)) state.shortcuts = data.shortcuts.filter(isShortcut);
  if (typeof data.showShortcuts === "boolean") state.showShortcuts = data.showShortcuts;
  if (typeof data.appearanceMode === "string") state.mode = data.appearanceMode;
  if (typeof data.activeSwatch === "string") state.swatch = data.activeSwatch;
  if (typeof data.customBg === "string") state.customBg = data.customBg;
}

buildSwatches();
renderAll();

if (store) {
  store.get(STORAGE_DEFAULTS).then((data) => {
    applyStored(data);
    writeListCache();
    Theme.cache(state.mode, state.swatch);
    renderAll();
  });

  // Keep every open New Tab in sync with changes made in another one.
  chrome.storage.onChanged.addListener((changes, area) => {
    if (area !== "local") return;
    const data = {};
    for (const [key, change] of Object.entries(changes)) data[key] = change.newValue;
    applyStored(data);
    renderAll();
  });
}
