# Developing Logix IDE

This page is for people who want to change Logix IDE itself. To *use* Logix IDE, see
the [README](README.md).

## How Logix IDE is developed

Logix IDE is developed **entirely by prompting**
[Claude Code](https://claude.com/claude-code), an AI coding assistant. The
developer describes what Logix IDE should do, in plain language, and Claude
writes and changes the code, tests and documentation, runs the tests, and
reports back. No code in this repository was written by hand.

So the code isn't meant to be read or edited by people, much like the
machine code a compiler makes from a high-level language. The "source" that
matters is:

- **What Logix IDE should do and why:** the **AI spec**, [`CLAUDE.md`](CLAUDE.md). It holds the
  goals, requirements, architecture, design decisions and open items, and
  Claude reads it at the start of every session. When the design changes,
  the AI spec is updated with it. It's meant to be complete enough that an
  AI agent could regenerate the whole repo from it (code, tests, workflows,
  scripts and docs), and it never holds personal information.
- **What Logix IDE must keep doing:** the tests (`scripts/test.sh`). They are how a
  change is checked, rather than by reading the code.
- **What users see:** the user documentation in [`docs/`](docs/).

To change Logix IDE, describe the change to Claude Code in this repository: what
you want, not how to code it. Then try it with `scripts/run.sh` and check that
`scripts/test.sh` passes. Editing the code by hand works too, but the next prompt
may change it again without knowing why it was edited; it's better to put
the reason in the AI spec or in the prompt.

## Setting up a development machine

Development happens on a **Mac with Apple Silicon**, the only platform
Logix IDE supports so far.

1. **Xcode Command Line Tools** (compilers, git):

   ```
   xcode-select --install
   ```

2. **[Homebrew](https://brew.sh)**, then Node.js and Rust:

   ```
   brew install node rustup
   /opt/homebrew/opt/rustup/bin/rustup default stable
   ```

   Homebrew's `rustup` isn't on the `PATH` by default (it's "keg-only").
   The scripts below add `/opt/homebrew/opt/rustup/bin` themselves; for your
   own shell, run `export PATH=/opt/homebrew/opt/rustup/bin:$PATH`.

3. **The code:**

   ```
   git clone https://github.com/openlogix/logix-ide.git
   cd logix-ide
   ```

4. **WebKit for the end-to-end UI tests** (once per machine):

   ```
   npx playwright install webkit
   ```

5. **[Claude Code](https://claude.com/claude-code)**, to make changes as
   described above. Start it in the repository folder (or open the folder
   in VS Code with the Claude Code extension).

That's all. The JavaScript packages are installed by the scripts when
needed, and Logix IDE downloads Logix Engine and the FPGA tools on its first
run, as it does for users (into `~/.logix/ide`, about 3 GB).

Optional, only for previewing the documentation site: Python 3.

## Where things are

| Folder | What's in it |
| :--- | :--- |
| `ui/` | The window's user interface (TypeScript, Svelte), and its tests |
| `src-tauri/` | The backend (Rust): runs Logix Engine, files, menus |
| `examples/` | The examples Logix IDE offers, one folder per board and example |
| `scripts/` | `run.sh`, `test.sh`, `clean.sh` |
| `docs/` | The user documentation site |
| `.github/` | The GitHub workflows (tests, releases, docs) |

## Everyday tasks

Run these from the repository folder (the `scripts/` ones work from anywhere).

| Task | Command |
| :--- | :--- |
| Run Logix IDE (with live reload) | `scripts/run.sh` |
| Run Logix IDE with a locally built engine | `scripts/run.sh --engine-repo-dir <your logix-engine clone>` |
| Run all checks and tests | `scripts/test.sh` |
| Delete all build output and Logix IDE's data (`~/.logix/ide`) | `scripts/clean.sh` |
| Type-check the UI | `npm run check` |
| UI unit tests | `npm test` |
| UI end-to-end tests | `npm run test:e2e` |
| One end-to-end test | `npx playwright test -g "Auto Format"` |
| Backend (Rust) tests | `cd src-tauri && cargo test` |
| Refresh the documentation screenshots | `npm run docs:screenshots` |

- **`scripts/run.sh`** starts Logix IDE in development mode. Changes to the UI show up
  in the window right away; changes to the Rust backend rebuild and restart
  it by themselves. It also reinstalls the JavaScript packages when they
  changed.
- **`scripts/run.sh --engine-repo-dir <folder>`** first builds Logix Engine
  in your local `logix-engine` clone, then runs Logix IDE with it instead of the
  released engine, for working on both together. Its tools go to
  `~/.logix/ide/dev/engine`, apart from the released engine's (the first run
  downloads them). Restart the script after changing the engine.
- **Any engine binary, by hand**: `scripts/run.sh --engine-repo-dir` just sets
  `LOGIX_DEV_ENGINE_BINARY`, the path to a `logix-engine` executable. You can
  set it yourself instead, such as for an engine built elsewhere:
  `LOGIX_DEV_ENGINE_BINARY=/path/to/logix-engine scripts/run.sh`. The
  real-engine Rust tests (`cargo test -- --ignored`) use the same variable.
- **A local engine with an installed Logix IDE** (for diagnosing a downloaded
  Logix IDE, or guiding a user through it): quit Logix IDE, then start it with
  the variable set:
  `open -a "Logix IDE" --env LOGIX_DEV_ENGINE_BINARY=/path/to/logix-engine`.
  A banner shows the local engine for the whole session; start Logix IDE
  normally to go back to the regular engine. This is a diagnostic feature,
  for Open Logix developers and testing, not for normal use. It's the Open Logix
  convention for every frontend.
- **`scripts/test.sh`** runs everything CI runs: the UI type check, unit tests, a
  production build and the end-to-end tests, then the Rust format check,
  lint and tests. It stops at the first failure. Run it before committing.
- **`scripts/clean.sh`** starts from zero, including Logix IDE's downloaded tools and
  settings. Use it to try the first-run experience; it isn't needed
  otherwise.

### The end-to-end tests

They run the real UI in WebKit (the browser engine of the macOS window)
against a **fake backend** (`ui/e2e/fakeBackend.ts`) that replays real
Logix Engine output, so they need no FPGA tools or board. The documentation screenshots
are made the same way.

### The log

Logix IDE writes a log to `~/.logix/ide/logs/logix-ide-latest.log` (older runs: `logix-ide-previous-1.log` to `-4`) (and the dev terminal shows
the same `[logix] ...` lines): versions, errors, and what the output pane
showed. Logix Engine keeps its own log in `~/.logix/ide/engine/logs/`. In the app: **Help > Show Log
Folder**.

### Moving to a new Logix Engine release

Logix IDE has no built-in engine pin. It uses the engine release its remote
config names, in the version folder `remote-config/logix-ide/001/` of this
repo:

- `channels.jsonc` lists which Logix IDE releases are on which channel. A
  release listed under `prod` is a prod release; every other release is on
  `dev`, and so are local builds (such as `scripts/run.sh`).
- `<channel>.jsonc`, such as `dev.jsonc` or `prod.jsonc`, names what that
  channel uses. A channel without its own file uses `prod.jsonc`.

```jsonc
"logix-engine": {
  "repo": "openlogix/logix-engine",
  "release": "2026-11-12",
  "file": "logix-engine-${PLATFORM}-${YYYYMMDD}.tgz"
}
```

Every Logix IDE reads its channel's file from `main` (cached for 24 hours),
so a change there reaches installed Logix IDEs on that channel without a new
release. Change it by pull request, with a comment saying why; CI checks
that the release and its file exist (and that `prod.jsonc` names only stable
releases), and runs the tests once per config file (a job per channel). A pre-release a config
names is kept, so dev pins don't expire.

Two settings change this on your own computer only:

- `LOGIX_IDE_CHANNEL` picks the channel, such as
  `LOGIX_IDE_CHANNEL=prod scripts/run.sh` to try what users get.
- `LOGIX_IDE_DEV_REMOTE_CONFIG` reads the version folder from elsewhere: a
  local folder or an `https://` URL (such as the folder on your pull
  request's branch):

```
LOGIX_IDE_DEV_REMOTE_CONFIG=$PWD/remote-config/logix-ide/001 scripts/run.sh
```

Logix IDE then shows a banner for the whole session. With
`LOGIX_IDE_DEV_REMOTE_CONFIG`, it keeps everything that folder decides (the
engine and its FPGA tools) in `~/.logix/ide/dev/config-override/`, so the
regular setup isn't touched. Help > Check for Updates reads the config again
now.

### Examples

The examples are in `examples/<board>/<name>/`, each a Logix IDE project with
a `logix-ide.toml` holding its board and description. They're Logix IDE's own,
maintained here: edit them in place.

Logix IDE doesn't read this folder: it downloads the `logix-ide-examples`
package that its remote config names. Each pre-release packs this folder
into one (`scripts/pack-examples.py`), so an edited example reaches users
once it's in a pre-release and the remote config names that release. The
tests (`cargo test`, `real_examples_build`) read this folder directly.

### Previewing the documentation site

```
python3 -m venv .venv-docs
.venv-docs/bin/pip install -r docs/requirements.txt
.venv-docs/bin/mkdocs serve
```

Then open http://127.0.0.1:8000.

## Releases and CI

Installers are **never built on a developer machine**; GitHub Actions does
it from the public code (see [`.github/workflows`](.github/workflows)):

- **test.yaml**: runs `scripts/test.sh` on every push and pull request, and daily.
- **build-pre-release.yaml**: builds the installer, scans it for viruses
  and publishes it as a dated pre-release, on every push to `main`, daily,
  and on demand.
- **publish-docs.yaml**: publishes the documentation site, on every push to
  `main`, daily, and on demand.

The daily runs catch breakage from changes outside this repository (GitHub's
runners, tool versions) even when nothing here changed.

In a fork, the workflows publish to the fork's own releases and GitHub
Pages.

## Working conventions

These are the house rules for changes, whether you prompt Claude or not:

- **Keep the AI spec and the docs in sync** with every change: a new
  feature, menu item, shortcut or message updates the matching page, and a
  new decision goes into the AI spec with its reason.
- **Pull requests are rules or prompts**, ideally: a change to the AI spec
  or the prompt to run, rather than hand-written code, so the code can be
  generated from it. If a pull request includes code, it also updates
  the AI spec to explain it, and its description gives the prompts used.
- **Commits are made by the developer**, after reviewing the change and
  running `scripts/test.sh`. Claude doesn't commit unless asked.
- **Keep Logix IDE simple**, like the Arduino IDE: ease of use for beginners
  comes before power-user features. Its motto is **The Arduino experience, for FPGAs.**
- **Logix Engine does the FPGA work**; Logix IDE is a front end. When Logix IDE
  needs something from the engine, prefer changing the engine over working
  around it.
- **Don't program a connected board** or stop other running programs
  without asking.

## License

Logix IDE is licensed under [MIT](LICENSE-MIT) or the [Apache License 2.0](LICENSE-APACHE), at your option.
