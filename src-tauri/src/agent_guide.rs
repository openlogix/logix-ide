//! The AI agent guide: small files in every new project's folder that
//! tell an AI agent working beside Logix IDE (such as Claude Code) that
//! this is a Logix IDE project, the model's main rules in a few lines, and
//! where the full model is (the user's decisions, 2026-11-20;
//! `spec/design/ai-side-by-side.md`, question 3). `AGENTS.md`, which many
//! agents read, and `CLAUDE.md`, which Claude Code reads, importing it.
//! Written when Logix IDE creates a project's folder (`project::create`),
//! only if missing; the examples don't carry them.
//!
//! The full model is `docs/ai-agents.md`, linked at the commit this Logix
//! IDE was built from: commits on `main` stay, while pre-release tags are
//! deleted after a while, so an older project keeps its own version's model.

use std::path::Path;

use anyhow::{Context, Result};

/// The guide's file, read by many agents.
pub const AGENTS_FILE: &str = "AGENTS.md";
/// Claude Code's file: it imports the guide.
pub const CLAUDE_FILE: &str = "CLAUDE.md";

/// Where the full project model is: the docs page at `commit` (`main` when
/// built outside a git checkout).
pub fn model_link(commit: &str) -> String {
    let at = if commit.is_empty() { "main" } else { commit };
    format!("https://github.com/openlogix/logix-ide/blob/{at}/docs/ai-agents.md")
}

/// The guide's text.
pub fn agents_text(link: &str) -> String {
    format!(
        "# AI agent guide

This folder is a Logix IDE project. Logix IDE may have it open: it saves its
own changes within a second and shows yours at once, so work on the files
directly.

Follow Logix IDE's project model, not Logix Engine's default one (no
logix.json):

- `main.v` holds the top module, `main`; other source files sit next to it.
- Testbenches are named `<name>_tb.v`, next to the code they test.
- One pin file, `pinout` with the board's extension (.pcf, .lpf, .cst or .xdc).
- `logix-ide.toml` holds the board (`board = \"<board ID>\"`); keep its other
  lines.
- Folders starting with `_` are generated: build into `_build`.

Write designs that start every register at 0 and rest at 0 (some FPGAs, such
as the iCE40, can only start a register at 0), never read past the end of a
constant or memory, and give testbench variables a starting value, so no
signal shows as x in a simulation.

The full model: {link}
"
    )
}

/// Claude Code's file: an import of the guide.
pub const CLAUDE_TEXT: &str = "@AGENTS.md\n";

/// Writes the guide into the project folder `dir`, leaving files that
/// already exist alone. Returns the files written.
pub fn add(dir: &Path, commit: &str) -> Result<Vec<&'static str>> {
    let mut written = Vec::new();
    for (name, text) in [
        (AGENTS_FILE, agents_text(&model_link(commit))),
        (CLAUDE_FILE, CLAUDE_TEXT.to_string()),
    ] {
        let path = dir.join(name);
        if path.exists() {
            continue;
        }
        crate::project::write_atomic(&path, text.as_bytes())
            .with_context(|| format!("can't write {}", path.display()))?;
        written.push(name);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_the_model_at_the_build_commit() {
        assert_eq!(
            model_link("abc1234"),
            "https://github.com/openlogix/logix-ide/blob/abc1234/docs/ai-agents.md"
        );
        assert!(model_link("").contains("/blob/main/"));
    }

    #[test]
    fn writes_the_guide_once_and_keeps_existing_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        std::fs::write(dir.join(CLAUDE_FILE), "my own rules\n").unwrap();
        assert_eq!(add(dir, "abc1234").unwrap(), [AGENTS_FILE]);
        let guide = std::fs::read_to_string(dir.join(AGENTS_FILE)).unwrap();
        assert!(guide.contains("blob/abc1234/docs/ai-agents.md"), "{guide}");
        assert_eq!(
            std::fs::read_to_string(dir.join(CLAUDE_FILE)).unwrap(),
            "my own rules\n"
        );
        assert!(add(dir, "abc1234").unwrap().is_empty());
    }
}
