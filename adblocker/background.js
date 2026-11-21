"use strict";

const DEFAULT_SETTINGS = {
  enabled: true,
  cosmeticEnabled: true,
  whitelist: [],
  customDomains: []
};

const RULESET_ID = "ruleset_1";
const WHITELIST_RULE_OFFSET = 100000;
const CUSTOM_RULE_OFFSET = 200000;

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
  const whitelist = Array.isArray(data.whitelist) ? data.whitelist : [];
  const customDomains = Array.isArray(data.customDomains) ? data.customDomains : [];

  await chrome.declarativeNetRequest.updateEnabledRulesets({
    enableRulesetIds: enabled ? [RULESET_ID] : [],
    disableRulesetIds: enabled ? [] : [RULESET_ID]
  });

  const existing = await chrome.declarativeNetRequest.getDynamicRules();
  const removeRuleIds = existing.map((r) => r.id);
  const addRules = [];

  if (enabled) {
    whitelist.forEach((entry, idx) => {
      const domain = normalizeDomain(entry);
      if (!domain) return;
      addRules.push({
        id: WHITELIST_RULE_OFFSET + idx,
        priority: 100,
        action: { type: "allowAllRequests" },
        condition: {
          urlFilter: `||${domain}^`,
          resourceTypes: ["main_frame", "sub_frame"]
        }
      });
    });

    customDomains.forEach((entry, idx) => {
      const domain = normalizeDomain(entry);
      if (!domain) return;
      addRules.push({
        id: CUSTOM_RULE_OFFSET + idx,
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
  }

  await chrome.declarativeNetRequest.updateDynamicRules({
    removeRuleIds,
    addRules
  });
}

chrome.runtime.onInstalled.addListener(async () => {
  const current = await chrome.storage.local.get(null);
  const initial = { ...DEFAULT_SETTINGS, ...current };
  await chrome.storage.local.set(initial);
  await syncRules();
});

chrome.runtime.onStartup.addListener(() => {
  syncRules();
});

chrome.storage.onChanged.addListener((changes, area) => {
  if (area !== "local") return;
  if (changes.enabled || changes.whitelist || changes.customDomains) {
    syncRules();
  }
});
