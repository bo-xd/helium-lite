"use strict";

const ENGINES = {
  ddg: { name: "DuckDuckGo", url: "https://duckduckgo.com/?q=" },
  brave: { name: "Brave", url: "https://search.brave.com/search?q=" },
  startpage: { name: "Startpage", url: "https://www.startpage.com/do/search?q=" },
  google: { name: "Google", url: "https://www.google.com/search?q=" }
};

const DEFAULT_SHORTCUTS = [
  { title: "GitHub", url: "https://github.com" },
  { title: "ArchWiki", url: "https://wiki.archlinux.org" },
  { title: "YouTube", url: "https://www.youtube.com" }
];

const searchForm = document.getElementById("searchForm");
const searchInput = document.getElementById("searchInput");
const engineTabs = Array.from(document.querySelectorAll(".eng-tab"));
const shortcutsGrid = document.getElementById("shortcutsGrid");
const addForm = document.getElementById("addForm");
const addTitle = document.getElementById("addTitle");
const addUrl = document.getElementById("addUrl");
const cancelAdd = document.getElementById("cancelAdd");

let activeEngine = "ddg";
let shortcuts = [...DEFAULT_SHORTCUTS];

function setEngine(key) {
  if (!ENGINES[key]) key = "ddg";
  activeEngine = key;
  searchInput.placeholder = `Search ${ENGINES[key].name} or enter address`;
  engineTabs.forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.engine === key);
  });
}

function normalizeUrl(raw) {
  const s = raw.trim();
  if (!s) return "";
  if (/^https?:\/\//i.test(s)) return s;
  return "https://" + s;
}

function looksLikeUrl(q) {
  return /^https?:\/\//i.test(q) || (!q.includes(" ") && /^[a-z0-9.-]+\.[a-z]{2,}(\/.*)?$/i.test(q));
}

function renderShortcuts() {
  shortcutsGrid.textContent = "";

  shortcuts.forEach((item, idx) => {
    const link = document.createElement("a");
    link.className = "shortcut-link";
    link.href = item.url;

    const title = document.createElement("span");
    title.textContent = item.title;

    const del = document.createElement("button");
    del.type = "button";
    del.className = "shortcut-del";
    del.title = "Remove";
    del.setAttribute("aria-label", `Remove ${item.title}`);
    del.textContent = "×";
    del.addEventListener("click", (e) => {
      e.preventDefault();
      e.stopPropagation();
      shortcuts.splice(idx, 1);
      saveShortcuts();
      renderShortcuts();
    });

    link.append(title, del);
    shortcutsGrid.appendChild(link);
  });

  const addBtn = document.createElement("button");
  addBtn.type = "button";
  addBtn.className = "shortcut-add";
  addBtn.title = "Add shortcut";
  addBtn.textContent = "+";
  addBtn.addEventListener("click", () => {
    addForm.classList.toggle("hidden");
    if (!addForm.classList.contains("hidden")) addTitle.focus();
  });

  shortcutsGrid.appendChild(addBtn);
}

function saveShortcuts() {
  if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
    chrome.storage.local.set({ shortcuts });
  }
}

searchForm.addEventListener("submit", (e) => {
  e.preventDefault();
  const q = searchInput.value.trim();
  if (!q) return;
  if (looksLikeUrl(q)) {
    location.href = normalizeUrl(q);
  } else {
    location.href = ENGINES[activeEngine].url + encodeURIComponent(q);
  }
});

engineTabs.forEach((btn) => {
  btn.addEventListener("click", () => {
    setEngine(btn.dataset.engine);
    if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
      chrome.storage.local.set({ searchEngine: activeEngine });
    }
    searchInput.focus();
  });
});

addForm.addEventListener("submit", (e) => {
  e.preventDefault();
  const title = addTitle.value.trim();
  const url = normalizeUrl(addUrl.value);
  if (!title || !url) return;
  shortcuts.push({ title, url });
  saveShortcuts();
  renderShortcuts();
  addTitle.value = "";
  addUrl.value = "";
  addForm.classList.add("hidden");
});

cancelAdd.addEventListener("click", () => {
  addForm.classList.add("hidden");
});

if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
  chrome.storage.local.get(
    { searchEngine: "ddg", shortcuts: DEFAULT_SHORTCUTS },
    (data) => {
      setEngine(data.searchEngine);
      if (Array.isArray(data.shortcuts)) {
        shortcuts = data.shortcuts;
      }
      renderShortcuts();
    }
  );
} else {
  setEngine("ddg");
  renderShortcuts();
}
