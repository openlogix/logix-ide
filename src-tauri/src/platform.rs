//! The single place that knows which OS/CPU we run on. Platform-specific
//! behavior (engine package names, data directories, file fixups, etc.)
//! lives here rather than in `cfg!(target_os)` checks elsewhere.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    MacosArm64,
    LinuxX86_64,
    WindowsX86_64,
}

impl Platform {
    /// Stable id used in the UI and logs.
    pub fn id(self) -> &'static str {
        match self {
            Platform::MacosArm64 => "macos-arm64",
            Platform::LinuxX86_64 => "linux-x86_64",
            Platform::WindowsX86_64 => "windows-x86_64",
        }
    }

    /// Whether the IDE is implemented for this platform yet.
    /// Only macOS ARM for now; Linux and Windows come later.
    pub fn is_implemented(self) -> bool {
        matches!(self, Platform::MacosArm64)
    }

    /// Fails with a clear message if this platform isn't implemented yet.
    pub fn ensure_implemented(self) -> Result<()> {
        if !self.is_implemented() {
            bail!("This computer ({}) is not supported yet.", self.id());
        }
        Ok(())
    }

    /// The platform id Logix Engine uses in its package names and reports
    /// in `get_engine_info`.
    pub fn engine_platform_id(self) -> &'static str {
        match self {
            Platform::MacosArm64 => "darwin-arm64",
            Platform::LinuxX86_64 => "linux-x86-64",
            Platform::WindowsX86_64 => "windows-x86-64",
        }
    }

    /// Name of the engine executable inside its package.
    pub fn engine_exe_name(self) -> &'static str {
        match self {
            Platform::WindowsX86_64 => "logix-engine.exe",
            _ => "logix-engine",
        }
    }

    /// Whether the menu bar has an application menu (with About and Quit),
    /// as on macOS. Elsewhere Quit goes in the File menu.
    pub fn has_app_menu(self) -> bool {
        matches!(self, Platform::MacosArm64)
    }

    /// Per-user directory where the IDE keeps Logix Engine, its packages
    /// (the FPGA tools), settings and scratch folders: `~/.logix`, or the
    /// folder `LOGIX_IDE_HOME` names (see `data_dir_from`).
    pub fn data_dir(self) -> Result<PathBuf> {
        self.ensure_implemented()?;
        data_dir_from(std::env::var_os(HOME_VARIABLE), dirs::home_dir)
    }

    /// Opens `dir` in the system's file manager (Finder on macOS).
    pub fn reveal_folder(self, dir: &Path) -> Result<()> {
        match self {
            Platform::MacosArm64 => {
                let status = std::process::Command::new("/usr/bin/open")
                    .arg(dir)
                    .status()
                    .context("can't run 'open'")?;
                if !status.success() {
                    bail!("'open' failed for {}", dir.display());
                }
                Ok(())
            }
            other => other.ensure_implemented(),
        }
    }

    /// `path` as typed in this platform's shells, so it can be pasted after
    /// `cd`: as is when that's safe, otherwise in single quotes (zsh, bash
    /// and fish on macOS and Linux), with each `'` written as `'\''`.
    pub fn shell_path(self, path: &str) -> String {
        let plain = |c: char| c.is_ascii_alphanumeric() || "/._-+,:@%".contains(c);
        if !path.is_empty() && path.chars().all(plain) {
            path.to_string()
        } else {
            format!("'{}'", path.replace('\'', r"'\''"))
        }
    }

    /// Puts `text` on the clipboard (`pbcopy` on macOS). Done here rather
    /// than in the webview, whose clipboard may refuse a copy that a menu
    /// item, not a click in the page, started.
    pub fn copy_text(self, text: &str) -> Result<()> {
        match self {
            Platform::MacosArm64 => {
                use std::io::Write;
                let mut child = std::process::Command::new("/usr/bin/pbcopy")
                    .stdin(std::process::Stdio::piped())
                    .spawn()
                    .context("can't run 'pbcopy'")?;
                child
                    .stdin
                    .take()
                    .context("no input for 'pbcopy'")?
                    .write_all(text.as_bytes())
                    .context("can't write to 'pbcopy'")?;
                if !child.wait().context("'pbcopy' didn't finish")?.success() {
                    bail!("'pbcopy' failed");
                }
                Ok(())
            }
            other => other.ensure_implemented(),
        }
    }

    /// Opens a web page in the default browser. Only `https://` links.
    pub fn open_url(self, url: &str) -> Result<()> {
        if !url.starts_with("https://") {
            bail!("not a web link: {url}");
        }
        match self {
            Platform::MacosArm64 => {
                std::process::Command::new("/usr/bin/open")
                    .arg(url)
                    .status()
                    .context("can't run 'open'")?;
                Ok(())
            }
            other => other.ensure_implemented(),
        }
    }

    /// Makes a freshly unpacked engine package runnable.
    pub fn prepare_unpacked_bundle(self, bundle_dir: &Path, exe: &Path) -> Result<()> {
        match self {
            Platform::MacosArm64 => {
                make_executable(exe)?;
                // Clear any quarantine attributes so Gatekeeper doesn't block
                // the binaries. Usually there are none; failures are harmless.
                let _ = std::process::Command::new("/usr/bin/xattr")
                    .arg("-cr")
                    .arg(bundle_dir)
                    .status();
                Ok(())
            }
            other => other.ensure_implemented(),
        }
    }
}

/// Moves a file or folder to the system's Trash (recoverable), e.g. a
/// project replaced by Save As.
pub fn move_to_trash(path: &Path) -> Result<()> {
    trash::delete(path).with_context(|| format!("can't move {} to the Trash", path.display()))
}

/// Process ids of the running GUI apps whose name or executable contains
/// `name` (any case). Must be called on the main thread (see
/// `AppHandle::run_on_main_thread`); elsewhere, and on other platforms for
/// now, returns nothing.
pub fn running_app_pids(name: &str) -> Vec<i32> {
    #[cfg(target_os = "macos")]
    {
        macos::running_app_pids(name)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = name;
        Vec::new()
    }
}

/// Brings the app with process id `pid` to the front, e.g. GTKWave after a
/// simulation (an app started in the background by another program stays
/// behind the IDE). Must be called on the main thread while the IDE is the
/// active app. Returns whether the request was made.
pub fn bring_app_to_front(pid: i32) -> bool {
    #[cfg(target_os = "macos")]
    {
        macos::bring_app_to_front(pid)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = pid;
        false
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{
        NSApplication, NSApplicationActivationOptions, NSRunningApplication, NSWorkspace,
    };

    pub fn running_app_pids(name: &str) -> Vec<i32> {
        if MainThreadMarker::new().is_none() {
            return Vec::new();
        }
        let name = name.to_lowercase();
        let apps = NSWorkspace::sharedWorkspace().runningApplications();
        apps.iter()
            .filter(|app| {
                let localized = app
                    .localizedName()
                    .map(|n| n.to_string().to_lowercase())
                    .unwrap_or_default();
                let exe = app
                    .executableURL()
                    .and_then(|u| u.path())
                    .map(|p| p.to_string().to_lowercase())
                    .unwrap_or_default();
                localized.contains(&name) || exe.contains(&name)
            })
            .map(|app| app.processIdentifier())
            .collect()
    }

    pub fn bring_app_to_front(pid: i32) -> bool {
        let Some(mtm) = MainThreadMarker::new() else {
            return false;
        };
        let apps = NSWorkspace::sharedWorkspace().runningApplications();
        let Some(target) = apps.iter().find(|a| a.processIdentifier() == pid) else {
            return false;
        };
        // Cooperative activation (macOS 14+): the active app (Logix) yields
        // to the target, which is then activated.
        NSApplication::sharedApplication(mtm).yieldActivationToApplication(&target);
        let target: &NSRunningApplication = &target;
        target.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows)
    }
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .with_context(|| format!("can't make {} executable", path.display()))
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}

/// The platform this build is running on, or `None` if it's not one of the
/// target platforms at all.
pub fn current() -> Option<Platform> {
    from_os_arch(std::env::consts::OS, std::env::consts::ARCH)
}

/// Like `current()`, but fails with a clear message on unknown or not yet
/// implemented platforms.
pub fn current_implemented() -> Result<Platform> {
    let p = current().with_context(|| {
        format!(
            "This computer ({}-{}) is not supported.",
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    })?;
    p.ensure_implemented()?;
    Ok(p)
}

fn from_os_arch(os: &str, arch: &str) -> Option<Platform> {
    match (os, arch) {
        ("macos", "aarch64") => Some(Platform::MacosArm64),
        ("linux", "x86_64") => Some(Platform::LinuxX86_64),
        ("windows", "x86_64") => Some(Platform::WindowsX86_64),
        _ => None,
    }
}

/// The environment variable that sets the IDE's data folder instead of
/// `~/.logix`: for testing (generated settings and projects, a clean first
/// start) without touching the real one.
pub const HOME_VARIABLE: &str = "LOGIX_IDE_HOME";

/// The IDE's data folder: `value` (`LOGIX_IDE_HOME`) when set, which must
/// be an absolute path (an error otherwise, never a silent fallback), else
/// `.logix` in the user's home.
pub fn data_dir_from(
    value: Option<std::ffi::OsString>,
    home: impl FnOnce() -> Option<PathBuf>,
) -> Result<PathBuf> {
    match value.filter(|v| !v.is_empty()).map(PathBuf::from) {
        Some(dir) if dir.is_absolute() => Ok(dir),
        Some(dir) => anyhow::bail!(
            "{HOME_VARIABLE} must be an absolute path, not \"{}\".",
            dir.display()
        ),
        None => Ok(home()
            .context("can't find the user's home directory")?
            .join(".logix")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_paths_are_quoted_only_when_needed() {
        let p = Platform::MacosArm64;
        assert_eq!(
            p.shell_path("/Users/you/Documents/Logix/blinky"),
            "/Users/you/Documents/Logix/blinky"
        );
        assert_eq!(
            p.shell_path("/Users/you/Documents/Logix/blinky 3"),
            "'/Users/you/Documents/Logix/blinky 3'"
        );
        assert_eq!(p.shell_path("/a/it's"), r"'/a/it'\''s'");
        assert_eq!(p.shell_path("/a/$HOME*(x)"), "'/a/$HOME*(x)'");
        assert_eq!(p.shell_path("/a/~b"), "'/a/~b'");
    }

    #[test]
    fn logix_ide_home_replaces_the_data_folder() {
        let home = || Some(PathBuf::from("/Users/you"));
        assert_eq!(
            data_dir_from(None, home).unwrap(),
            Path::new("/Users/you/.logix")
        );
        assert_eq!(
            data_dir_from(Some("".into()), home).unwrap(),
            Path::new("/Users/you/.logix")
        );
        assert_eq!(
            data_dir_from(Some("/tmp/test-home".into()), home).unwrap(),
            Path::new("/tmp/test-home")
        );
        let e = data_dir_from(Some("relative/dir".into()), home).unwrap_err();
        assert!(e.to_string().contains("LOGIX_IDE_HOME"), "{e}");
    }

    #[test]
    fn maps_target_platforms() {
        assert_eq!(from_os_arch("macos", "aarch64"), Some(Platform::MacosArm64));
        assert_eq!(from_os_arch("linux", "x86_64"), Some(Platform::LinuxX86_64));
        assert_eq!(
            from_os_arch("windows", "x86_64"),
            Some(Platform::WindowsX86_64)
        );
    }

    #[test]
    fn names_engine_packages_like_the_engine() {
        // The engine's `platform` in get_engine_info and its package names.
        assert_eq!(Platform::MacosArm64.engine_platform_id(), "darwin-arm64");
        assert_eq!(Platform::LinuxX86_64.engine_platform_id(), "linux-x86-64");
        assert_eq!(
            Platform::WindowsX86_64.engine_platform_id(),
            "windows-x86-64"
        );
        assert_eq!(Platform::MacosArm64.engine_exe_name(), "logix-engine");
        assert_eq!(
            Platform::WindowsX86_64.engine_exe_name(),
            "logix-engine.exe"
        );
    }

    #[test]
    fn rejects_other_platforms() {
        assert_eq!(from_os_arch("macos", "x86_64"), None);
        assert_eq!(from_os_arch("linux", "aarch64"), None);
    }

    #[test]
    fn only_macos_is_implemented_for_now() {
        assert!(Platform::MacosArm64.is_implemented());
        assert!(!Platform::LinuxX86_64.is_implemented());
        assert!(!Platform::WindowsX86_64.is_implemented());
        assert!(Platform::LinuxX86_64.data_dir().is_err());
    }
}
