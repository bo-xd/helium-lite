use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const ADS_URLS: &[(&str, &str)] = &[
    (
        "ublock-quick-fixes",
        "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/quick-fixes.txt",
    ),
    (
        "ublock-filters",
        "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/filters.txt",
    ),
    (
        "ublock-unbreak",
        "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/unbreak.txt",
    ),
    (
        "peterlowe",
        "https://pgl.yoyo.org/adservers/serverlist.php?hostformat=hosts&showintro=0&mimetype=plaintext",
    ),
    ("easylist", "https://easylist.to/easylist/easylist.txt"),
];

const PRIVACY_URLS: &[(&str, &str)] = &[
    (
        "ublock-privacy",
        "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/privacy.txt",
    ),
    ("easyprivacy", "https://easylist.to/easylist/easyprivacy.txt"),
];

const MALWARE_URLS: &[(&str, &str)] = &[
    (
        "ublock-badware",
        "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/badware.txt",
    ),
    (
        "urlhaus",
        "https://malware-filter.gitlab.io/malware-filter/urlhaus-filter-online.txt",
    ),
];

const ANNOYANCES_URLS: &[(&str, &str)] = &[
    (
        "ublock-cookies",
        "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/annoyances-cookies.txt",
    ),
    (
        "fanboy-cookie",
        "https://secure.fanboy.co.nz/fanboy-cookiemonster.txt",
    ),
    (
        "fanboy-annoyance",
        "https://easylist.to/easylist/fanboy-annoyance.txt",
    ),
];

const DEFAULT_RESOURCE_TYPES: &[&str] = &[
    "script",
    "image",
    "xmlhttprequest",
    "sub_frame",
    "ping",
    "media",
    "font",
    "stylesheet",
    "other",
];

const MALWARE_RESOURCE_TYPES: &[&str] = &[
    "main_frame",
    "sub_frame",
    "script",
    "image",
    "xmlhttprequest",
    "ping",
    "media",
    "font",
    "stylesheet",
    "other",
];

const TRACKING_PARAMS: &[&str] = &[
    "utm_source",
    "utm_medium",
    "utm_campaign",
    "utm_term",
    "utm_content",
    "utm_id",
    "utm_name",
    "utm_reader",
    "utm_viz_id",
    "utm_pubreferrer",
    "fbclid",
    "gclid",
    "dclid",
    "gbraid",
    "wbraid",
    "mc_cid",
    "mc_eid",
    "igshid",
    "si",
    "_hsenc",
    "_hsmi",
    "yclid",
    "msclkid",
    "wickedid",
    "twclid",
    "ttclid",
    "ref_src",
    "s_kwcid",
    "oly_anon_id",
    "oly_enc_id",
    "vero_id",
    "mkt_tok",
];

const SAFE_EXCLUSIONS: &[&str] = &[
    "bing.com",
    "t.co",
    "google.com",
    "facebook.com",
    "amazon.com",
    "github.com",
    "cloudflare.com",
    "youtube.com",
    "wikipedia.org",
];

const BUILTIN_ADS: &[&str] = &[
    "doubleclick.net",
    "googlesyndication.com",
    "googleadservices.com",
    "adservice.google.com",
    "pagead2.googlesyndication.com",
    "tpc.googlesyndication.com",
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
    "adform.net",
    "adsrvr.org",
    "bidswitch.net",
    "media.net",
    "sharethrough.com",
    "triplelift.com",
    "smartadserver.com",
    "indexww.com",
    "33across.com",
    "gumgum.com",
    "teads.tv",
    "revcontent.com",
    "mgid.com",
    "adroll.com",
    "serving-sys.com",
    "flashtalking.com",
    "sizmek.com",
    "advertising.com",
    "yieldmo.com",
    "spotxchange.com",
    "springserve.com",
    "innovid.com",
    "ads.linkedin.com",
    "ads-twitter.com",
    "ads.tiktok.com",
    "ads.reddit.com",
    "popads.net",
    "popcash.net",
    "propellerads.com",
    "exoclick.com",
    "juicyads.com",
    "trafficjunky.net",
    "analyticsengine.s3.amazonaws.com",
    "analytics.s3.amazonaws.com",
    "gemini.yahoo.com",
    "adtech.yahooinc.com",
    "offerwall.yandex.net",
    "adfox.yandex.ru",
    "unityads.unity3d.com",
    "auction.unityads.unity3d.com",
    "webview.unityads.unity3d.com",
    "config.unityads.unity3d.com",
    "adserver.unityads.unity3d.com",
    "bdapi-ads.realmemobile.com",
    "bdapi-in-ads.realmemobile.com",
    "adsfs.oppomobile.com",
    "ads.oppomobile.com",
    "adx.ads.oppomobile.com",
    "ck.ads.oppomobile.com",
    "data.ads.oppomobile.com",
    "iadsdk.apple.com",
    "api-adservices.apple.com",
    "advertising-api-eu.amazon.com",
    "aan.amazon.com",
    "fls-na.amazon.com",
    "bingads.microsoft.com",
    "ads.microsoft.com",
    "c.bing.com",
    "liftoff.io",
    "advertising.yahoo.com",
    "mc.yandex.ru",
    "advertising.yandex.ru",
    "api.fyber.com",
    "ironsource.mobi",
    "s.youtube.com",
    "redirector.googlevideo.com",
    "pangleglobal.com",
    "mineralt.io",
    "clickadu.com",
    "dai.google.com",
    "g.jwpsrv.com",
    "ssl.p.jwpcdn.com",
    "prd.jwpltx.com",
    "mssl.fwmrm.net",
    "bea4.v.fwmrm.net",
    "2975c.v.fwmrm.net",
    "metrics.brightcove.com",
    "partnerstack.com",
    "api.partnerstack.com",
];

const BUILTIN_AD_PATHS: &[&str] = &[
    "/pagead.js",
    "/widget/ads.js",
    "/js/pagead.js",
    "/js/widget/ads.js",
    "/pagead/js/",
    "||vk.com/rtrg",
];

const BUILTIN_PRIVACY: &[&str] = &[
    "google-analytics.com",
    "googletagmanager.com",
    "googletagservices.com",
    "analytics.google.com",
    "connect.facebook.net",
    "pixel.facebook.com",
    "an.facebook.com",
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
    "sentry.io",
    "bugsnag.com",
    "fullstory.com",
    "mouseflow.com",
    "crazyegg.com",
    "optimizely.com",
    "tapad.com",
    "rlcdn.com",
    "bluekai.com",
    "demdex.net",
    "omtrdc.net",
    "2o7.net",
    "krxd.net",
    "exelator.com",
    "agkn.com",
    "mathtag.com",
    "moatads.com",
    "doubleverify.com",
    "adsafeprotected.com",
    "branch.io",
    "adjust.com",
    "appsflyer.com",
    "kochava.com",
    "singular.net",
    "braze.com",
    "onesignal.com",
    "intercom.io",
    "drift.com",
    "js.hs-analytics.net",
    "js.hs-scripts.com",
    "marketo.net",
    "pardot.com",
    "bat.bing.com",
    "px.ads.linkedin.com",
    "snap.licdn.com",
    "analytics.twitter.com",
    "sc-static.net",
    "tr.snapchat.com",
    "analytics.tiktok.com",
    "ct.pinterest.com",
    "alb.reddit.com",
    "events.redditmedia.com",
    "clarity.ms",
    "newrelic.com",
    "bam.nr-data.net",
    "heapanalytics.com",
    "logrocket.com",
    "pendo.io",
    "kissmetrics.com",
    "statcounter.com",
    "clicky.com",
    "liveintent.com",
    "liadm.com",
    "id5-sync.com",
    "crwdcntrl.net",
    "lotame.com",
    "adobedtm.com",
    "tiqcdn.com",
    "ensighten.com",
    "bounceexchange.com",
    "klaviyo.com",
    "attentivemobile.com",
    "browser.sentry-cdn.com",
    "app.getsentry.com",
    "ads-api.twitter.com",
    "events.reddit.com",
    "ads-api.tiktok.com",
    "ads-sg.tiktok.com",
    "analytics-sg.tiktok.com",
    "business-api.tiktok.com",
    "log.byteoversea.com",
    "geo.yahoo.com",
    "udcm.yahoo.com",
    "analytics.query.yahoo.com",
    "log.fc.yahoo.com",
    "extmaps-api.yandex.net",
    "appmetrica.yandex.ru",
    "adfstat.yandex.ru",
    "metrika.yandex.ru",
    "iot-eu-logser.realme.com",
    "iot-logser.realme.com",
    "mistat.xiaomi.com",
    "data.mistat.xiaomi.com",
    "data.mistat.india.xiaomi.com",
    "data.mistat.rus.xiaomi.com",
    "mistat.india.xiaomi.com",
    "mistat.rus.xiaomi.com",
    "tracking.rus.miui.com",
    "metrics.data.hicloud.com",
    "metrics2.data.hicloud.com",
    "grs.hicloud.com",
    "logservice.hicloud.com",
    "logservice1.hicloud.com",
    "logbak.hicloud.com",
    "click.oneplus.cn",
    "smetrics.samsung.com",
    "nmetrics.samsung.com",
    "analytics-api.samsunghealthcn.com",
    "books-analytics-events.apple.com",
    "weather-analytics-events.apple.com",
    "notes-analytics-events.apple.com",
    "tagmanager.google.com",
    "analytics.adobe.io",
    "metrics.adobe.com",
    "app.posthog.com",
    "eu.posthog.com",
    "us.i.posthog.com",
    "rudderstack.com",
    "snowplowanalytics.com",
    "tracker.snowplowanalytics.com",
    "fingerprintjs.com",
    "onetag-sys.com",
    "bnc.lt",
    "browser-intake-datadoghq.com",
    "rum.browser-intake-datadoghq.com",
    "graph.facebook.com",
    "tr.facebook.com",
    "graph.instagram.com",
    "i.instagram.com",
    "ads.snapchat.com",
    "ads-api.x.com",
    "analytics.x.com",
    "ads.x.com",
    "pixel.redditmedia.com",
    "d.reddit.com",
    "mon.tiktokv.com",
    "widgets.pinterest.com",
    "pixel.quora.com",
    "qevents.quora.com",
    "px.srvcs.tumblr.com",
    "ads.vk.com",
    "ad.mail.ru",
    "top-fwz1.mail.ru",
    "advertising.apple.com",
    "metrics.apple.com",
    "xp.apple.com",
    "ads.huawei.com",
    "adlog.vivo.com",
    "ads-api.vivo.com",
    "a.lenovo.com",
    "us.ibs.lgappstv.com",
    "ngfts.lge.com",
    "yumenetworks.com",
    "smartclip.com",
    "settings-win.data.microsoft.com",
    "vortex.data.microsoft.com",
    "vortex-win.data.microsoft.com",
    "watson.telemetry.microsoft.com",
    "telemetry.microsoft.com",
    "browser.events.data.msn.com",
    "device-metrics-us.amazon.com",
    "device-metrics-us-2.amazon.com",
    "mads-eu.amazon.com",
    "ads.roku.com",
    "amoeba.web.roku.com",
    "ads.vizio.com",
    "tvinteractive.tv",
    "cdn.dynamicyield.com",
    "events.launchdarkly.com",
    "clientstream.launchdarkly.com",
    "list-manage.com",
    "track.hubspot.com",
    "click.mailchimp.com",
    "js.driftt.com",
];

const BUILTIN_MALWARE: &[&str] = &[
    "coinhive.com",
    "coin-hive.com",
    "authedmine.com",
    "cryptoloot.pro",
    "webminepool.com",
];

const BUILTIN_ANNOYANCES: &[&str] = &[
    "cdn.cookielaw.org",
    "geolocation.onetrust.com",
    "consent.cookiebot.com",
    "consent.trustarc.com",
    "app.termly.io",
    "cdn.privacy-mgmt.com",
];

const BUILTIN_COSMETIC_GLOBAL: &[&str] = &[
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
    ".adsbox",
    ".textads",
    ".banner_ads",
    ".banner-ads",
    ".adbox",
    ".adbox-wrapper",
    ".adSocial",
    ".advertisement",
    ".sponsored-content",
    "#taboola-below-article-thumbnails",
    ".trc_rbox_outer",
    ".OUTBRAIN",
    "#onetrust-consent-sdk",
    "#CybotCookiebotDialog",
    ".cc-window",
    ".cookie-banner",
    "#cookie-banner",
];

const BUILTIN_COSMETIC_YOUTUBE: &[&str] = &[
    "ytd-ad-slot-renderer",
    "ytd-in-feed-ad-layout-renderer",
    "ytd-banner-promo-renderer",
    "ytd-DisplayAdRenderer",
    "ytd-statement-banner-renderer",
    "ytd-promoted-sparkles-web-renderer",
    "ytd-promoted-video-renderer",
    "ytd-compact-promoted-video-renderer",
    "ytd-player-legacy-desktop-watch-ads-renderer",
    "ytd-engagement-panel-section-list-renderer[target-id='engagement-panel-ads']",
    "#masthead-ad",
    "#player-ads",
    ".ytp-ad-overlay-container",
    ".ytp-ad-message-container",
    ".ytp-ad-image-overlay",
    "ytd-merch-shelf-renderer",
];

#[derive(Clone, Eq, PartialEq, Hash)]
struct ParsedRule {
    action_type: &'static str,
    priority: u8,
    url_filter: String,
    domain_type: Option<&'static str>,
    resource_types: Vec<&'static str>,
}

struct CosmeticStore {
    global: Vec<String>,
    global_seen: HashSet<String>,
    domains: BTreeMap<String, Vec<String>>,
    domain_seen: HashSet<String>,
}

impl CosmeticStore {
    fn new() -> Self {
        let mut s = Self {
            global: Vec::new(),
            global_seen: HashSet::new(),
            domains: BTreeMap::new(),
            domain_seen: HashSet::new(),
        };
        for &sel in BUILTIN_COSMETIC_GLOBAL {
            s.add_global(sel);
        }
        for &sel in BUILTIN_COSMETIC_YOUTUBE {
            s.add_domain("youtube.com", sel);
        }
        s
    }

    fn add_global(&mut self, sel: &str) {
        if self.global.len() >= 1500 {
            return;
        }
        if self.global_seen.insert(sel.to_string()) {
            self.global.push(sel.to_string());
        }
    }

    fn add_domain(&mut self, domain: &str, sel: &str) {
        if self.domain_seen.len() >= 1800 {
            return;
        }
        let key = format!("{domain}##{sel}");
        if self.domain_seen.insert(key) {
            self.domains
                .entry(domain.to_string())
                .or_default()
                .push(sel.to_string());
        }
    }
}

fn is_valid_domain(s: &str) -> bool {
    if s.len() < 4 || s.starts_with('.') || s.ends_with('.') || !s.contains('.') {
        return false;
    }
    let tld = s.rsplit('.').next().unwrap_or("");
    if tld.len() < 2 || !tld.chars().all(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
}

fn is_valid_css_selector(sel: &str) -> bool {
    if sel.len() < 2 || sel.len() > 120 || !sel.is_ascii() {
        return false;
    }
    if sel.contains(":has-text")
        || sel.contains(":-abp-")
        || sel.contains(":xpath")
        || sel.contains(":upward")
        || sel.contains(":style")
        || sel.contains(":matches-css")
        || sel.contains(":matches-path")
        || sel.contains(":min-text-length")
        || sel.contains(":remove")
        || sel.contains("+js(")
        || sel.contains("##")
        || sel.contains('{')
        || sel.contains('}')
        || sel.contains('\\')
        || sel.contains('"')
    {
        return false;
    }
    true
}

fn parse_cosmetic_line(line: &str, store: &mut CosmeticStore) {
    if line.contains("#@#") || line.contains("#?#") || line.contains("#$#") {
        return;
    }
    let Some(pos) = line.find("##") else {
        return;
    };
    let left = line[..pos].trim();
    let right = line[pos + 2..].trim();
    if !is_valid_css_selector(right) {
        return;
    }
    if left.is_empty() {
        store.add_global(right);
        return;
    }
    for d in left.split(',') {
        let d = d.trim().to_ascii_lowercase();
        if d.starts_with('~') || d.ends_with(".*") {
            continue;
        }
        if is_valid_domain(&d) {
            store.add_domain(&d, right);
        }
    }
}

fn map_resource_type(opt: &str) -> Option<&'static str> {
    match opt {
        "script" => Some("script"),
        "image" => Some("image"),
        "xmlhttprequest" | "xhr" => Some("xmlhttprequest"),
        "subdocument" | "sub_frame" => Some("sub_frame"),
        "stylesheet" | "css" => Some("stylesheet"),
        "font" => Some("font"),
        "media" => Some("media"),
        "ping" | "beacon" => Some("ping"),
        "websocket" => Some("websocket"),
        "other" => Some("other"),
        _ => None,
    }
}

fn parse_abp_line(line: &str, default_types: &[&'static str]) -> Option<ParsedRule> {
    if line.starts_with("@@")
        || line.contains("##")
        || line.contains("#@#")
        || line.contains("#?#")
        || line.contains("#$#")
        || line.contains("$@$")
        || !line.is_ascii()
    {
        return None;
    }

    let (is_allow, body) = (false, line);

    let (pattern_raw, opts_raw) = match body.rfind('$') {
        Some(pos) if pos > 0 => (&body[..pos], Some(&body[pos + 1..])),
        _ => (body, None),
    };

    if pattern_raw.starts_with('/') && pattern_raw.ends_with('/') && pattern_raw.len() > 2 {
        return None;
    }

    let mut domain_type: Option<&'static str> = None;
    let mut rtypes: Vec<&'static str> = Vec::new();

    if let Some(opts) = opts_raw {
        if opts.chars().any(char::is_whitespace) {
            return None;
        }
        for opt in opts.split(',') {
            let o = opt.to_ascii_lowercase();
            if o.is_empty() || o == "important" {
                continue;
            }
            if o.starts_with('~')
                || o.starts_with("domain=")
                || o.starts_with("from=")
                || o.starts_with("to=")
                || o.starts_with("denyallow=")
                || o.starts_with("redirect")
                || o.starts_with("removeparam")
                || o.starts_with("csp")
                || o.starts_with("header")
                || matches!(
                    o.as_str(),
                    "elemhide"
                        | "generichide"
                        | "specifichide"
                        | "ehide"
                        | "ghide"
                        | "shide"
                        | "document"
                        | "doc"
                        | "popup"
                        | "popunder"
                        | "badfilter"
                        | "all"
                )
            {
                return None;
            }
            if o == "third-party" || o == "3p" {
                domain_type = Some("thirdParty");
            } else if o == "first-party" || o == "1p" {
                domain_type = Some("firstParty");
            } else if let Some(rt) = map_resource_type(&o) {
                if !rtypes.contains(&rt) {
                    rtypes.push(rt);
                }
            } else {
                return None;
            }
        }
    }

    if rtypes.is_empty() {
        rtypes.extend_from_slice(default_types);
    }

    let action_type = if is_allow { "allow" } else { "block" };
    let priority = if is_allow { 2 } else { 1 };

    if let Some(rest) = pattern_raw.strip_prefix("||") {
        let host_end = rest
            .find(|c| c == '^' || c == '/' || c == '*')
            .unwrap_or(rest.len());
        let host = rest[..host_end].to_ascii_lowercase();
        if !is_valid_domain(&host) {
            return None;
        }
        if !is_allow && SAFE_EXCLUSIONS.contains(&host.as_str()) {
            return None;
        }
        let url_filter = format!("||{}", rest);
        if url_filter.len() < 6 || url_filter.contains('"') || url_filter.contains('\\') {
            return None;
        }
        return Some(ParsedRule {
            action_type,
            priority,
            url_filter,
            domain_type,
            resource_types: rtypes,
        });
    }

    if (pattern_raw.starts_with('/') || pattern_raw.starts_with("http"))
        && pattern_raw.len() >= 8
        && !pattern_raw.contains('"')
        && !pattern_raw.contains('\\')
        && pattern_raw.chars().any(|c| c.is_ascii_alphanumeric())
    {
        return Some(ParsedRule {
            action_type,
            priority,
            url_filter: pattern_raw.to_string(),
            domain_type,
            resource_types: rtypes,
        });
    }

    None
}

fn parse_hosts_line(line: &str, default_types: &[&'static str]) -> Option<ParsedRule> {
    let rest = line
        .strip_prefix("0.0.0.0")
        .or_else(|| line.strip_prefix("127.0.0.1"))?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let token = rest
        .trim_start()
        .split(|c: char| c.is_whitespace() || c == '#')
        .next()?;
    let domain = token.to_ascii_lowercase();
    if !is_valid_domain(&domain) || SAFE_EXCLUSIONS.contains(&domain.as_str()) {
        return None;
    }
    Some(ParsedRule {
        action_type: "block",
        priority: 1,
        url_filter: format!("||{domain}^"),
        domain_type: None,
        resource_types: default_types.to_vec(),
    })
}

fn parse_list(
    text: &str,
    default_types: &[&'static str],
    out: &mut Vec<ParsedRule>,
    cosmetic: &mut CosmeticStore,
) {
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('!') || line.starts_with('[') {
            continue;
        }
        if line.contains("##") {
            parse_cosmetic_line(line, cosmetic);
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        if let Some(r) = parse_abp_line(line, default_types) {
            out.push(r);
        } else if let Some(r) = parse_hosts_line(line, default_types) {
            out.push(r);
        }
    }
}

fn fetch_list(url: &str) -> Result<String, String> {
    let output = Command::new("curl")
        .args([
            "-fsSL",
            "--max-time",
            "25",
            "-A",
            "KestrelShield-FilterBuilder/3.0",
            url,
        ])
        .output()
        .map_err(|e| format!("failed to execute curl: {e}"))?;
    if !output.status.success() {
        return Err(format!("curl exited with {}", output.status));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn emit_ruleset(rules: Vec<ParsedRule>, max_rules: usize, path: &Path) -> usize {
    let mut seen: HashSet<String> = HashSet::new();
    let mut json = String::with_capacity(rules.len().min(max_rules) * 140);
    json.push('[');
    let mut count = 0usize;

    for r in rules {
        let dedup_key = format!(
            "{}|{}|{}|{}",
            r.action_type,
            r.url_filter,
            r.domain_type.unwrap_or(""),
            r.resource_types.join(",")
        );
        if !seen.insert(dedup_key) {
            continue;
        }
        count += 1;
        if count > 1 {
            json.push(',');
        }
        let rtypes_json: Vec<String> = r.resource_types.iter().map(|t| format!("\"{t}\"")).collect();
        let dt_json = match r.domain_type {
            Some(dt) => format!(r#","domainType":"{dt}""#),
            None => String::new(),
        };
        json.push_str(&format!(
            r#"{{"id":{count},"priority":{},"action":{{"type":"{}"}},"condition":{{"urlFilter":{},"resourceTypes":[{}]{dt_json}}}}}"#,
            r.priority,
            r.action_type,
            json_escape(&r.url_filter),
            rtypes_json.join(",")
        ));
        if count >= max_rules {
            break;
        }
    }
    json.push(']');
    fs::write(path, json.as_bytes()).expect("failed to write ruleset");
    count
}

fn emit_tracking_params_ruleset(path: &Path) -> usize {
    let params: Vec<String> = TRACKING_PARAMS.iter().map(|p| format!("\"{p}\"")).collect();
    let json = format!(
        r#"[{{"id":1,"priority":1,"action":{{"type":"redirect","redirect":{{"transform":{{"queryTransform":{{"removeParams":[{}]}}}}}}}},"condition":{{"urlFilter":"*","resourceTypes":["main_frame","sub_frame"]}}}}]"#,
        params.join(",")
    );
    fs::write(path, json.as_bytes()).expect("failed to write tracking_params.json");
    1
}

fn emit_cosmetic_json(store: &CosmeticStore, path: &Path) -> (usize, usize) {
    let global_items: Vec<String> = store.global.iter().map(|s| json_escape(s)).collect();
    let mut domain_entries: Vec<String> = Vec::new();
    let mut domain_sel_count = 0usize;
    for (dom, sels) in &store.domains {
        domain_sel_count += sels.len();
        let escaped: Vec<String> = sels.iter().map(|s| json_escape(s)).collect();
        domain_entries.push(format!("{}:[{}]", json_escape(dom), escaped.join(",")));
    }
    let json = format!(
        r#"{{"global":[{}],"domains":{{{}}}}}"#,
        global_items.join(","),
        domain_entries.join(",")
    );
    fs::write(path, json.as_bytes()).expect("failed to write cosmetic.json");
    (store.global.len(), domain_sel_count)
}

fn default_rules_dir() -> PathBuf {
    if Path::new("manifest.json").exists() || Path::new("rules").exists() {
        return PathBuf::from("rules");
    }
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent().and_then(|p| p.parent()) {
            return parent.join("rules");
        }
    }
    PathBuf::from("rules")
}

fn build_category(
    name: &str,
    builtin: &[&str],
    urls: &[(&str, &str)],
    default_types: &[&'static str],
    max_rules: usize,
    offline: bool,
    out_path: &Path,
    cosmetic: &mut CosmeticStore,
) -> usize {
    let mut rules: Vec<ParsedRule> = builtin
        .iter()
        .map(|d| ParsedRule {
            action_type: "block",
            priority: 1,
            url_filter: format!("||{d}^"),
            domain_type: None,
            resource_types: default_types.to_vec(),
        })
        .collect();

    if name == "ads" {
        for &p in BUILTIN_AD_PATHS {
            rules.push(ParsedRule {
                action_type: "block",
                priority: 1,
                url_filter: p.to_string(),
                domain_type: None,
                resource_types: default_types.to_vec(),
            });
        }
    }

    if !offline {
        for (src, url) in urls {
            eprintln!("Fetching {src} ({name})...");
            match fetch_list(url) {
                Ok(content) => {
                    let before = rules.len();
                    parse_list(&content, default_types, &mut rules, cosmetic);
                    eprintln!("  parsed {} network rules from {src}", rules.len() - before);
                }
                Err(err) => {
                    eprintln!("  warning: failed to fetch {src}: {err}");
                }
            }
        }
    }

    let count = emit_ruleset(rules, max_rules, out_path);
    eprintln!("Wrote {count} rules to {}", out_path.display());
    count
}

fn print_help() {
    eprintln!(
        "Kestrel Shield Filter Builder\n\n\
         USAGE:\n  build_filters [OPTIONS]\n\n\
         OPTIONS:\n  \
           --rules-dir <DIR>    Output directory for rulesets (default: rules/)\n  \
           --offline            Skip network fetch and build from curated built-in lists\n  \
           -h, --help           Show this help message"
    );
}

fn main() {
    let mut rules_dir = default_rules_dir();
    let mut offline = false;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                return;
            }
            "--offline" => offline = true,
            "--rules-dir" => {
                let val = args.next().unwrap_or_else(|| {
                    eprintln!("error: --rules-dir requires a path");
                    std::process::exit(2);
                });
                rules_dir = PathBuf::from(val);
            }
            other => {
                eprintln!("error: unknown argument: {other}");
                std::process::exit(2);
            }
        }
    }

    fs::create_dir_all(&rules_dir).expect("failed to create rules directory");
    let mut cosmetic = CosmeticStore::new();

    // Keep total enabled static rules strictly <= 29,801 (< 30,000 MV3 guaranteed minimum).
    let ads = build_category(
        "ads",
        BUILTIN_ADS,
        ADS_URLS,
        DEFAULT_RESOURCE_TYPES,
        15_000,
        offline,
        &rules_dir.join("ads.json"),
        &mut cosmetic,
    );
    let privacy = build_category(
        "privacy",
        BUILTIN_PRIVACY,
        PRIVACY_URLS,
        DEFAULT_RESOURCE_TYPES,
        10_000,
        offline,
        &rules_dir.join("privacy.json"),
        &mut cosmetic,
    );
    let annoyances = build_category(
        "annoyances",
        BUILTIN_ANNOYANCES,
        ANNOYANCES_URLS,
        DEFAULT_RESOURCE_TYPES,
        4_000,
        offline,
        &rules_dir.join("annoyances.json"),
        &mut cosmetic,
    );
    let malware = build_category(
        "malware",
        BUILTIN_MALWARE,
        MALWARE_URLS,
        MALWARE_RESOURCE_TYPES,
        800,
        offline,
        &rules_dir.join("malware.json"),
        &mut cosmetic,
    );
    let tracking = emit_tracking_params_ruleset(&rules_dir.join("tracking_params.json"));
    let (cos_global, cos_domain) =
        emit_cosmetic_json(&cosmetic, &rules_dir.join("cosmetic.json"));

    eprintln!(
        "Total static DNR rules: {} (ads={ads}, privacy={privacy}, annoyances={annoyances}, malware={malware}, tracking_params={tracking})",
        ads + privacy + annoyances + malware + tracking
    );
    eprintln!(
        "Cosmetic selectors: {} total (global={cos_global}, domain-specific={cos_domain})",
        cos_global + cos_domain
    );
}
