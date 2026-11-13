//! Logix IDE's remote config (Remote updates, in the website repo's
//! `CLAUDE.md`): which release of each of its dependencies Logix IDE uses.
//! It's the version folder `remote-config/logix-ide/<NNN>/` in this repo, by
//! the config version this build hard-codes, read from GitHub and cached in
//! the data folder: `channels.jsonc` says which Logix IDE releases are on
//! which channel, and Logix IDE reads its channel's config file there,
//! `<channel>.jsonc` (`prod.jsonc` when its channel has none). Its channel:
//! `LOGIX_IDE_CHANNEL` when set; else, for a release build, the channel
//! `channels.jsonc` lists its own release under (dev when unlisted); a local
//! build is always dev. There's no built-in pin: the repo's files are the
//! pin lists, so a change there reaches every Logix IDE on this config
//! version and channel without a new release. Plain Rust, no Tauri.
//!
//! A config names, per dependency, a GitHub repo, a release tag and a file
//! template (`${PLATFORM}`, `${YYYYMMDD}`). Resolving it for this platform
//! takes each file's SHA-256 from its release's `SHA256SUMS`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};

/// The repo whose `remote-config/` holds Logix IDE's config.
pub const CONFIG_REPO: &str = "openlogix/logix-ide";
/// Logix IDE's config version: which `config-NNN.jsonc` it reads. Raise it
/// only when a new Logix IDE can't share the current file (a dependency
/// added or dropped, or a release older Logix IDEs can't use).
pub const CONFIG_VERSION: &str = "001";
/// The product folder in `remote-config/` (the package name).
const PRODUCT: &str = "logix-ide";
/// Where raw repo files are read from.
const RAW_BASE: &str = "https://raw.githubusercontent.com";
/// Where release files are downloaded from.
const RELEASE_BASE: &str = "https://github.com";
/// The diagnostic override (as Apio's): the `https://` URL of a version
/// folder, such as a pull request's on its branch, or a local folder, read
/// instead of the regular one. A bad value is an error, never a fallback.
pub const DEV_REMOTE_CONFIG_VARIABLE: &str = "LOGIX_IDE_DEV_REMOTE_CONFIG";
/// The local override of the channel (a runtime setting, no `DEV_`, since
/// `dev` is a channel name).
pub const CHANNEL_VARIABLE: &str = "LOGIX_IDE_CHANNEL";
/// Each version folder's channel lists.
pub const CHANNELS_FILE: &str = "channels.jsonc";
/// Whether this is a release build (build.rs): a local build is always dev,
/// since its version (today's date) could match a prod release's tag.
pub const IS_RELEASE_BUILD: bool = env!("LOGIX_IDE_BUILD_RELEASE").len() == 4; // "true"
/// How long a fetched config is used before it's fetched again.
pub const MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);
/// The schema every remote config names.
const SCHEMA: &str = "https://www.openlogix.org/schemas/logix-remote-config-v1.json";
/// Logix Engine's package name, its key in the config.
pub const ENGINE: &str = "logix-engine";
/// The examples' package name (built and published by this repo's
/// pre-release workflow, `scripts/pack-examples.py`).
pub const EXAMPLES: &str = "logix-ide-examples";
/// The dependencies every config of this config version must name, and no
/// others.
const DEPENDENCIES: [&str; 2] = [ENGINE, EXAMPLES];

/// The version folder's path in the repo.
pub fn repo_path() -> String {
    format!("remote-config/{PRODUCT}/{CONFIG_VERSION}")
}

/// The regular version folder's address: its raw files on `main`.
pub fn default_url() -> String {
    format!("{RAW_BASE}/{CONFIG_REPO}/main/{}/", repo_path())
}

/// Where Logix IDE reads its remote config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigSource {
    /// A version folder: an `https://` URL or a local folder.
    pub location: String,
    /// True for the diagnostic override.
    pub is_override: bool,
}

impl ConfigSource {
    pub fn regular() -> Self {
        ConfigSource {
            location: default_url(),
            is_override: false,
        }
    }

    /// The source to use: the override from `LOGIX_IDE_DEV_REMOTE_CONFIG`
    /// when it's set (unset or empty means none), else the regular one. An
    /// override that's neither an `https://` URL nor an existing folder is an
    /// error.
    pub fn from_variable(value: Option<std::ffi::OsString>) -> Result<Self> {
        let Some(value) = value.filter(|v| !v.is_empty()) else {
            return Ok(Self::regular());
        };
        let location = value.to_string_lossy().into_owned();
        if !location.starts_with("https://") && !Path::new(&location).is_dir() {
            bail!(
                "{DEV_REMOTE_CONFIG_VARIABLE} must be an https:// URL or the path of a remote \
                 config version folder, not {location}"
            );
        }
        Ok(ConfigSource {
            location,
            is_override: true,
        })
    }

    /// The source for this run, from the environment.
    pub fn from_env() -> Result<Self> {
        Self::from_variable(std::env::var_os(DEV_REMOTE_CONFIG_VARIABLE))
    }

    fn is_url(&self) -> bool {
        self.location.starts_with("https://")
    }

    /// A file of the version folder: a URL, or a local path.
    fn file(&self, name: &str) -> String {
        if self.is_url() {
            format!("{}/{name}", self.location.trim_end_matches('/'))
        } else {
            Path::new(&self.location).join(name).display().to_string()
        }
    }
}

/// The channel Logix IDE reads, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Channel {
    /// Such as prod or dev.
    pub name: String,
    /// Why: "listed" (channels.jsonc lists this release), "default" (it
    /// doesn't: dev), "local build" (always dev) or "LOGIX_IDE_CHANNEL".
    pub how: String,
    /// The config file read: `<channel>.jsonc`, or `prod.jsonc` when the
    /// channel has none.
    pub file: String,
}

/// A valid channel name: lowercase letters, digits and dashes, a letter first.
pub fn is_channel_name(s: &str) -> bool {
    s.bytes().next().is_some_and(|c| c.is_ascii_lowercase())
        && s.bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}

/// The channel override from `LOGIX_IDE_CHANNEL` (unset or empty means
/// none); a value that isn't a channel name is an error.
pub fn channel_from_variable(value: Option<std::ffi::OsString>) -> Result<Option<String>> {
    match value.filter(|v| !v.is_empty()) {
        None => Ok(None),
        Some(v) => {
            let v = v.to_string_lossy().into_owned();
            if !is_channel_name(&v) {
                bail!("{CHANNEL_VARIABLE} must be a channel name, such as prod or dev, not {v:?}");
            }
            Ok(Some(v))
        }
    }
}

/// The channel override for this run, from the environment.
pub fn channel_variable() -> Result<Option<String>> {
    channel_from_variable(std::env::var_os(CHANNEL_VARIABLE))
}

/// channels.jsonc: channel name to its release tags.
pub fn parse_channels(text: &str) -> Result<BTreeMap<String, Vec<String>>> {
    let json: serde_json::Value =
        serde_json::from_str(&strip_comments(text)?).context("it isn't valid JSONC")?;
    let obj = json
        .as_object()
        .ok_or_else(|| anyhow!("it isn't a JSON object"))?;
    let mut out = BTreeMap::new();
    for (name, tags) in obj.iter().filter(|(k, _)| *k != "$schema") {
        let tags = tags
            .as_array()
            .ok_or_else(|| anyhow!("{name} isn't a list"))?
            .iter()
            .map(|t| t.as_str().map(str::to_string))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| anyhow!("{name} lists something that isn't a release tag"))?;
        out.insert(name.clone(), tags);
    }
    Ok(out)
}

/// Logix IDE's channel, by the rules in the module's comment (`file` is the
/// channel's own; reading falls back to prod.jsonc). `release` says whether
/// this is a release build, and `tag` is this release's tag.
pub fn choose_channel(
    channels: &BTreeMap<String, Vec<String>>,
    variable: Option<&str>,
    release: bool,
    tag: &str,
) -> Result<Channel> {
    let (name, how) = if let Some(v) = variable {
        if v != "prod" && v != "dev" && !channels.contains_key(v) {
            bail!("{CHANNEL_VARIABLE}: {v} isn't a channel (prod, dev or one in {CHANNELS_FILE})");
        }
        (v.to_string(), CHANNEL_VARIABLE)
    } else if !release {
        ("dev".to_string(), "local build")
    } else {
        match channels
            .iter()
            .find(|(_, tags)| tags.iter().any(|t| t == tag))
        {
            Some((name, _)) => (name.clone(), "listed"),
            None => ("dev".to_string(), "default"),
        }
    };
    Ok(Channel {
        file: format!("{name}.jsonc"),
        name,
        how: how.to_string(),
    })
}

/// One dependency in a config, as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub repo: String,
    pub release: String,
    pub file: String,
}

/// A dependency resolved for this platform: what to download and check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    /// `owner/name`.
    pub repo: String,
    /// The release tag, `yyyy-mm-dd`.
    pub release: String,
    /// The release file's name.
    pub file: String,
    pub url: String,
    /// From the release's `SHA256SUMS`, lowercase hex.
    pub sha256: String,
}

/// JSONC to JSON: removes `//` and `/* */` comments outside strings.
pub fn strip_comments(text: &str) -> Result<String> {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_string = false;
    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            if c == '\\' {
                if let Some(n) = chars.next() {
                    out.push(n);
                }
            } else if c == '"' {
                in_string = false;
            }
        } else if c == '"' {
            in_string = true;
            out.push(c);
        } else if c == '/' && chars.peek() == Some(&'/') {
            for n in chars.by_ref() {
                if n == '\n' {
                    out.push('\n');
                    break;
                }
            }
        } else if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            let mut prev = ' ';
            let mut closed = false;
            for n in chars.by_ref() {
                if n == '\n' {
                    out.push('\n');
                }
                if prev == '*' && n == '/' {
                    closed = true;
                    break;
                }
                prev = n;
            }
            if !closed {
                bail!("a /* comment isn't closed");
            }
        } else {
            out.push(c);
        }
    }
    Ok(out)
}

/// Whether `tag` is a release tag, `yyyy-mm-dd`.
pub fn is_date_tag(tag: &str) -> bool {
    let b = tag.as_bytes();
    b.len() == 10
        && b.iter().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                *c == b'-'
            } else {
                c.is_ascii_digit()
            }
        })
}

/// Reads a config's text: its dependencies, by package name. Checks what
/// the schema (logix-remote-config-v1) does that matters here, and that it
/// names exactly the dependencies this Logix IDE needs.
pub fn parse(text: &str) -> Result<BTreeMap<String, Entry>> {
    let json: serde_json::Value =
        serde_json::from_str(&strip_comments(text)?).context("it isn't valid JSONC")?;
    let obj = json
        .as_object()
        .ok_or_else(|| anyhow!("it isn't a JSON object"))?;
    if obj.get("$schema").and_then(|v| v.as_str()) != Some(SCHEMA) {
        bail!("its $schema must be {SCHEMA}");
    }
    let mut out = BTreeMap::new();
    for (name, dep) in obj.iter().filter(|(k, _)| *k != "$schema") {
        let fields = dep
            .as_object()
            .ok_or_else(|| anyhow!("{name}: not an object"))?;
        if let Some(extra) = fields
            .keys()
            .find(|k| !["repo", "release", "file"].contains(&k.as_str()))
        {
            bail!("{name}: unknown key {extra}");
        }
        let field = |key: &str| -> Result<String> {
            fields
                .get(key)
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .ok_or_else(|| anyhow!("{name}: no {key}"))
        };
        let repo = field("repo")?;
        if !repo
            .split_once('/')
            .is_some_and(|(o, r)| safe(o) && safe(r))
        {
            bail!("{name}: repo must be owner/name, not {repo:?}");
        }
        let release = field("release")?;
        if !is_date_tag(&release) {
            bail!("{name}: release must be yyyy-mm-dd, not {release:?}");
        }
        let file = field("file")?;
        if !safe(&file.replace("${PLATFORM}", "x").replace("${YYYYMMDD}", "x")) {
            bail!("{name}: bad file {file:?}");
        }
        out.insert(
            name.clone(),
            Entry {
                repo,
                release,
                file,
            },
        );
    }
    let names: Vec<&str> = out.keys().map(String::as_str).collect();
    let mut wanted = DEPENDENCIES.to_vec();
    wanted.sort();
    if names != wanted {
        bail!(
            "it must name exactly {}, not {}",
            wanted.join(", "),
            names.join(", ")
        );
    }
    Ok(out)
}

/// A single path component made of safe characters.
fn safe(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with('.')
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
}

/// A release file's name for this platform (an engine platform id, such
/// as `darwin-arm64`).
pub fn file_name(entry: &Entry, platform: &str) -> String {
    entry
        .file
        .replace("${PLATFORM}", platform)
        .replace("${YYYYMMDD}", &entry.release.replace('-', ""))
}

/// The SHA-256 that a `SHA256SUMS` text gives `file`.
pub fn sha256_in(sums: &str, file: &str) -> Option<String> {
    sums.lines()
        .filter_map(|l| l.split_once(char::is_whitespace))
        .find(|(_, f)| f.trim().trim_start_matches('*') == file)
        .map(|(sha, _)| sha.to_lowercase())
        .filter(|sha| sha.len() == 64 && sha.bytes().all(|c| c.is_ascii_hexdigit()))
}

/// An HTTP client for the config and its checksums: short timeouts, since
/// these are small files read before an operation.
fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .user_agent(concat!("logix-ide/", env!("LOGIX_IDE_BUILD_VERSION")))
        .build()?)
}

async fn get_text(client: &reqwest::Client, url: &str) -> Result<String> {
    get_optional(client, url)
        .await?
        .ok_or_else(|| anyhow!("can't read {url}: HTTP 404 Not Found"))
}

/// A URL's text; None for a 404 (a file that isn't there).
async fn get_optional(client: &reqwest::Client, url: &str) -> Result<Option<String>> {
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("can't read {url}"))?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let response = response
        .error_for_status()
        .with_context(|| format!("can't read {url}"))?;
    Ok(Some(response.text().await?))
}

/// A version folder's file; None when it isn't there.
async fn read_file(
    source: &ConfigSource,
    name: &str,
    client: &reqwest::Client,
) -> Result<Option<String>> {
    let at = source.file(name);
    if source.is_url() {
        get_optional(client, &at).await
    } else if !Path::new(&at).exists() {
        Ok(None)
    } else {
        Ok(Some(
            std::fs::read_to_string(&at).with_context(|| format!("can't read {at}"))?,
        ))
    }
}

/// Reads the version folder from its source (`channels.jsonc`, then the
/// channel's config file, or `prod.jsonc` when the channel has none: a
/// rule, not an error) and resolves it for `platform`: each dependency's
/// file, URL and SHA-256.
pub async fn fetch(source: &ConfigSource, platform: &str) -> Result<Cached> {
    let client = client()?;
    let channels_text = read_file(source, CHANNELS_FILE, &client)
        .await?
        .ok_or_else(|| anyhow!("{}: there's no {CHANNELS_FILE}", source.location))?;
    let channels = parse_channels(&channels_text)
        .with_context(|| format!("{}{CHANNELS_FILE} isn't valid", source.file("")))?;
    let tag = crate::version::APP_VERSION.replace('.', "-");
    let mut channel = choose_channel(
        &channels,
        channel_variable()?.as_deref(),
        IS_RELEASE_BUILD,
        &tag,
    )?;
    let text = match read_file(source, &channel.file, &client).await? {
        Some(t) => t,
        None if channel.name != "prod" => {
            channel.file = "prod.jsonc".into();
            read_file(source, "prod.jsonc", &client)
                .await?
                .ok_or_else(|| {
                    anyhow!(
                        "{}: there's neither {}.jsonc nor prod.jsonc",
                        source.location,
                        channel.name
                    )
                })?
        }
        None => bail!("{}: there's no prod.jsonc", source.location),
    };
    let entries = parse(&text).with_context(|| {
        format!(
            "the remote config {} isn't valid",
            source.file(&channel.file)
        )
    })?;
    let mut deps = BTreeMap::new();
    for (name, entry) in entries {
        let file = file_name(&entry, platform);
        let base = format!(
            "{RELEASE_BASE}/{}/releases/download/{}",
            entry.repo, entry.release
        );
        let sums_url = format!("{base}/SHA256SUMS");
        let sums = get_text(&client, &sums_url).await?;
        let sha256 =
            sha256_in(&sums, &file).ok_or_else(|| anyhow!("{sums_url} doesn't list {file}"))?;
        deps.insert(
            name,
            Dependency {
                url: format!("{base}/{file}"),
                repo: entry.repo,
                release: entry.release,
                file,
                sha256,
            },
        );
    }
    Ok(Cached {
        source: source.location.clone(),
        config_version: CONFIG_VERSION.into(),
        channel,
        fetched: now(),
        dependencies: deps,
    })
}

/// The cached, resolved config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cached {
    /// Where it was read: a version folder's URL or a local folder.
    pub source: String,
    pub config_version: String,
    /// The channel and the config file read.
    pub channel: Channel,
    /// When it was read, in seconds since 1970 (UTC).
    pub fetched: u64,
    pub dependencies: BTreeMap<String, Dependency>,
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl Cached {
    /// Whether it's from this source and config version, read with the same
    /// channel override (none, or the same LOGIX_IDE_CHANNEL), with every
    /// dependency this Logix IDE needs.
    pub fn matches(&self, source: &ConfigSource) -> bool {
        let variable = channel_variable().ok().flatten();
        let cached_variable =
            (self.channel.how == CHANNEL_VARIABLE).then(|| self.channel.name.clone());
        self.source == source.location
            && self.config_version == CONFIG_VERSION
            && variable == cached_variable
            && DEPENDENCIES
                .iter()
                .all(|d| self.dependencies.contains_key(*d))
    }

    /// Whether it's older than MAX_AGE (or from the future: a clock that
    /// was set back).
    pub fn is_due(&self) -> bool {
        let now = now();
        now < self.fetched || now - self.fetched >= MAX_AGE.as_secs()
    }

    pub fn engine(&self) -> &Dependency {
        &self.dependencies[ENGINE]
    }

    pub fn examples(&self) -> &Dependency {
        &self.dependencies[EXAMPLES]
    }
}

/// The cache file in a dependencies folder (see `engine::Layout`).
pub fn cache_file(dir: &Path) -> PathBuf {
    dir.join("remote-config").join("config.json")
}

/// The cached config, if there is a readable one.
pub fn load_cache(dir: &Path) -> Option<Cached> {
    let text = std::fs::read_to_string(cache_file(dir)).ok()?;
    serde_json::from_str(&text).ok()
}

/// Writes the cache, replacing the old one in one step.
pub fn save_cache(dir: &Path, cached: &Cached) -> Result<()> {
    let file = cache_file(dir);
    let folder = file.parent().expect("the cache file has a folder");
    std::fs::create_dir_all(folder)?;
    let tmp = folder.join(format!(".config-{}.json", std::process::id()));
    std::fs::write(&tmp, serde_json::to_string_pretty(cached)? + "\n")?;
    std::fs::rename(&tmp, &file)?;
    Ok(())
}

/// The UTC time of a `fetched` value, as `yyyy-mm-dd hh:mm UTC`.
pub fn utc_time(secs: u64) -> String {
    let (hh, mm) = (secs / 3600 % 24, secs / 60 % 60);
    // Days since 1970-01-01 to a calendar date (Howard Hinnant's algorithm).
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02} {hh:02}:{mm:02} UTC")
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = r#"// What Logix IDE on config 001 uses.
{
  "$schema": "https://www.openlogix.org/schemas/logix-remote-config-v1.json",

  /* The engine. A "//" in a string isn't a comment: "a//b". */
  "logix-engine": {
    "repo": "openlogix/logix-engine",
    "release": "2026-11-12",
    "file": "logix-engine-${PLATFORM}-${YYYYMMDD}.tgz"
  },

  // The examples.
  "logix-ide-examples": {
    "repo": "openlogix/logix-ide",
    "release": "2026-11-12",
    "file": "logix-ide-examples-${YYYYMMDD}.tgz"
  }
}
"#;

    #[test]
    fn a_config_reads_with_its_comments() {
        let e = parse(CONFIG).unwrap();
        assert_eq!(
            e[ENGINE],
            Entry {
                repo: "openlogix/logix-engine".into(),
                release: "2026-11-12".into(),
                file: "logix-engine-${PLATFORM}-${YYYYMMDD}.tgz".into(),
            }
        );
        assert_eq!(
            file_name(&e[ENGINE], "darwin-arm64"),
            "logix-engine-darwin-arm64-20261112.tgz"
        );
        assert_eq!(
            strip_comments(r#"{"a": "x//y" /* c */}"#).unwrap(),
            r#"{"a": "x//y" }"#
        );
    }

    #[test]
    fn bad_configs_are_refused() {
        for (from, to, why) in [
            ("2026-11-12", "latest", "release"),
            (
                "\"logix-ide-examples\": {",
                "\"logix-ide-other\": {",
                "exactly",
            ),
            ("openlogix/logix-engine", "logix-engine", "owner/name"),
            ("logix-engine-${PLATFORM}", "../evil", "bad file"),
            ("\"logix-engine\": {", "\"logix-other\": {", "exactly"),
            (
                "logix-remote-config-v1",
                "logix-remote-config-v9",
                "$schema",
            ),
            (
                "\"repo\": \"openlogix/logix-engine\",",
                "\"repo\": \"openlogix/logix-engine\", \"x\": 1,",
                "unknown key",
            ),
        ] {
            assert!(CONFIG.contains(from), "{from}");
            let err = format!("{:#}", parse(&CONFIG.replacen(from, to, 1)).unwrap_err());
            assert!(err.contains(why), "{why}: {err}");
        }
        assert!(parse("{ /* open").is_err());
    }

    #[test]
    fn the_repo_config_is_valid() {
        let folder = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(repo_path());
        parse_channels(&std::fs::read_to_string(folder.join(CHANNELS_FILE)).unwrap()).unwrap();
        // Every config file there is valid: prod.jsonc, dev.jsonc, others.
        let mut files = 0;
        for f in std::fs::read_dir(&folder).unwrap().flatten() {
            let name = f.file_name().to_string_lossy().into_owned();
            if name != CHANNELS_FILE && name.ends_with(".jsonc") {
                parse(&std::fs::read_to_string(f.path()).unwrap()).unwrap();
                files += 1;
            }
        }
        assert!(files > 0, "no config file in {}", folder.display());
        assert_eq!(
            default_url(),
            "https://raw.githubusercontent.com/openlogix/logix-ide/main/remote-config/logix-ide/001/"
        );
    }

    #[test]
    fn the_channel_follows_the_rules() {
        let channels = parse_channels(
            r#"{"$schema": "x", /* promoted */ "prod": ["2026-11-20"], "beta": []}"#,
        )
        .unwrap();
        let pick = |var: Option<&str>, release: bool, tag: &str| {
            choose_channel(&channels, var, release, tag).map(|c| (c.name, c.how, c.file))
        };
        let c = |n: &str, h: &str| (n.to_string(), h.to_string(), format!("{n}.jsonc"));
        assert_eq!(pick(None, true, "2026-11-20").unwrap(), c("prod", "listed"));
        assert_eq!(pick(None, true, "2026-11-21").unwrap(), c("dev", "default"));
        // A local build is dev, even with a prod release's tag.
        assert_eq!(
            pick(None, false, "2026-11-20").unwrap(),
            c("dev", "local build")
        );
        assert_eq!(
            pick(Some("beta"), false, "x").unwrap(),
            c("beta", CHANNEL_VARIABLE)
        );
        assert!(
            format!("{:#}", pick(Some("nightly"), true, "x").unwrap_err())
                .contains("isn't a channel")
        );
        assert!(channel_from_variable(Some("Prod".into())).is_err());
        assert_eq!(channel_from_variable(Some("".into())).unwrap(), None);
        assert_eq!(
            channel_from_variable(Some("dev".into()))
                .unwrap()
                .as_deref(),
            Some("dev")
        );
        assert!(parse_channels(r#"{"prod": "2026-11-20"}"#).is_err());
    }

    #[test]
    fn the_override_is_a_url_or_a_file() {
        assert_eq!(
            ConfigSource::from_variable(None).unwrap(),
            ConfigSource::regular()
        );
        assert!(
            !ConfigSource::from_variable(Some("".into()))
                .unwrap()
                .is_override
        );
        let url =
            "https://raw.githubusercontent.com/you/logix-ide/test/remote-config/logix-ide/001/";
        let s = ConfigSource::from_variable(Some(url.into())).unwrap();
        assert!(s.is_override && s.location == url);
        let tmp = tempfile::tempdir().unwrap();
        let folder = tmp.path().join("001");
        assert!(ConfigSource::from_variable(Some(folder.clone().into())).is_err());
        std::fs::create_dir(&folder).unwrap();
        assert!(
            ConfigSource::from_variable(Some(folder.into()))
                .unwrap()
                .is_override
        );
        assert!(ConfigSource::from_variable(Some("http://plain/x".into())).is_err());
    }

    #[test]
    fn checksums_are_found_by_file_name() {
        let sha = "ab".repeat(32);
        let sums =
            format!("{sha}  logix-engine-darwin-arm64-20261112.tgz\n{sha} *BUILD-INFO.json\n");
        assert_eq!(
            sha256_in(&sums, "logix-engine-darwin-arm64-20261112.tgz"),
            Some(sha.clone())
        );
        assert_eq!(sha256_in(&sums, "BUILD-INFO.json"), Some(sha));
        assert_eq!(sha256_in(&sums, "other.tgz"), None);
        assert_eq!(sha256_in("zz  a.tgz", "a.tgz"), None);
    }

    #[tokio::test]
    async fn a_local_config_with_a_bad_release_fails_to_resolve() {
        // Resolving reads SHA256SUMS from GitHub; a config that doesn't
        // parse fails before any network access.
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(CHANNELS_FILE), r#"{"$schema": "x"}"#).unwrap();
        // prod.jsonc: every channel reads it, its own or as a fallback, so
        // this holds on whichever channel the tests run (CI sets one per
        // config file in the repo).
        std::fs::write(
            tmp.path().join("prod.jsonc"),
            CONFIG.replace("2026-11-12", "soon"),
        )
        .unwrap();
        let source = ConfigSource::from_variable(Some(tmp.path().into())).unwrap();
        let err = format!("{:#}", fetch(&source, "darwin-arm64").await.unwrap_err());
        assert!(
            err.contains("isn't valid") && err.contains("release"),
            "{err}"
        );
    }

    #[test]
    fn the_cache_round_trips_and_expires() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(load_cache(tmp.path()).is_none());
        // As read now: with LOGIX_IDE_CHANNEL when CI's per-file pass sets it.
        let variable = channel_variable().unwrap();
        let mut cached = Cached {
            channel: Channel {
                name: variable.clone().unwrap_or("dev".into()),
                how: if variable.is_some() {
                    CHANNEL_VARIABLE
                } else {
                    "local build"
                }
                .into(),
                file: "dev.jsonc".into(),
            },
            source: default_url(),
            config_version: CONFIG_VERSION.into(),
            fetched: now(),
            dependencies: [ENGINE, EXAMPLES]
                .map(|name| {
                    (
                        name.to_string(),
                        Dependency {
                            repo: "openlogix/logix-engine".into(),
                            release: "2026-11-12".into(),
                            file: format!("{name}-20261112.tgz"),
                            url: format!("https://example/{name}.tgz"),
                            sha256: "ab".repeat(32),
                        },
                    )
                })
                .into(),
        };
        save_cache(tmp.path(), &cached).unwrap();
        assert_eq!(load_cache(tmp.path()).as_ref(), Some(&cached));
        assert!(cached.matches(&ConfigSource::regular()) && !cached.is_due());
        let other = ConfigSource {
            location: "/tmp/other.jsonc".into(),
            is_override: true,
        };
        assert!(!cached.matches(&other));
        let mut no_examples = cached.clone();
        no_examples.dependencies.remove(EXAMPLES);
        assert!(!no_examples.matches(&ConfigSource::regular()));
        cached.fetched -= MAX_AGE.as_secs();
        assert!(cached.is_due());
        cached.fetched = now() + 3600;
        assert!(cached.is_due(), "a clock set back");
    }

    #[test]
    fn times_are_utc() {
        assert_eq!(utc_time(0), "1970-01-01 00:00 UTC");
        assert_eq!(utc_time(1_794_441_600 + 3_660), "2026-11-12 01:01 UTC");
    }
}
