"use strict";

// Loaded synchronously in <head>: applies the cached theme before first paint
// so a non-default color never flashes in on Ctrl+T.
(() => {
  const SWATCHES = [
    { id: "kestrel", name: "Kestrel amber", bg: "#111317", tile: "#1a1d24", accent: "#e09f3e" },
    { id: "slate", name: "Slate", bg: "#0f141c", tile: "#18202c", accent: "#6aa9ff" },
    { id: "indigo", name: "Indigo", bg: "#12111a", tile: "#1c1a27", accent: "#8f84f7" },
    { id: "forest", name: "Forest", bg: "#0f1512", tile: "#18221d", accent: "#4fbf6a" },
    { id: "crimson", name: "Crimson", bg: "#161113", tile: "#241b1e", accent: "#e56b6f" },
    { id: "teal", name: "Teal", bg: "#0e1517", tile: "#172226", accent: "#4ecdc4" },
    { id: "bronze", name: "Bronze", bg: "#141210", tile: "#211e1a", accent: "#d4a373" },
    { id: "mono", name: "Monochrome", bg: "#121212", tile: "#1e1e1e", accent: "#d4d4d8" }
  ];
  const CACHE_KEY = "kestrel-ntp-theme";
  const media = window.matchMedia("(prefers-color-scheme: light)");

  function apply(mode, swatchId) {
    const swatch = SWATCHES.find((s) => s.id === swatchId) || SWATCHES[0];
    const root = document.documentElement;
    let resolved = "matte";
    if (mode === "dark") resolved = "dark";
    else if (mode === "system" && media.matches) resolved = "light";
    root.dataset.mode = resolved;
    root.style.setProperty("--swatch-bg", swatch.bg);
    root.style.setProperty("--swatch-tile", swatch.tile);
    root.style.setProperty("--accent", swatch.accent);
  }

  function cache(mode, swatchId) {
    try {
      localStorage.setItem(CACHE_KEY, JSON.stringify({ mode, swatch: swatchId }));
    } catch {
      // Storage unavailable: the async chrome.storage path still applies the theme.
    }
  }

  function cached() {
    try {
      return JSON.parse(localStorage.getItem(CACHE_KEY)) || {};
    } catch {
      return {};
    }
  }

  const initial = cached();
  apply(initial.mode || "matte", initial.swatch || "kestrel");

  window.KestrelTheme = { SWATCHES, apply, cache, cached, media };
})();
