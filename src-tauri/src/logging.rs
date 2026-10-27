//! The Logix log: developer messages, also kept in a file for problem
//! reports, since an app opened from Finder has nowhere to print them.
//!
//! `log!(...)` prints `[logix] <message>` to stderr (the `./run.sh` terminal)
//! and appends it, with a UTC time, to `~/.logix/logs/logix.log`. What the
//! output pane shows is appended too (`output`), as `| <line>`, so a report
//! has the tools' messages; after `MAX_BYTES` only messages are kept. Each run
//! starts a new file; the previous runs are kept as `logix.1.log` (newest)
//! to `logix.<KEEP-1>.log`. Help > Show Log Folder opens the folder.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Log files kept: this run's and the previous runs'.
const KEEP: usize = 5;
const NAME: &str = "logix";

/// Above this size, output pane lines are no longer written (messages
/// still are), so a long session can't fill the disk.
const MAX_BYTES: u64 = 10 * 1024 * 1024;

struct Log {
    file: File,
    bytes: u64,
    output_truncated: bool,
}

static FILE: Mutex<Option<Log>> = Mutex::new(None);

/// Logs a message (format arguments, like `println!`).
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        $crate::logging::write(&format!($($arg)*))
    };
}

/// Where the log files are.
pub fn dir() -> Option<PathBuf> {
    Some(
        crate::platform::current_implemented()
            .ok()?
            .data_dir()
            .ok()?
            .join("logs"),
    )
}

/// Starts this run's log file, keeping the previous ones. Without a log
/// folder (e.g. an unsupported platform), messages only go to stderr.
pub fn init(header: &str) {
    let Some(dir) = dir() else { return };
    match open_new(&dir) {
        Ok(file) => {
            *lock() = Some(Log {
                file,
                bytes: 0,
                output_truncated: false,
            })
        }
        Err(e) => eprintln!(
            "[logix] Can't create the log file in {}: {e}",
            dir.display()
        ),
    }
    write(header);
}

/// Writes one message to stderr and the log file.
pub fn write(message: &str) {
    eprintln!("[logix] {message}");
    if let Some(log) = lock().as_mut() {
        append(log, &format!("{} {message}\n", utc_time(SystemTime::now())));
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
            utc_time(SystemTime::now()),
            MAX_BYTES / 1024 / 1024
        );
        append(log, &note);
        return;
    }
    let mut text = String::new();
    for line in lines {
        text.push_str("| ");
        text.push_str(crate::apio::strip_ansi(line).trim_end());
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

fn path(dir: &Path, index: usize) -> PathBuf {
    if index == 0 {
        dir.join(format!("{NAME}.log"))
    } else {
        dir.join(format!("{NAME}.{index}.log"))
    }
}

/// Shifts the old logs (logix.log -> logix.1.log -> ...), dropping the
/// oldest, and creates a fresh logix.log.
fn open_new(dir: &Path) -> std::io::Result<File> {
    std::fs::create_dir_all(dir)?;
    for index in (0..KEEP - 1).rev() {
        let from = path(dir, index);
        if from.exists() {
            std::fs::rename(&from, path(dir, index + 1))?;
        }
    }
    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path(dir, 0))
}

/// "2026-10-27 14:05:09Z" (UTC; std has no time zones).
fn utc_time(time: SystemTime) -> String {
    let secs = time.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (days, rest) = (secs / 86_400, secs % 86_400);
    let (y, m, d) = civil_from_days(days as i64);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

/// Days since 1970-01-01 to (year, month, day), from Howard Hinnant's
/// `civil_from_days` algorithm.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn formats_utc_times() {
        let at = |secs| utc_time(UNIX_EPOCH + Duration::from_secs(secs));
        assert_eq!(at(0), "1970-01-01 00:00:00Z");
        assert_eq!(at(951_782_400), "2000-02-29 00:00:00Z");
        assert_eq!(at(1_793_113_489), "2026-10-27 15:04:49Z");
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
        let text = std::fs::read_to_string(path(&dir, 0)).unwrap();
        assert!(text.starts_with("| Build succeeded\n"), "{text}");
        assert!(text.contains("Output truncated"), "{text}");
        assert!(!text.contains("dropped"), "{text}");
        assert_eq!(text.matches("truncated").count(), 1, "{text}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn keeps_the_last_runs() {
        let dir = std::env::temp_dir().join(format!("logix-log-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for run in 0..KEEP + 2 {
            let mut file = open_new(&dir).unwrap();
            write!(file, "run {run}").unwrap();
        }
        let read = |i| std::fs::read_to_string(path(&dir, i)).unwrap();
        assert_eq!(read(0), format!("run {}", KEEP + 1));
        assert_eq!(read(1), format!("run {KEEP}"));
        assert!(path(&dir, KEEP - 1).exists());
        assert!(!path(&dir, KEEP).exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
