//! The single place that knows which OS/CPU we run on. Platform-specific
//! behavior (Apio bundle selection, data directories, file fixups, etc.)
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

    /// The platform id Apio uses in its release file names.
    pub fn apio_platform_id(self) -> &'static str {
        match self {
            Platform::MacosArm64 => "darwin-arm64",
            Platform::LinuxX86_64 => "linux-x86-64",
            Platform::WindowsX86_64 => "windows-amd64",
        }
    }

    /// File extension of Apio's standalone bundle archive.
    pub fn apio_bundle_ext(self) -> &'static str {
        match self {
            Platform::WindowsX86_64 => "zip",
            _ => "tgz",
        }
    }

    /// Name of the Apio executable inside the bundle.
    pub fn apio_exe_name(self) -> &'static str {
        match self {
            Platform::WindowsX86_64 => "apio.exe",
            _ => "apio",
        }
    }

    /// Whether the menu bar has an application menu (with About and Quit),
    /// as on macOS. Elsewhere Quit goes in the File menu.
    pub fn has_app_menu(self) -> bool {
        matches!(self, Platform::MacosArm64)
    }

    /// Per-user directory where the IDE keeps the Apio bundle and Apio's
    /// packages. Apio rejects an `APIO_HOME` containing spaces or non-ASCII
    /// characters, so this can't be e.g. `~/Library/Application Support`.
    pub fn data_dir(self) -> Result<PathBuf> {
        self.ensure_implemented()?;
        let home = dirs::home_dir().context("can't find the user's home directory")?;
        Ok(home.join(".logix"))
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

    /// Makes a freshly unpacked Apio bundle runnable.
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

/// Prepares a child process so `stop_process_tree` can stop it together
/// with everything it starts (Apio runs scons, which runs yosys, ...).
pub fn isolate_process_tree(cmd: &mut tokio::process::Command) {
    #[cfg(unix)]
    {
        // Its own process group, so the whole group can be stopped at once.
        cmd.process_group(0);
    }
    #[cfg(not(unix))]
    {
        // TODO(revisit): use a job object on Windows to stop child processes.
        let _ = cmd;
    }
}

/// Stops a child started with `isolate_process_tree` and the processes it
/// started.
pub fn stop_process_tree(child: &mut tokio::process::Child) {
    #[cfg(unix)]
    if let Some(pid) = child.id() {
        // A negative pid addresses the whole process group.
        let _ = std::process::Command::new("/bin/kill")
            .args(["-TERM", &format!("-{pid}")])
            .status();
    }
    let _ = child.start_kill();
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

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn apio_release_names() {
        assert_eq!(Platform::MacosArm64.apio_platform_id(), "darwin-arm64");
        assert_eq!(Platform::LinuxX86_64.apio_platform_id(), "linux-x86-64");
        assert_eq!(Platform::WindowsX86_64.apio_platform_id(), "windows-amd64");
        assert_eq!(Platform::WindowsX86_64.apio_bundle_ext(), "zip");
        assert_eq!(Platform::MacosArm64.apio_bundle_ext(), "tgz");
    }
}
