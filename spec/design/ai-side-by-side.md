# Logix IDE and an AI agent, side by side

**Status:** approved by the user, 2026-11-20 (reviewed as a Claude Doc);
every question below is decided. Built in the order of the Plan.

## Goal

A user works on a project in Logix IDE while an AI agent, such as Claude
Code, changes the same project: it edits files, adds, renames and deletes
them, builds, tests and programs the board. Both must always see the same
project, with no extra steps for the user. This comes before the thin
launcher (the user's decision, 2026-11-20).

Today it doesn't work: Logix IDE keeps edits and file changes in memory
until Save and never notices changes on disk, so Upload can build the old
text in its tabs while the agent's new text is on disk, and a later Save
overwrites the agent's work.

## Principles

- **No questions about the agent's changes.** Logix IDE never asks the user
  to accept a change made on disk, and certainly not for a file without
  unsaved edits (the user's rule, 2026-11-20). Changes just appear.
- **Nothing is lost.** When both changed a file at the same moment, the
  user can still get their own text back.
- **A small project stays simple.** Nothing new shows for a user who never
  uses an agent.

## 1. From Logix IDE to disk: auto save

As in Arduino IDE 2 (Auto save, on by default) and VS Code's
`files.autoSave`:

- **Edits** are saved about a second after the user stops typing, and at
  once when the window loses focus (the user switches to the agent).
- **New, renamed and deleted files** and **board changes** (`logix-ide.toml`)
  are written at once.
- **Projects not saved yet** (a new project, an example just opened) have no
  folder of their own, so nothing is saved until their first Save, as in
  Arduino IDE.
- **The "●" on tabs and the title** shows only during the second before a
  save, so in practice it disappears. In a project not saved yet, it stays
  until the project's first Save. Save stays (⌘S saves at once).
- **Builds** keep building from Logix IDE's files, which now match the disk.

What changes for users: "Don't Save" on closing no longer throws away
changes, since there are none left to throw away. Question 1 is whether to
keep a way back to today's behavior.

## 2. From disk to Logix IDE: watching the project folder

- **How:** the operating system's file notifications on the project folder
  (Rust's `notify` crate: FSEvents on macOS, ReadDirectoryChangesW on
  Windows, inotify on Linux), not polling, so a change shows within a
  moment. Events are gathered for a short moment (about 100 ms), since one
  save is often several events.
- **Ignored:** folders starting with `_` (generated, such as an agent's
  `_build`), hidden files and folders (`.git`), and Logix IDE's own saves
  (recognized by their contents: a file whose text equals what Logix IDE
  holds is not a change).
- **A changed file** reloads in its tab, keeping the cursor and scroll
  position as far as the text allows. The reload is an ordinary change in
  the editor's undo history, so ⌘Z goes back past it.
- **A file with unsaved edits** in Logix IDE that changed on disk (rare with
  auto save: only while typing in the same file as the agent writes it):
  the disk wins, silently; ⌘Z brings back the user's text; the status bar
  says so in one line ("adder.v was changed on disk; ⌘Z brings back your
  edits."). No dialog.
- **New files** become tabs; **deleted files** lose their tabs (with their
  problem marks and Recent entries); a **rename** on disk arrives as a
  delete and a new file, which gives the same result. The shown file, if
  deleted, switches to `main.v`.
- **`logix-ide.toml`** changed on disk (another board, a variant, the
  serial baud rate): Logix IDE follows it, as if chosen in its menus.
- **The project folder itself** deleted or moved: the project stays open
  in memory, unsaved, and the status bar says so; Save As saves it
  elsewhere.
- **Problems** (Verify's marks) for a reloaded file are cleared, as for an
  edit; live checking rechecks it.

## 3. The agent and the board

Both may use the board: Logix IDE's Upload and Serial Monitor, and the
agent's `program_board` and serial tools. The operating system lets one
program open a serial port at a time, and a Logix Engine pauses only its
own serial sessions while it programs the board. So with two engines,
Logix IDE's Serial Monitor can make the agent's upload fail, and the other
way round. Question 2 covers the ways out.

## 4. What the agent knows about Logix IDE projects

A Logix IDE project follows Logix IDE's model: `main.v` with the top
module `main`, `*_tb.v` testbenches, one pin file of the board's kind,
`logix-ide.toml` with the board, a flat folder, `_` folders generated.
Logix Engine's instructions describe only its tools and a default project
model for agents working alone, which differs (the board in `logix.json`,
the build in `_build` inside the project), so an agent may add a
`logix.json` next to `logix-ide.toml`. Question 3 covers how the agent
learns the right model.

## Questions, decided

1. **Auto save: a setting, or always on?**
   - (a) A Settings switch, on by default, as in Arduino IDE 2; off keeps
     today's behavior (changes in memory until Save; folder watching still
     on, reloading files without unsaved edits).
   - (b) Always on, no switch: one behavior to test and document.
   - *Recommendation:* (a). It costs little, and some users (and
     classrooms) expect Save and Don't Save to mean something.
   - **Decided:** (a), a per-user setting in Settings (`settings.json`),
     not per project.

2. **Which engine does the agent use?**
   - (a) Its own Logix Engine, installed and set up as the engine tutorial
     says (its own engine folder, `~/.logix/claude-code/`). Works today.
     Costs: a second download of the FPGA tools, possibly another engine
     release (other yosys and nextpnr: other warnings or numbers), and two
     programs using one board (section 3).
   - (b) Logix IDE's engine binary, run by the agent. Not workable as is:
     its path changes with each release, and hosts never share an engine
     folder, so the tools would still be downloaded twice.
   - (c) Logix IDE offers its running engine to the agent, as an MCP server
     the agent connects to (such as a local socket or a small bridge
     program that Logix IDE's docs tell the agent to start). One engine,
     one set of tools, one release, and one owner of the board, which can
     pause the Serial Monitor for the agent's upload. It could also carry
     Logix IDE's own tools: the project's model (question 3), the shown
     file, the selected board and device.
   - *Recommendation:* (a) now, documented; (c) as the goal, designed after
     sync works, since it touches the engine's process model and
     Logix IDE's lifetime (the agent's tools stop when Logix IDE closes).
   - **Decided:** (a) for now; (c) stays the goal.

3. **How does the agent learn Logix IDE's project model?**
   - (a) A guide file in the project for agents (`CLAUDE.md`, read by
     Claude Code; `AGENTS.md` for others), written by a command such as
     Tools > Add AI Agent Guide, so beginners' projects stay clean. The
     user can edit it with their own rules.
   - (b) Through Logix IDE's MCP server (question 2c): instructions and a
     `project_info` tool. Needs 2c.
   - (c) A docs page, "Using Logix IDE with an AI agent", with a short text
     to paste into the agent.
   - **Decided** (revised after the user's comment): (c) and (a) now, with
     (a) as a small file: a few lines saying this is a Logix IDE project,
     and a link to the docs page for the model, so the text lives in one
     place. The link names the Logix IDE release, so an older project
     keeps its own model; an agent without web access still gets the few
     lines. (b) later, with 2c. And ask Logix Engine to say, in its
     instructions, that an agent working in a frontend's project follows
     that frontend's model, not the default one.

4. **Conflicts: is "the disk wins, ⌘Z brings your text back" right?** The
   alternative is "Logix IDE wins" (its next save overwrites the agent's
   change), which loses the agent's work silently. **Decided:** the disk
   wins.

5. **Builds by both at once.** The agent's builds go to its own build
   folder (with its own engine, `_build` in the project, which Logix IDE
   ignores); Logix IDE's to its scratch folder. Nothing to do now; with 2c,
   one engine could also share build results. **Decided:** nothing now.

## Plan

1. Auto save (section 1), with its setting if question 1 says so.
2. Folder watching and reloads (section 2), tested with Claude Code editing
   a project while Logix IDE is open (`~/work-large`).
3. The docs page and the guide file (question 3).
4. The engine sharing design (question 2c), as its own design note.
