"use strict";

const toggleGlobal = document.getElementById("toggleGlobal");
const toggleWhitelist = document.getElementById("toggleWhitelist");
const toggleCosmetic = document.getElementById("toggleCosmetic");
const whitelistRow = document.getElementById("whitelistRow");
const siteHostEl = document.getElementById("siteHost");
const statusDot = document.getElementById("statusDot");
const statusBadge = document.getElementById("statusBadge");
const ruleCountEl = document.getElementById("ruleCount");
const openOptionsBtn = document.getElementById("openOptions");

let currentHost = "";

function normalizeHost(urlStr) {
  try {
    const u = new URL(urlStr);
    if (u.protocol !== "http:" && u.protocol !== "https:") return "";
    return u.hostname.toLowerCase().replace(/^www\./, "");
  } catch {
    return "";
  }
}

function updateStatusUI(enabled, whitelisted) {
  statusDot.classList.remove("paused", "off");
  if (!enabled) {
    statusDot.classList.add("off");
    statusBadge.textContent = "Disabled";
  } else if (whitelisted) {
    statusDot.classList.add("paused");
    statusBadge.textContent = "Paused";
  } else {
    statusBadge.textContent = "Active";
  }
}

async function init() {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  currentHost = tab && tab.url ? normalizeHost(tab.url) : "";

  if (currentHost) {
    siteHostEl.textContent = currentHost;
  } else {
    siteHostEl.textContent = "Internal / Non-web page";
    whitelistRow.classList.add("disabled");
  }

  const data = await chrome.storage.local.get({
    enabled: true,
    cosmeticEnabled: true,
    whitelist: [],
    customDomains: []
  });

  const enabled = data.enabled !== false;
  const cosmeticEnabled = data.cosmeticEnabled !== false;
  const whitelist = Array.isArray(data.whitelist) ? data.whitelist : [];
  const isWhitelisted = Boolean(currentHost && whitelist.includes(currentHost));

  toggleGlobal.checked = enabled;
  toggleCosmetic.checked = cosmeticEnabled;
  toggleWhitelist.checked = isWhitelisted;
  updateStatusUI(enabled, isWhitelisted);

  try {
    const dynamic = await chrome.declarativeNetRequest.getDynamicRules();
    const customCount = dynamic.filter((r) => r.action.type === "block").length;
    ruleCountEl.textContent = customCount > 0 ? `Static + ${customCount} custom` : "Static DNR ruleset";
  } catch {
    ruleCountEl.textContent = "DNR ready";
  }
}

toggleGlobal.addEventListener("change", async () => {
  const enabled = toggleGlobal.checked;
  await chrome.storage.local.set({ enabled });
  updateStatusUI(enabled, toggleWhitelist.checked);
});

toggleCosmetic.addEventListener("change", async () => {
  await chrome.storage.local.set({ cosmeticEnabled: toggleCosmetic.checked });
});

toggleWhitelist.addEventListener("change", async () => {
  if (!currentHost) return;
  const data = await chrome.storage.local.get({ whitelist: [] });
  const set = new Set(Array.isArray(data.whitelist) ? data.whitelist : []);
  if (toggleWhitelist.checked) {
    set.add(currentHost);
  } else {
    set.delete(currentHost);
  }
  await chrome.storage.local.set({ whitelist: Array.from(set).sort() });
  updateStatusUI(toggleGlobal.checked, toggleWhitelist.checked);
});

openOptionsBtn.addEventListener("click", () => {
  chrome.runtime.openOptionsPage();
});

init();
