"use strict";

(() => {
  const STYLE_ID = "helium-shield-cosmetic";
  const AD_SELECTORS = [
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
    ".OUTBRAIN"
  ];

  const CSS_TEXT = `${AD_SELECTORS.join(",\n")} {\n  display: none !important;\n  visibility: hidden !important;\n}`;

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

  function applyCosmetic(shouldHide) {
    const existing = document.getElementById(STYLE_ID);
    if (!shouldHide) {
      if (existing) existing.remove();
      return;
    }
    if (existing) return;
    const style = document.createElement("style");
    style.id = STYLE_ID;
    style.textContent = CSS_TEXT;
    (document.head || document.documentElement).appendChild(style);
  }

  function update() {
    const host = normalizeHost(location.hostname);
    chrome.storage.local.get(
      { enabled: true, cosmeticEnabled: true, whitelist: [] },
      (data) => {
        if (chrome.runtime.lastError) return;
        const active =
          data.enabled !== false &&
          data.cosmeticEnabled !== false &&
          !isWhitelisted(host, data.whitelist);
        applyCosmetic(active);
      }
    );
  }

  update();

  chrome.storage.onChanged.addListener((changes, area) => {
    if (area !== "local") return;
    if (changes.enabled || changes.cosmeticEnabled || changes.whitelist) {
      update();
    }
  });
})();
