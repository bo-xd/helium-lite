"use strict";

(() => {
  let fpEnabled = true;
  let aggressive = false;
  let shieldsUp = true;

  const noiseSeed = (Math.random() * 255) | 0;
  const host = (location.hostname || "").toLowerCase();
  const isYouTube = host === "youtube.com" || host.endsWith(".youtube.com");

  window.addEventListener("kestrel-shield-config", (e) => {
    if (e && e.detail) {
      fpEnabled = Boolean(e.detail.fp);
      aggressive = Boolean(e.detail.aggressive);
      shieldsUp = e.detail.shieldsUp !== false;
    }
  });

  // 1. Anti-adblock bait property traps (uBlock/Brave scriptlet defense)
  try {
    const baitProps = {
      canRunAds: true,
      isAdBlockActive: false,
      adblockEnabled: false,
      google_ad_status: 1
    };
    for (const [prop, val] of Object.entries(baitProps)) {
      if (!(prop in window)) {
        Object.defineProperty(window, prop, {
          get: () => (shieldsUp ? val : undefined),
          set: () => {},
          configurable: true
        });
      }
    }
  } catch {
    // Ignore restricted window properties
  }

  // 2. YouTube ad-JSON pruning (strips adPlacements, playerAds, adSlots)
  const YT_AD_KEYS = [
    "adPlacements",
    "playerAds",
    "adSlots",
    "adBreakHeartbeatParams"
  ];

  function pruneYouTubeAds(obj) {
    if (!obj || typeof obj !== "object") return obj;
    for (const k of YT_AD_KEYS) {
      if (k in obj) delete obj[k];
    }
    if (obj.playerResponse && typeof obj.playerResponse === "object") {
      for (const k of YT_AD_KEYS) {
        if (k in obj.playerResponse) delete obj.playerResponse[k];
      }
    }
    return obj;
  }

  if (isYouTube) {
    const origJsonParse = JSON.parse;
    JSON.parse = function (...args) {
      const res = origJsonParse.apply(this, args);
      return shieldsUp ? pruneYouTubeAds(res) : res;
    };

    const origRespJson = Response.prototype.json;
    Response.prototype.json = async function (...args) {
      const res = await origRespJson.apply(this, args);
      return shieldsUp ? pruneYouTubeAds(res) : res;
    };

    let ytPlayerResp;
    try {
      Object.defineProperty(window, "ytInitialPlayerResponse", {
        configurable: true,
        get: () => ytPlayerResp,
        set: (val) => {
          ytPlayerResp = shieldsUp ? pruneYouTubeAds(val) : val;
        }
      });
    } catch {
      // Ignore if already locked
    }

    // Fast skip fallback if a player ad state ever triggers
    document.addEventListener(
      "timeupdate",
      () => {
        if (!shieldsUp) return;
        const player = document.querySelector(".html5-video-player.ad-showing");
        if (!player) return;
        const skipBtn = player.querySelector(
          ".ytp-ad-skip-button, .ytp-ad-skip-button-modern, .ytp-skip-ad-button"
        );
        if (skipBtn) skipBtn.click();
        const video = player.querySelector("video");
        if (video && Number.isFinite(video.duration) && video.duration > 0) {
          video.currentTime = video.duration;
        }
      },
      true
    );
  }

  // 3. Canvas fingerprint protection (1-LSB jitter on 1 pixel when read)
  const origGetImageData = CanvasRenderingContext2D.prototype.getImageData;
  CanvasRenderingContext2D.prototype.getImageData = function (...args) {
    const img = origGetImageData.apply(this, args);
    if (fpEnabled && img && img.data && img.data.length >= 4) {
      img.data[0] = img.data[0] ^ (noiseSeed & 1);
    }
    return img;
  };

  const origToDataURL = HTMLCanvasElement.prototype.toDataURL;
  HTMLCanvasElement.prototype.toDataURL = function (...args) {
    if (fpEnabled && this.width > 0 && this.height > 0 && this.width * this.height <= 262144) {
      try {
        const ctx = this.getContext("2d");
        if (ctx) {
          const px = origGetImageData.call(ctx, 0, 0, 1, 1);
          px.data[0] = px.data[0] ^ (noiseSeed & 1);
          ctx.putImageData(px, 0, 0);
        }
      } catch {
        // Ignore tainted canvas
      }
    }
    return origToDataURL.apply(this, args);
  };

  // 4. AudioContext fingerprint protection (sub-audible 1e-7 noise)
  const origGetChannelData = AudioBuffer.prototype.getChannelData;
  AudioBuffer.prototype.getChannelData = function (channel) {
    const data = origGetChannelData.call(this, channel);
    if (fpEnabled && data && data.length > 0) {
      data[0] += ((noiseSeed % 3) - 1) * 1e-7;
    }
    return data;
  };

  // 5. Tracker sendBeacon & <a ping> blocking
  const TRACKER_BEACON_RE =
    /(analytics|telemetry|collect|beacon|pixel|track|metrics|logging|sentry|segment|mixpanel|amplitude|hotjar|doubleclick|youtube\.com\/api\/stats\/ads)/i;

  if (navigator.sendBeacon) {
    const origBeacon = navigator.sendBeacon.bind(navigator);
    navigator.sendBeacon = function (url, data) {
      if (fpEnabled && (aggressive || TRACKER_BEACON_RE.test(String(url)))) {
        return true;
      }
      return origBeacon(url, data);
    };
  }

  document.addEventListener(
    "click",
    (e) => {
      if (!fpEnabled) return;
      const anchor = e.target && e.target.closest ? e.target.closest("a[ping]") : null;
      if (anchor) anchor.removeAttribute("ping");
    },
    true
  );
})();
