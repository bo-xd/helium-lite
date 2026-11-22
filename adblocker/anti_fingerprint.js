"use strict";

(() => {
  let fpEnabled = true;
  let aggressive = false;

  // Per-session deterministic tiny jitter (-1, 0, or +1 LSB)
  const noiseSeed = (Math.random() * 255) | 0;

  window.addEventListener("kestrel-shield-config", (e) => {
    if (e && e.detail) {
      fpEnabled = Boolean(e.detail.fp);
      aggressive = Boolean(e.detail.aggressive);
    }
  });

  // 1. Canvas fingerprint protection (1-LSB jitter on 1 pixel when read)
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

  // 2. AudioContext fingerprint protection (sub-audible 1e-7 noise)
  const origGetChannelData = AudioBuffer.prototype.getChannelData;
  AudioBuffer.prototype.getChannelData = function (channel) {
    const data = origGetChannelData.call(this, channel);
    if (fpEnabled && data && data.length > 0) {
      data[0] += ((noiseSeed % 3) - 1) * 1e-7;
    }
    return data;
  };

  // 3. Tracker sendBeacon & <a ping> blocking
  const TRACKER_BEACON_RE =
    /(analytics|telemetry|collect|beacon|pixel|track|metrics|logging|sentry|segment|mixpanel|amplitude|hotjar|doubleclick)/i;

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
