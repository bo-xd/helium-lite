"use strict";

const toggleSiteShield = document.getElementById("toggleSiteShield");
const toggleScripts = document.getElementById("toggleScripts");
const toggleFingerprint = document.getElementById("toggleFingerprint");
const toggleCosmetic = document.getElementById("toggleCosmetic");
const toggleAnnoyances = document.getElementById("toggleAnnoyances");
const toggleTrackingParams = document.getElementById("toggleTrackingParams");
const toggleGlobalBtn = document.getElementById("toggleGlobalBtn");
const scriptsRow = document.getElementById("scriptsRow");
const siteHostEl = document.getElementById("siteHost");
const statusDot = document.getElementById("statusDot");
const statusBadge = document.getElementById("statusBadge");
const blockedCountEl = document.getElementById("blockedCount");
const ruleCountEl = document.getElementById("ruleCount");
const openOptionsBtn = document.getElementById("openOptions");
const segBtns = Array.from(document.querySelectorAll(".seg-btn"));

let currentHost = "";
let globalEnabled = true;

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
  statusBadge.classList.remove("paused", "off");
  if (!enabled) {
    statusDot.classList.add("off");
    statusBadge.classList.add("off");
    statusBadge.textContent = "Global Shields OFF";
  } else if (whitelisted) {
    statusDot.classList.add("paused");
    statusBadge.classList.add("paused");
    statusBadge.textContent = "Shields DOWN for site";
  } else {
    statusBadge.textContent = "Shields UP";
  }
  toggleGlobalBtn.textContent = enabled ? "Global: ON" : "Global: OFF";
}

function setLevelUI(level) {
  segBtns.forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.level === level);
  });
}

async function init() {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  currentHost = tab && tab.url ? normalizeHost(tab.url) : "";

  if (currentHost) {
    siteHostEl.textContent = currentHost;
  } else {
    siteHostEl.textContent = "Internal / Non-web page";
    scriptsRow.classList.add("disabled");
    toggleSiteShield.parentElement.classList.add("disabled");
  }

  if (tab && typeof tab.id === "number") {
    try {
      const badge = await chrome.action.getBadgeText({ tabId: tab.id });
      blockedCountEl.textContent = badge && /^\d+$/.test(badge) ? badge : "0";
    } catch {
      blockedCountEl.textContent = "0";
    }
  }

  const data = await chrome.storage.local.get({
    enabled: true,
    blockingLevel: "standard",
    cosmeticEnabled: true,
    antiFingerprintEnabled: true,
    annoyancesEnabled: true,
    trackingParamsEnabled: true,
    whitelist: [],
    blockScriptsSites: []
  });

  globalEnabled = data.enabled !== false;
  const whitelist = Array.isArray(data.whitelist) ? data.whitelist : [];
  const blockScripts = Array.isArray(data.blockScriptsSites) ? data.blockScriptsSites : [];
  const isWhitelisted = Boolean(currentHost && whitelist.includes(currentHost));

  toggleSiteShield.checked = !isWhitelisted;
  toggleScripts.checked = Boolean(currentHost && blockScripts.includes(currentHost));
  toggleFingerprint.checked = data.antiFingerprintEnabled !== false;
  toggleCosmetic.checked = data.cosmeticEnabled !== false;
  toggleAnnoyances.checked = data.annoyancesEnabled !== false;
  toggleTrackingParams.checked = data.trackingParamsEnabled !== false;

  setLevelUI(data.blockingLevel === "aggressive" ? "aggressive" : "standard");
  updateStatusUI(globalEnabled, isWhitelisted);

  try {
    const dynamic = await chrome.declarativeNetRequest.getDynamicRules();
    const customCount = dynamic.filter((r) => r.id >= 200000 && r.id < 300000).length;
    ruleCountEl.textContent = customCount > 0 ? `25.5k + ${customCount} custom` : "5 DNR rulesets active";
  } catch {
    ruleCountEl.textContent = "5 DNR rulesets";
  }
}

toggleSiteShield.addEventListener("change", async () => {
  if (!currentHost) return;
  const data = await chrome.storage.local.get({ whitelist: [] });
  const set = new Set(Array.isArray(data.whitelist) ? data.whitelist : []);
  if (toggleSiteShield.checked) {
    set.delete(currentHost);
  } else {
    set.add(currentHost);
  }
  await chrome.storage.local.set({ whitelist: Array.from(set).sort() });
  updateStatusUI(globalEnabled, !toggleSiteShield.checked);
});

toggleScripts.addEventListener("change", async () => {
  if (!currentHost) return;
  const data = await chrome.storage.local.get({ blockScriptsSites: [] });
  const set = new Set(Array.isArray(data.blockScriptsSites) ? data.blockScriptsSites : []);
  if (toggleScripts.checked) {
    set.add(currentHost);
  } else {
    set.delete(currentHost);
  }
  await chrome.storage.local.set({ blockScriptsSites: Array.from(set).sort() });
});

segBtns.forEach((btn) => {
  btn.addEventListener("click", async () => {
    const level = btn.dataset.level === "aggressive" ? "aggressive" : "standard";
    setLevelUI(level);
    await chrome.storage.local.set({ blockingLevel: level });
  });
});

toggleFingerprint.addEventListener("change", () => {
  chrome.storage.local.set({ antiFingerprintEnabled: toggleFingerprint.checked });
});

toggleCosmetic.addEventListener("change", () => {
  chrome.storage.local.set({ cosmeticEnabled: toggleCosmetic.checked });
});

toggleAnnoyances.addEventListener("change", () => {
  chrome.storage.local.set({ annoyancesEnabled: toggleAnnoyances.checked });
});

toggleTrackingParams.addEventListener("change", () => {
  chrome.storage.local.set({ trackingParamsEnabled: toggleTrackingParams.checked });
});

toggleGlobalBtn.addEventListener("click", async () => {
  globalEnabled = !globalEnabled;
  await chrome.storage.local.set({ enabled: globalEnabled });
  updateStatusUI(globalEnabled, !toggleSiteShield.checked);
});

openOptionsBtn.addEventListener("click", () => {
  chrome.runtime.openOptionsPage();
});

init();
