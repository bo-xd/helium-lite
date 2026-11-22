"use strict";

const optGlobal = document.getElementById("optGlobal");
const optLevel = document.getElementById("optLevel");
const optFingerprint = document.getElementById("optFingerprint");
const optCosmetic = document.getElementById("optCosmetic");
const optAnnoyances = document.getElementById("optAnnoyances");
const optTrackingParams = document.getElementById("optTrackingParams");

const customForm = document.getElementById("customForm");
const customInput = document.getElementById("customInput");
const customList = document.getElementById("customList");

const scriptsForm = document.getElementById("scriptsForm");
const scriptsInput = document.getElementById("scriptsInput");
const scriptsList = document.getElementById("scriptsList");

const whitelistForm = document.getElementById("whitelistForm");
const whitelistInput = document.getElementById("whitelistInput");
const whitelistList = document.getElementById("whitelistList");

const DOMAIN_RE = /^[a-z0-9.-]+\.[a-z]{2,}$/;

function normalizeDomain(raw) {
  return raw
    .trim()
    .toLowerCase()
    .replace(/^https?:\/\//, "")
    .replace(/^www\./, "")
    .split("/")[0]
    .split(":")[0];
}

function renderList(ul, items, key) {
  ul.textContent = "";
  if (!items.length) {
    const li = document.createElement("li");
    li.className = "empty";
    li.textContent = "None configured.";
    ul.appendChild(li);
    return;
  }

  for (const domain of items) {
    const li = document.createElement("li");
    const span = document.createElement("span");
    span.textContent = domain;

    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "remove";
    btn.textContent = "Remove";
    btn.addEventListener("click", async () => {
      const data = await chrome.storage.local.get({ [key]: [] });
      const next = (Array.isArray(data[key]) ? data[key] : []).filter((d) => d !== domain);
      await chrome.storage.local.set({ [key]: next });
      renderList(ul, next, key);
    });

    li.append(span, btn);
    ul.appendChild(li);
  }
}

async function load() {
  const data = await chrome.storage.local.get({
    enabled: true,
    blockingLevel: "standard",
    antiFingerprintEnabled: true,
    cosmeticEnabled: true,
    annoyancesEnabled: true,
    trackingParamsEnabled: true,
    customDomains: [],
    blockScriptsSites: [],
    whitelist: []
  });

  optGlobal.checked = data.enabled !== false;
  optLevel.value = data.blockingLevel === "aggressive" ? "aggressive" : "standard";
  optFingerprint.checked = data.antiFingerprintEnabled !== false;
  optCosmetic.checked = data.cosmeticEnabled !== false;
  optAnnoyances.checked = data.annoyancesEnabled !== false;
  optTrackingParams.checked = data.trackingParamsEnabled !== false;

  renderList(customList, Array.isArray(data.customDomains) ? data.customDomains : [], "customDomains");
  renderList(scriptsList, Array.isArray(data.blockScriptsSites) ? data.blockScriptsSites : [], "blockScriptsSites");
  renderList(whitelistList, Array.isArray(data.whitelist) ? data.whitelist : [], "whitelist");
}

optGlobal.addEventListener("change", () => {
  chrome.storage.local.set({ enabled: optGlobal.checked });
});
optLevel.addEventListener("change", () => {
  chrome.storage.local.set({ blockingLevel: optLevel.value });
});
optFingerprint.addEventListener("change", () => {
  chrome.storage.local.set({ antiFingerprintEnabled: optFingerprint.checked });
});
optCosmetic.addEventListener("change", () => {
  chrome.storage.local.set({ cosmeticEnabled: optCosmetic.checked });
});
optAnnoyances.addEventListener("change", () => {
  chrome.storage.local.set({ annoyancesEnabled: optAnnoyances.checked });
});
optTrackingParams.addEventListener("change", () => {
  chrome.storage.local.set({ trackingParamsEnabled: optTrackingParams.checked });
});

async function handleAdd(input, key, ul) {
  const domain = normalizeDomain(input.value);
  if (!DOMAIN_RE.test(domain)) {
    input.focus();
    return;
  }
  const data = await chrome.storage.local.get({ [key]: [] });
  const set = new Set(Array.isArray(data[key]) ? data[key] : []);
  set.add(domain);
  const next = Array.from(set).sort();
  await chrome.storage.local.set({ [key]: next });
  input.value = "";
  renderList(ul, next, key);
}

customForm.addEventListener("submit", (e) => {
  e.preventDefault();
  handleAdd(customInput, "customDomains", customList);
});

scriptsForm.addEventListener("submit", (e) => {
  e.preventDefault();
  handleAdd(scriptsInput, "blockScriptsSites", scriptsList);
});

whitelistForm.addEventListener("submit", (e) => {
  e.preventDefault();
  handleAdd(whitelistInput, "whitelist", whitelistList);
});

load();
