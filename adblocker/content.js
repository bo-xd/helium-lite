"use strict";

(() => {
  const STYLE_ID = "kestrel-shield-cosmetic";

  const CORE_SELECTORS = [
    "ins.adsbygoogle",
    "iframe[id^='google_ads_iframe']",
    "iframe[src*='doubleclick.net']",
    "iframe[src*='googlesyndication.com']",
    "div[id^='div-gpt-ad']",
    "div[data-google-query-id]",
    "[data-ad-slot]",
    "[data-ad-unit]",
    "[data-testid='ad']",
    ".ad-banner",
    ".ad-container",
    ".ad-wrapper",
    ".ad-slot",
    ".ad-unit",
    ".advertisement",
    ".sponsored-content",
    "#taboola-below-article-thumbnails",
    ".trc_rbox_outer",
    ".OUTBRAIN",
    "#onetrust-consent-sdk",
    "#CybotCookiebotDialog"
  ];

  let cosmeticData = null;

  function normalizeHost(host) {
    return (host || "").toLowerCase().replace(/^www\./, "");
  }

  function isWhitelisted(host, list) {
    if (!Array.isArray(list) || !host) return false;
    return list.some((entry) => {
      const domain = normalizeHost(entry);
      return domain && (host === domain || host.endsWith("." + domain));
    });
  }

  function buildCssForHost(host) {
    const selectors = new Set(CORE_SELECTORS);
    if (cosmeticData) {
      if (Array.isArray(cosmeticData.global)) {
        for (const s of cosmeticData.global) selectors.add(s);
      }
      if (host && cosmeticData.domains) {
        const parts = host.split(".");
        for (let i = 0; i < parts.length - 1; i++) {
          const candidate = parts.slice(i).join(".");
          const domainSels = cosmeticData.domains[candidate];
          if (Array.isArray(domainSels)) {
            for (const s of domainSels) selectors.add(s);
          }
        }
      }
    }
    return `${Array.from(selectors).join(",\n")} {\n  display: none !important;\n  visibility: hidden !important;\n}`;
  }

  function applyCosmetic(shouldHide, host) {
    let style = document.getElementById(STYLE_ID);
    if (!shouldHide) {
      if (style) style.remove();
      return;
    }
    if (!style) {
      style = document.createElement("style");
      style.id = STYLE_ID;
      (document.head || document.documentElement).appendChild(style);
    }
    style.textContent = buildCssForHost(host);
  }

  async function ensureCosmeticRules() {
    if (cosmeticData) return;
    try {
      const url = chrome.runtime.getURL("rules/cosmetic.json");
      const resp = await fetch(url);
      if (resp.ok) {
        cosmeticData = await resp.json();
      }
    } catch {
      // Fallback to CORE_SELECTORS
    }
  }

  function update() {
    const host = normalizeHost(location.hostname);
    chrome.storage.local.get(
      {
        enabled: true,
        cosmeticEnabled: true,
        antiFingerprintEnabled: true,
        blockingLevel: "standard",
        whitelist: []
      },
      async (data) => {
        if (chrome.runtime.lastError) return;
        const shieldsUp = data.enabled !== false && !isWhitelisted(host, data.whitelist);
        const showCosmetic = shieldsUp && data.cosmeticEnabled !== false;

        window.dispatchEvent(
          new CustomEvent("kestrel-shield-config", {
            detail: {
              fp: shieldsUp && data.antiFingerprintEnabled !== false,
              aggressive: shieldsUp && data.blockingLevel === "aggressive",
              shieldsUp
            }
          })
        );

        applyCosmetic(showCosmetic, host);
        if (showCosmetic && !cosmeticData) {
          await ensureCosmeticRules();
          applyCosmetic(true, host);
        }
      }
    );
  }

  update();

  chrome.storage.onChanged.addListener((changes, area) => {
    if (area !== "local") return;
    if (
      changes.enabled ||
      changes.cosmeticEnabled ||
      changes.antiFingerprintEnabled ||
      changes.blockingLevel ||
      changes.whitelist
    ) {
      update();
    }
  });
})();
