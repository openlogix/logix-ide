//! IDE settings kept between runs (e.g. the last project), in
//! `settings.json` in the IDE's data folder. Plain Rust, no Tauri.

use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Folder of the project last opened or saved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_project: Option<PathBuf>,
    /// Recently opened or saved project folders, newest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recent_projects: Vec<PathBuf>,
    /// The board last chosen; new projects start with it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_board: Option<String>,
    /// User preferences (File > Settings). Unset means the default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projects_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editor_font_size: Option<u32>,
    /// Interface scale in percent; unset means automatic (100%).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_scale: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<Theme>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lint_on_verify: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub live_checking: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hover_info: Option<bool>,
    /// Unknown settings (e.g. from a newer version) are kept.
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    /// Follow the system's light/dark setting.
    System,
    Light,
    Dark,
}

/// The preferences as the Settings dialog shows them, defaults filled in.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    /// Where new projects go by default (Arduino's "sketchbook location").
    pub projects_dir: PathBuf,
    pub editor_font_size: u32,
    /// Interface scale in percent; `None` for automatic.
    pub ui_scale: Option<u32>,
    pub theme: Theme,
    /// Whether Verify runs lint (Verilator) for warnings.
    pub lint_on_verify: bool,
    /// Whether the editor checks the code while typing (Verible's language
    /// server).
    pub live_checking: bool,
    /// Whether hovering over a name shows info about it (when live
    /// checking is on).
    pub hover_info: bool,
}

pub const DEFAULT_FONT_SIZE: u32 = 13;
pub const FONT_SIZES: std::ops::RangeInclusive<u32> = 8..=32;
pub const UI_SCALES: std::ops::RangeInclusive<u32> = 50..=300;

impl Settings {
    /// The preferences, with defaults for anything unset.
    pub fn preferences(&self, default_projects_dir: &Path) -> Preferences {
        Preferences {
            projects_dir: self
                .projects_dir
                .clone()
                .unwrap_or_else(|| default_projects_dir.to_path_buf()),
            editor_font_size: self.editor_font_size.unwrap_or(DEFAULT_FONT_SIZE),
            ui_scale: self.ui_scale,
            theme: self.theme.unwrap_or(Theme::System),
            lint_on_verify: self.lint_on_verify.unwrap_or(true),
            live_checking: self.live_checking.unwrap_or(true),
            hover_info: self.hover_info.unwrap_or(false),
        }
    }

    /// Stores `prefs` (after checking them). Values equal to the defaults
    /// are stored as unset, so a later change of default applies.
    pub fn set_preferences(
        &mut self,
        prefs: &Preferences,
        default_projects_dir: &Path,
    ) -> Result<()> {
        if !FONT_SIZES.contains(&prefs.editor_font_size) {
            anyhow::bail!(
                "The font size must be between {} and {}.",
                FONT_SIZES.start(),
                FONT_SIZES.end()
            );
        }
        if prefs.ui_scale.is_some_and(|s| !UI_SCALES.contains(&s)) {
            anyhow::bail!(
                "The interface scale must be between {}% and {}%.",
                UI_SCALES.start(),
                UI_SCALES.end()
            );
        }
        if !prefs.projects_dir.is_absolute() {
            anyhow::bail!("Choose a full path for the projects folder.");
        }
        self.projects_dir =
            (prefs.projects_dir != default_projects_dir).then(|| prefs.projects_dir.clone());
        self.editor_font_size =
            (prefs.editor_font_size != DEFAULT_FONT_SIZE).then_some(prefs.editor_font_size);
        self.ui_scale = prefs.ui_scale;
        self.theme = (prefs.theme != Theme::System).then_some(prefs.theme);
        self.lint_on_verify = (!prefs.lint_on_verify).then_some(false);
        self.live_checking = (!prefs.live_checking).then_some(false);
        self.hover_info = prefs.hover_info.then_some(true);
        Ok(())
    }
}

/// How many recent projects File > Open Recent lists.
pub const MAX_RECENT: usize = 10;

/// Where the settings are kept: `settings.json` in the IDE's data folder.
pub fn default_path() -> Result<PathBuf> {
    Ok(crate::platform::current_implemented()?
        .data_dir()?
        .join("settings.json"))
}

/// Reads the settings. A missing or damaged file gives the defaults.
pub fn load(path: &Path) -> Settings {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub fn save(path: &Path, settings: &Settings) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    crate::project::write_atomic(path, &serde_json::to_vec_pretty(settings)?)
}

/// Records `dir` as the last project and the most recent one.
pub fn remember_project(path: &Path, dir: &Path) -> Result<()> {
    let mut settings = load(path);
    settings.last_project = Some(dir.to_path_buf());
    settings.recent_projects.retain(|p| p != dir);
    settings.recent_projects.insert(0, dir.to_path_buf());
    settings.recent_projects.truncate(MAX_RECENT);
    save(path, &settings)
}

/// Removes `dir` from the recent projects (e.g. its folder is gone).
pub fn forget_project(path: &Path, dir: &Path) -> Result<()> {
    let mut settings = load(path);
    settings.recent_projects.retain(|p| p != dir);
    if settings.last_project.as_deref() == Some(dir) {
        settings.last_project = None;
    }
    save(path, &settings)
}

/// Empties the recent projects list.
pub fn clear_recent(path: &Path) -> Result<()> {
    let mut settings = load(path);
    settings.recent_projects.clear();
    save(path, &settings)
}

/// Records `board` as the last chosen board.
pub fn remember_board(path: &Path, board: &str) -> Result<()> {
    let mut settings = load(path);
    settings.last_board = Some(board.to_string());
    save(path, &settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remembers_last_project_and_keeps_unknown_settings() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("logix/settings.json");
        assert!(load(&path).last_project.is_none());

        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, r#"{"future": 1}"#).unwrap();
        remember_project(&path, Path::new("/p/blinky")).unwrap();

        let s = load(&path);
        assert_eq!(s.last_project.as_deref(), Some(Path::new("/p/blinky")));
        let json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(json["future"], 1);
    }

    #[test]
    fn remembers_last_board_alongside_project() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        remember_project(&path, Path::new("/p/a")).unwrap();
        remember_board(&path, "alhambra-ii").unwrap();
        let s = load(&path);
        assert_eq!(s.last_board.as_deref(), Some("alhambra-ii"));
        assert_eq!(s.last_project.as_deref(), Some(Path::new("/p/a")));
    }

    #[test]
    fn keeps_recent_projects_newest_first_without_repeats() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        for i in 0..12 {
            remember_project(&path, Path::new(&format!("/p/{i}"))).unwrap();
        }
        remember_project(&path, Path::new("/p/5")).unwrap();
        let recent = load(&path).recent_projects;
        assert_eq!(recent.len(), MAX_RECENT);
        assert_eq!(recent[0], Path::new("/p/5"));
        assert_eq!(recent[1], Path::new("/p/11"));
        assert_eq!(recent.iter().filter(|p| *p == Path::new("/p/5")).count(), 1);

        forget_project(&path, Path::new("/p/5")).unwrap();
        let s = load(&path);
        assert!(!s.recent_projects.contains(&PathBuf::from("/p/5")));
        assert_eq!(s.last_project, None);

        clear_recent(&path).unwrap();
        assert!(load(&path).recent_projects.is_empty());
    }

    #[test]
    fn preferences_have_defaults_and_round_trip() {
        let default_dir = Path::new("/Users/a/Documents/Logix");
        let mut s = Settings::default();
        let p = s.preferences(default_dir);
        assert_eq!(p.projects_dir, default_dir);
        assert_eq!(
            (p.editor_font_size, p.ui_scale, p.theme, p.lint_on_verify),
            (13, None, Theme::System, true)
        );
        assert_eq!((p.live_checking, p.hover_info), (true, false));

        let changed = Preferences {
            projects_dir: PathBuf::from("/Users/a/fpga"),
            editor_font_size: 16,
            ui_scale: Some(125),
            theme: Theme::Dark,
            lint_on_verify: false,
            live_checking: false,
            hover_info: true,
        };
        s.set_preferences(&changed, default_dir).unwrap();
        assert_eq!(s.preferences(default_dir), changed);

        // Back to the defaults: nothing is stored.
        s.set_preferences(&p, default_dir).unwrap();
        assert!(s.projects_dir.is_none() && s.editor_font_size.is_none() && s.theme.is_none());
    }

    #[test]
    fn rejects_bad_preferences() {
        let default_dir = Path::new("/d");
        let mut s = Settings::default();
        let ok = s.preferences(default_dir);
        for bad in [
            Preferences {
                editor_font_size: 4,
                ..ok.clone()
            },
            Preferences {
                ui_scale: Some(1000),
                ..ok.clone()
            },
            Preferences {
                projects_dir: PathBuf::from("relative/dir"),
                ..ok.clone()
            },
        ] {
            assert!(s.set_preferences(&bad, default_dir).is_err());
        }
    }

    #[test]
    fn damaged_file_gives_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(&path, "not json").unwrap();
        assert!(load(&path).last_project.is_none());
    }
}
