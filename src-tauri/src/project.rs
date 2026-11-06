//! The Logix IDE project model, independent of Tauri.
//!
//! A project is a folder (any name) containing:
//! - `main.v` (or `main.sv` for SystemVerilog): the main file, whose top
//!   module is `main`;
//! - optionally more source files (Verilog, pin constraints, testbenches),
//!   which the IDE shows as tabs;
//! - optionally anything else (subfolders, docs, images), kept as is;
//! - `logix-ide.toml`: the project's settings, such as its board and the
//!   board's variant (TOML, so it can hold comments; Logix keeps them when
//!   it updates a value);
//! - optionally `custom-boards.json`: the project's own boards, a JSON list
//!   in Logix Engine's custom board format, shown as a tab like the
//!   sources.
//!
//! The UI holds the files in memory. Creating, renaming and deleting files
//! take effect on save, like text edits.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

/// The main file of new projects.
pub const MAIN_FILE: &str = "main.v";
/// Accepted main file names, in order of preference.
pub const MAIN_FILES: &[&str] = &["main.v", "main.sv"];
pub const SETTINGS_FILE: &str = "logix-ide.toml";
/// The project's own boards: a JSON list of Logix Engine custom boards.
pub const CUSTOM_BOARDS_FILE: &str = "custom-boards.json";

/// Extensions of files shown as editor tabs: Verilog/SystemVerilog sources
/// and headers, and the pin constraint formats of the supported boards.
pub const EDITABLE_EXTENSIONS: &[&str] = &[
    "v", "sv", "vh", "svh", "pcf", "lpf", "cst", "xdc", "pdc", "ccf",
];

/// Names that belong to the IDE or the build, never project files.
pub(crate) const RESERVED_NAMES: &[&str] = &[SETTINGS_FILE, "_build"];

/// The first lines of a new project file.
const SETTINGS_HEADER: &str = "# Logix IDE project settings.\n";

/// Whether `dir` is a Logix project folder (it has a project file).
pub fn is_project_dir(dir: &Path) -> bool {
    dir.join(SETTINGS_FILE).is_file()
}

/// One source file of a project (a top-level file in the folder).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectFile {
    pub name: String,
    pub text: String,
}

/// A project as loaded from disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub dir: PathBuf,
    pub name: String,
    /// `main.v` first, then the other source files by name.
    pub files: Vec<ProjectFile>,
    pub board: Option<String>,
    /// The board's variant, e.g. `{"memory": "ram"}`: the `[variant]`
    /// table. Empty means the board's default.
    pub variant: Variant,
}

/// A board's variant: values of its dimensions, by dimension id.
pub type Variant = BTreeMap<String, String>;

/// The variant in the project settings (the `[variant]` table; values that
/// aren't text are ignored).
fn read_variant(settings: &toml_edit::DocumentMut) -> Variant {
    settings
        .get("variant")
        .and_then(|v| v.as_table_like())
        .map(|table| {
            table
                .iter()
                .filter_map(|(k, v)| Some((k.to_string(), v.as_str()?.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

/// The project's display name: its folder name.
pub fn project_name(dir: &Path) -> String {
    dir.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| dir.display().to_string())
}

/// Whether a file with this name is shown as an editor tab: sources, pin
/// files and the project's custom boards file.
pub fn is_editable(name: &str) -> bool {
    name == CUSTOM_BOARDS_FILE
        || Path::new(name)
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| {
                EDITABLE_EXTENSIONS
                    .iter()
                    .any(|x| x.eq_ignore_ascii_case(e))
            })
}

/// Whether a file is Verilog or SystemVerilog source (what Auto Format takes).
pub fn is_hdl_source(name: &str) -> bool {
    Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| {
            ["v", "sv", "vh", "svh"]
                .iter()
                .any(|x| x.eq_ignore_ascii_case(e))
        })
}

/// Whether `name` is a plain Verilog identifier (e.g. a module name).
pub fn is_verilog_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// Whether a file is a testbench: `*_tb.v` or `*_tb.sv`.
pub fn is_testbench(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with("_tb.v") || lower.ends_with("_tb.sv")
}

/// Checks a name for a new or renamed file. `existing` holds the names of
/// the project's other files. Returns a message for the user if invalid.
pub fn check_new_file_name(name: &str, existing: &[String]) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Enter a file name.".into());
    }
    check_plain_name(name)?;
    // Spaces work in folder names, but tool command lines may split on them.
    if name.contains(char::is_whitespace) {
        return Err("A file name can't contain spaces.".into());
    }
    if !is_editable(name) {
        return Err(format!(
            "Use one of these extensions: {}, or name it {CUSTOM_BOARDS_FILE} for your own boards.",
            EDITABLE_EXTENSIONS
                .iter()
                .map(|e| format!(".{e}"))
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }
    if MAIN_FILES.iter().any(|m| m.eq_ignore_ascii_case(name)) {
        return Err(format!("\"{name}\" is reserved for the main file."));
    }
    if existing.iter().any(|e| e.eq_ignore_ascii_case(name)) {
        return Err(format!("There's already a file named \"{name}\"."));
    }
    Ok(())
}

/// Loads the project in `dir`.
pub fn load(dir: &Path) -> Result<Project> {
    let Some(main) = MAIN_FILES.iter().find(|m| dir.join(m).is_file()) else {
        bail!(
            "The folder \"{}\" isn't a Logix project: it has no main.v or main.sv.",
            project_name(dir)
        );
    };
    let mut names = Vec::new();
    for entry in std::fs::read_dir(dir).with_context(|| format!("can't read {}", dir.display()))? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type()?.is_file()
            && name != *main
            && check_plain_name(&name).is_ok()
            && is_editable(&name)
        {
            names.push(name);
        }
    }
    names.sort_by_key(|n| n.to_lowercase());
    names.insert(0, main.to_string());

    let mut files = Vec::new();
    for name in names {
        let path = dir.join(&name);
        match std::fs::read_to_string(&path) {
            Ok(text) => files.push(ProjectFile { name, text }),
            // Not text (e.g. binary with a source extension): leave it alone.
            Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {}
            Err(e) => return Err(e).with_context(|| format!("can't read {}", path.display())),
        }
    }
    if files.first().map(|f| f.name.as_str()) != Some(*main) {
        bail!("{main} isn't a text file.");
    }
    let settings = read_settings(dir)?;
    Ok(Project {
        dir: dir.to_path_buf(),
        name: project_name(dir),
        files,
        board: settings
            .get("board")
            .and_then(|b| b.as_str())
            .map(String::from),
        variant: read_variant(&settings),
    })
}

/// Saves into the existing project folder `dir`: deletes the `removed`
/// files (deleted or renamed in the IDE), then writes `files` and the
/// settings.
pub fn save(
    dir: &Path,
    files: &[ProjectFile],
    removed: &[String],
    board: Option<&str>,
    variant: &Variant,
) -> Result<()> {
    if !dir.is_dir() {
        bail!("The project folder {} no longer exists.", dir.display());
    }
    check_files(files)?;
    for name in removed {
        check_plain_name(name).map_err(anyhow::Error::msg)?;
        if MAIN_FILES.contains(&name.as_str()) || files.iter().any(|f| &f.name == name) {
            continue;
        }
        let path = dir.join(name);
        if path.is_file() {
            std::fs::remove_file(&path)
                .with_context(|| format!("can't delete {}", path.display()))?;
        }
    }
    for f in files {
        write_atomic(&dir.join(&f.name), f.text.as_bytes())?;
    }
    // Only the values Logix owns change; the user's comments, layout and
    // other settings stay.
    let path = dir.join(SETTINGS_FILE);
    let is_new = !path.exists();
    let mut settings = read_settings(dir).unwrap_or_default();
    match board {
        Some(b) => settings["board"] = toml_edit::value(b),
        None => {
            settings.remove("board");
        }
    }
    if variant.is_empty() {
        settings.remove("variant");
    } else {
        let mut table = toml_edit::Table::new();
        for (k, v) in variant {
            table[k] = toml_edit::value(v);
        }
        settings["variant"] = toml_edit::Item::Table(table);
    }
    let mut text = settings.to_string();
    if is_new {
        text.insert_str(0, SETTINGS_HEADER);
    }
    write_atomic(&path, text.as_bytes())
}

/// Creates a new project folder `target` ("Save As") with `files`. If
/// `source` is given, its other contents are copied too, except the files
/// in `removed`. `target` must not exist yet.
pub fn create(
    target: &Path,
    source: Option<&Path>,
    files: &[ProjectFile],
    removed: &[String],
    board: Option<&str>,
    variant: &Variant,
) -> Result<()> {
    if target.exists() {
        bail!(
            "A file or folder named \"{}\" already exists there. Choose another name.",
            project_name(target)
        );
    }
    // A project inside another one would be copied into its own builds
    // (and, inside `source`, into itself).
    let inside = source
        .filter(|s| target.starts_with(s))
        .map(Path::to_path_buf)
        .or_else(|| enclosing_project(target));
    if let Some(outer) = inside {
        bail!(
            "That location is inside the Logix project \"{}\". Save the project outside \
             that project's folder, e.g. next to it.",
            project_name(&outer)
        );
    }
    check_files(files)?;
    std::fs::create_dir_all(target)
        .with_context(|| format!("can't create {}", target.display()))?;
    if let Some(source) = source {
        let skip: Vec<&str> = files
            .iter()
            .map(|f| f.name.as_str())
            .chain(removed.iter().map(String::as_str))
            .chain(RESERVED_NAMES.iter().copied())
            .collect();
        copy_project_contents(source, target, &skip, CopyMode::All)?;
    }
    save(target, files, &[], board, variant)
}

/// Save As into `target`: the project's own folder (`source`) is saved in
/// place, as Save does (the user's decision: trashing it first would lose
/// the files Save As copies, such as data files, and in place gives the
/// same result); another existing folder, which the save dialog already
/// asked to replace, is checked (`check_replace`) and moved away with
/// `trash` first; then the project is created there with `source`'s other
/// files.
pub fn save_as(
    target: &Path,
    source: Option<&Path>,
    files: &[ProjectFile],
    removed: &[String],
    board: Option<&str>,
    variant: &Variant,
    trash: impl FnOnce(&Path) -> Result<()>,
) -> Result<()> {
    let real = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    if target.exists() && source.is_some_and(|source| real(target) == real(source)) {
        return save(target, files, removed, board, variant).context("Couldn't save the project.");
    }
    if target.exists() {
        check_replace(target, source)?;
        trash(target)?;
    }
    create(target, source, files, removed, board, variant).context("Couldn't save the project.")
}

/// Checks that Save As may replace the existing `target`, which the user
/// agreed to in the save dialog (the caller moves it to the Trash first).
/// Only a Logix project or an empty folder is replaced, so a mistyped name
/// can't throw away other files, and never the project being saved
/// (`source`) or a folder containing it.
pub fn check_replace(target: &Path, source: Option<&Path>) -> Result<()> {
    let name = project_name(target);
    if !target.is_dir() {
        bail!("\"{name}\" there is a file, not a project. Choose another name.");
    }
    let real = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    if let Some(source) = source {
        if real(source).starts_with(real(target)) {
            bail!(
                "Can't replace \"{name}\": it holds the project you're saving. \
                 Choose another name."
            );
        }
    }
    let empty = std::fs::read_dir(target)
        .with_context(|| format!("can't read {}", target.display()))?
        .flatten()
        .all(|e| e.file_name().to_string_lossy().starts_with('.'));
    if !empty && !is_project_dir(target) {
        bail!(
            "The folder \"{name}\" isn't a Logix project, so Logix won't replace it. \
             Choose another name."
        );
    }
    Ok(())
}

/// The Logix project folder (one with a project file) that contains
/// `path`, if any.
pub fn enclosing_project(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .skip(1)
        .find(|dir| is_project_dir(dir))
        .map(Path::to_path_buf)
}

/// The built-in projects folder, like Arduino's sketchbook:
/// `~/Documents/Logix`.
pub fn builtin_projects_root() -> Option<PathBuf> {
    dirs::document_dir().map(|d| d.join("Logix"))
}

/// Where new projects go: the folder chosen in Settings, or else the
/// built-in one.
pub fn default_projects_root() -> Option<PathBuf> {
    let builtin = builtin_projects_root()?;
    let chosen = crate::settings::default_path()
        .ok()
        .map(|p| crate::settings::load(&p).preferences(&builtin).projects_dir);
    Some(chosen.unwrap_or(builtin))
}

/// Where the Save As dialog starts: next to the project's own folder
/// (`project`, the folder holding it), else the projects folder (`root`),
/// such as for a project never saved, even one copied from an example
/// (never next to the example); never inside a Logix project, since a
/// project can't be saved there.
pub fn save_as_start(project: Option<&Path>, root: &Path) -> PathBuf {
    let start = project
        .and_then(Path::parent)
        .filter(|dir| dir.is_dir())
        .unwrap_or(root);
    // Step out of any project: to the folder that holds it, until no
    // project is around.
    let mut dir = start.to_path_buf();
    loop {
        let project = if is_project_dir(&dir) {
            Some(dir.clone())
        } else {
            enclosing_project(&dir)
        };
        match project.as_deref().and_then(Path::parent) {
            Some(outside) => dir = outside.to_path_buf(),
            None => return dir,
        }
    }
}

/// A name for a new project in `dir` that nothing there has yet: `base`,
/// else "`base` 2", "`base` 3"..., so accepting the suggestion never
/// replaces anything.
pub fn unique_name(dir: &Path, base: &str) -> String {
    if !dir.join(base).exists() {
        return base.to_string();
    }
    (2..)
        .map(|n| format!("{base} {n}"))
        .find(|name| !dir.join(name).exists())
        .unwrap_or_else(|| base.to_string())
}

/// Strips a `.v` the user may have typed in a "Save As" name.
pub fn normalize_new_project_path(path: &Path) -> PathBuf {
    match path.extension() {
        Some(ext) if ext.eq_ignore_ascii_case("v") => path.with_extension(""),
        _ => path.to_path_buf(),
    }
}

/// A plain file name: no folders, not hidden, not reserved.
fn check_plain_name(name: &str) -> Result<(), String> {
    if name.contains(['/', '\\', ':']) || name == "." || name == ".." {
        return Err("A file name can't contain / \\ or :.".into());
    }
    if name.starts_with('.') {
        return Err("A file name can't start with a dot.".into());
    }
    if RESERVED_NAMES.iter().any(|r| r.eq_ignore_ascii_case(name)) {
        return Err(format!("\"{name}\" is reserved for Logix."));
    }
    Ok(())
}

/// Checks that a name sent by the UI is a plain file name (no folders).
pub(crate) fn check_plain_file_name(name: &str) -> Result<()> {
    check_plain_name(name).map_err(anyhow::Error::msg)
}

/// Validates the files sent by the UI before writing anything.
pub(crate) fn check_files(files: &[ProjectFile]) -> Result<()> {
    if !files.iter().any(|f| MAIN_FILES.contains(&f.name.as_str())) {
        bail!("The project has no main.v or main.sv.");
    }
    for (i, f) in files.iter().enumerate() {
        check_plain_name(&f.name).map_err(anyhow::Error::msg)?;
        if files[..i]
            .iter()
            .any(|g| g.name.eq_ignore_ascii_case(&f.name))
        {
            bail!("The project has two files named \"{}\".", f.name);
        }
    }
    Ok(())
}

/// The project file, or an empty one if there's none yet.
/// The board in a project folder's project file, without loading the
/// project. `None` if there's no project file, no board or it can't be read.
pub fn board_of(dir: &Path) -> Option<String> {
    read_settings(dir)
        .ok()?
        .get("board")?
        .as_str()
        .map(String::from)
}

fn read_settings(dir: &Path) -> Result<toml_edit::DocumentMut> {
    let path = dir.join(SETTINGS_FILE);
    if !path.exists() {
        return Ok(toml_edit::DocumentMut::new());
    }
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("can't read {}", path.display()))?;
    text.parse()
        .with_context(|| format!("{} isn't valid TOML", path.display()))
}

/// Writes via a temporary file and rename, so a crash never leaves a
/// half-written file behind.
pub(crate) fn write_atomic(path: &Path, data: &[u8]) -> Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".logix-tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, data).with_context(|| format!("can't write {}", tmp.display()))?;
    std::fs::rename(&tmp, path).with_context(|| format!("can't write {}", path.display()))
}

/// What `copy_project_contents` copies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CopyMode {
    /// Everything (Save As: the user's files).
    All,
    /// For a build: everything but a project nested in a subfolder (a
    /// folder with its own project file), which would be copied into every
    /// build (and, for Save As inside the source, into itself). The engine
    /// builds only the files Logix lists, so other files can go along.
    Build,
}

/// Copies a project folder's top-level entries, except hidden ones and the
/// names in `skip`.
pub(crate) fn copy_project_contents(
    source: &Path,
    target: &Path,
    skip: &[&str],
    mode: CopyMode,
) -> Result<()> {
    for entry in
        std::fs::read_dir(source).with_context(|| format!("can't read {}", source.display()))?
    {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') || skip.contains(&name_str.as_ref()) {
            continue;
        }
        copy_recursive(&entry.path(), &target.join(&name), mode)?;
    }
    Ok(())
}

fn copy_recursive(from: &Path, to: &Path, mode: CopyMode) -> Result<()> {
    if from.is_dir() {
        if mode == CopyMode::Build && is_project_dir(from) {
            return Ok(());
        }
        std::fs::create_dir_all(to)?;
        for entry in std::fs::read_dir(from)? {
            let entry = entry?;
            copy_recursive(&entry.path(), &to.join(entry.file_name()), mode)?;
        }
    } else {
        std::fs::copy(from, to)
            .with_context(|| format!("can't copy {} to {}", from.display(), to.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NO_VARIANT: &Variant = &Variant::new();

    fn file(name: &str, text: &str) -> ProjectFile {
        ProjectFile {
            name: name.into(),
            text: text.into(),
        }
    }

    fn write(dir: &Path, name: &str, text: &str) {
        let path = dir.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn create_save_and_load_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("my blinky");
        let files = [
            file("main.v", "module main; endmodule\n"),
            file("pinout.pcf", "pins"),
        ];
        create(&dir, None, &files, &[], Some("alhambra-ii"), NO_VARIANT).unwrap();

        let p = load(&dir).unwrap();
        assert_eq!(p.name, "my blinky");
        assert_eq!(p.files, files);
        assert_eq!(p.board.as_deref(), Some("alhambra-ii"));

        save(&dir, &[file("main.v", "changed\n")], &[], None, NO_VARIANT).unwrap();
        let p = load(&dir).unwrap();
        assert_eq!(p.files[0].text, "changed\n");
        assert_eq!(p.board, None);
        assert!(!dir.join("main.v.logix-tmp").exists());
    }

    #[test]
    fn load_lists_main_first_then_sources_by_name() {
        let tmp = tempfile::tempdir().unwrap();
        for name in [
            "Zeta.v",
            "main.v",
            "alpha.pcf",
            "notes.txt",
            ".hidden.v",
            "settings.ini",
        ] {
            write(tmp.path(), name, "x");
        }
        write(tmp.path(), "sub/inner.v", "x");
        let names: Vec<_> = load(tmp.path())
            .unwrap()
            .files
            .into_iter()
            .map(|f| f.name)
            .collect();
        assert_eq!(names, ["main.v", "alpha.pcf", "Zeta.v"]);
    }

    #[test]
    fn accepts_main_sv() {
        let tmp = tempfile::tempdir().unwrap();
        for name in ["main.sv", "util.sv"] {
            write(tmp.path(), name, "x");
        }
        let names: Vec<_> = load(tmp.path())
            .unwrap()
            .files
            .into_iter()
            .map(|f| f.name)
            .collect();
        assert_eq!(names, ["main.sv", "util.sv"]);
        assert!(save(tmp.path(), &[file("main.sv", "y")], &[], None, NO_VARIANT).is_ok());
    }

    #[test]
    fn load_skips_binary_files() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "main.v", "x");
        std::fs::write(tmp.path().join("blob.v"), [0xff, 0xfe, 0x00]).unwrap();
        assert_eq!(load(tmp.path()).unwrap().files.len(), 1);
    }

    #[test]
    fn load_rejects_folder_without_main() {
        let tmp = tempfile::tempdir().unwrap();
        let err = load(tmp.path()).unwrap_err();
        assert!(err.to_string().contains("isn't a Logix project"));
    }

    #[test]
    fn save_applies_renames_and_deletes() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        for name in ["main.v", "old.v", "gone.pcf", "keep.txt"] {
            write(dir, name, "x");
        }
        // old.v renamed to new.v, gone.pcf deleted.
        let files = [file("main.v", "m"), file("new.v", "n")];
        save(
            dir,
            &files,
            &["old.v".into(), "gone.pcf".into()],
            None,
            NO_VARIANT,
        )
        .unwrap();
        assert!(!dir.join("old.v").exists());
        assert!(!dir.join("gone.pcf").exists());
        assert_eq!(std::fs::read_to_string(dir.join("new.v")).unwrap(), "n");
        assert!(dir.join("keep.txt").exists());
    }

    #[test]
    fn save_handles_case_only_rename() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "main.v", "x");
        write(tmp.path(), "Foo.v", "x");
        save(
            tmp.path(),
            &[file("main.v", "m"), file("foo.v", "f")],
            &["Foo.v".into()],
            None,
            NO_VARIANT,
        )
        .unwrap();
        let names: Vec<_> = load(tmp.path())
            .unwrap()
            .files
            .into_iter()
            .map(|f| f.name)
            .collect();
        assert_eq!(names, ["main.v", "foo.v"]);
    }

    #[test]
    fn save_rejects_unsafe_names() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "main.v", "x");
        let err = save(
            tmp.path(),
            &[file("main.v", "m"), file("../x.v", "x")],
            &[],
            None,
            NO_VARIANT,
        );
        assert!(err.is_err());
        let err = save(
            tmp.path(),
            &[file("main.v", "m")],
            &["../../etc".into()],
            None,
            NO_VARIANT,
        );
        assert!(err.is_err());
        assert!(save(tmp.path(), &[file("other.v", "x")], &[], None, NO_VARIANT).is_err());
    }

    #[test]
    fn save_keeps_comments_and_unknown_settings() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), MAIN_FILE, "x");
        let text = "# My notes.\nboard = \"a\" # the old board\n\n[future]\nx = 1\n";
        write(tmp.path(), SETTINGS_FILE, text);
        save(
            tmp.path(),
            &[file("main.v", "y")],
            &[],
            Some("b"),
            NO_VARIANT,
        )
        .unwrap();
        let saved = std::fs::read_to_string(tmp.path().join(SETTINGS_FILE)).unwrap();
        assert!(saved.starts_with("# My notes.\nboard = \"b\""), "{saved}");
        assert!(saved.contains("[future]\nx = 1"), "{saved}");
        assert_eq!(load(tmp.path()).unwrap().board.as_deref(), Some("b"));
    }

    #[test]
    fn saves_and_loads_the_variant() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), MAIN_FILE, "x");
        let ram = Variant::from([("memory".to_string(), "ram".to_string())]);
        save(
            tmp.path(),
            &[file("main.v", "y")],
            &[],
            Some("icesugar-pro"),
            &ram,
        )
        .unwrap();
        let saved = std::fs::read_to_string(tmp.path().join(SETTINGS_FILE)).unwrap();
        assert!(
            saved.ends_with("board = \"icesugar-pro\"\n\n[variant]\nmemory = \"ram\"\n"),
            "{saved}"
        );
        assert_eq!(load(tmp.path()).unwrap().variant, ram);
        // The board's default: no [variant] table.
        save(
            tmp.path(),
            &[file("main.v", "y")],
            &[],
            Some("icesugar-pro"),
            NO_VARIANT,
        )
        .unwrap();
        let saved = std::fs::read_to_string(tmp.path().join(SETTINGS_FILE)).unwrap();
        assert!(!saved.contains("variant"), "{saved}");
        assert!(load(tmp.path()).unwrap().variant.is_empty());
    }

    #[test]
    fn a_new_project_file_has_a_header() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), MAIN_FILE, "x");
        save(
            tmp.path(),
            &[file("main.v", "y")],
            &[],
            Some("alhambra-ii"),
            NO_VARIANT,
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(tmp.path().join(SETTINGS_FILE)).unwrap(),
            "# Logix IDE project settings.\nboard = \"alhambra-ii\"\n"
        );
    }

    #[test]
    fn replaces_only_projects_and_empty_folders() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("blinky");
        let files = [file("main.v", "module main; endmodule")];
        create(&project, None, &files, &[], Some("b"), NO_VARIANT).unwrap();
        assert!(check_replace(&project, None).is_ok());
        let empty = tmp.path().join("empty");
        std::fs::create_dir(&empty).unwrap();
        write(&empty, ".DS_Store", "x");
        assert!(check_replace(&empty, None).is_ok());
        let other = tmp.path().join("Documents");
        std::fs::create_dir(&other).unwrap();
        write(&other, "letter.txt", "hi");
        assert!(check_replace(&other, None).is_err());
        write(tmp.path(), "file.v", "x");
        assert!(check_replace(&tmp.path().join("file.v"), None).is_err());
        // Never the project being saved, or a folder that holds it.
        assert!(check_replace(&project, Some(&project)).is_err());
        let parent = tmp.path().join("work");
        create(&parent, None, &files, &[], None, NO_VARIANT).unwrap();
        let inner = parent.join("data");
        std::fs::create_dir(&inner).unwrap();
        assert!(check_replace(&parent, Some(&inner)).is_err());
    }

    #[test]
    fn save_as_suggests_a_free_name_outside_any_project() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("Logix");
        let files = [file("main.v", "m")];
        create(&root.join("untitled"), None, &files, &[], None, NO_VARIANT).unwrap();
        create(
            &root.join("untitled copy"),
            None,
            &files,
            &[],
            None,
            NO_VARIANT,
        )
        .unwrap();

        // A new project: the projects folder, with a name nothing has yet.
        assert_eq!(save_as_start(None, &root), root);
        assert_eq!(unique_name(&root, "untitled"), "untitled 2");
        assert_eq!(unique_name(&root, "untitled copy"), "untitled copy 2");
        assert_eq!(unique_name(&root, "blinky"), "blinky");

        // A saved project: next to it.
        let work = tmp.path().join("work");
        create(&work.join("pll"), None, &files, &[], None, NO_VARIANT).unwrap();
        assert_eq!(save_as_start(Some(&work.join("pll")), &root), work);

        // Never inside a project: a project folder in another project's
        // folder (a projects folder set inside a project) steps out.
        let outer = tmp.path().join("outer");
        create(&outer, None, &files, &[], None, NO_VARIANT).unwrap();
        let inner = outer.join("sub");
        std::fs::create_dir(&inner).unwrap();
        assert_eq!(save_as_start(None, &inner), tmp.path());
        assert_eq!(save_as_start(None, &outer), tmp.path());
    }

    #[test]
    fn save_as_onto_its_own_folder_saves_in_place() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("untitled copy");
        let files = [file("main.v", "old"), file("old.v", "x")];
        create(&project, None, &files, &[], Some("b"), NO_VARIANT).unwrap();
        write(&project, "data.hex", "00");
        std::fs::create_dir(project.join("docs")).unwrap();
        write(&project.join("docs"), "notes.md", "hi");

        // The same folder: saved as Save does, never trashed; other files stay.
        let edited = [file("main.v", "new")];
        save_as(
            &project,
            Some(&project),
            &edited,
            &["old.v".to_string()],
            Some("b"),
            NO_VARIANT,
            |_| panic!("the project's own folder must not be trashed"),
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(project.join("main.v")).unwrap(),
            "new"
        );
        assert!(!project.join("old.v").exists(), "removed in the IDE");
        assert!(project.join("data.hex").exists());
        assert!(project.join("docs/notes.md").exists());

        // Another project folder: replaced (trashed first), with the source's
        // other files copied.
        let other = tmp.path().join("blinky");
        create(
            &other,
            None,
            &[file("main.v", "theirs")],
            &[],
            None,
            NO_VARIANT,
        )
        .unwrap();
        let mut trashed = None;
        save_as(
            &other,
            Some(&project),
            &edited,
            &[],
            Some("b"),
            NO_VARIANT,
            |old| {
                trashed = Some(old.to_path_buf());
                std::fs::remove_dir_all(old)?;
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(trashed.as_deref(), Some(other.as_path()));
        assert_eq!(
            std::fs::read_to_string(other.join("main.v")).unwrap(),
            "new"
        );
        assert!(other.join("data.hex").exists());

        // A folder holding the project is still refused.
        assert!(save_as(
            tmp.path(),
            Some(&project),
            &edited,
            &[],
            None,
            NO_VARIANT,
            |_| Ok(())
        )
        .is_err());
    }

    #[test]
    fn save_as_refuses_a_location_inside_a_project() {
        let tmp = tempfile::tempdir().unwrap();
        let outer = tmp.path().join("blinky");
        let files = [file("main.v", "module main; endmodule")];
        create(&outer, None, &files, &[], None, NO_VARIANT).unwrap();

        // Inside the project being saved, or inside any other project.
        let err = create(
            &outer.join("blinky"),
            Some(&outer),
            &files,
            &[],
            None,
            NO_VARIANT,
        )
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("inside the Logix project \"blinky\""),
            "{err}"
        );
        let deep = outer.join("sub").join("copy");
        assert!(create(&deep, None, &files, &[], None, NO_VARIANT).is_err());
        assert!(!outer.join("blinky").exists());

        // Next to it is fine.
        create(
            &tmp.path().join("copy"),
            Some(&outer),
            &files,
            &[],
            None,
            NO_VARIANT,
        )
        .unwrap();
    }

    #[test]
    fn save_as_copies_other_contents_but_not_generated_or_removed() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        for (name, data) in [
            ("main.v", "old"),
            ("pinout.pcf", "pins"),
            ("removed.v", "r"),
            ("logix-ide.toml", "# mine\nboard = \"x\"\n"),
            (".DS_Store", "x"),
            ("_build/hardware.bin", "bin"),
            ("docs/readme.txt", "doc"),
        ] {
            write(&src, name, data);
        }

        let dst = tmp.path().join("copy");
        let files = [file("main.v", "new"), file("pinout.pcf", "edited")];
        create(
            &dst,
            Some(&src),
            &files,
            &["removed.v".into()],
            None,
            NO_VARIANT,
        )
        .unwrap();

        assert_eq!(std::fs::read_to_string(dst.join("main.v")).unwrap(), "new");
        assert_eq!(
            std::fs::read_to_string(dst.join("pinout.pcf")).unwrap(),
            "edited"
        );
        assert_eq!(
            std::fs::read_to_string(dst.join("docs/readme.txt")).unwrap(),
            "doc"
        );
        for gone in ["removed.v", ".DS_Store", "_build"] {
            assert!(!dst.join(gone).exists(), "{gone} should not be copied");
        }
    }

    #[test]
    fn create_refuses_existing_target() {
        let tmp = tempfile::tempdir().unwrap();
        let err = create(
            tmp.path(),
            None,
            &[ProjectFile {
                name: MAIN_FILE.into(),
                text: "module main;\nendmodule\n".into(),
            }],
            &[],
            None,
            NO_VARIANT,
        )
        .unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn checks_new_file_names() {
        let existing = vec!["main.v".to_string(), "pinout.pcf".to_string()];
        assert!(check_new_file_name("counter.v", &existing).is_ok());
        assert!(check_new_file_name("Pins.LPF", &existing).is_ok());
        assert!(check_new_file_name("", &existing).is_err());
        assert!(check_new_file_name("MAIN.V", &existing).is_err());
        assert!(check_new_file_name("notes.txt", &existing).is_err());
        assert!(check_new_file_name("a/b.v", &existing).is_err());
        assert!(check_new_file_name(".x.v", &existing).is_err());
        assert!(check_new_file_name("my file.v", &existing).is_err());
        assert!(check_new_file_name("main.sv", &existing).is_err());
    }

    #[test]
    fn checks_verilog_identifiers() {
        assert!(is_verilog_identifier("main"));
        assert!(is_verilog_identifier("_x$1"));
        assert!(!is_verilog_identifier("1x"));
        assert!(!is_verilog_identifier("-t"));
        assert!(!is_verilog_identifier("a b"));
        assert!(!is_verilog_identifier(""));
    }

    #[test]
    fn detects_hdl_sources() {
        assert!(is_hdl_source("main.v"));
        assert!(is_hdl_source("defs.SVH"));
        assert!(!is_hdl_source("pinout.pcf"));
        assert!(!is_hdl_source("notes"));
    }

    #[test]
    fn detects_testbenches() {
        assert!(is_testbench("main_tb.v"));
        assert!(is_testbench("Alu_TB.sv"));
        assert!(!is_testbench("main.v"));
        assert!(!is_testbench("tb.v"));
        assert!(!is_testbench("main_tb.vh"));
    }

    #[test]
    fn strips_typed_v_extension() {
        assert_eq!(
            normalize_new_project_path(Path::new("/a/blinky.v")),
            PathBuf::from("/a/blinky")
        );
        assert_eq!(
            normalize_new_project_path(Path::new("/a/blinky")),
            PathBuf::from("/a/blinky")
        );
    }

    #[test]
    fn reads_only_the_board() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(board_of(tmp.path()), None);
        std::fs::write(
            tmp.path().join(SETTINGS_FILE),
            "# mine\nboard = \"upduino2\"\n",
        )
        .unwrap();
        assert_eq!(board_of(tmp.path()).as_deref(), Some("upduino2"));
        std::fs::write(tmp.path().join(SETTINGS_FILE), "not = [toml").unwrap();
        assert_eq!(board_of(tmp.path()), None);
        assert_eq!(board_of(&tmp.path().join("missing")), None);
    }

    #[test]
    fn the_custom_boards_file_is_a_tab() {
        assert!(is_editable("custom-boards.json"));
        assert!(!is_editable("other.json"));
        assert!(check_new_file_name("custom-boards.json", &[]).is_ok());
        let e = check_new_file_name("boards.json", &[]).unwrap_err();
        assert!(e.contains("custom-boards.json"), "{e}");
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("main.v"), "m").unwrap();
        std::fs::write(tmp.path().join("custom-boards.json"), "[]").unwrap();
        std::fs::write(tmp.path().join("data.json"), "{}").unwrap();
        let names: Vec<_> = load(tmp.path())
            .unwrap()
            .files
            .into_iter()
            .map(|f| f.name)
            .collect();
        assert_eq!(names, ["main.v", "custom-boards.json"]);
    }
}
