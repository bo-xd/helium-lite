use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const LIST_URLS: &[(&str, &str)] = &[
    ("easylist", "https://easylist.to/easylist/easylist.txt"),
    ("easyprivacy", "https://easylist.to/easylist/easyprivacy.txt"),
    (
        "peterlowe",
        "https://pgl.yoyo.org/adservers/serverlist.php?hostformat=hosts&showintro=0&mimetype=plaintext",
    ),
];

const RESOURCE_TYPES_JSON: &str =
    r#"["script","image","xmlhttprequest","sub_frame","ping","media","font","stylesheet","other"]"#;

const SAFE_EXCLUSIONS: &[&str] = &[
    "bing.com",
    "t.co",
    "google.com",
    "facebook.com",
    "amazon.com",
    "github.com",
    "cloudflare.com",
];

const BUILTIN_DOMAINS: &[&str] = &[
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
    "bat.bing.com",
    "ads.linkedin.com",
    "px.ads.linkedin.com",
    "snap.licdn.com",
    "ads-twitter.com",
    "static.ads-twitter.com",
    "analytics.twitter.com",
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
];

fn is_valid_domain(s: &str) -> bool {
    if s.len() < 4 || s.starts_with('.') || s.ends_with('.') || !s.contains('.') {
        return false;
    }
    let parts: Vec<&str> = s.split('.').collect();
    let tld = parts.last().copied().unwrap_or("");
    if tld.len() < 2 || !tld.chars().all(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
}

fn parse_abp_line(line: &str) -> Option<(String, bool)> {
    let rest = line.strip_prefix("||")?;
    let caret_pos = rest.find('^')?;
    let domain = rest[..caret_pos].to_ascii_lowercase();
    if !is_valid_domain(&domain) {
        return None;
    }
    let after = &rest[caret_pos + 1..];
    if after.is_empty() {
        return Some((domain, false));
    }
    let opts_str = after.strip_prefix('$')?;
    if opts_str.chars().any(char::is_whitespace) {
        return None;
    }
    let mut third_party = false;
    for opt in opts_str.split(',') {
        let o = opt.to_ascii_lowercase();
        if o.starts_with('~')
            || matches!(o.as_str(), "elemhide" | "generichide" | "document" | "popup")
        {
            return None;
        }
        if o == "third-party" || o == "3p" {
            third_party = true;
        }
    }
    Some((domain, third_party))
}

fn parse_hosts_line(line: &str) -> Option<String> {
    let rest = line
        .strip_prefix("0.0.0.0")
        .or_else(|| line.strip_prefix("127.0.0.1"))?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let token = rest.trim_start().split(|c: char| c.is_whitespace() || c == '#').next()?;
    let domain = token.to_ascii_lowercase();
    if is_valid_domain(&domain) {
        Some(domain)
    } else {
        None
    }
}

fn parse_domains(text: &str, out: &mut Vec<(String, bool)>) {
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('!') || line.starts_with('#') || line.starts_with('[')
        {
            continue;
        }
        if let Some((d, tp)) = parse_abp_line(line) {
            out.push((d, tp));
        } else if let Some(d) = parse_hosts_line(line) {
            out.push((d, false));
        }
    }
}

fn fetch_list(url: &str) -> Result<String, String> {
    let output = Command::new("curl")
        .args([
            "-fsSL",
            "--max-time",
            "20",
            "-A",
            "KestrelShield-FilterBuilder/1.0",
            url,
        ])
        .output()
        .map_err(|e| format!("failed to execute curl: {e}"))?;
    if !output.status.success() {
        return Err(format!("curl exited with {}", output.status));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn default_output_path() -> PathBuf {
    let cwd_rules = Path::new("rules/ruleset.json");
    if Path::new("manifest.json").exists() || Path::new("rules").exists() {
        return cwd_rules.to_path_buf();
    }
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent().and_then(|p| p.parent()) {
            return parent.join("rules").join("ruleset.json");
        }
    }
    cwd_rules.to_path_buf()
}

fn print_help() {
    eprintln!(
        "Kestrel Shield Filter Builder\n\n\
         USAGE:\n  build_filters [OPTIONS]\n\n\
         OPTIONS:\n  \
           --output <PATH>      Output path for ruleset.json (default: rules/ruleset.json)\n  \
           --max-rules <N>      Maximum static rules to emit (default: 25000)\n  \
           --offline            Skip network fetch and build from curated built-in list\n  \
           -h, --help           Show this help message"
    );
}

fn main() {
    let mut output_path = default_output_path();
    let mut max_rules: usize = 25_000;
    let mut offline = false;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                return;
            }
            "--offline" => offline = true,
            "--output" => {
                let val = args.next().unwrap_or_else(|| {
                    eprintln!("error: --output requires a path");
                    std::process::exit(2);
                });
                output_path = PathBuf::from(val);
            }
            "--max-rules" => {
                let val = args.next().unwrap_or_else(|| {
                    eprintln!("error: --max-rules requires a number");
                    std::process::exit(2);
                });
                max_rules = val.parse().unwrap_or_else(|_| {
                    eprintln!("error: invalid --max-rules value: {val}");
                    std::process::exit(2);
                });
            }
            other => {
                eprintln!("error: unknown argument: {other}");
                std::process::exit(2);
            }
        }
    }

    let mut entries: Vec<(String, bool)> = BUILTIN_DOMAINS
        .iter()
        .map(|d| ((*d).to_string(), false))
        .collect();

    if !offline {
        for (name, url) in LIST_URLS {
            eprintln!("Fetching {name}...");
            match fetch_list(url) {
                Ok(content) => {
                    let before = entries.len();
                    parse_domains(&content, &mut entries);
                    eprintln!("  parsed {} domain rules from {name}", entries.len() - before);
                }
                Err(err) => {
                    eprintln!("  warning: failed to fetch {name}: {err}");
                }
            }
        }
    }

    let exclusions: HashSet<&str> = SAFE_EXCLUSIONS.iter().copied().collect();
    let mut seen: HashSet<String> = HashSet::new();
    let mut json = String::from("[");
    let mut count: usize = 0;

    for (domain, third_party) in entries {
        if exclusions.contains(domain.as_str()) || !seen.insert(domain.clone()) {
            continue;
        }
        count += 1;
        if count > 1 {
            json.push(',');
        }
        let domain_type = if third_party {
            r#","domainType":"thirdParty""#
        } else {
            ""
        };
        json.push_str(&format!(
            r#"{{"id":{count},"priority":1,"action":{{"type":"block"}},"condition":{{"urlFilter":"||{domain}^","resourceTypes":{RESOURCE_TYPES_JSON}{domain_type}}}}}"#
        ));
        if count >= max_rules {
            break;
        }
    }
    json.push(']');

    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).expect("failed to create output directory");
        }
    }
    fs::write(&output_path, json.as_bytes()).expect("failed to write ruleset.json");
    eprintln!("Wrote {count} rules to {}", output_path.display());
}
