//! The Logix IDE log: developer messages, also kept in a file for problem
//! reports, since an app opened from Finder has nowhere to print them.
//!
//! The Open Logix log layout (the website repo's `CLAUDE.md`, Log files;
//! Logix Engine's docs/command-line.md, Logs): Logix IDE is a top program,
//! so each run makes a session folder in `~/.logix/ide/logs`, named by its
//! start (local time) and process ID, `<yyyymmdd>-<hhmmss>-<pid>`, and writes
//! `logix-ide.log` there after a separator line. The programs it starts log
//! into the same folder, named by their chain: the engine
//! `logix-ide.logix-engine.log` (`--log-folder`, `--log-name`, and its
//! GTKWave `logix-ide.logix-engine.gtkwave.log`), Verible's language server
//! `logix-ide.verible-verilog-ls.log` (`child_file`). Old session folders
//! are deleted by the engine's `clean_logs` tool once the engine runs
//! (`commands::setup`), so the policy is the engine's, the same for every
//! frontend.
//!
//! `log!(...)` prints `[logix] <message>` to stderr (the `./run.sh` terminal)
//! and appends it, with the local time, to the file. What the output pane
//! shows is appended too (`output`), as `| <line>`, so a report has the
//! tools' messages; after `MAX_BYTES` only messages are kept. Help > Show
//! Log Folder opens this session's folder.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, OnceLock};

use time::macros::format_description;
use time::{OffsetDateTime, UtcOffset};

use regex::Regex;

/// Terminal escape sequences: colors and styles (`ESC[...m`), cursor and
/// erase commands, and title/link sequences (`ESC]...BEL`).
static ANSI: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\x1b(?:\[[0-?]*[ -/]*[@-~]|\][^\x07\x1b]*(?:\x07|\x1b\\)|[@-Z\\-_])").unwrap()
});

/// Removes terminal escape sequences (e.g. colors): the log keeps the
/// output pane's lines as plain text.
pub fn strip_ansi(text: &str) -> std::borrow::Cow<'_, str> {
    ANSI.replace_all(text, "")
}

/// Logix IDE's name in log file names.
pub const NAME: &str = "logix-ide";

/// Above this size, output pane lines are no longer written (messages
/// still are), so a long session can't fill the disk.
const MAX_BYTES: u64 = 10 * 1024 * 1024;

struct Log {
    file: File,
    bytes: u64,
    output_truncated: bool,
}

static FILE: Mutex<Option<Log>> = Mutex::new(None);
/// This run's session folder, once `init` made it.
static SESSION: OnceLock<PathBuf> = OnceLock::new();
/// The local time's offset from UTC, read once at startup.
static LOCAL_OFFSET: OnceLock<UtcOffset> = OnceLock::new();

/// Reads the local time's offset from UTC. Call it first in `run`, before
/// any thread starts: the time crate reads it reliably only then on macOS
/// and Linux. Falls back to UTC when it can't be read.
pub fn init_local_offset() {
    let _ = LOCAL_OFFSET.set(UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC));
}

fn now_local() -> OffsetDateTime {
    OffsetDateTime::now_utc().to_offset(*LOCAL_OFFSET.get().unwrap_or(&UtcOffset::UTC))
}

/// A session folder's name: `<yyyymmdd>-<hhmmss>-<pid>` (local time).
fn session_name(at: OffsetDateTime, pid: u32) -> String {
    let started = at
        .format(format_description!(
            "[year][month][day]-[hour][minute][second]"
        ))
        .unwrap_or_default();
    format!("{started}-{pid}")
}

/// A separator line, written before each run's lines in a log file.
pub fn separator(what: &str) -> String {
    separator_at(now_local(), what)
}

fn separator_at(at: OffsetDateTime, what: &str) -> String {
    let when = at
        .format(format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second] [offset_hour sign:mandatory]:[offset_minute]"
        ))
        .unwrap_or_default();
    format!("=== {when}  {what} ===\n")
}

/// A log line's time: local, to the second (the offset is in the separator).
fn line_time(at: OffsetDateTime) -> String {
    at.format(format_description!(
        "[year]-[month]-[day] [hour]:[minute]:[second]"
    ))
    .unwrap_or_default()
}

/// Logs a message (format arguments, like `println!`).
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        $crate::logging::write(&format!($($arg)*))
    };
}

/// Where the session folders are: `~/.logix/ide/logs`.
pub fn dir() -> Option<PathBuf> {
    Some(
        crate::platform::current_implemented()
            .ok()?
            .data_dir()
            .ok()?
            .join("logs"),
    )
}

/// This run's session folder, once `init` has made it.
pub fn session() -> Option<PathBuf> {
    SESSION.get().cloned()
}

/// This run's log file, once `init` has created it.
pub fn current_file() -> Option<PathBuf> {
    lock().as_ref()?;
    session().map(|s| s.join(format!("{NAME}.log")))
}

/// The log file of a program Logix IDE starts, such as
/// `verible-verilog-ls`: `logix-ide.<program>.log` in this session's folder.
pub fn child_file(program: &str) -> Option<PathBuf> {
    session().map(|s| s.join(format!("{NAME}.{program}.log")))
}

/// Opens a log file of this session for appending, with a separator line
/// saying `what`, for a program's output (such as the language server's).
pub fn open_child(program: &str, what: &str) -> Option<File> {
    let path = child_file(program)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .ok()?;
    file.write_all(separator(what).as_bytes()).ok()?;
    Some(file)
}

/// Makes this run's session folder and log file, and writes its separator.
/// Without a log folder (e.g. an unsupported platform), messages only go to
/// stderr.
pub fn init(header: &str) {
    let Some(dir) = dir() else { return };
    let session = dir.join(session_name(now_local(), std::process::id()));
    match open_new(&session) {
        Ok(file) => {
            let _ = SESSION.set(session);
            let mut log = Log {
                file,
                bytes: 0,
                output_truncated: false,
            };
            append(&mut log, &separator(header));
            *lock() = Some(log);
        }
        Err(e) => eprintln!(
            "[logix] Can't create the log file in {}: {e}",
            session.display()
        ),
    }
    eprintln!("[logix] {header}");
}

/// Writes one message to stderr and the log file.
pub fn write(message: &str) {
    eprintln!("[logix] {message}");
    if let Some(log) = lock().as_mut() {
        append(log, &format!("{} {message}\n", line_time(now_local())));
    }
}

/// Appends lines the output pane showed (ANSI colors removed), to the file
/// only: the developer terminal doesn't need them.
pub fn output(lines: &[String]) {
    if let Some(log) = lock().as_mut() {
        output_into(log, lines);
    }
}

fn output_into(log: &mut Log, lines: &[String]) {
    if log.output_truncated {
        return;
    }
    if log.bytes > MAX_BYTES {
        log.output_truncated = true;
        let note = format!(
            "{} Output truncated: the log is over {} MB.\n",
            line_time(now_local()),
            MAX_BYTES / 1024 / 1024
        );
        append(log, &note);
        return;
    }
    let mut text = String::new();
    for line in lines {
        text.push_str("| ");
        text.push_str(strip_ansi(line).trim_end());
        text.push('\n');
    }
    append(log, &text);
}

fn append(log: &mut Log, text: &str) {
    if log.file.write_all(text.as_bytes()).is_ok() {
        log.bytes += text.len() as u64;
    }
}

fn lock() -> std::sync::MutexGuard<'static, Option<Log>> {
    // A panic while writing must not stop later messages.
    FILE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Creates a session folder and its `logix-ide.log`.
fn open_new(session: &Path) -> std::io::Result<File> {
    std::fs::create_dir_all(session)?;
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(session.join(format!("{NAME}.log")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_ansi_sequences() {
        assert_eq!(strip_ansi("\x1b[32mSUCCESS\x1b[0m"), "SUCCESS");
        assert_eq!(strip_ansi("a\x1b[1;38;5;208mb\x1b[0m c"), "ab c");
        assert_eq!(strip_ansi("\x1b[2K\x1b[?25lline"), "line");
        assert_eq!(strip_ansi("\x1b]8;;http://x\x07link\x1b]8;;\x07"), "link");
        assert_eq!(strip_ansi("plain"), "plain");
    }
    #[test]
    fn session_names_separators_and_times() {
        let t = OffsetDateTime::from_unix_timestamp(1_793_145_600 + 3_723)
            .unwrap()
            .to_offset(UtcOffset::from_hms(-8, 0, 0).unwrap()); // 2026-10-27 17:02:03 -08:00
        assert_eq!(session_name(t, 4412), "20261027-170203-4412");
        assert_eq!(
            separator_at(t, "Logix IDE 2026.11.15 started"),
            "=== 2026-10-27 17:02:03 -08:00  Logix IDE 2026.11.15 started ===\n"
        );
        assert_eq!(line_time(t), "2026-10-27 17:02:03");
    }

    #[test]
    fn writes_output_lines_until_the_cap() {
        let dir = std::env::temp_dir().join(format!("logix-out-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut log = Log {
            file: open_new(&dir).unwrap(),
            bytes: 0,
            output_truncated: false,
        };
        output_into(&mut log, &["\x1b[32mBuild succeeded\x1b[0m  ".to_string()]);
        log.bytes = MAX_BYTES + 1;
        output_into(&mut log, &["dropped".to_string()]);
        output_into(&mut log, &["dropped too".to_string()]);
        let text = std::fs::read_to_string(dir.join("logix-ide.log")).unwrap();
        assert!(text.starts_with("| Build succeeded\n"), "{text}");
        assert!(text.contains("Output truncated"), "{text}");
        assert!(!text.contains("dropped"), "{text}");
        assert_eq!(text.matches("truncated").count(), 1, "{text}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
