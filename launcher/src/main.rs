use std::env;
use std::fs;
use std::io::{self, BufRead, Read, Write};
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
    ram_profile: String,
    strict_security: bool,
    doh_provider: String,
    webrtc_privacy: bool,
    tab_sleeping: bool,
    hardware_accel: bool,
    widevine_drm: bool,
    adblocker: bool,
    custom_ui: bool,
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
            ram_profile: "ultra".to_string(),
            strict_security: true,
            doh_provider: "quad9".to_string(),
            webrtc_privacy: true,
            tab_sleeping: true,
            hardware_accel: true,
            widevine_drm: true,
            adblocker: true,
            custom_ui: true,
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
    Num(String),
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
                    b'b' => out.push('\u{0008}'),
                    b'f' => out.push('\u{000C}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        if self.pos + 4 <= self.bytes.len() {
                            if let Ok(hex) = std::str::from_utf8(&self.bytes[self.pos..self.pos + 4]) {
                                if let Ok(code) = u32::from_str_radix(hex, 16) {
                                    if let Some(ch) = char::from_u32(code) {
                                        out.push(ch);
                                        self.pos += 4;
                                        continue;
                                    }
                                }
                            }
                        }
                        out.push_str("\\u");
                    }
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
        let mut is_float = false;
        if self.bytes.get(self.pos) == Some(&b'.') {
            is_float = true;
            self.pos += 1;
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }
        if matches!(self.bytes.get(self.pos), Some(b'e' | b'E')) {
            is_float = true;
            self.pos += 1;
            if matches!(self.bytes.get(self.pos), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }
        let s = std::str::from_utf8(&self.bytes[start..self.pos]).map_err(|e| e.to_string())?;
        if is_float {
            Ok(JsonVal::Num(s.to_string()))
        } else if let Ok(n) = s.parse::<i64>() {
            Ok(JsonVal::Int(n))
        } else {
            Ok(JsonVal::Num(s.to_string()))
        }
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
            '\u{0008}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn json_serialize(val: &JsonVal) -> String {
    match val {
        JsonVal::Null => "null".to_string(),
        JsonVal::Bool(b) => b.to_string(),
        JsonVal::Int(n) => n.to_string(),
        JsonVal::Num(s) => s.clone(),
        JsonVal::Str(s) => json_escape(s),
        JsonVal::Arr(items) => {
            let inner = items.iter().map(json_serialize).collect::<Vec<_>>().join(",");
            format!("[{inner}]")
        }
        JsonVal::Obj(entries) => {
            let inner = entries
                .iter()
                .map(|(k, v)| format!("{}:{}", json_escape(k), json_serialize(v)))
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{inner}}}")
        }
    }
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
                ("ram_profile", JsonVal::Str(s)) => cfg.ram_profile = s,
                ("strict_security", JsonVal::Bool(b)) => cfg.strict_security = b,
                ("doh_provider", JsonVal::Str(s)) => cfg.doh_provider = s,
                ("webrtc_privacy", JsonVal::Bool(b)) => cfg.webrtc_privacy = b,
                ("tab_sleeping", JsonVal::Bool(b)) => cfg.tab_sleeping = b,
                ("hardware_accel", JsonVal::Bool(b)) => cfg.hardware_accel = b,
                ("widevine_drm", JsonVal::Bool(b)) => cfg.widevine_drm = b,
                ("adblocker", JsonVal::Bool(b)) => cfg.adblocker = b,
                ("custom_ui", JsonVal::Bool(b)) => cfg.custom_ui = b,
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
            "{{\n  \"browser_binary\": {},\n  \"profile_dir\": {},\n  \"cache_dir\": {},\n  \"search_engine\": {},\n  \"ram_profile\": {},\n  \"strict_security\": {},\n  \"doh_provider\": {},\n  \"webrtc_privacy\": {},\n  \"tab_sleeping\": {},\n  \"hardware_accel\": {},\n  \"widevine_drm\": {},\n  \"adblocker\": {},\n  \"custom_ui\": {},\n  \"extension_support\": {},\n  \"renderer_process_limit\": {},\n  \"v8_max_old_space_mb\": {},\n  \"widevine_path\": {},\n  \"extra_flags\": [{}]\n}}\n",
            json_escape(&self.browser_binary),
            json_escape(&self.profile_dir),
            json_escape(&self.cache_dir),
            json_escape(&self.search_engine),
            json_escape(&self.ram_profile),
            self.strict_security,
            json_escape(&self.doh_provider),
            self.webrtc_privacy,
            self.tab_sleeping,
            self.hardware_accel,
            self.widevine_drm,
            self.adblocker,
            self.custom_ui,
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

fn ui_component_dir(sub: &str) -> PathBuf {
    let local_share = home_dir().join(".local/share/kestrel/ui").join(sub);
    if local_share.join("manifest.json").is_file() {
        return local_share;
    }
    repo_root().join("ui").join(sub)
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
    let pick_bundled_dir = |base: PathBuf| -> Option<PathBuf> {
        if base.join("manifest.json").is_file() {
            return Some(base);
        }
        if let Ok(entries) = fs::read_dir(&base) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() && p.join("manifest.json").is_file() {
                    return Some(p);
                }
            }
        }
        None
    };

    if !configured.is_empty() && configured != "auto" {
        let p = expand_tilde(configured);
        return pick_bundled_dir(p);
    }
    let home = home_dir();
    let candidates = [
        PathBuf::from("/usr/lib/chromium/WidevineCdm"),
        PathBuf::from("/usr/lib/ungoogled-chromium/WidevineCdm"),
        PathBuf::from("/opt/google/chrome/WidevineCdm"),
        home.join(".config/sh.cider.genten/WidevineCdm"),
        home.join(".config/net.imput.helium/WidevineCdm"),
    ];
    for c in candidates {
        if let Some(found) = pick_bundled_dir(c) {
            return Some(found);
        }
    }
    None
}

struct SearchEngineSpec {
    id: &'static str,
    short_name: &'static str,
    keyword: &'static str,
    favicon_url: &'static str,
    url: &'static str,
    suggest_url: &'static str,
    new_tab_url: &'static str,
    synced_guid: &'static str,
    prepopulate_id: i64,
}

fn search_engine_spec(name: &str) -> Option<SearchEngineSpec> {
    match name.trim().to_ascii_lowercase().as_str() {
        "duckduckgo" | "ddg" => Some(SearchEngineSpec {
            id: "3",
            short_name: "DuckDuckGo",
            keyword: "duckduckgo.com",
            favicon_url: "https://duckduckgo.com/favicon.ico",
            url: "https://duckduckgo.com/?q={searchTerms}",
            suggest_url: "https://duckduckgo.com/ac/?q={searchTerms}&type=list",
            new_tab_url: "https://duckduckgo.com/chrome_newtab",
            synced_guid: "485bf7d3-0215-45af-87dc-538868000092",
            prepopulate_id: 92,
        }),
        "brave" => Some(SearchEngineSpec {
            id: "4",
            short_name: "Brave",
            keyword: "search.brave.com",
            favicon_url: "https://cdn.search.brave.com/serp/v2/_app/immutable/assets/favicon.ico",
            url: "https://search.brave.com/search?q={searchTerms}",
            suggest_url: "https://search.brave.com/api/suggest?q={searchTerms}",
            new_tab_url: "",
            synced_guid: "485bf7d3-0215-45af-87dc-538868000109",
            prepopulate_id: 109,
        }),
        "startpage" => Some(SearchEngineSpec {
            id: "2",
            short_name: "Startpage",
            keyword: "startpage.com",
            favicon_url: "https://www.startpage.com/sp/cdn/favicons/favicon-16x16-gradient.png",
            url: "https://www.startpage.com/sp/search?query={searchTerms}",
            suggest_url: "https://www.startpage.com/osuggestions?q={searchTerms}",
            new_tab_url: "",
            synced_guid: "485bf7d3-0215-45af-87dc-538868000000",
            prepopulate_id: 0,
        }),
        "google" => Some(SearchEngineSpec {
            id: "2",
            short_name: "Google",
            keyword: "google.com",
            favicon_url: "https://www.google.com/favicon.ico",
            url: "https://www.google.com/search?q={searchTerms}",
            suggest_url: "https://www.google.com/complete/search?client=chrome&q={searchTerms}",
            new_tab_url: "",
            synced_guid: "485bf7d3-0215-45af-87dc-538868000001",
            prepopulate_id: 1,
        }),
        _ => None,
    }
}

fn build_template_url_json(spec: &SearchEngineSpec) -> JsonVal {
    JsonVal::Obj(vec![
        ("alternate_urls".into(), JsonVal::Arr(vec![])),
        ("contextual_search_url".into(), JsonVal::Str(String::new())),
        ("created_from_play_api".into(), JsonVal::Bool(false)),
        ("date_created".into(), JsonVal::Str("0".into())),
        ("doodle_url".into(), JsonVal::Str(String::new())),
        ("enforced_by_policy".into(), JsonVal::Bool(false)),
        ("favicon_url".into(), JsonVal::Str(spec.favicon_url.into())),
        ("featured_by_policy".into(), JsonVal::Bool(false)),
        ("id".into(), JsonVal::Str(spec.id.into())),
        ("input_encodings".into(), JsonVal::Arr(vec![JsonVal::Str("UTF-8".into())])),
        ("is_active".into(), JsonVal::Int(0)),
        ("keyword".into(), JsonVal::Str(spec.keyword.into())),
        ("last_modified".into(), JsonVal::Str("0".into())),
        ("last_visited".into(), JsonVal::Str("0".into())),
        ("new_tab_url".into(), JsonVal::Str(spec.new_tab_url.into())),
        ("originating_url".into(), JsonVal::Str(String::new())),
        ("policy_origin".into(), JsonVal::Int(0)),
        ("preconnect_to_search_url".into(), JsonVal::Bool(false)),
        ("prefetch_likely_navigations".into(), JsonVal::Bool(false)),
        ("prepopulate_id".into(), JsonVal::Int(spec.prepopulate_id)),
        ("safe_for_autoreplace".into(), JsonVal::Bool(true)),
        ("search_intent_params".into(), JsonVal::Arr(vec![])),
        ("search_url_post_params".into(), JsonVal::Str(String::new())),
        ("send_x_geo_header".into(), JsonVal::Bool(false)),
        ("short_name".into(), JsonVal::Str(spec.short_name.into())),
        ("starter_pack_id".into(), JsonVal::Int(0)),
        ("suggestions_url".into(), JsonVal::Str(spec.suggest_url.into())),
        ("suggestions_url_post_params".into(), JsonVal::Str(String::new())),
        ("synced_guid".into(), JsonVal::Str(spec.synced_guid.into())),
        ("url".into(), JsonVal::Str(spec.url.into())),
        ("usage_count".into(), JsonVal::Int(0)),
    ])
}

fn sql_escape(s: &str) -> String {
    s.replace('\'', "''")
}

fn json_set_obj_key(entries: &mut Vec<(String, JsonVal)>, key: &str, val: JsonVal) {
    if let Some((_, existing)) = entries.iter_mut().find(|(k, _)| k == key) {
        *existing = val;
    } else {
        entries.push((key.to_string(), val));
    }
}

fn json_set_nested(entries: &mut Vec<(String, JsonVal)>, path: &[&str], val: JsonVal) {
    if path.is_empty() {
        return;
    }
    if path.len() == 1 {
        json_set_obj_key(entries, path[0], val);
        return;
    }
    if let Some((_, JsonVal::Obj(sub))) = entries.iter_mut().find(|(k, _)| k == path[0]) {
        json_set_nested(sub, &path[1..], val);
    } else {
        let mut sub = Vec::new();
        json_set_nested(&mut sub, &path[1..], val);
        json_set_obj_key(entries, path[0], JsonVal::Obj(sub));
    }
}

fn doh_template_url(provider: &str) -> Option<&'static str> {
    match provider.trim().to_ascii_lowercase().as_str() {
        "quad9" => Some("https://dns.quad9.net/dns-query"),
        "cloudflare" => Some("https://cloudflare-dns.com/dns-query"),
        "mullvad" => Some("https://dns.mullvad.net/dns-query"),
        _ => None,
    }
}

fn ensure_profile_preferences(
    profile_dir: &Path,
    cfg: &Config,
    force_search: bool,
    cmd_hint: Option<&[String]>,
) {
    if profile_dir.join("SingletonLock").exists() {
        return;
    }

    let prefs_path = profile_dir.join("Default/Preferences");
    if !prefs_path.is_file() {
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

    if prefs_path.is_file() {
        if let Ok(raw) = fs::read_to_string(&prefs_path) {
            if let Ok(JsonVal::Obj(mut entries)) = JsonParser::new(&raw).parse_val() {
                if let Some(spec) = search_engine_spec(&cfg.search_engine) {
                    let already_set = if !force_search {
                        entries.iter().find(|(k, _)| k == "default_search_provider_data").is_some_and(|(_, v)| {
                            if let JsonVal::Obj(sub) = v {
                                sub.iter().any(|(sk, sv)| {
                                    if sk == "template_url_data" {
                                        if let JsonVal::Obj(tpl) = sv {
                                            return tpl.iter().any(|(tk, tv)| {
                                                tk == "url"
                                                    && matches!(tv, JsonVal::Str(u) if u != "http://{searchTerms}" && !u.is_empty())
                                            });
                                        }
                                    }
                                    false
                                })
                            } else {
                                false
                            }
                        })
                    } else {
                        false
                    };

                    if !already_set {
                        let tpl = build_template_url_json(&spec);
                        let dspd = JsonVal::Obj(vec![
                            ("mirrored_template_url_data".into(), tpl.clone()),
                            ("template_url_data".into(), tpl),
                        ]);
                        entries.retain(|(k, _)| k != "default_search_provider_data");
                        entries.push(("default_search_provider_data".into(), dspd));

                        if let Some((_, JsonVal::Obj(prot))) = entries.iter_mut().find(|(k, _)| k == "protection") {
                            if let Some((_, JsonVal::Obj(macs))) = prot.iter_mut().find(|(k, _)| k == "macs") {
                                macs.retain(|(k, _)| k != "default_search_provider_data");
                            }
                        }
                    }
                }

                if cfg.strict_security {
                    json_set_obj_key(&mut entries, "https_only_mode_enabled", JsonVal::Bool(true));
                    json_set_obj_key(&mut entries, "enable_do_not_track", JsonVal::Bool(true));
                    json_set_nested(&mut entries, &["profile", "cookie_controls_mode"], JsonVal::Int(1));
                    json_set_nested(&mut entries, &["net", "network_prediction_options"], JsonVal::Int(2));
                    json_set_nested(&mut entries, &["alternate_error_pages", "enabled"], JsonVal::Bool(false));
                    json_set_nested(&mut entries, &["autofill", "credit_card_enabled"], JsonVal::Bool(false));
                    json_set_nested(&mut entries, &["payments", "can_make_payment_enabled"], JsonVal::Bool(false));
                    json_set_nested(&mut entries, &["safebrowsing", "enabled"], JsonVal::Bool(false));
                }

                if let Some(tpl) = doh_template_url(&cfg.doh_provider) {
                    json_set_nested(&mut entries, &["dns_over_https", "mode"], JsonVal::Str("secure".into()));
                    json_set_nested(&mut entries, &["dns_over_https", "templates"], JsonVal::Str(tpl.into()));
                } else {
                    json_set_nested(&mut entries, &["dns_over_https", "mode"], JsonVal::Str("off".into()));
                }

                if cfg.tab_sleeping {
                    json_set_nested(
                        &mut entries,
                        &["performance_tuning", "high_efficiency_mode", "state"],
                        JsonVal::Int(2),
                    );
                }

                let _ = fs::write(&prefs_path, json_serialize(&JsonVal::Obj(entries)));
            }
        }
    }

    let web_data = profile_dir.join("Default/Web Data");
    if web_data.is_file() && which("sqlite3").is_some() {
        if let Some(spec) = search_engine_spec(&cfg.search_engine) {
            let sql = format!(
                "UPDATE keywords SET short_name='{}', keyword='{}', favicon_url='{}', url='{}', suggest_url='{}', prepopulate_id={} WHERE id=2 AND (short_name='No Search' OR url='http://{{searchTerms}}');",
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
    }
}

fn install_webstore() -> Result<(), String> {
    println!("Fetching latest chromium-web-store release metadata...");
    let api_out = Command::new("curl")
        .args([
            "-fsSL",
            "-H",
            "User-Agent: kestrel-installer",
            "https://api.github.com/repos/NeverDecouple/chromium-web-store/releases/latest",
        ])
        .output()
        .map_err(|e| format!("Failed to run curl: {e}"))?;

    if !api_out.status.success() {
        return Err("Failed to query NeverDecouple/chromium-web-store releases API".into());
    }

    let json_str = String::from_utf8(api_out.stdout).map_err(|e| e.to_string())?;
    let parsed = JsonParser::new(&json_str).parse_val()?;
    let url = find_download_url(&parsed, ".crx")
        .ok_or_else(|| "No .crx asset found in latest chromium-web-store release".to_string())?;

    let ext_dir = home_dir().join(".local/share/kestrel/extensions");
    fs::create_dir_all(&ext_dir).map_err(|e| e.to_string())?;
    let crx_path = ext_dir.join("chromium-web-store.crx");

    println!("Downloading {url}...");
    let dl_status = Command::new("curl")
        .args(["-fL", "--progress-bar", "-o"])
        .arg(&crx_path)
        .arg(&url)
        .status()
        .map_err(|e| format!("Failed to run curl: {e}"))?;
    if !dl_status.success() {
        return Err("Download of chromium-web-store.crx failed".into());
    }

    println!(
        "Downloaded {}\nTo install it in Kestrel: open Kestrel and drag-and-drop the .crx onto chrome://extensions (or run: kestrel {})",
        crx_path.display(),
        crx_path.display()
    );
    Ok(())
}

fn sync_widevine_symlink(enabled: bool, widevine_cfg: &str) {
    let target_link = home_dir().join(".local/share/kestrel/runtime/WidevineCdm");
    if enabled {
        if let Some(src) = resolve_widevine_path(widevine_cfg) {
            if let Some(parent) = target_link.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if target_link.is_symlink() {
                let _ = fs::remove_file(&target_link);
            }
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

    let mut enable_features: Vec<&str> = Vec::new();
    let mut disable_features: Vec<&str> = Vec::new();

    if cfg.strict_security {
        flags.push("--site-per-process".to_string());
        enable_features.extend(["HttpsUpgrades", "HttpsFirstBalancedModeAutoEnable"]);
        disable_features.extend([
            "PrivacySandboxAdsAPIsOverride",
            "InterestGroupStorage",
            "Fledge",
            "Topics",
            "AttributionReportingCrossAppWeb",
            "WebBluetooth",
            "WebUSB",
            "PaymentRequest",
        ]);
    } else {
        enable_features.push("HttpsUpgrades");
    }

    if cfg.webrtc_privacy {
        flags.push("--force-webrtc-ip-handling-policy=disable_non_proxied_udp".to_string());
    }

    if cfg.tab_sleeping {
        let (proc_limit, v8_mb) = if cfg.ram_profile.eq_ignore_ascii_case("ultra") {
            (cfg.renderer_process_limit.min(4), cfg.v8_max_old_space_mb.min(512))
        } else {
            (cfg.renderer_process_limit, cfg.v8_max_old_space_mb)
        };
        flags.push(format!("--renderer-process-limit={proc_limit}"));
        flags.push(format!("--js-flags=--max-old-space-size={v8_mb}"));
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
                "--disable-translate",
                "--disable-speech-api",
            ]
            .into_iter()
            .map(String::from),
        );
        enable_features.extend(["HighEfficiencyModeAvailable", "MemorySaverModeAvailable"]);
        disable_features.extend([
            "NtpFooter",
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

    let mut load_exts: Vec<String> = Vec::new();
    let adb = adblocker_dir();
    if cfg.adblocker && adb.join("manifest.json").is_file() {
        load_exts.push(adb.display().to_string());
    }
    if cfg.custom_ui {
        for sub in ["newtab", "theme"] {
            let dir = ui_component_dir(sub);
            if dir.join("manifest.json").is_file() {
                load_exts.push(dir.display().to_string());
            }
        }
    }

    if !load_exts.is_empty() {
        let joined = load_exts.join(",");
        flags.push(format!("--load-extension={joined}"));
        if !cfg.extension_support {
            flags.push(format!("--disable-extensions-except={joined}"));
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

const TOGGLES: [(&str, &str, &str); 8] = [
    ("strict_security", "Strict Security (Site Isolation + HTTPS-First + 3P Cookie Block)", "Enforce --site-per-process, HTTPS-First, block 3rd-party cookies & Privacy Sandbox"),
    ("webrtc_privacy", "WebRTC IP Privacy", "Prevent WebRTC local IP leaks (--force-webrtc-ip-handling-policy=disable_non_proxied_udp)"),
    ("tab_sleeping", "Tab Sleeping & Low-RAM Mode", "Aggressive Memory Saver, renderer cap, V8 heap limit, no background net/telemetry"),
    ("hardware_accel", "Hardware Acceleration (VA-API + WebGL)", "GPU rasterization, zero-copy, VA-API video decode/encode for YouTube"),
    ("widevine_drm", "Widevine DRM", "Enable Widevine CDM symlink for protected media (Spotify, Netflix, etc.)"),
    ("adblocker", "Built-in Adblocker (Kestrel Shield)", "Load custom low-RAM MV3 adblocker extension"),
    ("custom_ui", "Kestrel Dark Theme & Custom New Tab", "Load Kestrel obsidian dark theme and custom start page"),
    ("extension_support", "Extension & Web Store Support", "Allow installing extensions and .crx packages from Chrome Web Store"),
];

fn get_toggle(cfg: &Config, key: &str) -> Option<bool> {
    match key {
        "strict_security" => Some(cfg.strict_security),
        "webrtc_privacy" => Some(cfg.webrtc_privacy),
        "tab_sleeping" => Some(cfg.tab_sleeping),
        "hardware_accel" => Some(cfg.hardware_accel),
        "widevine_drm" => Some(cfg.widevine_drm),
        "adblocker" => Some(cfg.adblocker),
        "custom_ui" => Some(cfg.custom_ui),
        "extension_support" => Some(cfg.extension_support),
        _ => None,
    }
}

fn set_toggle(cfg: &mut Config, key: &str, val: bool) -> bool {
    match key {
        "strict_security" => cfg.strict_security = val,
        "webrtc_privacy" => cfg.webrtc_privacy = val,
        "tab_sleeping" => cfg.tab_sleeping = val,
        "hardware_accel" => cfg.hardware_accel = val,
        "widevine_drm" => cfg.widevine_drm = val,
        "adblocker" => cfg.adblocker = val,
        "custom_ui" => cfg.custom_ui = val,
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
    println!("  [9]  [{}] Renderer Process Limit (renderer_process_limit, 1-32)", cfg.renderer_process_limit);
    println!("  [10] [{}] V8 Max Heap per Tab (MB) (v8_max_old_space_mb, 128-4096)", cfg.v8_max_old_space_mb);
    println!("  [11] [{}] RAM Profile (ultra | balanced)", cfg.ram_profile);
    println!("  [12] [{}] DNS-over-HTTPS Provider (quad9 | cloudflare | mullvad | off)", cfg.doh_provider);
    println!("  [13] [{}] Default Search Engine (duckduckgo | brave | startpage | google)", cfg.search_engine);
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
            "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" => {
                let idx = choice.parse::<usize>().unwrap() - 1;
                let key = TOGGLES[idx].0;
                let cur = get_toggle(&cfg, key).unwrap_or(false);
                set_toggle(&mut cfg, key, !cur);
                let _ = save_config(&cfg);
            }
            "9" => {
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
            "10" => {
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
            "11" => {
                cfg.ram_profile = if cfg.ram_profile == "ultra" {
                    cfg.renderer_process_limit = 6;
                    cfg.v8_max_old_space_mb = 768;
                    "balanced".into()
                } else {
                    cfg.renderer_process_limit = 4;
                    cfg.v8_max_old_space_mb = 512;
                    "ultra".into()
                };
                let _ = save_config(&cfg);
            }
            "12" => {
                print!("Enter DoH provider (quad9, cloudflare, mullvad, off) [{}]: ", cfg.doh_provider);
                let _ = stdout.flush();
                let mut raw = String::new();
                if stdin.lock().read_line(&mut raw).is_ok() {
                    let val = raw.trim().to_ascii_lowercase();
                    if matches!(val.as_str(), "quad9" | "cloudflare" | "mullvad" | "off") {
                        cfg.doh_provider = val;
                        let _ = save_config(&cfg);
                    }
                }
            }
            "13" => {
                print!("Enter search engine (duckduckgo, brave, startpage, google) [{}]: ", cfg.search_engine);
                let _ = stdout.flush();
                let mut raw = String::new();
                if stdin.lock().read_line(&mut raw).is_ok() {
                    let val = raw.trim().to_ascii_lowercase();
                    if search_engine_spec(&val).is_some() {
                        cfg.search_engine = val;
                        let _ = save_config(&cfg);
                        ensure_profile_preferences(&expand_tilde(&cfg.profile_dir), &cfg, true, None);
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
            "strict_security" | "webrtc_privacy" | "tab_sleeping" | "hardware_accel" | "widevine_drm" | "adblocker" | "custom_ui" | "extension_support" => {
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
            "ram_profile" => {
                let low = val.trim().to_ascii_lowercase();
                if !matches!(low.as_str(), "ultra" | "balanced") {
                    eprintln!("Unsupported ram_profile '{val}'. Choose: ultra, balanced");
                    return 1;
                }
                cfg.ram_profile = low.clone();
                if low == "ultra" {
                    cfg.renderer_process_limit = 4;
                    cfg.v8_max_old_space_mb = 512;
                } else {
                    cfg.renderer_process_limit = 6;
                    cfg.v8_max_old_space_mb = 768;
                }
            }
            "doh_provider" => {
                let low = val.trim().to_ascii_lowercase();
                if !matches!(low.as_str(), "quad9" | "cloudflare" | "mullvad" | "off") {
                    eprintln!("Unsupported doh_provider '{val}'. Choose: quad9, cloudflare, mullvad, off");
                    return 1;
                }
                cfg.doh_provider = low;
            }
            "search_engine" => {
                let low = val.trim().to_ascii_lowercase();
                if search_engine_spec(&low).is_none() {
                    eprintln!("Unsupported search_engine '{val}'. Choose: duckduckgo, brave, startpage, google");
                    return 1;
                }
                cfg.search_engine = low;
                ensure_profile_preferences(&expand_tilde(&cfg.profile_dir), &cfg, true, None);
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

fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let mut msg = data.to_vec();
    let bit_len = (data.len() as u64) * 8;
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[4 * i], chunk[4 * i + 1], chunk[4 * i + 2], chunk[4 * i + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

fn unpacked_extension_id(path: &Path) -> Option<String> {
    let canon = fs::canonicalize(path).ok()?;
    let digest = sha256_bytes(canon.to_string_lossy().as_bytes());
    let mut id = String::with_capacity(32);
    for byte in &digest[..16] {
        id.push((b'a' + (byte >> 4)) as char);
        id.push((b'a' + (byte & 0x0f)) as char);
    }
    Some(id)
}

fn ensure_native_messaging_manifest(profile_dir: &Path) {
    let host_bin = home_dir().join(".local/bin/kestrel-native-host");
    let host_path = if host_bin.is_file() {
        host_bin
    } else if let Ok(exe) = env::current_exe() {
        exe
    } else {
        host_bin
    };

    let mut origins: Vec<String> = Vec::new();
    for candidate in [
        home_dir().join(".local/share/kestrel/ui/newtab"),
        repo_root().join("ui/newtab"),
    ] {
        if let Some(id) = unpacked_extension_id(&candidate) {
            let origin = format!("chrome-extension://{id}/");
            if !origins.contains(&origin) {
                origins.push(origin);
            }
        }
    }
    if origins.is_empty() {
        return;
    }

    let origins_json = origins
        .iter()
        .map(|o| json_escape(o))
        .collect::<Vec<_>>()
        .join(", ");
    let manifest = format!(
        "{{\n  \"name\": \"org.kestrel.settings\",\n  \"description\": \"Kestrel Browser Settings Host\",\n  \"path\": {},\n  \"type\": \"stdio\",\n  \"allowed_origins\": [{}]\n}}\n",
        json_escape(&host_path.to_string_lossy()),
        origins_json
    );

    let targets = [
        profile_dir.join("NativeMessagingHosts/org.kestrel.settings.json"),
        home_dir().join(".config/chromium/NativeMessagingHosts/org.kestrel.settings.json"),
    ];
    for t in targets {
        if let Some(parent) = t.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(t, &manifest);
    }
}

fn send_native_msg(stdout: &mut impl Write, json_body: &str) -> io::Result<()> {
    let bytes = json_body.as_bytes();
    let len = (bytes.len() as u32).to_le_bytes();
    stdout.write_all(&len)?;
    stdout.write_all(bytes)?;
    stdout.flush()
}

fn run_native_messaging() -> i32 {
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();
    let mut len_buf = [0u8; 4];

    loop {
        match stdin.read_exact(&mut len_buf) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return 0,
            Err(_) => return 1,
        }
        let len = u32::from_le_bytes(len_buf) as usize;
        if len == 0 || len > 1024 * 1024 {
            let _ = send_native_msg(&mut stdout, "{\"ok\":false,\"error\":\"invalid frame length\"}");
            return 1;
        }
        let mut msg_buf = vec![0u8; len];
        if stdin.read_exact(&mut msg_buf).is_err() {
            return 1;
        }
        let Ok(msg_str) = std::str::from_utf8(&msg_buf) else {
            let _ = send_native_msg(&mut stdout, "{\"ok\":false,\"error\":\"invalid utf-8\"}");
            continue;
        };
        let Ok(JsonVal::Obj(entries)) = JsonParser::new(msg_str).parse_val() else {
            let _ = send_native_msg(&mut stdout, "{\"ok\":false,\"error\":\"invalid json\"}");
            continue;
        };

        let action = entries
            .iter()
            .find(|(k, _)| k == "action")
            .and_then(|(_, v)| if let JsonVal::Str(s) = v { Some(s.as_str()) } else { None })
            .unwrap_or("get");

        let mut cfg = load_or_create_config();
        let resp = match action {
            "get" => format!(
                "{{\"ok\":true,\"restart_required\":false,\"config\":{}}}",
                cfg.to_json().trim()
            ),
            "reset" => {
                cfg = Config::default();
                let _ = save_config(&cfg);
                format!(
                    "{{\"ok\":true,\"restart_required\":true,\"config\":{}}}",
                    cfg.to_json().trim()
                )
            }
            "set" => {
                let key = entries
                    .iter()
                    .find(|(k, _)| k == "key")
                    .and_then(|(_, v)| if let JsonVal::Str(s) = v { Some(s.as_str()) } else { None })
                    .unwrap_or("");
                let val = entries.iter().find(|(k, _)| k == "value").map(|(_, v)| v);
                let mut err_msg: Option<String> = None;
                match (key, val) {
                    (k, Some(JsonVal::Bool(b))) if set_toggle(&mut cfg, k, *b) => {}
                    ("renderer_process_limit", Some(JsonVal::Int(n))) => cfg.renderer_process_limit = (*n).clamp(1, 32),
                    ("v8_max_old_space_mb", Some(JsonVal::Int(n))) => cfg.v8_max_old_space_mb = (*n).clamp(128, 4096),
                    ("ram_profile", Some(JsonVal::Str(s))) if matches!(s.as_str(), "ultra" | "balanced") => {
                        cfg.ram_profile = s.clone();
                        if s == "ultra" {
                            cfg.renderer_process_limit = 4;
                            cfg.v8_max_old_space_mb = 512;
                        } else {
                            cfg.renderer_process_limit = 6;
                            cfg.v8_max_old_space_mb = 768;
                        }
                    }
                    ("doh_provider", Some(JsonVal::Str(s))) if matches!(s.as_str(), "quad9" | "cloudflare" | "mullvad" | "off") => {
                        cfg.doh_provider = s.clone();
                    }
                    ("search_engine", Some(JsonVal::Str(s))) if search_engine_spec(s).is_some() => {
                        cfg.search_engine = s.clone();
                    }
                    _ => err_msg = Some(format!("Unsupported key/value for '{key}'")),
                }
                if let Some(err) = err_msg {
                    format!("{{\"ok\":false,\"error\":{}}}", json_escape(&err))
                } else {
                    let _ = save_config(&cfg);
                    ensure_profile_preferences(&expand_tilde(&cfg.profile_dir), &cfg, key == "search_engine", None);
                    format!(
                        "{{\"ok\":true,\"restart_required\":true,\"config\":{}}}",
                        cfg.to_json().trim()
                    )
                }
            }
            other => format!("{{\"ok\":false,\"error\":{}}}", json_escape(&format!("Unknown action: {other}"))),
        };
        if send_native_msg(&mut stdout, &resp).is_err() {
            return 1;
        }
    }
}

fn print_launcher_help() {
    println!(
        "Usage: kestrel [LAUNCHER_OPTION | CHROMIUM_ARGS...]\n\n\
         Launcher options:\n\
           install-runtime        Download standalone Ungoogled Chromium runtime\n\
           install-webstore       Download chromium-web-store .crx for Chrome Web Store\n\
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
    if bin_name == "kestrel-native-host" {
        process::exit(run_native_messaging());
    }

    if let Some(first) = args.get(1).map(String::as_str) {
        match first {
            "-h" | "--help" => {
                print_launcher_help();
                return;
            }
            "--native-messaging" => {
                process::exit(run_native_messaging());
            }
            "install-runtime" | "--install-runtime" => {
                if let Err(e) = install_runtime() {
                    eprintln!("Error: {e}");
                    process::exit(1);
                }
                return;
            }
            "install-webstore" | "--install-webstore" => {
                if let Err(e) = install_webstore() {
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

    sync_widevine_symlink(cfg.widevine_drm, &cfg.widevine_path);
    ensure_native_messaging_manifest(&profile_dir);
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

    ensure_profile_preferences(&profile_dir, &cfg, false, Some(&cmd));

    let err = Command::new(&cmd[0])
        .env("CHROME_DESKTOP", "kestrel.desktop")
        .args(&cmd[1..])
        .args(&flags)
        .args(&pass_args)
        .exec();
    eprintln!("Failed to launch {}: {err}", cmd[0]);
    process::exit(1);
}
