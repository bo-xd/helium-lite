"use strict";

const HOST_NAME = "org.kestrel.settings";

const DEFAULTS = {
  strict_security: true,
  doh_provider: "quad9",
  webrtc_privacy: true,
  search_engine: "duckduckgo",
  ram_profile: "ultra",
  tab_sleeping: true,
  renderer_process_limit: 4,
  v8_max_old_space_mb: 512,
  hardware_accel: true,
  adblocker: true,
  widevine_drm: true,
  extension_support: true,
  custom_ui: true
};

const statusBanner = document.getElementById("statusBanner");
const statusText = document.getElementById("statusText");
const resetBtn = document.getElementById("resetBtn");
const controls = Array.from(document.querySelectorAll("[data-key]"));
const actionBtns = Array.from(document.querySelectorAll(".action-btn[data-url]"));

function setBanner(text, mode = "") {
  statusBanner.classList.remove("saved", "error");
  if (mode) statusBanner.classList.add(mode);
  statusText.textContent = text;
}

function sendNative(payload) {
  return new Promise((resolve, reject) => {
    if (typeof chrome === "undefined" || !chrome.runtime || !chrome.runtime.sendNativeMessage) {
      reject(new Error("Native messaging unavailable outside extension context"));
      return;
    }
    chrome.runtime.sendNativeMessage(HOST_NAME, payload, (resp) => {
      if (chrome.runtime.lastError) {
        reject(new Error(chrome.runtime.lastError.message));
        return;
      }
      resolve(resp);
    });
  });
}

function applyConfig(cfg) {
  const merged = { ...DEFAULTS, ...(cfg || {}) };
  controls.forEach((el) => {
    const key = el.dataset.key;
    const val = merged[key];
    if (el.type === "checkbox") {
      el.checked = Boolean(val);
    } else if (el.type === "number") {
      el.value = String(val);
    } else {
      el.value = String(val);
    }
  });
}

async function loadSettings() {
  try {
    const resp = await sendNative({ action: "get" });
    if (resp && resp.ok === false) {
      throw new Error(resp.error || "Failed to load settings");
    }
    const cfg = resp && resp.config ? resp.config : resp;
    applyConfig(cfg);
    setBanner("Loaded from ~/.config/kestrel/config.json");
  } catch (err) {
    applyConfig(DEFAULTS);
    setBanner(`Native host unavailable (${err.message}) — showing defaults`, "error");
  }
}

controls.forEach((el) => {
  el.addEventListener("change", async () => {
    const key = el.dataset.key;
    let value;
    if (el.type === "checkbox") {
      value = el.checked;
    } else if (el.type === "number") {
      const min = Number(el.min);
      const max = Number(el.max);
      let num = Number(el.value);
      if (Number.isNaN(num)) num = DEFAULTS[key];
      num = Math.max(min, Math.min(max, Math.round(num)));
      el.value = String(num);
      value = num;
    } else {
      value = el.value;
    }

    try {
      const resp = await sendNative({ action: "set", key, value });
      if (resp && resp.ok === false) {
        throw new Error(resp.error || "Failed to save");
      }
      if (resp && resp.config) {
        applyConfig(resp.config);
      }
      setBanner("Saved to config.json — restart Kestrel to apply launch flags", "saved");
    } catch (err) {
      setBanner(`Failed to save ${key}: ${err.message}`, "error");
    }
  });
});

if (resetBtn) {
  resetBtn.addEventListener("click", async () => {
    try {
      const resp = await sendNative({ action: "reset" });
      if (resp && resp.ok === false) {
        throw new Error(resp.error || "Failed to reset");
      }
      applyConfig(resp && resp.config ? resp.config : DEFAULTS);
      setBanner("Reset to defaults in config.json — restart Kestrel to apply launch flags", "saved");
    } catch (err) {
      setBanner(`Failed to reset: ${err.message}`, "error");
    }
  });
}

actionBtns.forEach((btn) => {
  btn.addEventListener("click", () => {
    const url = btn.dataset.url;
    if (!url) return;
    if (typeof chrome !== "undefined" && chrome.tabs && chrome.tabs.create) {
      chrome.tabs.create({ url });
    } else {
      location.href = url;
    }
  });
});

loadSettings();
