use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::os::unix::fs as unix_fs;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Stdio};

const REPO_ROOT_COMPILE: &str = env!("CARGO_MANIFEST_DIR");

#[derive(Clone, Debug)]
struct Config {
    browser_binary: String,
    profile_dir: String,
    cache_dir: String,
    search_engine: String,
    tab_sleeping: bool,
    hardware_accel: bool,
    widevine_drm: bool,
    adblocker: bool,
    extension_support: bool,
    renderer_process_limit: i64,
    v8_max_old_space_mb: i64,
    widevine_path: String,
    extra_flags: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            browser_binary: "auto".to_string(),
            profile_dir: "~/.config/kestrel/profile".to_string(),
            cache_dir: "~/.cache/kestrel".to_string(),
            search_engine: "duckduckgo".to_string(),
            tab_sleeping: true,
            hardware_accel: true,
            widevine_drm: false,
            adblocker: true,
            extension_support: true,
            renderer_process_limit: 4,
            v8_max_old_space_mb: 512,
            widevine_path: "auto".to_string(),
            extra_flags: Vec::new(),
        }
    }
}

// --- Minimal stdlib-only JSON parser & serializer for Config ---

#[derive(Debug, Clone)]
enum JsonVal {
    Bool(bool),
    Int(i64),
    Str(String),
    Arr(Vec<JsonVal>),
    Obj(Vec<(String, JsonVal)>),
    Null,
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn new(s: &'a str) -> Self {
        Self { bytes: s.as_bytes(), pos: 0 }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_ws();
        self.bytes.get(self.pos).copied()
    }

    fn parse_val(&mut self) -> Result<JsonVal, String> {
        match self.peek() {
            Some(b'{') => self.parse_obj(),
            Some(b'[') => self.parse_arr(),
            Some(b'"') => Ok(JsonVal::Str(self.parse_str()?)),
            Some(b't') => {
                if self.bytes[self.pos..].starts_with(b"true") {
                    self.pos += 4;
                    Ok(JsonVal::Bool(true))
                } else {
                    Err("Invalid literal".into())
                }
            }
            Some(b'f') => {
                if self.bytes[self.pos..].starts_with(b"false") {
                    self.pos += 5;
                    Ok(JsonVal::Bool(false))
                } else {
                    Err("Invalid literal".into())
                }
            }
            Some(b'n') => {
                if self.bytes[self.pos..].starts_with(b"null") {
                    self.pos += 4;
                    Ok(JsonVal::Null)
                } else {
                    Err("Invalid literal".into())
                }
            }
            Some(c) if c == b'-' || c.is_ascii_digit() => self.parse_int(),
            Some(c) => Err(format!("Unexpected byte: {}", c as char)),
            None => Err("Unexpected EOF".into()),
        }
    }

    fn parse_str(&mut self) -> Result<String, String> {
        self.skip_ws();
        if self.bytes.get(self.pos) != Some(&b'"') {
            return Err("Expected '\"'".into());
        }
        self.pos += 1;
        let mut out = String::new();
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            self.pos += 1;
            if b == b'"' {
                return Ok(out);
            }
            if b == b'\\' {
                let esc = *self.bytes.get(self.pos).ok_or("Unterminated escape")?;
                self.pos += 1;
                match esc {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    _ => out.push(esc as char),
                }
            } else {
                out.push(b as char);
            }
        }
        Err("Unterminated string".into())
    }

    fn parse_int(&mut self) -> Result<JsonVal, String> {
        self.skip_ws();
        let start = self.pos;
        if self.bytes.get(self.pos) == Some(&b'-') {
            self.pos += 1;
        }
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        if self.bytes.get(self.pos) == Some(&b'.') {
            self.pos += 1;
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
            return Ok(JsonVal::Null);
        }
        let s = std::str::from_utf8(&self.bytes[start..self.pos]).map_err(|e| e.to_string())?;
        let n = s.parse::<i64>().map_err(|e| e.to_string())?;
        Ok(JsonVal::Int(n))
    }

    fn parse_arr(&mut self) -> Result<JsonVal, String> {
        self.pos += 1; // '['
        let mut items = Vec::new();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(JsonVal::Arr(items));
        }
        loop {
            items.push(self.parse_val()?);
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(JsonVal::Arr(items));
                }
                _ => return Err("Expected ',' or ']'".into()),
            }
        }
    }

    fn parse_obj(&mut self) -> Result<JsonVal, String> {
        self.pos += 1; // '{'
        let mut entries = Vec::new();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(JsonVal::Obj(entries));
        }
        loop {
            let k = self.parse_str()?;
            if self.peek() != Some(b':') {
                return Err("Expected ':'".into());
            }
            self.pos += 1;
            let v = self.parse_val()?;
            entries.push((k, v));
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(JsonVal::Obj(entries));
                }
                _ => return Err("Expected ',' or '}'".into()),
            }
        }
    }
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
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

impl Config {
    fn from_json(s: &str) -> Result<Self, String> {
        let mut cfg = Config::default();
        let val = JsonParser::new(s).parse_val()?;
        let JsonVal::Obj(entries) = val else {
            return Err("Config root must be a JSON object".into());
        };
        for (k, v) in entries {
            match (k.as_str(), v) {
                ("browser_binary", JsonVal::Str(s)) => cfg.browser_binary = s,
                ("profile_dir", JsonVal::Str(s)) => cfg.profile_dir = s,
                ("cache_dir", JsonVal::Str(s)) => cfg.cache_dir = s,
                ("search_engine", JsonVal::Str(s)) => cfg.search_engine = s,
                ("tab_sleeping", JsonVal::Bool(b)) => cfg.tab_sleeping = b,
                ("hardware_accel", JsonVal::Bool(b)) => cfg.hardware_accel = b,
                ("widevine_drm", JsonVal::Bool(b)) => cfg.widevine_drm = b,
                ("adblocker", JsonVal::Bool(b)) => cfg.adblocker = b,
                ("extension_support", JsonVal::Bool(b)) => cfg.extension_support = b,
                ("renderer_process_limit", JsonVal::Int(n)) => cfg.renderer_process_limit = n,
                ("v8_max_old_space_mb", JsonVal::Int(n)) => cfg.v8_max_old_space_mb = n,
                ("widevine_path", JsonVal::Str(s)) => cfg.widevine_path = s,
                ("extra_flags", JsonVal::Arr(arr)) => {
                    cfg.extra_flags = arr
                        .into_iter()
                        .filter_map(|x| if let JsonVal::Str(s) = x { Some(s) } else { None })
                        .collect();
                }
                _ => {}
            }
        }
        Ok(cfg)
    }

    fn to_json(&self) -> String {
        let flags_str = self
            .extra_flags
            .iter()
            .map(|s| json_escape(s))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "{{\n  \"browser_binary\": {},\n  \"profile_dir\": {},\n  \"cache_dir\": {},\n  \"search_engine\": {},\n  \"tab_sleeping\": {},\n  \"hardware_accel\": {},\n  \"widevine_drm\": {},\n  \"adblocker\": {},\n  \"extension_support\": {},\n  \"renderer_process_limit\": {},\n  \"v8_max_old_space_mb\": {},\n  \"widevine_path\": {},\n  \"extra_flags\": [{}]\n}}\n",
            json_escape(&self.browser_binary),
            json_escape(&self.profile_dir),
            json_escape(&self.cache_dir),
            json_escape(&self.search_engine),
            self.tab_sleeping,
            self.hardware_accel,
            self.widevine_drm,
            self.adblocker,
            self.extension_support,
            self.renderer_process_limit,
            self.v8_max_old_space_mb,
            json_escape(&self.widevine_path),
            flags_str
        )
    }
}

// --- Path & Environment Helpers ---

fn home_dir() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}

fn config_dir() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".config"))
        .join("kestrel")
}

fn config_file() -> PathBuf {
    config_dir().join("config.json")
}

fn expand_tilde(p: &str) -> PathBuf {
    if p == "~" {
        return home_dir();
    }
    if let Some(rest) = p.strip_prefix("~/") {
        return home_dir().join(rest);
    }
    PathBuf::from(p)
}

fn repo_root() -> PathBuf {
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            if parent.join("../adblocker/manifest.json").is_file() {
                return parent.parent().unwrap_or(parent).to_path_buf();
            }
            if parent.join("../../adblocker/manifest.json").is_file() {
                return parent
                    .parent()
                    .and_then(|p| p.parent())
                    .unwrap_or(parent)
                    .to_path_buf();
            }
        }
    }
    Path::new(REPO_ROOT_COMPILE)
        .parent()
        .unwrap_or_else(|| Path::new(REPO_ROOT_COMPILE))
        .to_path_buf()
}

fn adblocker_dir() -> PathBuf {
    let local_share = home_dir().join(".local/share/kestrel/adblocker");
    if local_share.join("manifest.json").is_file() {
        return local_share;
    }
    repo_root().join("adblocker")
}

fn import_script() -> PathBuf {
    if let Some(found) = which("kestrel-import") {
        return found;
    }
    repo_root().join("scripts/kestrel-import")
}

fn find_download_url(val: &JsonVal, suffix: &str) -> Option<String> {
    match val {
        JsonVal::Obj(entries) => {
            for (k, v) in entries {
                if k == "browser_download_url" {
                    if let JsonVal::Str(url) = v {
                        if url.ends_with(suffix) {
                            return Some(url.clone());
                        }
                    }
                } else if let Some(found) = find_download_url(v, suffix) {
                    return Some(found);
                }
            }
            None
        }
        JsonVal::Arr(items) => items.iter().find_map(|item| find_download_url(item, suffix)),
        _ => None,
    }
}

fn install_runtime() -> Result<PathBuf, String> {
    let suffix = match env::consts::ARCH {
        "x86_64" => "x86_64_linux.tar.xz",
        "aarch64" => "arm64_linux.tar.xz",
        other => return Err(format!("Unsupported architecture for portable runtime: {other}")),
    };

    println!("Fetching latest Ungoogled Chromium portable release metadata...");
    let api_out = Command::new("curl")
        .args([
            "-fsSL",
            "-H",
            "User-Agent: kestrel-installer",
            "https://api.github.com/repos/ungoogled-software/ungoogled-chromium-portablelinux/releases/latest",
        ])
        .output()
        .map_err(|e| format!("Failed to run curl: {e}"))?;

    if !api_out.status.success() {
        return Err("Failed to query GitHub releases API".into());
    }

    let json_str = String::from_utf8(api_out.stdout).map_err(|e| e.to_string())?;
    let parsed = JsonParser::new(&json_str).parse_val()?;
    let url = find_download_url(&parsed, suffix)
        .ok_or_else(|| format!("No release asset ending in '{suffix}' found"))?;

    let runtime_root = home_dir().join(".local/share/kestrel/runtime");
    let staging_dir = home_dir().join(".local/share/kestrel/runtime.staging");
    let archive_path = home_dir().join(".local/share/kestrel/runtime.tar.xz");
    if let Some(parent) = runtime_root.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let _ = fs::remove_dir_all(&staging_dir);
    let _ = fs::remove_file(&archive_path);
    fs::create_dir_all(&staging_dir).map_err(|e| e.to_string())?;

    println!("Downloading {url}...");
    let dl_status = Command::new("curl")
        .args(["-fL", "--progress-bar", "-o"])
        .arg(&archive_path)
        .arg(&url)
        .status()
        .map_err(|e| format!("Failed to run curl: {e}"))?;
    if !dl_status.success() {
        let _ = fs::remove_dir_all(&staging_dir);
        let _ = fs::remove_file(&archive_path);
        return Err("Runtime download failed".into());
    }

    println!("Extracting runtime to {}...", runtime_root.display());
    let tar_status = Command::new("tar")
        .args(["--strip-components=1", "-xJf"])
        .arg(&archive_path)
        .arg("-C")
        .arg(&staging_dir)
        .status()
        .map_err(|e| format!("Failed to run tar: {e}"))?;
    let _ = fs::remove_file(&archive_path);

    if !tar_status.success() {
        let _ = fs::remove_dir_all(&staging_dir);
        return Err("Failed to extract runtime archive".into());
    }

    let staged_chrome = staging_dir.join("chrome");
    if !staged_chrome.is_file() {
        let _ = fs::remove_dir_all(&staging_dir);
        return Err("Extracted archive does not contain 'chrome' binary".into());
    }

    let _ = fs::remove_dir_all(&runtime_root);
    fs::rename(&staging_dir, &runtime_root).map_err(|e| e.to_string())?;
    let final_chrome = runtime_root.join("chrome");
    println!("Standalone runtime installed at {}", final_chrome.display());
    Ok(final_chrome)
}

fn which(bin: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(bin);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn load_or_create_config() -> Config {
    let dir = config_dir();
    let file = config_file();
    let _ = fs::create_dir_all(&dir);
    if !file.is_file() {
        let cfg = Config::default();
        let _ = fs::write(&file, cfg.to_json());
        return cfg;
    }
    match fs::read_to_string(&file) {
        Ok(content) => Config::from_json(&content).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

fn save_config(cfg: &Config) -> io::Result<()> {
    let dir = config_dir();
    fs::create_dir_all(&dir)?;
    fs::write(config_file(), cfg.to_json())
}

// --- Runtime & Widevine Resolution ---

fn resolve_browser_command(configured: &str) -> Result<Vec<String>, String> {
    if !configured.is_empty() && configured != "auto" {
        let expanded = expand_tilde(configured);
        if expanded.is_file() {
            return Ok(vec![expanded.to_string_lossy().into_owned()]);
        }
        if let Some(found) = which(configured) {
            return Ok(vec![found.to_string_lossy().into_owned()]);
        }
        return Err(format!("Configured browser_binary not found: {configured}"));
    }

    let local_runtime = home_dir().join(".local/share/kestrel/runtime/chrome");
    if local_runtime.is_file() {
        return Ok(vec![local_runtime.to_string_lossy().into_owned()]);
    }

    for candidate in ["ungoogled-chromium", "ungoogled-chromium-bin", "chromium"] {
        if let Some(found) = which(candidate) {
            return Ok(vec![found.to_string_lossy().into_owned()]);
        }
    }

    if which("flatpak").is_some() {
        let status = Command::new("flatpak")
            .args(["info", "io.github.ungoogled_software.ungoogled_chromium"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if matches!(status, Ok(s) if s.success()) {
            return Ok(vec![
                "flatpak".into(),
                "run".into(),
                "io.github.ungoogled_software.ungoogled_chromium".into(),
            ]);
        }
    }

    Err(format!(
        "No Chromium runtime found. Install `ungoogled-chromium`, place a standalone runtime at {}, or set `browser_binary` in {}.",
        local_runtime.display(),
        config_file().display()
    ))
}

fn resolve_widevine_path(configured: &str) -> Option<PathBuf> {
    if !configured.is_empty() && configured != "auto" {
        let p = expand_tilde(configured);
        return if p.exists() { Some(p) } else { None };
    }
    let home = home_dir();
    let candidates = [
        home.join(".local/share/kestrel/runtime/WidevineCdm"),
        PathBuf::from("/usr/lib/chromium/WidevineCdm"),
        PathBuf::from("/usr/lib/ungoogled-chromium/WidevineCdm"),
        PathBuf::from("/opt/google/chrome/WidevineCdm"),
        home.join(".config/sh.cider.genten/WidevineCdm"),
        home.join(".config/net.imput.helium/WidevineCdm"),
    ];
    candidates.into_iter().find(|p| p.exists())
}

struct SearchEngineSpec {
    short_name: &'static str,
    keyword: &'static str,
    favicon_url: &'static str,
    url: &'static str,
    suggest_url: &'static str,
    prepopulate_id: i64,
}

fn search_engine_spec(name: &str) -> Option<SearchEngineSpec> {
    match name.trim().to_ascii_lowercase().as_str() {
        "duckduckgo" | "ddg" => Some(SearchEngineSpec {
            short_name: "DuckDuckGo",
            keyword: "duckduckgo.com",
            favicon_url: "https://duckduckgo.com/favicon.ico",
            url: "https://duckduckgo.com/?q={searchTerms}",
            suggest_url: "https://duckduckgo.com/ac/?q={searchTerms}&type=list",
            prepopulate_id: 92,
        }),
        "brave" => Some(SearchEngineSpec {
            short_name: "Brave",
            keyword: "search.brave.com",
            favicon_url: "https://cdn.search.brave.com/serp/v2/_app/immutable/assets/favicon.ico",
            url: "https://search.brave.com/search?q={searchTerms}",
            suggest_url: "https://search.brave.com/api/suggest?q={searchTerms}",
            prepopulate_id: 109,
        }),
        "startpage" => Some(SearchEngineSpec {
            short_name: "Startpage",
            keyword: "startpage.com",
            favicon_url: "https://www.startpage.com/sp/cdn/favicons/favicon-16x16-gradient.png",
            url: "https://www.startpage.com/sp/search?query={searchTerms}",
            suggest_url: "https://www.startpage.com/osuggestions?q={searchTerms}",
            prepopulate_id: 0,
        }),
        "google" => Some(SearchEngineSpec {
            short_name: "Google",
            keyword: "google.com",
            favicon_url: "https://www.google.com/favicon.ico",
            url: "https://www.google.com/search?q={searchTerms}",
            suggest_url: "https://www.google.com/complete/search?client=chrome&q={searchTerms}",
            prepopulate_id: 1,
        }),
        _ => None,
    }
}

fn sql_escape(s: &str) -> String {
    s.replace('\'', "''")
}

fn ensure_search_engine(profile_dir: &Path, engine: &str, force: bool, cmd_hint: Option<&[String]>) {
    let Some(spec) = search_engine_spec(engine) else {
        return;
    };
    if which("sqlite3").is_none() {
        return;
    }
    if profile_dir.join("SingletonLock").exists() {
        return;
    }

    let web_data = profile_dir.join("Default/Web Data");
    if !web_data.is_file() {
        if let Some(cmd) = cmd_hint {
            let _ = Command::new(&cmd[0])
                .args(&cmd[1..])
                .args([
                    "--headless",
                    "--no-sandbox",
                    &format!("--user-data-dir={}", profile_dir.display()),
                    "--dump-dom",
                    "about:blank",
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }

    if !web_data.is_file() {
        return;
    }

    if !force {
        let check = Command::new("sqlite3")
            .arg(&web_data)
            .arg("SELECT k.short_name, k.url, m.value FROM keywords k LEFT JOIN meta m ON m.key='Default Search Provider ID' WHERE k.id=2;")
            .output();
        if let Ok(out) = check {
            let s = String::from_utf8_lossy(&out.stdout);
            let trimmed = s.trim();
            let is_nosearch = trimmed.contains("No Search") || trimmed.contains("http://{searchTerms}");
            let has_meta = trimmed.ends_with('|') == false && !trimmed.is_empty();
            if !is_nosearch && has_meta {
                return;
            }
        }
    }

    let sql = format!(
        "UPDATE keywords SET short_name='{}', keyword='{}', favicon_url='{}', url='{}', suggest_url='{}', prepopulate_id={} WHERE id=2; \
         INSERT OR REPLACE INTO meta(key, value) VALUES ('Default Search Provider ID', '2');",
        sql_escape(spec.short_name),
        sql_escape(spec.keyword),
        sql_escape(spec.favicon_url),
        sql_escape(spec.url),
        sql_escape(spec.suggest_url),
        spec.prepopulate_id
    );
    let _ = Command::new("sqlite3")
        .arg(&web_data)
        .arg(sql)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn sync_widevine_symlink(profile_dir: &Path, enabled: bool, widevine_cfg: &str) {
    let target_link = profile_dir.join("WidevineCdm");
    if enabled {
        if let Some(src) = resolve_widevine_path(widevine_cfg) {
            if !target_link.exists() {
                let _ = unix_fs::symlink(src, &target_link);
            }
        }
    } else if target_link.is_symlink() {
        let _ = fs::remove_file(&target_link);
    }
}

fn build_flags(cfg: &Config, profile_dir: &Path, cache_dir: &Path) -> Vec<String> {
    let mut flags = vec![
        "--class=Kestrel".to_string(),
        format!("--user-data-dir={}", profile_dir.display()),
        format!("--disk-cache-dir={}", cache_dir.display()),
        "--no-first-run".to_string(),
        "--no-default-browser-check".to_string(),
    ];

    let mut enable_features: Vec<&str> = vec!["HttpsUpgrades"];
    let mut disable_features: Vec<&str> = Vec::new();

    if cfg.tab_sleeping {
        flags.push(format!("--renderer-process-limit={}", cfg.renderer_process_limit));
        flags.push(format!("--js-flags=--max-old-space-size={}", cfg.v8_max_old_space_mb));
        flags.extend(
            [
                "--disable-background-networking",
                "--disable-component-update",
                "--disable-domain-reliability",
                "--disable-breakpad",
                "--disable-crash-reporter",
                "--no-pings",
                "--dns-prefetch-disable",
                "--disable-sync",
            ]
            .into_iter()
            .map(String::from),
        );
        enable_features.extend(["HighEfficiencyModeAvailable", "MemorySaverModeAvailable"]);
        disable_features.extend([
            "OptimizationHints",
            "OptimizationTargetPrediction",
            "PreloadMediaEngagementData",
            "SpeculationRulesPrefetchProxy",
            "Prerender2",
            "NetworkTimeServiceQuerying",
        ]);
    }

    if cfg.hardware_accel {
        flags.extend(
            [
                "--ozone-platform-hint=auto",
                "--enable-gpu-rasterization",
                "--enable-zero-copy",
                "--ignore-gpu-blocklist",
            ]
            .into_iter()
            .map(String::from),
        );
        enable_features.extend([
            "VaapiVideoDecoder",
            "VaapiVideoEncoder",
            "VaapiVideoDecodeLinuxGL",
        ]);
    } else {
        flags.push("--disable-gpu".to_string());
        flags.push("--disable-webgl".to_string());
    }

    let adb = adblocker_dir();
    let adb_ready = cfg.adblocker && adb.join("manifest.json").is_file();

    if adb_ready {
        flags.push(format!("--load-extension={}", adb.display()));
        if !cfg.extension_support {
            flags.push(format!("--disable-extensions-except={}", adb.display()));
        }
    } else if !cfg.extension_support {
        flags.push("--disable-extensions".to_string());
    }

    if cfg.extension_support {
        flags.push("--extension-mime-request-handling=always-prompt-for-install".to_string());
    }

    if !enable_features.is_empty() {
        flags.push(format!("--enable-features={}", enable_features.join(",")));
    }
    if !disable_features.is_empty() {
        flags.push(format!("--disable-features={}", disable_features.join(",")));
    }

    for extra in &cfg.extra_flags {
        let trimmed = extra.trim();
        if !trimmed.is_empty() {
            flags.push(trimmed.to_string());
        }
    }

    flags
}

fn apply_flatpak_filesystems(
    cmd: Vec<String>,
    cfg: &Config,
    profile_dir: &Path,
    cache_dir: &Path,
) -> Vec<String> {
    if cmd.len() >= 3 && cmd[0] == "flatpak" && cmd[1] == "run" {
        let mut out = vec![
            cmd[0].clone(),
            cmd[1].clone(),
            format!("--filesystem={}", config_dir().display()),
            format!("--filesystem={}", profile_dir.display()),
            format!("--filesystem={}", cache_dir.display()),
        ];
        let adb = adblocker_dir();
        if cfg.adblocker && adb.join("manifest.json").is_file() {
            out.push(format!("--filesystem={}", adb.display()));
        }
        if cfg.widevine_drm {
            if let Some(wv) = resolve_widevine_path(&cfg.widevine_path) {
                out.push(format!("--filesystem={}:ro", wv.display()));
            }
        }
        out.extend_from_slice(&cmd[2..]);
        return out;
    }
    cmd
}

// --- Settings Mode (TUI + CLI) ---

const TOGGLES: [(&str, &str, &str); 5] = [
    ("tab_sleeping", "Tab Sleeping & Low-RAM Mode", "Aggressive Memory Saver, renderer cap, V8 heap limit, no background net/telemetry"),
    ("hardware_accel", "Hardware Acceleration (VA-API + WebGL)", "GPU rasterization, zero-copy, VA-API video decode/encode for YouTube"),
    ("widevine_drm", "Widevine DRM", "Enable Widevine CDM symlink for protected media (Spotify, Netflix, etc.)"),
    ("adblocker", "Built-in Adblocker", "Load custom low-RAM MV3 adblocker extension"),
    ("extension_support", "Extension & Web Store Support", "Allow installing extensions and .crx packages from Chrome Web Store"),
];

fn get_toggle(cfg: &Config, key: &str) -> Option<bool> {
    match key {
        "tab_sleeping" => Some(cfg.tab_sleeping),
        "hardware_accel" => Some(cfg.hardware_accel),
        "widevine_drm" => Some(cfg.widevine_drm),
        "adblocker" => Some(cfg.adblocker),
        "extension_support" => Some(cfg.extension_support),
        _ => None,
    }
}

fn set_toggle(cfg: &mut Config, key: &str, val: bool) -> bool {
    match key {
        "tab_sleeping" => cfg.tab_sleeping = val,
        "hardware_accel" => cfg.hardware_accel = val,
        "widevine_drm" => cfg.widevine_drm = val,
        "adblocker" => cfg.adblocker = val,
        "extension_support" => cfg.extension_support = val,
        _ => return false,
    }
    true
}

fn parse_bool(val: &str) -> Result<bool, String> {
    match val.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(format!("Invalid boolean value: {val}")),
    }
}

fn print_settings_status(cfg: &Config) {
    println!("Kestrel Settings ({})\n", config_file().display());
    for (idx, (key, label, desc)) in TOGGLES.iter().enumerate() {
        let state = if get_toggle(cfg, key).unwrap_or(false) { "ON " } else { "OFF" };
        println!("  [{}] [{}] {} ({})", idx + 1, state, label, key);
        println!("           {}", desc);
    }
    println!();
    println!("  [6] [{}] Renderer Process Limit (renderer_process_limit, 1-32)", cfg.renderer_process_limit);
    println!("  [7] [{}] V8 Max Heap per Tab (MB) (v8_max_old_space_mb, 128-4096)", cfg.v8_max_old_space_mb);
    println!("  [8] [{}] Default Search Engine (duckduckgo | brave | startpage | google)", cfg.search_engine);
    println!();
}

fn run_settings_interactive(mut cfg: Config) -> i32 {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    loop {
        print!("\x1b[2J\x1b[H");
        let _ = stdout.flush();
        print_settings_status(&cfg);
        println!("  [r] Reset to defaults    [q] Save & Quit");
        print!("\nSelect option to toggle/edit: ");
        let _ = stdout.flush();

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) | Err(_) => return 0,
            Ok(_) => {}
        }
        let choice = line.trim().to_ascii_lowercase();
        match choice.as_str() {
            "q" | "quit" | "exit" | "" => {
                if let Err(e) = save_config(&cfg) {
                    eprintln!("Error saving config: {e}");
                    return 1;
                }
                println!("Saved settings to {}", config_file().display());
                return 0;
            }
            "r" => {
                cfg = Config::default();
                let _ = save_config(&cfg);
            }
            "1" | "2" | "3" | "4" | "5" => {
                let idx = choice.parse::<usize>().unwrap() - 1;
                let key = TOGGLES[idx].0;
                let cur = get_toggle(&cfg, key).unwrap_or(false);
                set_toggle(&mut cfg, key, !cur);
                let _ = save_config(&cfg);
            }
            "6" => {
                print!("Enter Renderer Process Limit (1-32) [{}]: ", cfg.renderer_process_limit);
                let _ = stdout.flush();
                let mut raw = String::new();
                if stdin.lock().read_line(&mut raw).is_ok() {
                    if let Ok(n) = raw.trim().parse::<i64>() {
                        if (1..=32).contains(&n) {
                            cfg.renderer_process_limit = n;
                            let _ = save_config(&cfg);
                        }
                    }
                }
            }
            "7" => {
                print!("Enter V8 Max Heap per Tab in MB (128-4096) [{}]: ", cfg.v8_max_old_space_mb);
                let _ = stdout.flush();
                let mut raw = String::new();
                if stdin.lock().read_line(&mut raw).is_ok() {
                    if let Ok(n) = raw.trim().parse::<i64>() {
                        if (128..=4096).contains(&n) {
                            cfg.v8_max_old_space_mb = n;
                            let _ = save_config(&cfg);
                        }
                    }
                }
            }
            "8" => {
                print!("Enter search engine (duckduckgo, brave, startpage, google) [{}]: ", cfg.search_engine);
                let _ = stdout.flush();
                let mut raw = String::new();
                if stdin.lock().read_line(&mut raw).is_ok() {
                    let val = raw.trim().to_ascii_lowercase();
                    if search_engine_spec(&val).is_some() {
                        cfg.search_engine = val;
                        let _ = save_config(&cfg);
                        ensure_search_engine(&expand_tilde(&cfg.profile_dir), &cfg.search_engine, true, None);
                    }
                }
            }
            _ => {}
        }
    }
}

fn run_settings(args: &[String]) -> i32 {
    let mut list_only = false;
    let mut reset = false;
    let mut toggles: Vec<String> = Vec::new();
    let mut sets: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                    println!(
                    "Usage: kestrel-settings [--list] [--toggle KEY] [--set KEY=VALUE] [--reset]\n\n\
                     Keys: search_engine, tab_sleeping, hardware_accel, widevine_drm, adblocker, extension_support,\n\
                           renderer_process_limit, v8_max_old_space_mb, browser_binary, profile_dir, cache_dir, widevine_path"
                );
                return 0;
            }
            "--list" => list_only = true,
            "--reset" => reset = true,
            "--toggle" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("--toggle requires a KEY");
                    return 1;
                }
                toggles.push(args[i].clone());
            }
            "--set" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("--set requires KEY=VALUE");
                    return 1;
                }
                sets.push(args[i].clone());
            }
            other => {
                eprintln!("Unknown option: {other}");
                return 1;
            }
        }
        i += 1;
    }

    if reset {
        let cfg = Config::default();
        if let Err(e) = save_config(&cfg) {
            eprintln!("Error saving config: {e}");
            return 1;
        }
        println!("Reset settings to defaults.");
        return 0;
    }

    let mut cfg = load_or_create_config();
    let mut modified = false;

    for key in toggles {
        if let Some(cur) = get_toggle(&cfg, &key) {
            set_toggle(&mut cfg, &key, !cur);
            modified = true;
        } else {
            eprintln!("Unknown toggle key '{key}'");
            return 1;
        }
    }

    for item in sets {
        let Some((key, val)) = item.split_once('=') else {
            eprintln!("Invalid --set format '{item}', expected KEY=VALUE");
            return 1;
        };
        match key {
            "tab_sleeping" | "hardware_accel" | "widevine_drm" | "adblocker" | "extension_support" => {
                match parse_bool(val) {
                    Ok(b) => {
                        set_toggle(&mut cfg, key, b);
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        return 1;
                    }
                }
            }
            "renderer_process_limit" => match val.parse::<i64>() {
                Ok(n) => cfg.renderer_process_limit = n,
                Err(_) => {
                    eprintln!("Invalid integer: {val}");
                    return 1;
                }
            },
            "v8_max_old_space_mb" => match val.parse::<i64>() {
                Ok(n) => cfg.v8_max_old_space_mb = n,
                Err(_) => {
                    eprintln!("Invalid integer: {val}");
                    return 1;
                }
            },
            "search_engine" => {
                let low = val.trim().to_ascii_lowercase();
                if search_engine_spec(&low).is_none() {
                    eprintln!("Unsupported search_engine '{val}'. Choose: duckduckgo, brave, startpage, google");
                    return 1;
                }
                cfg.search_engine = low;
                ensure_search_engine(&expand_tilde(&cfg.profile_dir), &cfg.search_engine, true, None);
            }
            "browser_binary" => cfg.browser_binary = val.to_string(),
            "profile_dir" => cfg.profile_dir = val.to_string(),
            "cache_dir" => cfg.cache_dir = val.to_string(),
            "widevine_path" => cfg.widevine_path = val.to_string(),
            _ => {
                eprintln!("Unsupported key: {key}");
                return 1;
            }
        }
        modified = true;
    }

    if modified {
        if let Err(e) = save_config(&cfg) {
            eprintln!("Error saving config: {e}");
            return 1;
        }
    }

    if list_only || modified {
        print_settings_status(&cfg);
        return 0;
    }

    run_settings_interactive(cfg)
}

fn print_launcher_help() {
    println!(
        "Usage: kestrel [LAUNCHER_OPTION | CHROMIUM_ARGS...]\n\n\
         Launcher options:\n\
           install-runtime        Download standalone Ungoogled Chromium runtime\n\
           --auto-runtime         Auto-download standalone runtime if none is found\n\
           --settings, settings   Open interactive settings manager\n\
           --import               Run Kestrel profile/bookmark importer\n\
           --dry-run, --print-cmd Print resolved command and flags without launching\n\
           -h, --help             Show this help message\n\n\
         Config file: {}",
        config_file().display()
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let bin_name = Path::new(&args[0])
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("kestrel");

    if bin_name == "kestrel-settings" {
        process::exit(run_settings(&args[1..]));
    }

    if let Some(first) = args.get(1).map(String::as_str) {
        match first {
            "-h" | "--help" => {
                print_launcher_help();
                return;
            }
            "install-runtime" | "--install-runtime" => {
                if let Err(e) = install_runtime() {
                    eprintln!("Error: {e}");
                    process::exit(1);
                }
                return;
            }
            "--settings" | "settings" => {
                process::exit(run_settings(&args[2..]));
            }
            "--import" => {
                let err = Command::new(import_script()).args(&args[2..]).exec();
                eprintln!("Failed to execute importer: {err}");
                process::exit(1);
            }
            _ => {}
        }
    }

    let mut dry_run = false;
    let mut auto_runtime = false;
    let mut pass_args: Vec<String> = Vec::new();
    for arg in &args[1..] {
        if arg == "--dry-run" || arg == "--print-cmd" {
            dry_run = true;
        } else if arg == "--auto-runtime" {
            auto_runtime = true;
        } else {
            pass_args.push(arg.clone());
        }
    }

    let cfg = load_or_create_config();
    let profile_dir = expand_tilde(&cfg.profile_dir);
    let cache_dir = expand_tilde(&cfg.cache_dir);
    let _ = fs::create_dir_all(&profile_dir);
    let _ = fs::create_dir_all(&cache_dir);

    sync_widevine_symlink(&profile_dir, cfg.widevine_drm, &cfg.widevine_path);
    let flags = build_flags(&cfg, &profile_dir, &cache_dir);

    let cmd = match resolve_browser_command(&cfg.browser_binary) {
        Ok(c) => apply_flatpak_filesystems(c, &cfg, &profile_dir, &cache_dir),
        Err(e) => {
            if dry_run {
                println!("<missing: {e}> {}", flags.join(" "));
                return;
            }
            let should_install = if auto_runtime {
                true
            } else if io::IsTerminal::is_terminal(&io::stdin()) {
                print!("No Chromium runtime found. Download standalone Kestrel runtime now? [Y/n] ");
                let _ = io::stdout().flush();
                let mut ans = String::new();
                let _ = io::stdin().lock().read_line(&mut ans);
                let trimmed = ans.trim().to_ascii_lowercase();
                trimmed.is_empty() || trimmed == "y" || trimmed == "yes"
            } else {
                false
            };

            if should_install {
                match install_runtime() {
                    Ok(chrome_path) => vec![chrome_path.to_string_lossy().into_owned()],
                    Err(err) => {
                        eprintln!("Error: {err}");
                        process::exit(1);
                    }
                }
            } else {
                eprintln!("Error: {e}\nRun `kestrel install-runtime` to install the standalone runtime.");
                process::exit(1);
            }
        }
    };

    if dry_run {
        let mut all = cmd;
        all.extend(flags);
        all.extend(pass_args);
        println!("{}", all.join(" "));
        return;
    }

    ensure_search_engine(&profile_dir, &cfg.search_engine, false, Some(&cmd));

    let err = Command::new(&cmd[0])
        .args(&cmd[1..])
        .args(&flags)
        .args(&pass_args)
        .exec();
    eprintln!("Failed to launch {}: {err}", cmd[0]);
    process::exit(1);
}
