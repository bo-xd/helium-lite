"use strict";

const DEFAULT_SHORTCUTS = [
  { title: "YouTube", url: "https://www.youtube.com" },
  { title: "GitHub", url: "https://github.com" },
  { title: "ArchWiki", url: "https://wiki.archlinux.org" },
  { title: "DuckDuckGo", url: "https://duckduckgo.com" },
  { title: "Proton", url: "https://proton.me" },
  { title: "Wikipedia", url: "https://www.wikipedia.org" }
];

// Clean zero-network inline vector paths for common domains
const DOMAIN_ICONS = {
  "youtube.com": {
    color: "#ff4e45",
    path: "M21.6 7.2a2.7 2.7 0 0 0-1.9-1.9C18 5 12 5 12 5s-6 0-7.7.3a2.7 2.7 0 0 0-1.9 1.9C2 9 2 12 2 12s0 3 .4 4.8a2.7 2.7 0 0 0 1.9 1.9C6 19 12 19 12 19s6 0 7.7-.3a2.7 2.7 0 0 0 1.9-1.9C22 15 22 12 22 12s0-3-.4-4.8ZM10 15V9l5.2 3-5.2 3Z"
  },
  "github.com": {
    color: "#e6e9ef",
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
    color: "#e6e9ef",
    path: "M3 6h3.5v1.2h-.9l2.8 7.8 2.2-5.6-1.1-2.2h-.8V6h3.8v1.2h-.8l1.8 4.4 2.1-4.4h-.9V6H18v1.2h-.8L14 14.3l1.2 2.7 3.2-9.8h-.9V6H21v1.2h-.9L15.4 19h-1.3l-2-4.6L10 19H8.7L4 7.2H3V6Z"
  }
};

const tilesGrid = document.getElementById("tilesGrid");
const editToggle = document.getElementById("editToggle");
const editorPanel = document.getElementById("editorPanel");
const closeEditor = document.getElementById("closeEditor");
const editorList = document.getElementById("editorList");
const shortcutForm = document.getElementById("shortcutForm");
const editIndexInput = document.getElementById("editIndex");
const inputTitle = document.getElementById("inputTitle");
const inputUrl = document.getElementById("inputUrl");
const saveBtn = document.getElementById("saveBtn");
const cancelEditBtn = document.getElementById("cancelEditBtn");

let shortcuts = [...DEFAULT_SHORTCUTS];

function normalizeUrl(raw) {
  const s = raw.trim();
  if (!s) return "";
  if (/^https?:\/\//i.test(s)) return s;
  return "https://" + s;
}

function getHost(urlStr) {
  try {
    return new URL(urlStr).hostname.toLowerCase().replace(/^www\./, "");
  } catch {
    return "";
  }
}

function findDomainIcon(urlStr) {
  const host = getHost(urlStr);
  if (!host) return null;
  for (const [domain, spec] of Object.entries(DOMAIN_ICONS)) {
    if (host === domain || host.endsWith("." + domain)) {
      return spec;
    }
  }
  return null;
}

function createIconNode(item) {
  const spec = findDomainIcon(item.url);
  if (spec) {
    const ns = "http://www.w3.org/2000/svg";
    const svg = document.createElementNS(ns, "svg");
    svg.setAttribute("viewBox", "0 0 24 24");
    svg.setAttribute("class", "tile-icon");
    svg.setAttribute("aria-hidden", "true");
    const path = document.createElementNS(ns, "path");
    path.setAttribute("fill", spec.color);
    path.setAttribute("d", spec.path);
    svg.appendChild(path);
    return svg;
  }
  const mono = document.createElement("span");
  mono.className = "tile-mono";
  mono.textContent = (item.title || getHost(item.url) || "?").charAt(0).toUpperCase();
  return mono;
}

function saveShortcuts() {
  if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
    chrome.storage.local.set({ shortcuts });
  }
}

function resetForm() {
  editIndexInput.value = "-1";
  inputTitle.value = "";
  inputUrl.value = "";
  saveBtn.textContent = "Add";
  cancelEditBtn.classList.add("hidden");
}

function renderAll() {
  tilesGrid.textContent = "";
  editorList.textContent = "";

  shortcuts.forEach((item, idx) => {
    const a = document.createElement("a");
    a.className = "tile-item";
    a.href = item.url;

    const box = document.createElement("span");
    box.className = "tile-box";
    box.appendChild(createIconNode(item));

    const label = document.createElement("span");
    label.className = "tile-label";
    label.textContent = item.title;

    a.append(box, label);
    tilesGrid.appendChild(a);

    const li = document.createElement("li");
    li.className = "editor-row";

    const name = document.createElement("span");
    name.className = "editor-row-name";
    name.textContent = item.title;

    const btns = document.createElement("div");
    btns.className = "editor-row-btns";

    const editBtn = document.createElement("button");
    editBtn.type = "button";
    editBtn.className = "row-btn";
    editBtn.textContent = "Edit";
    editBtn.addEventListener("click", () => {
      editIndexInput.value = String(idx);
      inputTitle.value = item.title;
      inputUrl.value = item.url;
      saveBtn.textContent = "Save";
      cancelEditBtn.classList.remove("hidden");
      inputTitle.focus();
    });

    const delBtn = document.createElement("button");
    delBtn.type = "button";
    delBtn.className = "row-btn del";
    delBtn.textContent = "Remove";
    delBtn.addEventListener("click", () => {
      shortcuts.splice(idx, 1);
      saveShortcuts();
      resetForm();
      renderAll();
    });

    btns.append(editBtn, delBtn);
    li.append(name, btns);
    editorList.appendChild(li);
  });
}

function toggleEditor(forceOpen) {
  const isHidden = editorPanel.classList.contains("hidden");
  const nextOpen = typeof forceOpen === "boolean" ? forceOpen : isHidden;
  editorPanel.classList.toggle("hidden", !nextOpen);
  editToggle.setAttribute("aria-expanded", nextOpen ? "true" : "false");
  if (!nextOpen) resetForm();
}

editToggle.addEventListener("click", () => {
  toggleEditor();
});

closeEditor.addEventListener("click", () => {
  toggleEditor(false);
});

cancelEditBtn.addEventListener("click", () => {
  resetForm();
});

shortcutForm.addEventListener("submit", (e) => {
  e.preventDefault();
  const title = inputTitle.value.trim();
  const url = normalizeUrl(inputUrl.value);
  if (!title || !url) return;

  const idx = Number(editIndexInput.value);
  if (idx >= 0 && idx < shortcuts.length) {
    shortcuts[idx] = { title, url };
  } else {
    shortcuts.push({ title, url });
  }
  saveShortcuts();
  resetForm();
  renderAll();
});

document.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && !editorPanel.classList.contains("hidden")) {
    toggleEditor(false);
  }
});

if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
  chrome.storage.local.get({ shortcuts: DEFAULT_SHORTCUTS }, (data) => {
    if (Array.isArray(data.shortcuts)) {
      shortcuts = data.shortcuts;
    }
    renderAll();
  });
} else {
  renderAll();
}
