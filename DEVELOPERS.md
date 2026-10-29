# Developing Logix

This page is for people who want to change Logix itself. To *use* Logix, see
the [README](README.md).

## How Logix is developed

Logix is developed **entirely by prompting**
[Claude Code](https://claude.com/claude-code), an AI coding assistant. The
developer describes what Logix should do, in plain language, and Claude
writes and changes the code, tests and documentation, runs the tests, and
reports back. No code in this repository was written by hand.

So the code isn't meant to be read or edited by people, much like the
machine code a compiler makes from a high-level language. The "source" that
matters is:

- **What Logix should do and why:** [`CLAUDE.md`](CLAUDE.md). It holds the
  goals, requirements, architecture, design decisions and open items, and
  Claude reads it at the start of every session. When the design changes,
  `CLAUDE.md` is updated with it. It's meant to be complete enough that an
  AI agent could regenerate the whole repo from it (code, tests, workflows,
  scripts and docs), and it never holds personal information.
- **What Logix must keep doing:** the tests (`scripts/test.sh`). They are how a
  change is checked, rather than by reading the code.
- **What users see:** the user documentation in [`docs/`](docs/).

To change Logix, describe the change to Claude Code in this repository: what
you want, not how to code it. Then try it with `scripts/run.sh` and check that
`scripts/test.sh` passes. Editing the code by hand works too, but the next prompt
may change it again without knowing why it was edited; it's better to put
the reason in `CLAUDE.md` or in the prompt.

## Setting up a development machine

Development happens on a **Mac with Apple Silicon**, the only platform
Logix supports so far.

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
   cd logix
   ```

4. **WebKit for the end-to-end UI tests** (once per machine):

   ```
   npx playwright install webkit
   ```

5. **[Claude Code](https://claude.com/claude-code)**, to make changes as
   described above. Start it in the repository folder (or open the folder
   in VS Code with the Claude Code extension).

That's all. The JavaScript packages are installed by the scripts when
needed, and Logix downloads Apio and the FPGA tools on its first run, as it
does for users (into `~/.logix`, about 3 GB).

Optional, only for previewing the documentation site: Python 3.

## Where things are

| Folder | What's in it |
| :--- | :--- |
| `ui/` | The window's user interface (TypeScript, Svelte), and its tests |
| `src-tauri/` | The backend (Rust): runs Apio, files, menus |
| `scripts/` | `run.sh`, `test.sh`, `clean.sh` |
| `docs/` | The user documentation site |
| `.github/` | The GitHub workflows (tests, releases, docs) |

## Everyday tasks

Run these from the repository folder (the `scripts/` ones work from anywhere).

| Task | Command |
| :--- | :--- |
| Run Logix (with live reload) | `scripts/run.sh` |
| Run all checks and tests | `scripts/test.sh` |
| Delete all build output and Logix's data (`~/.logix`) | `scripts/clean.sh` |
| Type-check the UI | `npm run check` |
| UI unit tests | `npm test` |
| UI end-to-end tests | `npm run test:e2e` |
| One end-to-end test | `npx playwright test -g "Auto Format"` |
| Backend (Rust) tests | `cd src-tauri && cargo test` |
| Refresh the documentation screenshots | `npm run docs:screenshots` |

- **`scripts/run.sh`** starts Logix in development mode. Changes to the UI show up
  in the window right away; changes to the Rust backend rebuild and restart
  it by themselves. It also reinstalls the JavaScript packages when they
  changed.
- **`scripts/test.sh`** runs everything CI runs: the UI type check, unit tests, a
  production build and the end-to-end tests, then the Rust format check,
  lint and tests. It stops at the first failure. Run it before committing.
- **`scripts/clean.sh`** starts from zero, including Logix's downloaded tools and
  settings. Use it to try the first-run experience; it isn't needed
  otherwise.

### The end-to-end tests

They run the real UI in WebKit (the browser engine of the macOS window)
against a **fake backend** (`ui/e2e/fakeBackend.ts`) that replays real Apio
output, so they need no FPGA tools or board. The documentation screenshots
are made the same way.

### The log

Logix writes a log to `~/.logix/logs/logix.log` (and the dev terminal shows
the same `[logix] ...` lines): versions, every Apio command and its result,
errors, and what the output pane showed. In the app: **Help > Show Log
Folder**.

### Moving to a new Apio release

Logix uses a fixed Apio release, set in `package.json`, as in the Apio
VS Code extension:

```json
"appConstants": {
  "apioCliReleaseRepo": "fpgawars/apio",
  "apioCliReleaseTag": "2026-10-22"
}
```

Change the tag to a newer (non-pre-release) Apio release and restart
`scripts/run.sh`: Logix downloads that Apio on its next start.

### Using a local Apio

To try Logix with your own Apio (e.g. when changing Apio itself), set
`LOGIX_APIO_BIN` to its executable before `scripts/run.sh`:

```
LOGIX_APIO_BIN=/path/to/apio scripts/run.sh
```

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

- **Keep `CLAUDE.md` and the docs in sync** with every change: a new
  feature, menu item, shortcut or message updates the matching page, and a
  new decision goes into `CLAUDE.md` with its reason.
- **Pull requests are rules or prompts**, ideally: a change to `CLAUDE.md`
  or the prompt to run, rather than hand-written code, so the code can be
  generated from it. If a pull request includes code, it also updates
  `CLAUDE.md` to explain it, and its description gives the prompts used.
- **Commits are made by the developer**, after reviewing the change and
  running `scripts/test.sh`. Claude doesn't commit unless asked.
- **Keep Logix simple**, like the Arduino IDE: ease of use for beginners
  comes before power-user features. Its motto is **"The Arduino experience, for FPGAs."**
- **Apio does the FPGA work**; Logix is a front end. When Logix needs
  something from Apio, prefer asking for an Apio change over working
  around it.
- **Don't program a connected board** or stop other running programs
  without asking.

## License

Logix is licensed under the [Apache License 2.0](LICENSE).
