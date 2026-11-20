# Using Logix IDE with an AI agent

You can keep a project open in Logix IDE while an AI agent, such as Claude
Code, works on the same project: it edits files, adds and deletes them,
builds, tests and programs the board, and you see its changes as it makes
them.

## Working side by side

- **Your changes reach the agent.** Logix IDE saves a saved project's
  changes by itself, a second after you stop typing and as soon as you
  switch to another app, so the agent always reads what you see (Settings,
  **Save changes automatically**, on by default).
- **The agent's changes reach you.** When the agent changes the project
  folder, Logix IDE shows it at once, without asking: changed files reload
  in their tabs, new files get tabs, deleted files lose theirs, and a board
  the agent chooses in `logix-ide.toml` shows in the toolbar. **Undo** goes
  back past a change, so if you were typing in the same file at that
  moment, your text comes back.
- **One board, one user at a time.** Logix IDE and the agent each have
  their own Logix Engine, and only one program at a time can open a board's
  serial port. Before the agent uploads to the board or reads its serial
  port, choose **Disconnect** in Logix IDE's Serial Monitor, and don't
  upload from both at once.

## Setting up the agent

The agent needs its own Logix Engine, which gives it the FPGA tools:
follow [Logix Engine's tutorial](https://www.openlogix.org/logix-engine/tutorial-claude-code/)
to set it up for Claude Code. Then start the agent in the project folder.

## Telling the agent about the project

Choose **Tools > Add AI Agent Guide** with the project open. It adds two
small files to the project folder:

- **`AGENTS.md`**, read by many agents: it says the folder is a Logix IDE
  project, gives the main rules below in a few lines, and links to this
  page, as it was for your version of Logix IDE.
- **`CLAUDE.md`**, read by Claude Code: one line that includes `AGENTS.md`.

Files that already exist are left as they are. You can add your own rules
to them.

## The project model, for AI agents

A Logix IDE project follows these rules. They differ from Logix Engine's
default project model (which keeps the board in `logix.json`): in a Logix
IDE project, follow these.

- **One folder** holds the project, with every file at its top level.
  Logix IDE uses only the top level; other files and folders are kept and
  go along to builds.
- **`main.v`** holds the top module, `main` (or **`main.sv`** for
  SystemVerilog). It's always there and is never renamed.
- **Other source files** (`.v`, `.sv`) and include files (`.vh`, `.svh`)
  sit next to it.
- **Testbenches** are named `<name>_tb.v` (or `_tb.sv`), next to the code
  they test, such as `adder_tb.v` for `adder.v`. They report failures with
  `$error` and end with `$finish`.
- **One pin file** in the board's format: `pinout.pcf` (iCE40),
  `pinout.lpf` (ECP5), `pinout.cst` (Gowin) or `pinout.xdc` (Xilinx). Logix
  Engine's `get_catalog` gives the board's FPGA.
- **`logix-ide.toml`** holds the project's settings: `board` (a board ID),
  `[variant]` (how the board is connected and programmed) and `[serial]`
  (the Serial Monitor's speed). Change only what you need, and keep its
  other lines. Don't add a `logix.json`.
- **GTKWave layouts** are named `<testbench>.gtkw`, next to the testbench,
  such as `main_tb.gtkw`.
- **Folders starting with `_` are generated**, such as `_build`: build
  into `_build` inside the project, and never put sources there.
- **File names** have no spaces.

When building with Logix Engine, send the project folder as
`source_folder`, `main` as `top_module`, the project's source files
without the testbenches as `source_files`, the pin file as
`constraint_file`, and the board and variant from `logix-ide.toml`.
