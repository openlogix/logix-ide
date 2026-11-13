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

    /// Whether the IDE is implemented for this platform yet: macOS ARM,
    /// and Windows x64 (Milestone 3, being ported); Linux comes later.
    pub fn is_implemented(self) -> bool {
        matches!(self, Platform::MacosArm64 | Platform::WindowsX86_64)
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

    /// Opens `dir` in the system's file manager (Finder on macOS, File
    /// Explorer on Windows).
    pub fn reveal_folder(self, dir: &Path) -> Result<()> {
        match self {
            Platform::WindowsX86_64 => {
                // explorer.exe exits with 1 even when it opened the folder.
                let mut cmd = std::process::Command::new("explorer.exe");
                cmd.arg(dir);
                hide_console(&mut cmd);
                cmd.spawn().context("can't run 'explorer'")?;
                Ok(())
            }
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
    /// and fish on macOS and Linux), with each `'` written as `'\''`; on
    /// Windows in double quotes (cmd and PowerShell; a Windows path can't
    /// hold one).
    pub fn shell_path(self, path: &str) -> String {
        if self == Platform::WindowsX86_64 {
            let plain = |c: char| c.is_ascii_alphanumeric() || "\\/._-+,:@".contains(c);
            return if !path.is_empty() && path.chars().all(plain) {
                path.to_string()
            } else {
                format!("\"{path}\"")
            };
        }
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
            // Windows' clipboard API, as Unicode text. Not clip.exe: it kept
            // the byte order mark that its UTF-16 input needs, an invisible
            // character that made a pasted path invalid after `cd `.
            Platform::WindowsX86_64 => {
                #[cfg(windows)]
                return windows::copy_text(text);
                #[cfg(not(windows))]
                bail!("not on Windows");
            }
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
            // The default browser, through Windows' URL handler: no cmd, so
            // characters such as & in the link stay as they are.
            Platform::WindowsX86_64 => {
                let mut cmd = std::process::Command::new("rundll32.exe");
                cmd.args(["url.dll,FileProtocolHandler", url]);
                hide_console(&mut cmd);
                cmd.spawn().context("can't open the link")?;
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
            // Nothing to do: Windows has no execute bit, and files Logix IDE
            // writes itself carry no downloaded-from-the-internet mark.
            Platform::WindowsX86_64 => {
                let _ = (bundle_dir, exe);
                Ok(())
            }
            other => other.ensure_implemented(),
        }
    }
}

/// Windows: runs a console program (the engine, the language server)
/// without opening a console window, which a windowed app's console child
/// otherwise gets. Elsewhere: nothing.
pub fn hide_console(cmd: &mut std::process::Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = cmd;
}

/// hide_console for tokio's Command.
pub fn hide_console_async(cmd: &mut tokio::process::Command) {
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    #[cfg(not(windows))]
    let _ = cmd;
}

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Moves a file or folder to the system's Trash (recoverable), e.g. a
/// project replaced by Save As.
pub fn move_to_trash(path: &Path) -> Result<()> {
    trash::delete(path)
        .with_context(|| format!("can't move {} to the {}", path.display(), trash_name()))
}

/// What the system calls its trash: the Trash on macOS, the Recycle Bin on
/// Windows.
pub fn trash_name() -> &'static str {
    if cfg!(windows) {
        "Recycle Bin"
    } else {
        "Trash"
    }
}

/// Process ids of the running GUI apps whose name or executable contains
/// `name` (any case); on Windows, of the processes whose executable's file
/// name does. Must be called on the main thread (see
/// `AppHandle::run_on_main_thread`); elsewhere, and on other platforms for
/// now, returns nothing.
pub fn running_app_pids(name: &str) -> Vec<i32> {
    #[cfg(target_os = "macos")]
    {
        macos::running_app_pids(name)
    }
    #[cfg(windows)]
    {
        windows::running_app_pids(name)
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        let _ = name;
        Vec::new()
    }
}

/// Brings the app with process id `pid` to the front, e.g. GTKWave after a
/// simulation (an app started in the background by another program stays
/// behind the IDE). Must be called on the main thread while the IDE is the
/// active app. Returns whether the request was made: on Windows, false
/// until the process has a visible window.
pub fn bring_app_to_front(pid: i32) -> bool {
    #[cfg(target_os = "macos")]
    {
        macos::bring_app_to_front(pid)
    }
    #[cfg(windows)]
    {
        windows::bring_app_to_front(pid)
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        let _ = pid;
        false
    }
}

#[cfg(windows)]
mod windows {
    use anyhow::{bail, Context, Result};
    use windows_sys::Win32::Foundation::{
        CloseHandle, GlobalFree, HWND, INVALID_HANDLE_VALUE, LPARAM,
    };
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
    };
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::Memory::{
        GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE,
    };
    use windows_sys::Win32::System::Ole::CF_UNICODETEXT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindow, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
        SetForegroundWindow, ShowWindow, GW_OWNER, SW_RESTORE,
    };

    pub fn running_app_pids(name: &str) -> Vec<i32> {
        let name = name.to_lowercase();
        let mut pids = Vec::new();
        // SAFETY: a process snapshot, walked with a correctly sized entry
        // and closed at the end.
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snap == INVALID_HANDLE_VALUE {
                return pids;
            }
            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            let mut more = Process32FirstW(snap, &mut entry) != 0;
            while more {
                let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(0);
                let exe = String::from_utf16_lossy(&entry.szExeFile[..len]).to_lowercase();
                if exe.contains(&name) {
                    pids.push(entry.th32ProcessID as i32);
                }
                more = Process32NextW(snap, &mut entry) != 0;
            }
            CloseHandle(snap);
        }
        pids
    }

    /// The process's visible top-level window, restored if minimized, is
    /// made the foreground window. Logix IDE, the foreground app, may do so.
    pub fn bring_app_to_front(pid: i32) -> bool {
        struct Search {
            pid: u32,
            found: HWND,
        }
        unsafe extern "system" fn each(hwnd: HWND, data: LPARAM) -> i32 {
            // SAFETY: `data` is the `Search` below, alive during EnumWindows.
            let search = unsafe { &mut *(data as *mut Search) };
            let mut owner_pid = 0u32;
            // SAFETY: plain queries on a window EnumWindows gave.
            unsafe {
                GetWindowThreadProcessId(hwnd, &mut owner_pid);
                if owner_pid == search.pid
                    && IsWindowVisible(hwnd) != 0
                    && GetWindow(hwnd, GW_OWNER).is_null()
                {
                    search.found = hwnd;
                    return 0;
                }
            }
            1
        }
        let mut search = Search {
            pid: pid as u32,
            found: std::ptr::null_mut(),
        };
        // SAFETY: the callback only uses `search`, which outlives the call.
        unsafe {
            EnumWindows(Some(each), &mut search as *mut Search as LPARAM);
            if search.found.is_null() {
                return false;
            }
            if IsIconic(search.found) != 0 {
                ShowWindow(search.found, SW_RESTORE);
            }
            SetForegroundWindow(search.found) != 0
        }
    }

    /// Puts `text` on the clipboard as Unicode text (UTF-16, ending in a
    /// zero, with no byte order mark).
    pub fn copy_text(text: &str) -> Result<()> {
        let utf16: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        // Another program may have the clipboard open for a moment.
        let mut opened = false;
        for _ in 0..20 {
            // SAFETY: no owner window; closed below.
            if unsafe { OpenClipboard(std::ptr::null_mut()) } != 0 {
                opened = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        if !opened {
            bail!("the clipboard is in use by another program");
        }
        // SAFETY: the clipboard is open; the memory is filled while locked
        // and, once SetClipboardData succeeds, owned by the system.
        let result = unsafe {
            (|| {
                EmptyClipboard();
                let bytes = utf16.len() * 2;
                let mem = GlobalAlloc(GMEM_MOVEABLE, bytes);
                if mem.is_null() {
                    bail!("no memory for the clipboard");
                }
                let dst = GlobalLock(mem) as *mut u16;
                if dst.is_null() {
                    GlobalFree(mem);
                    bail!("can't fill the clipboard");
                }
                std::ptr::copy_nonoverlapping(utf16.as_ptr(), dst, utf16.len());
                GlobalUnlock(mem);
                if SetClipboardData(CF_UNICODETEXT as u32, mem).is_null() {
                    GlobalFree(mem);
                    bail!("can't set the clipboard");
                }
                Ok(())
            })()
        };
        // SAFETY: opened above.
        unsafe { CloseClipboard() };
        result.context("can't copy to the clipboard")
    }

    #[cfg(test)]
    mod tests {
        use windows_sys::Win32::System::DataExchange::{
            CloseClipboard, GetClipboardData, OpenClipboard,
        };
        use windows_sys::Win32::System::Memory::{GlobalLock, GlobalUnlock};
        use windows_sys::Win32::System::Ole::CF_UNICODETEXT;

        fn clipboard_text() -> String {
            // SAFETY: reads the clipboard's Unicode text up to its zero.
            unsafe {
                assert!(OpenClipboard(std::ptr::null_mut()) != 0);
                let mem = GetClipboardData(CF_UNICODETEXT as u32);
                assert!(!mem.is_null());
                let src = GlobalLock(mem) as *const u16;
                let mut len = 0;
                while *src.add(len) != 0 {
                    len += 1;
                }
                let text = String::from_utf16(std::slice::from_raw_parts(src, len)).unwrap();
                GlobalUnlock(mem);
                CloseClipboard();
                text
            }
        }

        // The user's report (2026-11-13): a copied path pasted after `cd `
        // wasn't valid; clip.exe had kept an invisible byte order mark.
        #[test]
        fn copied_text_is_exactly_the_text() {
            for text in [r"C:\Users\you\Documents\OpenLogix\IDE\blinky", "\"Ä ü\" 💡"] {
                super::copy_text(text).unwrap();
                assert_eq!(clipboard_text(), text);
            }
        }

        #[test]
        fn finds_its_own_process() {
            let me = std::env::current_exe().unwrap();
            let name = me.file_stem().unwrap().to_string_lossy().to_string();
            let pids = super::running_app_pids(&name);
            assert!(pids.contains(&(std::process::id() as i32)), "{pids:?}");
        }
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
        // Cooperative activation (macOS 14+): the active app (Logix IDE) yields
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
/// `~/.logix/ide`: for testing (generated settings and projects, a clean first
/// start) without touching the real one.
pub const HOME_VARIABLE: &str = "LOGIX_IDE_HOME";

/// The IDE's data folder: `value` (`LOGIX_IDE_HOME`) when set, which must
/// be an absolute path (an error otherwise, never a silent fallback), else
/// `~/.logix/ide`: every Open Logix program has its folder in `~/.logix`
/// (the IDE's `CLAUDE.md`, Files Logix IDE writes).
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
            .join(".logix")
            .join("ide")),
    }
}

/// What `~/.logix/README.txt` says: what the folder is, and that each
/// program's folder may be deleted.
pub const LOGIX_README: &str = "\
This folder holds the data of Open Logix programs (www.openlogix.org),
one folder per program:

  ide/           Logix IDE: settings, logs, scratch files, and its copy of
                 Logix Engine with its FPGA tools
  claude-code/   Logix Engine for Claude Code (the Logix Engine tutorial)
  engine-dev/    Logix Engine developers' scripts

Deleting a program's folder is safe: the program downloads what it needs
again, and its settings go back to their defaults. Your projects aren't here (Logix IDE keeps them in
~/Documents/OpenLogix/IDE by default).
";

/// Writes `README.txt` next to the IDE's data folder, in `~/.logix`, when
/// it's missing there. Not with `LOGIX_IDE_HOME`, whose folder is the
/// tester's.
pub fn write_logix_readme(data_dir: &Path, home_variable_set: bool) -> Result<()> {
    if home_variable_set {
        return Ok(());
    }
    let Some(root) = data_dir.parent() else {
        return Ok(());
    };
    let readme = root.join("README.txt");
    if !readme.exists() {
        std::fs::create_dir_all(root)?;
        std::fs::write(&readme, LOGIX_README)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_paths_are_quoted_only_when_needed() {
        let p = Platform::MacosArm64;
        assert_eq!(
            p.shell_path("/Users/you/Documents/OpenLogix/IDE/blinky"),
            "/Users/you/Documents/OpenLogix/IDE/blinky"
        );
        assert_eq!(
            p.shell_path("/Users/you/Documents/OpenLogix/IDE/blinky 3"),
            "'/Users/you/Documents/OpenLogix/IDE/blinky 3'"
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
            Path::new("/Users/you/.logix/ide")
        );
        assert_eq!(
            data_dir_from(Some("".into()), home).unwrap(),
            Path::new("/Users/you/.logix/ide")
        );
        // An absolute path on this platform.
        let abs = if cfg!(windows) {
            r"C:\tmp\test-home"
        } else {
            "/tmp/test-home"
        };
        assert_eq!(
            data_dir_from(Some(abs.into()), home).unwrap(),
            Path::new(abs)
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
    fn windows_shell_paths_use_double_quotes() {
        let w = Platform::WindowsX86_64;
        assert_eq!(w.shell_path(r"C:\Users\me\fpga"), r"C:\Users\me\fpga");
        assert_eq!(
            w.shell_path(r"C:\Users\me\my fpga"),
            r#""C:\Users\me\my fpga""#
        );
    }

    #[test]
    fn macos_and_windows_are_implemented_linux_not_yet() {
        assert!(Platform::MacosArm64.is_implemented());
        assert!(Platform::WindowsX86_64.is_implemented());
        assert!(!Platform::LinuxX86_64.is_implemented());
        assert!(Platform::LinuxX86_64.data_dir().is_err());
    }

    #[test]
    fn the_logix_readme_is_written_once_next_to_the_data_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path().join(".logix").join("ide");
        write_logix_readme(&data, true).unwrap();
        assert!(
            !tmp.path().join(".logix/README.txt").exists(),
            "not for testers"
        );
        write_logix_readme(&data, false).unwrap();
        let readme = tmp.path().join(".logix/README.txt");
        assert!(std::fs::read_to_string(&readme).unwrap().contains("ide/"));
        std::fs::write(&readme, "edited").unwrap();
        write_logix_readme(&data, false).unwrap();
        assert_eq!(std::fs::read_to_string(&readme).unwrap(), "edited");
    }
}
