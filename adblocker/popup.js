"use strict";

const DEFAULTS = {
  enabled: true,
  blockingLevel: "standard",
  cosmeticEnabled: true,
  antiFingerprintEnabled: true,
  annoyancesEnabled: true,
  trackingParamsEnabled: true,
  whitelist: [],
  blockScriptsSites: []
};

// Dynamic rule ID range background.js uses for user-added blocked domains.
const CUSTOM_RULE_MIN = 200000;
const CUSTOM_RULE_MAX = 300000;

const STATUS_TEXT = {
  up: "Shields up",
  down: "Shields down on this site",
  off: "Paused on all sites",
  na: "Shields don't run on browser pages"
};

const $ = (id) => document.getElementById(id);
const ui = {
  panel: $("panel"),
  host: $("siteHost"),
  status: $("siteStatus"),
  siteShield: $("siteShield"),
  count: $("blockedCount"),
  countLabel: $("blockedLabel"),
  rules: $("ruleCount"),
  levelRow: $("levelRow"),
  levelBtns: Array.from(document.querySelectorAll(".seg[data-level]")),
  scriptsRow: $("scriptsRow"),
  scripts: $("toggleScripts"),
  globalRows: [$("fpRow"), $("cosmeticRow"), $("annoyancesRow"), $("trackingRow")],
  fp: $("toggleFingerprint"),
  cosmetic: $("toggleCosmetic"),
  annoyances: $("toggleAnnoyances"),
  tracking: $("toggleTracking"),
  globalBtn: $("globalToggle"),
  options: $("openOptions")
};

const view = { tabId: null, host: "", settings: { ...DEFAULTS } };

function hostFromUrl(url) {
  try {
    const u = new URL(url);
    if (u.protocol !== "http:" && u.protocol !== "https:") return "";
    return u.hostname.toLowerCase().replace(/^www\./, "");
  } catch {
    return "";
  }
}

// Matches background.js and content.js: an entry covers its own subdomains.
function covers(entry, host) {
  const domain = String(entry || "").toLowerCase().replace(/^www\./, "");
  return domain !== "" && (host === domain || host.endsWith(`.${domain}`));
}

const listCovers = (list, host) => Array.isArray(list) && list.some((e) => covers(e, host));
const listWithout = (list, host) => (Array.isArray(list) ? list : []).filter((e) => !covers(e, host));

function setRowDisabled(row, disabled) {
  row.classList.toggle("is-disabled", disabled);
  row.querySelectorAll("input, button").forEach((control) => {
    control.disabled = disabled;
  });
}

function render() {
  const s = view.settings;
  const web = view.host !== "";
  const enabled = s.enabled !== false;
  const siteDown = web && listCovers(s.whitelist, view.host);
  let state = "up";
  if (!enabled) state = "off";
  else if (!web) state = "na";
  else if (siteDown) state = "down";

  ui.panel.dataset.state = state;
  ui.host.textContent = web ? view.host : "Browser page";
  ui.host.title = ui.host.textContent;
  ui.status.textContent = STATUS_TEXT[state];

  ui.siteShield.checked = web && !siteDown;
  ui.siteShield.disabled = !web || !enabled;

  ui.scripts.checked = web && listCovers(s.blockScriptsSites, view.host);
  setRowDisabled(ui.scriptsRow, !web || !enabled || siteDown);

  ui.fp.checked = s.antiFingerprintEnabled !== false;
  ui.cosmetic.checked = s.cosmeticEnabled !== false;
  ui.annoyances.checked = s.annoyancesEnabled !== false;
  ui.tracking.checked = s.trackingParamsEnabled !== false;
  ui.globalRows.forEach((row) => setRowDisabled(row, !enabled));

  const level = s.blockingLevel === "aggressive" ? "aggressive" : "standard";
  ui.levelBtns.forEach((btn) => {
    btn.setAttribute("aria-pressed", String(btn.dataset.level === level));
  });
  setRowDisabled(ui.levelRow, !enabled);

  ui.globalBtn.textContent = enabled ? "Pause on all sites" : "Resume protection";
}

async function refreshCount() {
  if (view.tabId === null || view.host === "") {
    ui.count.hidden = true;
    ui.countLabel.textContent = "Nothing to block here";
    return;
  }
  ui.count.hidden = false;
  ui.countLabel.textContent = "blocked on this page";

  let blocked = null;
  try {
    // With declarativeNetRequestFeedback this is the real per-tab action count.
    const text = await chrome.action.getBadgeText({ tabId: view.tabId });
    if (/^\d+$/.test(text)) blocked = Number(text);
    else if (text === "") blocked = 0;
  } catch {
    // Fall through to getMatchedRules.
  }
  if (blocked === null) {
    try {
      const { rulesMatchedInfo } = await chrome.declarativeNetRequest.getMatchedRules({ tabId: view.tabId });
      blocked = rulesMatchedInfo.length;
    } catch {
      // Count unavailable.
    }
  }
  ui.count.textContent = blocked === null ? "–" : blocked.toLocaleString();
}

const DEFAULT_RULESET_COUNTS = {
  ads: 15000,
  privacy: 10000,
  annoyances: 3361,
  malware: 224,
  tracking_params: 1
};

async function refreshRuleCount() {
  try {
    const [stats, enabledIds, dynamicRules] = await Promise.all([
      fetch("rules/stats.json").then((r) => (r.ok ? r.json() : null)).catch(() => null),
      chrome.declarativeNetRequest.getEnabledRulesets(),
      chrome.declarativeNetRequest.getDynamicRules()
    ]);
    const perRuleset = stats && stats.rulesets ? stats.rulesets : DEFAULT_RULESET_COUNTS;
    const listRules = enabledIds.reduce((sum, id) => sum + (Number(perRuleset[id]) || 0), 0);
    const custom = dynamicRules.filter((r) => r.id >= CUSTOM_RULE_MIN && r.id < CUSTOM_RULE_MAX).length;
    const total = listRules + custom;
    ui.rules.textContent = total > 0 ? `${total.toLocaleString()} rules` : "No rules active";
    ui.rules.title = custom > 0
      ? `${listRules.toLocaleString()} filter-list rules + ${custom} custom`
      : `${listRules.toLocaleString()} filter-list rules`;
  } catch {
    ui.rules.textContent = "28,586 rules";
  }
}

async function commit(patch, { reload = false } = {}) {
  view.settings = { ...view.settings, ...patch };
  render();
  await chrome.storage.local.set(patch);
  try {
    await chrome.runtime.sendMessage({ type: "kestrel-sync" });
  } catch {
    // Worker restarting; its storage listener applies the change.
  }
  refreshRuleCount();
  if (reload && view.tabId !== null && view.host !== "") {
    ui.count.textContent = "0";
    chrome.tabs.reload(view.tabId);
  }
}

ui.siteShield.addEventListener("change", () => {
  const rest = listWithout(view.settings.whitelist, view.host);
  const whitelist = ui.siteShield.checked ? rest : [...rest, view.host].sort();
  commit({ whitelist }, { reload: true });
});

ui.scripts.addEventListener("change", () => {
  const rest = listWithout(view.settings.blockScriptsSites, view.host);
  const blockScriptsSites = ui.scripts.checked ? [...rest, view.host].sort() : rest;
  commit({ blockScriptsSites }, { reload: true });
});

ui.levelBtns.forEach((btn) => {
  btn.addEventListener("click", () => {
    commit({ blockingLevel: btn.dataset.level === "aggressive" ? "aggressive" : "standard" });
  });
});

ui.fp.addEventListener("change", () => commit({ antiFingerprintEnabled: ui.fp.checked }));
ui.cosmetic.addEventListener("change", () => commit({ cosmeticEnabled: ui.cosmetic.checked }));
ui.annoyances.addEventListener("change", () => commit({ annoyancesEnabled: ui.annoyances.checked }));
ui.tracking.addEventListener("change", () => commit({ trackingParamsEnabled: ui.tracking.checked }));

ui.globalBtn.addEventListener("click", () => {
  commit({ enabled: view.settings.enabled === false }, { reload: true });
});

ui.options.addEventListener("click", () => chrome.runtime.openOptionsPage());

chrome.tabs.onUpdated.addListener((tabId, info) => {
  if (tabId === view.tabId && info.status === "complete") refreshCount();
});

async function init() {
  try {
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    if (tab) {
      view.tabId = typeof tab.id === "number" ? tab.id : null;
      view.host = tab.url ? hostFromUrl(tab.url) : "";
    }
  } catch {
    // No tab context; render as a browser page.
  }
  view.settings = { ...DEFAULTS, ...(await chrome.storage.local.get(DEFAULTS)) };
  render();
  refreshCount();
  refreshRuleCount();
}

init();
