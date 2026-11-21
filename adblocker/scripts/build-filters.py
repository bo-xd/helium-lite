#!/usr/bin/env python3
"""Build compact MV3 declarativeNetRequest ruleset.json from filter lists."""

import argparse
import json
import re
import sys
import urllib.request
from pathlib import Path

LIST_URLS = {
    "easylist": "https://easylist.to/easylist/easylist.txt",
    "easyprivacy": "https://easylist.to/easylist/easyprivacy.txt",
    "peterlowe": "https://pgl.yoyo.org/adservers/serverlist.php?hostformat=hosts&showintro=0&mimetype=plaintext",
}

RESOURCE_TYPES = [
    "script",
    "image",
    "xmlhttprequest",
    "sub_frame",
    "ping",
    "media",
    "font",
    "stylesheet",
    "other",
]

# Curated high-impact ad, tracker, telemetry, and analytics domains for offline/default builds.
BUILTIN_DOMAINS = [
    "doubleclick.net",
    "googlesyndication.com",
    "googleadservices.com",
    "google-analytics.com",
    "googletagmanager.com",
    "googletagservices.com",
    "adservice.google.com",
    "pagead2.googlesyndication.com",
    "tpc.googlesyndication.com",
    "analytics.google.com",
    "connect.facebook.net",
    "pixel.facebook.com",
    "an.facebook.com",
    "amazon-adsystem.com",
    "aax.amazon-adsystem.com",
    "fls-na.amazon-adsystem.com",
    "adnxs.com",
    "ib.adnxs.com",
    "rubiconproject.com",
    "fastlane.rubiconproject.com",
    "pubmatic.com",
    "ads.pubmatic.com",
    "openx.net",
    "rtb.openx.net",
    "casalemedia.com",
    "criteo.com",
    "criteo.net",
    "taboola.com",
    "cdn.taboola.com",
    "trc.taboola.com",
    "outbrain.com",
    "widgets.outbrain.com",
    "scorecardresearch.com",
    "sb.scorecardresearch.com",
    "quantserve.com",
    "pixel.quantserve.com",
    "chartbeat.com",
    "static.chartbeat.com",
    "hotjar.com",
    "static.hotjar.com",
    "mixpanel.com",
    "api.mixpanel.com",
    "segment.io",
    "api.segment.io",
    "cdn.segment.com",
    "amplitude.com",
    "api.amplitude.com",
    "api2.amplitude.com",
    "sentry.io",
    "bugsnag.com",
    "notify.bugsnag.com",
    "fullstory.com",
    "rs.fullstory.com",
    "mouseflow.com",
    "crazyegg.com",
    "script.crazyegg.com",
    "optimizely.com",
    "cdn.optimizely.com",
    "adform.net",
    "track.adform.net",
    "adsrvr.org",
    "match.adsrvr.org",
    "bidswitch.net",
    "tapad.com",
    "pixel.tapad.com",
    "rlcdn.com",
    "bluekai.com",
    "tags.bluekai.com",
    "demdex.net",
    "dpm.demdex.net",
    "omtrdc.net",
    "2o7.net",
    "krxd.net",
    "exelator.com",
    "agkn.com",
    "mathtag.com",
    "pixel.mathtag.com",
    "media.net",
    "contextual.media.net",
    "sharethrough.com",
    "triplelift.com",
    "eb2.3lift.com",
    "smartadserver.com",
    "indexww.com",
    "33across.com",
    "gumgum.com",
    "teads.tv",
    "a.teads.tv",
    "zemanta.com",
    "revcontent.com",
    "mgid.com",
    "adroll.com",
    "d.adroll.com",
    "serving-sys.com",
    "bs.serving-sys.com",
    "flashtalking.com",
    "sizmek.com",
    "adtech.de",
    "advertising.com",
    "atdmt.com",
    "yieldmo.com",
    "spotxchange.com",
    "springserve.com",
    "innovid.com",
    "moatads.com",
    "z.moatads.com",
    "doubleverify.com",
    "tps.doubleverify.com",
    "adsafeprotected.com",
    "pixel.adsafeprotected.com",
    "branch.io",
    "api.branch.io",
    "app.link",
    "adjust.com",
    "app.adjust.com",
    "appsflyer.com",
    "t.appsflyer.com",
    "kochava.com",
    "singular.net",
    "braze.com",
    "onesignal.com",
    "intercom.io",
    "widget.intercom.io",
    "drift.com",
    "js.driftt.com",
    "hubspot.com",
    "js.hs-analytics.net",
    "js.hs-scripts.com",
    "marketo.net",
    "munchkin.marketo.net",
    "pardot.com",
    "pi.pardot.com",
    "bing.com",
    "bat.bing.com",
    "ads.linkedin.com",
    "px.ads.linkedin.com",
    "snap.licdn.com",
    "ads-twitter.com",
    "static.ads-twitter.com",
    "analytics.twitter.com",
    "t.co",
    "sc-static.net",
    "tr.snapchat.com",
    "analytics.tiktok.com",
    "business-api.tiktok.com",
    "ads.tiktok.com",
    "ct.pinterest.com",
    "log.pinterest.com",
    "ads.reddit.com",
    "alb.reddit.com",
    "events.redditmedia.com",
    "clarity.ms",
    "www.clarity.ms",
    "newrelic.com",
    "bam.nr-data.net",
    "js-agent.newrelic.com",
    "heapanalytics.com",
    "cdn.heapanalytics.com",
    "logrocket.com",
    "cdn.logrocket.io",
    "pendo.io",
    "cdn.pendo.io",
    "kissmetrics.com",
    "trk.kissmetrics.com",
    "statcounter.com",
    "c.statcounter.com",
    "clicky.com",
    "in.getclicky.com",
    "liveintent.com",
    "liadm.com",
    "idx.liadm.com",
    "id5-sync.com",
    "crwdcntrl.net",
    "b.crwdcntrl.net",
    "lotame.com",
    "adobedtm.com",
    "assets.adobedtm.com",
    "tiqcdn.com",
    "tags.tiqcdn.com",
    "ensighten.com",
    "nexus.ensighten.com",
    "bounceexchange.com",
    "tag.bounceexchange.com",
    "klaviyo.com",
    "static.klaviyo.com",
    "attentivemobile.com",
    "cdn.attn.tv",
    "yotpo.com",
    "staticw2.yotpo.com",
    "trustpilot.com",
    "widget.trustpilot.com",
    "popads.net",
    "popcash.net",
    "propellerads.com",
    "exoclick.com",
    "juicyads.com",
    "trafficjunky.net",
]

# Never block top-level multi-purpose domains even if accidentally listed.
SAFE_EXCLUSIONS = {
    "bing.com",
    "t.co",
    "google.com",
    "facebook.com",
    "amazon.com",
    "github.com",
    "cloudflare.com",
}

ABP_DOMAIN_RE = re.compile(r"^\|\|([a-z0-9.-]+\.[a-z]{2,})\^(?:\$([^,\s]+(?:,[^,\s]+)*))?$", re.IGNORECASE)
HOSTS_RE = re.compile(r"^(?:0\.0\.0\.0|127\.0\.0\.1)\s+([a-z0-9.-]+\.[a-z]{2,})(?:\s|#|$)", re.IGNORECASE)


def fetch_list(url: str, timeout: int = 15) -> str:
    req = urllib.request.Request(url, headers={"User-Agent": "HeliumShield-FilterBuilder/1.0"})
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        return resp.read().decode("utf-8", errors="ignore")


def parse_domains(text: str) -> list[tuple[str, bool]]:
    """Extract (domain, third_party_only) pairs from ABP or hosts format."""
    results: list[tuple[str, bool]] = []
    for raw_line in text.splitlines():
        line = raw_line.strip()
        if not line or line.startswith(("!", "#", "[")):
            continue
        m_abp = ABP_DOMAIN_RE.match(line)
        if m_abp:
            domain = m_abp.group(1).lower()
            opts = (m_abp.group(2) or "").lower().split(",") if m_abp.group(2) else []
            if any(o.startswith("~") or o in {"elemhide", "generichide", "document", "popup"} for o in opts):
                continue
            third_party = "third-party" in opts or "3p" in opts
            results.append((domain, third_party))
            continue
        m_host = HOSTS_RE.match(line)
        if m_host:
            results.append((m_host.group(1).lower(), False))
    return results


def build_rules(domains: list[tuple[str, bool]], max_rules: int) -> list[dict]:
    seen: set[str] = set()
    rules: list[dict] = []
    for domain, third_party in domains:
        if domain in SAFE_EXCLUSIONS or domain in seen:
            continue
        seen.add(domain)
        condition: dict = {
            "urlFilter": f"||{domain}^",
            "resourceTypes": RESOURCE_TYPES,
        }
        if third_party:
            condition["domainType"] = "thirdParty"
        rules.append(
            {
                "id": len(rules) + 1,
                "priority": 1,
                "action": {"type": "block"},
                "condition": condition,
            }
        )
        if len(rules) >= max_rules:
            break
    return rules


def main() -> int:
    parser = argparse.ArgumentParser(description="Build Helium Shield ruleset.json")
    parser.add_argument(
        "--output",
        default=str(Path(__file__).resolve().parent.parent / "rules" / "ruleset.json"),
        help="Output path for ruleset.json",
    )
    parser.add_argument(
        "--max-rules",
        type=int,
        default=25000,
        help="Maximum static rules to emit (Chromium MV3 limit is 30000 per ruleset)",
    )
    parser.add_argument(
        "--offline",
        action="store_true",
        help="Skip network fetch and build from built-in curated list only",
    )
    args = parser.parse_args()

    entries: list[tuple[str, bool]] = [(d, False) for d in BUILTIN_DOMAINS]

    if not args.offline:
        for name, url in LIST_URLS.items():
            try:
                print(f"Fetching {name}...", file=sys.stderr)
                content = fetch_list(url)
                parsed = parse_domains(content)
                print(f"  parsed {len(parsed)} domain rules from {name}", file=sys.stderr)
                entries.extend(parsed)
            except Exception as exc:
                print(f"  warning: failed to fetch {name}: {exc}", file=sys.stderr)

    rules = build_rules(entries, args.max_rules)
    out_path = Path(args.output)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(rules, separators=(",", ":")), encoding="utf-8")
    print(f"Wrote {len(rules)} rules to {out_path}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
