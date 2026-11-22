//! The build folder the IDE runs Logix Engine in. Plain Rust, no Tauri.
//!
//! Builds use the project's in-memory files, so unsaved edits are built
//! too. The folder is reused between builds of the same project, so the
//! engine's `_build/` output stays and repeated builds are faster.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::project::{self, ProjectFile, RESERVED_NAMES};

/// Records which project the folder was prepared for.
const KEY_FILE: &str = ".logix-build-key";
/// The engine's output folder, kept between builds.
const OUTPUT_DIR: &str = "_build";

pub struct BuildInput<'a> {
    /// Identifies the project (e.g. its folder). A different key starts
    /// from an empty folder.
    pub key: &'a str,
    /// Folder whose other files (e.g. memory init data) are copied too.
    pub source_dir: Option<&'a Path>,
    pub files: &'a [ProjectFile],
}

/// The project's pin constraint file for a board whose pin files have the
/// extension `wanted` (any case): the first such file, or else the first
/// pin file of any kind (the engine then explains that it's the wrong
/// kind). `None` if the project has no pin file.
pub fn constraint_file<'a>(files: &'a [ProjectFile], wanted: Option<&str>) -> Option<&'a str> {
    let pins = || {
        files
            .iter()
            .map(|f| f.name.as_str())
            .filter(|n| is_constraint_file(n))
    };
    let matches = |n: &&str| {
        let ext = Path::new(n).extension().and_then(|e| e.to_str());
        ext.zip(wanted)
            .is_some_and(|(e, w)| e.eq_ignore_ascii_case(w))
    };
    pins().find(matches).or_else(|| pins().next())
}

/// The first pin constraint file among file names.
#[cfg(test)]
pub fn constraint_file_name(names: &[String]) -> Option<&str> {
    names
        .iter()
        .map(String::as_str)
        .find(|n| is_constraint_file(n))
}

/// A pin constraint file of any FPGA family's format (Logix Engine's
/// `constraint_extension`s, `formats`).
fn is_constraint_file(name: &str) -> bool {
    crate::formats::current().is_pin_file(name)
}

/// Fills `root` with the project's files and the project folder's other
/// contents.
/// Returns the folder to run the engine in.
pub fn prepare(root: &Path, input: &BuildInput) -> Result<PathBuf> {
    project::check_files(input.files)?;

    let key_path = root.join(KEY_FILE);
    let same_project = std::fs::read_to_string(&key_path).is_ok_and(|k| k == input.key);
    if !same_project && root.exists() {
        std::fs::remove_dir_all(root).with_context(|| format!("can't clear {}", root.display()))?;
    }
    std::fs::create_dir_all(root).with_context(|| format!("can't create {}", root.display()))?;

    // Remove everything from the previous build except the engine's output, so
    // deleted or renamed files don't linger.
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == OUTPUT_DIR || name == KEY_FILE {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            std::fs::remove_dir_all(&path)?;
        } else {
            std::fs::remove_file(&path)?;
        }
    }

    if let Some(source) = input.source_dir.filter(|d| d.is_dir()) {
        let skip: Vec<&str> = input
            .files
            .iter()
            .map(|f| f.name.as_str())
            .chain(RESERVED_NAMES.iter().copied())
            .collect();
        project::copy_project_contents(source, root, &skip, project::CopyMode::Build)?;
    }
    for f in input.files {
        project::write_atomic(&root.join(&f.name), f.text.as_bytes())?;
    }
    std::fs::write(&key_path, input.key)?;
    Ok(root.to_path_buf())
}

/// Fills `root` with just `file` (for Auto Format, which formats the
/// editor's text, saved or not). Returns the folder.
pub fn prepare_single(root: &Path, file: &ProjectFile) -> Result<PathBuf> {
    project::check_plain_file_name(&file.name)?;
    if root.exists() {
        std::fs::remove_dir_all(root).with_context(|| format!("can't clear {}", root.display()))?;
    }
    std::fs::create_dir_all(root).with_context(|| format!("can't create {}", root.display()))?;
    std::fs::write(root.join(&file.name), &file.text)?;
    Ok(root.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str, text: &str) -> ProjectFile {
        ProjectFile {
            name: name.into(),
            text: text.into(),
        }
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut v: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }

    #[test]
    fn writes_the_files() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("build");
        let files = [file("main.v", "m"), file("pinout.pcf", "p")];
        let dir = prepare(
            &root,
            &BuildInput {
                key: "/p/a",
                source_dir: None,
                files: &files,
            },
        )
        .unwrap();
        assert_eq!(names(&dir), [".logix-build-key", "main.v", "pinout.pcf"]);
    }

    #[test]
    fn keeps_output_for_same_project_and_drops_stale_files() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("build");
        let two = [file("main.v", "x"), file("old.v", "x")];
        let one = [file("main.v", "x")];
        let input = |key, files| BuildInput {
            key,
            source_dir: None,
            files,
        };

        prepare(&root, &input("k1", &two)).unwrap();
        std::fs::create_dir_all(root.join("_build")).unwrap();

        prepare(&root, &input("k1", &one)).unwrap();
        assert!(root.join("_build").exists(), "same project keeps output");
        assert!(!root.join("old.v").exists(), "removed files are dropped");

        prepare(&root, &input("k2", &one)).unwrap();
        assert!(
            !root.join("_build").exists(),
            "another project starts clean"
        );
    }

    #[test]
    fn copies_other_source_files_but_not_ide_files() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        std::fs::create_dir_all(src.join("_build")).unwrap();
        for (name, text) in [
            ("main.v", "on disk"),
            ("rom.hex", "00"),
            ("logix-ide.toml", "board = \"b\""),
            (".hidden", "x"),
        ] {
            std::fs::write(src.join(name), text).unwrap();
        }
        let root = tmp.path().join("build");
        let files = [file("main.v", "in memory")];
        prepare(
            &root,
            &BuildInput {
                key: "k",
                source_dir: Some(&src),
                files: &files,
            },
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join("main.v")).unwrap(),
            "in memory"
        );
        assert_eq!(std::fs::read_to_string(root.join("rom.hex")).unwrap(), "00");
        assert!(!root.join("logix-ide.toml").exists());
        assert!(!root.join(".hidden").exists());
        assert!(!root.join("_build").exists());
    }

    #[test]
    fn copies_subfolders_but_not_nested_projects() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("proj");
        for (name, text) in [
            ("main.v", "on disk"),
            ("data/rom.hex", "00"),
            ("data/old.v", "module old; endmodule"),
            ("data/pins.pcf", "set_io a 1"),
            ("copy/logix-ide.toml", ""),
            ("copy/main.v", "module main; endmodule"),
            ("copy/notes.txt", "hi"),
        ] {
            let path = src.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        let root = tmp.path().join("build");
        let files = [file("main.v", "in memory")];
        prepare(
            &root,
            &BuildInput {
                key: "k",
                source_dir: Some(&src),
                files: &files,
            },
        )
        .unwrap();
        // Subfolders go along (the engine builds only the listed files),
        // but not a nested project.
        assert!(root.join("data/rom.hex").exists());
        assert!(root.join("data/old.v").exists());
        assert!(!root.join("copy").exists());
    }

    #[test]
    fn rejects_bad_input() {
        let tmp = tempfile::tempdir().unwrap();
        let ok_files = [file("main.v", "m")];
        let ok = BuildInput {
            key: "k",
            source_dir: None,
            files: &ok_files,
        };
        assert!(prepare(tmp.path(), &ok).is_ok());
        let no_main = [file("other.v", "x")];
        let bad_files = BuildInput {
            key: "k",
            source_dir: None,
            files: &no_main,
        };
        assert!(prepare(tmp.path(), &bad_files).is_err());
    }

    #[test]
    fn single_file_project() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("format");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("stale.v"), "x").unwrap();
        let dir = prepare_single(&root, &file("alu.v", "module alu; endmodule")).unwrap();
        assert_eq!(names(&dir), ["alu.v"]);
        assert!(prepare_single(&root, &file("../x.v", "")).is_err());
    }

    #[test]
    fn finds_constraint_file() {
        let pcf = Some("pcf");
        assert_eq!(
            constraint_file(&[file("main.v", ""), file("Pins.PCF", "")], pcf),
            Some("Pins.PCF")
        );
        assert_eq!(constraint_file(&[file("main.v", "")], pcf), None);
        // The board's kind wins over an earlier pin file of another kind.
        let both = [file("main.v", ""), file("a.lpf", ""), file("b.pcf", "")];
        assert_eq!(constraint_file(&both, pcf), Some("b.pcf"));
        assert_eq!(constraint_file(&both, Some("lpf")), Some("a.lpf"));
        // None of the board's kind: the first one, for the engine to refuse.
        assert_eq!(constraint_file(&both, Some("cst")), Some("a.lpf"));
        assert_eq!(constraint_file(&both, None), Some("a.lpf"));
    }
}
