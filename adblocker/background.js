"use strict";

const DEFAULT_SETTINGS = {
  enabled: true,
  blockingLevel: "standard",
  cosmeticEnabled: true,
  antiFingerprintEnabled: true,
  annoyancesEnabled: true,
  trackingParamsEnabled: true,
  whitelist: [],
  blockScriptsSites: [],
  customDomains: []
};

const CORE_RULESETS = ["ads", "privacy", "malware"];
const WHITELIST_OFFSET = 100000;
const SCRIPT_BLOCK_OFFSET = 150000;
const CUSTOM_OFFSET = 200000;
const AGGRESSIVE_RULE_ID = 300001;

function normalizeDomain(raw) {
  if (typeof raw !== "string") return "";
  return raw
    .trim()
    .toLowerCase()
    .replace(/^https?:\/\//, "")
    .replace(/^www\./, "")
    .split("/")[0]
    .split(":")[0]
    .replace(/[^a-z0-9.-]/g, "");
}

async function syncRules() {
  const data = await chrome.storage.local.get(DEFAULT_SETTINGS);
  const enabled = data.enabled !== false;
  const annoyances = enabled && data.annoyancesEnabled !== false;
  const trackingParams = enabled && data.trackingParamsEnabled !== false;
  const aggressive = enabled && data.blockingLevel === "aggressive";

  const enableRulesetIds = [];
  const disableRulesetIds = [];

  for (const id of CORE_RULESETS) {
    (enabled ? enableRulesetIds : disableRulesetIds).push(id);
  }
  (annoyances ? enableRulesetIds : disableRulesetIds).push("annoyances");
  (trackingParams ? enableRulesetIds : disableRulesetIds).push("tracking_params");

  await chrome.declarativeNetRequest.updateEnabledRulesets({
    enableRulesetIds,
    disableRulesetIds
  });

  const existing = await chrome.declarativeNetRequest.getDynamicRules();
  const removeRuleIds = existing.map((r) => r.id);
  const addRules = [];

  if (enabled) {
    const whitelist = Array.isArray(data.whitelist) ? data.whitelist : [];
    const whitelistSet = new Set();

    whitelist.forEach((entry, idx) => {
      const domain = normalizeDomain(entry);
      if (!domain || whitelistSet.has(domain)) return;
      whitelistSet.add(domain);
      addRules.push({
        id: WHITELIST_OFFSET + idx,
        priority: 1000,
        action: { type: "allowAllRequests" },
        condition: {
          urlFilter: `||${domain}^`,
          resourceTypes: ["main_frame", "sub_frame"]
        }
      });
    });

    const blockScripts = Array.isArray(data.blockScriptsSites) ? data.blockScriptsSites : [];
    blockScripts.forEach((entry, idx) => {
      const domain = normalizeDomain(entry);
      if (!domain || whitelistSet.has(domain)) return;
      addRules.push({
        id: SCRIPT_BLOCK_OFFSET + idx,
        priority: 500,
        action: { type: "block" },
        condition: {
          initiatorDomains: [domain],
          resourceTypes: ["script"]
        }
      });
    });

    const customDomains = Array.isArray(data.customDomains) ? data.customDomains : [];
    customDomains.forEach((entry, idx) => {
      const domain = normalizeDomain(entry);
      if (!domain) return;
      addRules.push({
        id: CUSTOM_OFFSET + idx,
        priority: 10,
        action: { type: "block" },
        condition: {
          urlFilter: `||${domain}^`,
          resourceTypes: [
            "script",
            "image",
            "xmlhttprequest",
            "sub_frame",
            "ping",
            "media",
            "font",
            "stylesheet",
            "other"
          ]
        }
      });
    });

    if (aggressive) {
      addRules.push({
        id: AGGRESSIVE_RULE_ID,
        priority: 5,
        action: { type: "block" },
        condition: {
          urlFilter: "*",
          domainType: "thirdParty",
          resourceTypes: ["ping", "csp_report"]
        }
      });
    }
  }

  await chrome.declarativeNetRequest.updateDynamicRules({
    removeRuleIds,
    addRules
  });
}

function initBadge() {
  if (chrome.declarativeNetRequest.setExtensionActionOptions) {
    chrome.declarativeNetRequest
      .setExtensionActionOptions({ displayActionCountAsBadgeText: true })
      .catch(() => {});
  }
}

chrome.runtime.onInstalled.addListener(async () => {
  const current = await chrome.storage.local.get(null);
  await chrome.storage.local.set({ ...DEFAULT_SETTINGS, ...current });
  initBadge();
  await syncRules();
});

chrome.runtime.onStartup.addListener(() => {
  initBadge();
  syncRules();
});

chrome.storage.onChanged.addListener((changes, area) => {
  if (area !== "local") return;
  if (
    changes.enabled ||
    changes.blockingLevel ||
    changes.annoyancesEnabled ||
    changes.trackingParamsEnabled ||
    changes.whitelist ||
    changes.blockScriptsSites ||
    changes.customDomains
  ) {
    syncRules();
  }
});
