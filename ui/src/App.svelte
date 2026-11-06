<script lang="ts">
  import { onMount, tick, untrack } from "svelte";
  import {
    backend,
    toUserError,
    withReason,
    type AppInfo,
    type Board,
    type ConnectedDevice,
    type Example,
    type Devices,
    type UsbDevice,
    type Diagnostic,
    type Preferences,
    type MenuId,
    type MenuState,
    type ProjectDoc,
    type Task,
    type SetupEvent,
    type SetupStage,
    type Variant,
  } from "./lib/backend";
  import Editor from "./lib/Editor.svelte";
  import { setHoverEnabled, startLanguageServer } from "./lib/lsp";
  import type { LSPClient } from "@codemirror/lsp-client";
  import ExamplesDialog from "./lib/ExamplesDialog.svelte";
  import BoardPicker from "./lib/BoardPicker.svelte";
  import { withProjectBoards } from "./lib/boards";
  import VariantPanel from "./lib/VariantPanel.svelte";
  import {
    choose as chooseVariant,
    isConnected as variantConnected,
    menus as variantMenus,
    parseMenuId,
    summary as variantSummary,
    toStore,
  } from "./lib/variants";
  import DiagramDialog from "./lib/DiagramDialog.svelte";
  import DevicesDialog from "./lib/DevicesDialog.svelte";
  import BoardInfoDialog from "./lib/BoardInfoDialog.svelte";
  import DevicePicker from "./lib/DevicePicker.svelte";
  import PreferencesDialog from "./lib/PreferencesDialog.svelte";
  import { deviceSummaryLines, isBoardNotFound, needsDeviceChoice } from "./lib/devices";
  import Icon from "./lib/Icon.svelte";
  import { pinCommentToken } from "./lib/pinFiles";
  import {
    addFile,
    buildKey,
    copySource,
    deleteFile,
    fileRole,
    roleTooltip,
    filesForSave,
    findFile,
    fromDoc,
    isHdlSource,
    isTestbench,
    isDirty,
    isFileDirty,
    MAIN_FILE,
    mainFile,
    customBoardsText,
    CUSTOM_BOARDS_FILE,
    formatJson,
    markSaved,
    renameFile,
    projectModules,
    starterText,
    testbenches,
    type WorkingProject,
  } from "./lib/workingProject";
  import { appendCapped, formatBytes, progressFraction } from "./lib/format";
  import { addLinks, parseAnsi, type Segment } from "./lib/ansi";
  import { examplesSummary, type ExampleEntry } from "./lib/examples";
  import {
    byFile,
    firstLocated,
    location,
    outcomeText,
    runningText,
    testbenchLines,
  } from "./lib/tasks";
  import { reportLines } from "./lib/buildReport";
  import {
    clampConsoleHeight,
    DEFAULT_CONSOLE_HEIGHT,
    dragHidesConsole,
    outputComesBack,
  } from "./lib/layout";

  const MAX_CONSOLE_LINES = 5000;
  const DIVIDER_HEIGHT = 6;
  const CONSOLE_HEIGHT_KEY = "logix.consoleHeight";

  // Editor/console split. The console height is remembered between runs.
  let splitEl = $state<HTMLElement | null>(null);
  let consoleHeight = $state(loadConsoleHeight());
  let dragging = $state(false);
  /** Dragged near the bottom: the output hides on release (like Hide). */
  let collapsing = $state(false);
  /** The height before the drag, kept for Show after a collapse. */
  let heightBeforeDrag = 0;

  function loadConsoleHeight(): number {
    try {
      const saved = Number(localStorage.getItem(CONSOLE_HEIGHT_KEY));
      return saved > 0 ? saved : DEFAULT_CONSOLE_HEIGHT;
    } catch {
      return DEFAULT_CONSOLE_HEIGHT;
    }
  }

  function saveConsoleHeight() {
    try {
      localStorage.setItem(CONSOLE_HEIGHT_KEY, String(consoleHeight));
    } catch {
      // Not critical; the default is used next time.
    }
  }

  function fitConsoleHeight(requested: number) {
    if (!splitEl) return;
    consoleHeight = clampConsoleHeight(
      requested,
      splitEl.clientHeight,
      DIVIDER_HEIGHT,
    );
  }

  function onDividerDown(e: PointerEvent) {
    dragging = true;
    // From the hidden state the drag starts collapsed.
    collapsing = !consoleVisible;
    heightBeforeDrag = consoleHeight;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    e.preventDefault();
  }

  function onDividerMove(e: PointerEvent) {
    if (!dragging || !splitEl) return;
    const bottom = splitEl.getBoundingClientRect().bottom;
    const requested = bottom - e.clientY - DIVIDER_HEIGHT / 2;
    collapsing = dragHidesConsole(requested);
    if (!collapsing) {
      // Dragged up from the hidden state: the output comes back.
      if (!consoleVisible) setConsoleVisible(true);
      fitConsoleHeight(requested);
    }
  }

  function onDividerUp() {
    if (!dragging) return;
    dragging = false;
    if (collapsing) {
      collapsing = false;
      // Show brings it back at the height it had before this drag.
      consoleHeight = heightBeforeDrag;
      setConsoleVisible(false);
    }
    saveConsoleHeight();
  }

  // ---- Showing and hiding the output (console) ----

  const CONSOLE_VISIBLE_KEY = "logix.consoleVisible";
  let consoleVisible = $state(loadConsoleVisible());

  function loadConsoleVisible(): boolean {
    try {
      return localStorage.getItem(CONSOLE_VISIBLE_KEY) !== "false";
    } catch {
      return true;
    }
  }

  async function setConsoleVisible(visible: boolean) {
    if (visible === consoleVisible) return;
    consoleVisible = visible;
    try {
      localStorage.setItem(CONSOLE_VISIBLE_KEY, String(visible));
    } catch {
      // Not critical; the default is used next time.
    }
    if (visible) {
      // Show the newest output.
      await tick();
      consoleEl?.scrollTo({ top: consoleEl.scrollHeight });
    }
  }

  /**
   * Shows the hidden output by itself when a command starts or something
   * fails, as the "Show the output" setting says.
   */
  function showOutput(reason: "command" | "error") {
    if (outputComesBack(prefs?.showOutput ?? "onCommand", reason)) {
      setConsoleVisible(true);
    }
  }

  // Keep the split valid when the window (or banner) changes size.
  $effect(() => {
    if (!splitEl) return;
    const el = splitEl;
    const observer = new ResizeObserver(() => {
      if (consoleVisible) fitConsoleHeight(consoleHeight);
    });
    observer.observe(el);
    return () => observer.disconnect();
  });

  let info = $state<AppInfo | null>(null);
  /** Console lines, already split into colored segments. */
  let consoleLines = $state<Segment[][]>([]);
  let consoleEl = $state<HTMLElement | null>(null);

  let stage = $state<SetupStage | null>(null);
  let setupError = $state<string | null>(null);
  let download = $state<{ received: number; total: number | null } | null>(
    null,
  );

  /** The engine's package install: current phase and its percentage. */
  let installStep = $state<{ message: string; percent: number | null } | null>(
    null,
  );

  const ready = $derived(stage === "ready");
  const downloadFraction = $derived(
    download
      ? progressFraction(download.received, download.total)
      : installStep?.percent != null
        ? installStep.percent / 100
        : null,
  );

  /** Output lines not yet sent to the log file (sent in batches). */
  let unlogged: string[] = [];

  function sendToLogFile(lines: string[]) {
    if (unlogged.length === 0) {
      setTimeout(() => {
        const batch = unlogged;
        unlogged = [];
        backend.logOutput(batch).catch(() => {
          // Not essential.
        });
      }, 250);
    }
    unlogged.push(...lines);
  }

  async function log(...lines: string[]) {
    sendToLogFile(lines);
    const el = consoleEl;
    const atBottom =
      !el || el.scrollHeight - el.scrollTop - el.clientHeight < 20;
    consoleLines = appendCapped(
      consoleLines,
      lines.map((line) => addLinks(parseAnsi(line))),
      MAX_CONSOLE_LINES,
    );
    if (atBottom) {
      await tick();
      el?.scrollTo({ top: el.scrollHeight });
    }
  }

  /**
   * The local engine in use instead of the released one
   * (LOGIX_DEV_ENGINE_BINARY), shown for the whole session so it's never
   * forgotten while diagnosing.
   */
  let devEngine = $state<string | null>(null);

  function onSetupEvent(event: SetupEvent) {
    switch (event.kind) {
      case "stage":
        stage = event.stage;
        if (event.stage !== "downloadingEngine") download = null;
        if (event.stage !== "installingEnginePackages") installStep = null;
        break;
      case "download":
        download = { received: event.received, total: event.total };
        break;
      case "progress":
        installStep = { message: event.message, percent: event.percent };
        break;
      case "log":
        log(event.line);
        break;
      case "devEngine":
        devEngine = event.path;
        break;
    }
  }

  async function runSetup() {
    setupError = null;
    stage = "checking";
    try {
      await backend.setup(onSetupEvent);
      // The "ready" event comes through another channel and may arrive
      // after this returns; don't wait for it.
      stage = "ready";
      // Boards and examples first, so "Ready." comes last.
      await loadCatalog();
      startLsp();
      log("Ready.");
    } catch (e) {
      const err = toUserError(e);
      setupError = err.message;
      log(`Error: ${err.details}`);
      showOutput("error");
    }
  }

  // ---- Boards and examples ----

  /** Logix Engine's catalog boards. */
  let catalogBoards = $state<Board[]>([]);
  /** The boards chosen most recently, newest first ("Recently used"). */
  let recentBoards = $state<string[]>([]);

  /** Remembers the board the user chose, for new projects and "Recently used". */
  function rememberBoard(id: string, variant: Variant) {
    backend
      .setLastBoard(id, variant)
      .then((recent) => (recentBoards = recent))
      .catch(() => {});
  }
  /** The project's own boards (custom-boards.json), as the engine reads them. */
  let projectBoards = $state<Board[]>([]);
  /** Why the project's custom-boards.json can't be used, if it can't. */
  let projectBoardsError = $state<string | null>(null);
  /** The boards to offer: the project's own first, then the catalog's. */
  const boards = $derived(withProjectBoards(catalogBoards, projectBoards));
  let examples = $state<Example[]>([]);
  let examplesOpen = $state(false);

  async function loadCatalog() {
    try {
      const [boardList, exampleList] = await Promise.all([
        backend.getBoards(),
        backend.getExamples(),
      ]);
      catalogBoards = boardList;
      recentBoards = await backend.recentBoards();
      examples = exampleList.examples;
      log(examplesSummary(exampleList.examples.length, exampleList.hidden));
    } catch (e) {
      showFileError(e);
    }
  }

  // ---- Connected devices (the board list's device groups) ----

  /** The connected USB devices that fit boards, polled while idle. */
  let connectedDevices = $state<ConnectedDevice[]>([]);
  /** Every board a connected device may be (a board may fit several). */
  const connected = $derived(connectedDevices.flatMap((d) => d.boards));
  let scanning = false;

  async function refreshConnected() {
    // Not while a task runs: Upload uses the USB device.
    if (scanning || !ready || isBusy) return;
    scanning = true;
    try {
      const found = await backend.connectedDevices(customBoards);
      if (JSON.stringify(found) !== JSON.stringify(connectedDevices)) connectedDevices = found;
    } catch {
      // Keep the last result; Tools > Connected Devices shows errors.
    } finally {
      scanning = false;
    }
  }

  // As in Arduino IDE 2: notice boards being plugged in and out.
  $effect(() => {
    if (!ready) return;
    untrack(refreshConnected);
    const timer = setInterval(() => {
      if (document.hasFocus()) refreshConnected();
    }, 3000);
    window.addEventListener("focus", refreshConnected);
    return () => {
      clearInterval(timer);
      window.removeEventListener("focus", refreshConnected);
    };
  });

  /** Whether the toolbar's board is connected the way its variant programs it. */
  function isConnected(board: string | null | undefined): boolean {
    if (board == null) return false;
    const info = boards.find((b) => b.id === board) ?? null;
    return variantConnected(info, board, currentVariant, connected);
  }

  /**
   * A board chosen in the toolbar while its examples are offered: applied
   * only if the user keeps the current files (opening an example brings
   * its own board), so the "Save changes?" question is only about files.
   */
  let pendingBoard = $state<string | null>(null);
  /** The recent projects for `pendingBoard`, offered with its examples. */
  let boardRecents = $state<string[]>([]);

  /**
   * Toolbar board change. As in the Arduino IDE, it never changes the
   * files by itself; the board's examples are offered, with "Keep Current
   * Files" as the default.
   */
  function setBoard(id: string) {
    if (!id) return;
    // One of the project's own boards: no examples, and not a board for new
    // projects (they don't have its definition).
    if (project && projectBoards.some((b) => b.id === id)) {
      applyBoard(project, id);
      return;
    }
    // Another board starts with its default variant.
    const variant = id === currentBoard ? currentVariant : {};
    rememberBoard(id, variant);
    // No project (the start screen): it's the board for the next one.
    if (!project) {
      homeBoard = id;
      homeVariant = variant;
      return;
    }
    if (examples.length === 0) {
      applyBoard(project, id);
      return;
    }
    pendingBoard = id;
    boardRecents = [];
    examplesOpen = true;
    // The open project itself isn't offered.
    const current = project.dir;
    backend
      .recentProjectsForBoard(id)
      .then((dirs) => {
        if (pendingBoard === id) boardRecents = dirs.filter((d) => d !== current);
      })
      .catch(() => {});
  }

  /** A recent project picked in the board's examples dialog. */
  function openBoardRecent(dir: string) {
    pendingBoard = null;
    examplesOpen = false;
    openRecent(dir);
  }

  /** Sets the project's board; another board starts with its default variant. */
  function applyBoard(p: WorkingProject, id: string) {
    if (p.board !== id) p.variant = {};
    p.board = id;
  }

  /**
   * Chooses a value of one of the board's dimensions (the toolbar's
   * variant panel, or Tools > <dimension>). If it doesn't go with the
   * other current values, they change too, and the status says so.
   */
  function setVariant(dim: string, value: string) {
    const info = currentBoardInfo;
    if (!info) return;
    const { variant, changed } = chooseVariant(info, currentVariant, dim, value);
    if (project) project.variant = variant;
    else homeVariant = variant;
    rememberBoard(info.id, variant);
    const now = variantSummary(info, variant);
    status = {
      text: changed.length
        ? `${info.description}: ${now} (${changed.join(", ")} changed with it).`
        : `${info.description}: ${now}.`,
      kind: "info",
    };
  }

  function closeExamples(keepFiles: boolean) {
    if (keepFiles && pendingBoard && project) applyBoard(project, pendingBoard);
    pendingBoard = null;
    examplesOpen = false;
  }

  function boardName(id: string | null): string {
    return boards.find((b) => b.id === id)?.description ?? id ?? "none";
  }

  function showExamples() {
    if (examples.length === 0) {
      fileError = "The examples are available once the FPGA tools are ready.";
      return;
    }
    examplesOpen = true;
  }

  function openExample(ex: ExampleEntry) {
    pendingBoard = null;
    examplesOpen = false;
    exclusive(async () => {
      if (!(await confirmLeave())) return;
      try {
        const doc = await during(`Opening example ${ex.name}…`, () =>
          ex.bare
            ? backend.openBareMinimum(ex.board)
            : backend.openExample(ex.board, ex.name),
        );
        loadDoc(doc);
        finish(
          ex.bare
            ? `Opened Bare Minimum for ${boardName(ex.board)}.`
            : `Opened example ${ex.board}/${ex.name}.`,
        );
      } catch (e) {
        commandFailed(e);
      }
    });
  }

  // ---- Verify (build) and upload ----

  let running = $state<Task | null>(null);
  /** The running task's current phase, e.g. "Writing flash" at 42%. */
  let taskStep = $state<{ message: string; percent: number | null } | null>(
    null,
  );
  let status = $state<{ text: string; kind: "info" | "ok" | "error" } | null>(
    null,
  );
  /**
   * A file operation in progress (e.g. "Opening example blinky…"), shown in
   * the status bar with the busy indicator.
   */
  let activity = $state<string | null>(null);
  /** Auto Format is running. */
  let formatting = $state(false);

  /** Shows `label` as the current activity while `work` runs. */
  async function during<T>(label: string, work: () => Promise<T>): Promise<T> {
    activity = label;
    try {
      return await work();
    } finally {
      activity = null;
    }
  }

  /** Whether Logix is doing something (the status bar shows a spinner). */
  const isBusy = $derived(
    activity !== null ||
      running !== null ||
      formatting ||
      (stage !== null && stage !== "ready" && !setupError),
  );

  /** Setup is under way (shown in the status bar, with its progress). */
  const settingUp = $derived(
    stage !== null && stage !== "ready" && !setupError,
  );

  /** Setup's progress detail: bytes downloaded, or the package and its %. */
  const setupDetail = $derived.by(() => {
    if (!settingUp) return "";
    if (download)
      return (
        formatBytes(download.received) +
        (download.total ? ` of ${formatBytes(download.total)}` : "")
      );
    if (installStep?.message)
      return (
        installStep.message +
        (installStep.percent != null ? ` ${installStep.percent}%` : "")
      );
    return "";
  });

  /** The status bar text: what's running, else the last result or Ready. */
  const statusText = $derived.by(() => {
    if (activity) return activity;
    if (stage && stage !== "ready" && !setupError && !running)
      return stageText[stage];
    if (status) return status.text;
    return ready ? "Ready." : "";
  });

  /** Files with build problems, for marking their tabs. */
  let fileProblems = $state(new Map<string, "error" | "warning">());

  /** The problems currently marked in the editor. */
  let shownDiagnostics: Diagnostic[] = [];

  function showDiagnostics(diags: Diagnostic[]) {
    shownDiagnostics = diags;
    const map = byFile(diags);
    editor?.setDiagnostics(map);
    fileProblems = new Map(
      [...map].map(([file, list]) => [
        file,
        list.some((d) => d.severity === "error") ? "error" : "warning",
      ]),
    );
  }

  // ---- Auto Format (Tools menu, Cmd+T, as in the Arduino IDE) ----


  /**
   * Ends a command's output: its result (the status bar's text, green
   * after success, red after an error), then "Ready.".
   */
  function logDone() {
    if (status) {
      const color =
        status.kind === "ok" ? "\x1b[32m" : status.kind === "error" ? "\x1b[31m" : "";
      log(color ? `${color}${status.text}\x1b[0m` : status.text);
    }
    log("Ready.");
  }

  /** Ends a file command (New, Open, Save, ...) that succeeded: `text` in
   * the status bar and the output, green, then "Ready.". */
  function finish(text: string) {
    status = { text, kind: "ok" };
    logDone();
  }

  /** Ends a file command that failed: the error (banner, and in red in
   * the output with its details), then "Ready.". */
  function commandFailed(e: unknown) {
    const err = toUserError(e);
    // The banner says it in red, with the reason; the status line repeats
    // the message without a second red line.
    fileError = withReason(err);
    status = { text: err.message, kind: "info" };
    log(`\x1b[31mError: ${err.details}\x1b[0m`);
    log("Ready.");
  }

  async function autoFormat() {
    if (!project || formatting) return;
    const p = project;
    const file = activeFile;
    const f = findFile(p, file);
    if (!f) return;
    // The project's boards file is JSON: formatted here, by the IDE.
    if (file === CUSTOM_BOARDS_FILE) {
      const formatted = formatJson(f.text);
      if ("error" in formatted) {
        status = { text: `Auto Format failed: ${formatted.error}`, kind: "error" };
      } else if (formatted.text === f.text) {
        status = { text: "No changes necessary for Auto Format.", kind: "ok" };
      } else {
        editor?.replaceText(file, formatted.text);
        status = { text: "Auto Format finished.", kind: "ok" };
      }
      return;
    }
    if (!isHdlSource(file)) {
      status = {
        text: `Auto Format works on Verilog and SystemVerilog files (.v .sv .vh .svh) and ${CUSTOM_BOARDS_FILE}.`,
        kind: "error",
      };
      return;
    }
    if (!ready) {
      fileError = "The FPGA tools aren't ready yet.";
      return;
    }
    const before = f.text;
    formatting = true;
    status = { text: "Formatting…", kind: "info" };
    log(`Formatting ${file}…`);
    try {
      const result = await backend.formatFile(file, before);
      if (project !== p) return; // Another project was opened meanwhile.
      // Old markers in this file no longer match its lines.
      const others = shownDiagnostics.filter((d) => d.file !== file);
      if (!result.ok || result.text === null) {
        showOutput("error");
        showDiagnostics([...others, ...result.diagnostics]);
        const first = firstLocated(result.diagnostics);
        const why = first ?? result.diagnostics[0];
        status = {
          text: why
            ? `Auto Format failed: ${location(why) ? `${location(why)}: ` : ""}${why.message}`
            : "Auto Format failed. See the output below.",
          kind: "error",
        };
        if (first?.file && first.line) editor?.reveal(first.file, first.line);
      } else if (f.text !== before) {
        status = {
          text: "The file changed while formatting. Try Auto Format again.",
          kind: "error",
        };
      } else if (result.text === before) {
        showDiagnostics(others);
        status = { text: "No changes necessary for Auto Format.", kind: "ok" };
      } else {
        editor?.replaceText(file, result.text);
        showDiagnostics(others);
        status = { text: "Auto Format finished.", kind: "ok" };
      }
    } catch (e) {
      const err = toUserError(e);
      status = { text: err.message, kind: "error" };
      log(`Error: ${err.details}`);
    } finally {
      formatting = false;
      if (project === p) logDone();
    }
  }

  const VERB: Record<Task, string> = {
    build: "Building",
    upload: "Uploading",
    test: "Testing",
    simulate: "Simulating",
    graph: "Drawing the diagram of",
  };

  // ---- Live errors while typing (Verible's language server) ----

  let lsp = $state<LSPClient | null>(null);
  let lspStarting = false;
  /** Whether live checking is on (Settings) and the FPGA tools are ready. */
  const lspWanted = () => ready && prefs?.liveChecking !== false;

  async function startLsp() {
    if (lsp || lspStarting || !lspWanted()) return;
    lspStarting = true;
    try {
      lsp = await startLanguageServer();
    } catch (e) {
      // Not essential: Verify still finds errors.
      log(`Verible live error checking is off: ${toUserError(e).details}`);
    } finally {
      lspStarting = false;
    }
    // Turned off in Settings while it was starting.
    if (!lspWanted()) stopLsp();
  }

  async function stopLsp() {
    const old = lsp;
    lsp = null;
    old?.disconnect();
    try {
      await backend.lspStop();
    } catch {
      // Already stopped.
    }
  }

  function onLspClosed() {
    const old = lsp;
    lsp = null;
    old?.disconnect();
    // Stopped on purpose (Settings), or never wanted: nothing to do.
    if (!old || !lspWanted()) return;
    log("The Verible live error checker stopped; restarting it.");
    setTimeout(startLsp, 2000);
  }

  // ---- Settings (Cmd+,) ----

  let prefs = $state<Preferences | null>(null);
  let settingsOpen = $state(false);

  // Whether the OS is in dark mode (for the "System" theme).
  const darkQuery = window.matchMedia("(prefers-color-scheme: dark)");
  let systemDark = $state(darkQuery.matches);
  $effect(() => {
    const onChange = () => (systemDark = darkQuery.matches);
    darkQuery.addEventListener("change", onChange);
    return () => darkQuery.removeEventListener("change", onChange);
  });

  const dark = $derived(
    prefs?.theme === "dark" || (prefs?.theme !== "light" && systemDark),
  );

  /** Applies the preferences the UI handles (the backend does the rest). */
  function applyPrefs(p: Preferences) {
    const root = document.documentElement;
    root.dataset.theme = p.theme;
    root.style.setProperty("--editor-font-size", `${p.editorFontSize}px`);
    setHoverEnabled(p.hoverInfo);
    // Live checking on or off (it starts once the FPGA tools are ready).
    if (p.liveChecking) startLsp();
    else if (lsp) stopLsp();
  }

  // Start the live checker once it's wanted: the tools just became ready,
  // or it was turned on in Settings. (Only those trigger this; restarts
  // after a crash go through `onLspClosed`, with a delay.)
  $effect(() => {
    if (lspWanted()) untrack(startLsp);
  });

  async function loadPrefs() {
    try {
      prefs = await backend.getPreferences();
      applyPrefs(prefs);
    } catch (e) {
      log(`Couldn't read the settings: ${toUserError(e).details}`);
    }
  }

  async function savePrefs(p: Preferences): Promise<string | null> {
    try {
      prefs = await backend.setPreferences(p);
      applyPrefs(prefs);
      return null;
    } catch (e) {
      return toUserError(e).message;
    }
  }

  // ---- Connected devices (Tools > Connected Devices) ----

  let devicesOpen = $state(false);
  /** Tools > Board Info… (or the ⓘ next to the board), while it's open. */
  let boardInfoOpen = $state(false);
  let devices = $state<Devices | null>(null);
  let devicesError = $state<string | null>(null);
  let devicesBusy = $state(false);

  async function refreshDevices() {
    devicesBusy = true;
    try {
      devices = await backend.scanDevices(customBoards);
      devicesError = null;
    } catch (e) {
      devicesError = toUserError(e).message;
    } finally {
      devicesBusy = false;
    }
  }

  function showDevices() {
    devicesOpen = true;
    refreshDevices();
  }

  /** After Upload couldn't find the board: say what is connected. */
  async function logConnectedDevices() {
    try {
      log("", ...deviceSummaryLines(await backend.scanDevices(customBoards)));
    } catch {
      // Only a hint; the upload error is already shown.
    }
  }

  /** Upload's device picker, while several devices fit the board. */
  let devicePicker = $state<{ board: string; devices: UsbDevice[] } | null>(null);

  function chooseUploadDevice(id: string) {
    devicePicker = null;
    runTask("upload", null, id);
  }

  /** The diagram window's content, while it's open. */
  let diagram = $state<{ svg: string; module: string } | null>(null);

  /** Draws `module` (main if null) and shows it in the diagram window. */
  function showDiagram(module: string | null) {
    runTask("graph", module === "main" ? null : module);
  }

  /** The testbench to simulate: the open one, or the project's only one. */
  function simulationTestbench(p: WorkingProject): string | null {
    if (isTestbench(activeFile)) return activeFile;
    const all = testbenches(p);
    return all.length === 1 ? all[0] : null;
  }

  /**
   * Runs a build, upload, test or simulation. `testbench` picks a single
   * testbench for "test" and "simulate". `device` is the device Upload
   * uses, once the user chose one (null: the only one that fits).
   */
  async function runTask(
    task: Task,
    testbench: string | null = null,
    device: string | null = null,
  ) {
    if (!project || running) return;
    if (!ready) {
      fileError = "The FPGA tools aren't ready yet.";
      return;
    }
    if (!project.board) {
      fileError = "Select a board first (in the toolbar).";
      return;
    }
    const p = project;
    const board = project.board;
    const boardInfo = boards.find((b) => b.id === board) ?? null;
    const variant = boardInfo ? toStore(boardInfo, p.variant) : p.variant;
    if (task === "test" && testbenches(p).length === 0) {
      fileError =
        "This project has no testbenches. Testbench file names end with _tb.v or _tb.sv.";
      return;
    }
    if (task === "simulate") {
      testbench ??= simulationTestbench(p);
      if (!testbench) {
        fileError =
          "Open the testbench you want to simulate, then choose Simulate again.";
        return;
      }
    }
    fileError = null;
    running = task;
    taskStep = null;
    status = { text: runningText(task), kind: "info" };
    consoleLines = [];
    showOutput("command");
    const which = testbench ? ` (${testbench})` : "";
    const summary = boardInfo ? variantSummary(boardInfo, variant) : "";
    log(
      `${VERB[task]} ${p.name}${which} for ${boardName(board)}${summary ? ` (${summary})` : ""}…`,
    );
    try {
      const outcome = await backend.runTask(
        task,
        testbench,
        buildKey(p),
        copySource(p),
        p.dir,
        filesForSave(p),
        board,
        variant,
        task === "upload" ? device : null,
        (event) => {
          if (event.kind === "line") log(event.line);
          else taskStep = { message: event.message, percent: event.percent };
        },
      );
      taskStep = null;
      if (project !== p) return; // Another project was opened meanwhile.
      if (task === "test") log("", ...testbenchLines(outcome));
      if (outcome.ok && outcome.report) log("", ...reportLines(outcome.report));
      if (
        task === "upload" &&
        !outcome.ok &&
        !outcome.cancelled &&
        isBoardNotFound(outcome)
      ) {
        await logConnectedDevices();
      }
      if (task === "upload" && needsDeviceChoice(outcome)) {
        // Not a failure: Upload goes on once the user picks a device.
        devicePicker = { board, devices: outcome.devices };
        status = { text: "Choose the device to upload to.", kind: "info" };
        return;
      }
      if (task === "graph" && outcome.ok && outcome.graphSvg) {
        diagram = { svg: outcome.graphSvg, module: testbench ?? "main" };
      }
      showDiagnostics(outcome.diagnostics);
      if (!outcome.ok && !outcome.cancelled) showOutput("error");
      status = {
        text: outcomeText(task, outcome),
        kind: outcome.ok ? "ok" : outcome.cancelled ? "info" : "error",
      };
      const first =
        outcome.ok || outcome.cancelled
          ? null
          : firstLocated(outcome.diagnostics);
      if (first?.file && first.line && findFile(p, first.file)) {
        activeFile = first.file;
        editor?.reveal(first.file, first.line);
      }
    } catch (e) {
      const err = toUserError(e);
      status = { text: err.message, kind: "error" };
      log(`Error: ${err.details}`);
      showOutput("error");
    } finally {
      running = null;
      taskStep = null;
      if (project === p) logDone();
    }
  }

  // ---- Project and editor ----

  let editor = $state<Editor | null>(null);
  let project = $state<WorkingProject | null>(null);
  let activeFile = $state(MAIN_FILE);
  /** Short error from a file operation, shown in a dismissible banner. */
  let fileError = $state<string | null>(null);
  /** Set while a file operation (with its dialogs) is in progress. */
  let busy = false;

  const dirty = $derived(project !== null && isDirty(project));

  /**
   * The text of the project's custom-boards.json (unsaved edits included),
   * sent as it is with every engine call that deals with boards, errors and
   * all: Logix Engine reports its problems at line and column. Null when
   * the project has none.
   */
  const customBoards = $derived(project ? customBoardsText(project) : null);

  /** Shows the custom boards file's problems as its markers, or clears them. */
  function markCustomBoards(diagnostics: Diagnostic[]) {
    const others = shownDiagnostics.filter((d) => d.file !== CUSTOM_BOARDS_FILE);
    if (others.length !== shownDiagnostics.length || diagnostics.length > 0) {
      showDiagnostics([...others, ...diagnostics]);
    }
  }

  // The project's own boards, as Logix Engine reads them, for the board
  // list; a file with problems is shown in a banner and as markers, and its
  // boards are left out until it's fixed.
  $effect(() => {
    const text = customBoards;
    if (!ready) return;
    if (text === null) {
      projectBoards = [];
      projectBoardsError = null;
      untrack(() => markCustomBoards([]));
      return;
    }
    // Wait for typing to pause.
    const timer = setTimeout(() => {
      backend
        .projectBoards(text)
        .then((found) => {
          if (customBoards !== text) return;
          projectBoards = found.boards;
          projectBoardsError = found.problem?.message ?? null;
          markCustomBoards(found.problem?.diagnostics ?? []);
        })
        .catch((e) => {
          if (customBoards !== text) return;
          projectBoards = [];
          projectBoardsError = toUserError(e).message;
        });
    }, 400);
    return () => clearTimeout(timer);
  });
  /**
   * No project open (first start, or after Close Project): the editor area
   * shows the start screen. The toolbar still has a board, `homeBoard`
   * (the last one chosen), which New Project and the examples use.
   */
  const welcome = $derived(project === null);
  let homeBoard = $state<string | null>(null);
  /** The variant of `homeBoard` ({} for its default). */
  let homeVariant = $state<Variant>({});
  /** The board in the toolbar: the project's, or else `homeBoard`. */
  const currentBoard = $derived(project ? project.board : homeBoard);
  /** The toolbar board's catalog entry (null if the engine doesn't list it). */
  const currentBoardInfo = $derived(boards.find((b) => b.id === currentBoard) ?? null);
  /** The toolbar board's variant, as stored ({} for its default). */
  const currentVariant = $derived<Variant>(project ? project.variant : homeVariant);
  /** The current board's pin file extension, e.g. "pcf" (null if unknown). */
  const pinExtension = $derived(
    boards.find((b) => b.id === currentBoard)?.pinExtension ?? null,
  );
  /** The start screen's recent projects (folders). */
  let recents = $state<string[]>([]);
  /** Whether Verify and Upload can be started now. */
  const canRun = $derived(
    ready && project !== null && running === null && activity === null,
  );

  /**
   * What the menus can act on now: Auto Format only in Verilog tabs (pin
   * files have no formatter), Comment/Uncomment in Verilog and pin files
   * (each with its line comment), Run Tests with testbenches, Simulate
   * with one to run.
   */
  const menuState = $derived<MenuState>({
    project: project !== null,
    formattable:
      project !== null && (isHdlSource(activeFile) || activeFile === CUSTOM_BOARDS_FILE),
    comment:
      project !== null && (isHdlSource(activeFile) || pinCommentToken(activeFile) !== null),
    definition: project !== null && lsp !== null && isHdlSource(activeFile),
    testbenches: project !== null && testbenches(project).length > 0,
    simulate: project !== null && simulationTestbench(project) !== null,
    testbenchOpen: project !== null && isTestbench(activeFile),
    board: currentBoardInfo !== null,
    variantMenus: variantMenus(currentBoardInfo, currentVariant),
  });

  $effect(() => {
    backend.setMenuState(menuState);
  });

  $effect(() => {
    // An empty name: no project, the title is just "Logix IDE".
    if (project) backend.setDocumentState(project.name, dirty);
    else backend.setDocumentState("", false);
  });

  /** Reloads the start screen's recent projects. */
  function loadRecents() {
    backend
      .recentProjects()
      .then((r) => (recents = r))
      .catch(() => {});
  }

  $effect(() => {
    if (project === null) loadRecents();
  });

  function showFileError(e: unknown) {
    const err = toUserError(e);
    fileError = withReason(err);
    log(`Error: ${err.details}`);
  }

  function loadDoc(doc: ProjectDoc) {
    project = fromDoc(doc);
    const main = mainFile(project);
    activeFile = main;
    fileProblems = new Map();
    fileError = null;
    cancelNaming();
    editor?.reset(doc.files, main);
    editor?.focus();
  }

  function onEdit(file: string, text: string) {
    const f = project && findFile(project, file);
    if (f) f.text = text;
  }

  function selectFile(name: string) {
    activeFile = name;
    editor?.show(name);
    editor?.focus();
  }

  // ---- New file / rename: an inline name box in the tab bar ----

  type Naming = { mode: "new" } | { mode: "rename"; file: string };
  let naming = $state<Naming | null>(null);
  let nameInput = $state("");
  let nameError = $state<string | null>(null);

  function startNewFile() {
    if (!project) return;
    naming = { mode: "new" };
    nameInput = "";
    nameError = null;
  }

  function startRename(file: string) {
    if (!project || file === mainFile(project)) return;
    naming = { mode: "rename", file };
    nameInput = file;
    nameError = null;
  }

  function cancelNaming() {
    naming = null;
    nameError = null;
  }

  async function commitNaming() {
    const current = naming;
    if (!project || !current) return;
    const name = nameInput.trim();
    if (current.mode === "rename" && name === current.file) {
      cancelNaming();
      return;
    }
    const others = project.files
      .map((f) => f.name)
      .filter((n) => current.mode !== "rename" || n !== current.file);
    const problem = await backend.checkFileName(name, others);
    if (problem) {
      nameError = problem;
      return;
    }
    if (current.mode === "new") {
      const text = starterText(name);
      addFile(project, name, text);
      editor?.add(name, text);
      cancelNaming();
      selectFile(name);
    } else {
      renameFile(project, current.file, name);
      editor?.rename(current.file, name);
      if (activeFile === current.file) activeFile = name;
      cancelNaming();
      editor?.focus();
    }
  }

  function onNameKey(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      commitNaming();
    } else if (e.key === "Escape") {
      cancelNaming();
      editor?.focus();
    }
  }

  async function askDeleteFile(file: string) {
    if (!project || file === mainFile(project)) return;
    const ok = await backend.askConfirm(
      `Delete "${file}"?`,
      "The file is removed from the project folder when you save.",
      "Delete",
    );
    if (!ok || !project) return;
    deleteFile(project, file);
    editor?.remove(file);
    if (activeFile === file) selectFile(mainFile(project));
  }

  // Right-click menu on a tab.
  let tabMenu = $state<{ file: string; x: number; y: number } | null>(null);

  function openTabMenu(e: MouseEvent, file: string) {
    e.preventDefault();
    tabMenu = { file, x: e.clientX, y: e.clientY };
  }

  function fromTabMenu(action: (file: string) => void) {
    const file = tabMenu?.file;
    tabMenu = null;
    if (file) action(file);
  }

  // ---- Project file operations ----

  /** Handles unsaved changes before they'd be lost. False to stay put. */
  async function confirmLeave(): Promise<boolean> {
    if (!project || !dirty) return true;
    const choice = await backend.askSaveChanges(project.name);
    if (choice === "cancel") return false;
    if (choice === "save") return save();
    return true;
  }

  /** The variant the toolbar had for `board` before a project replaced it. */
  function currentVariantOf(board: string | null): Variant {
    return board !== null && board === homeBoard ? homeVariant : {};
  }

  /** File > New Project > Empty Project: the board's pins and an empty
   * design (its Bare Minimum). */
  async function newProject() {
    const board = currentBoard;
    if (!(await confirmLeave())) return;
    try {
      loadDoc(await during("Creating a new project…", () => backend.newProject(board)));
      // It starts with the toolbar's variant, as with its board.
      if (project && project.board === board) {
        project.variant = { ...currentVariantOf(board) };
        project.savedVariant = { ...project.variant };
      }
      finish("New project.");
    } catch (e) {
      commandFailed(e);
    }
  }

  /** File > Close Project: back to the start screen (asks to save first). */
  async function closeProject() {
    if (!project || !(await confirmLeave())) return;
    if (project.board) {
      homeBoard = project.board;
      homeVariant = project.variant;
    }
    project = null;
    fileProblems = new Map();
    fileError = null;
    cancelNaming();
    await backend.closeProject();
    finish("Closed the project.");
  }

  /** File > Open Recent. */
  function openRecent(dir: string) {
    exclusive(async () => {
      if (!(await confirmLeave())) return;
      try {
        const doc = await during("Opening the project…", () =>
          backend.openProjectAt(dir),
        );
        loadDoc(doc);
        finish(`Opened ${doc.dir}.`);
      } catch (e) {
        commandFailed(e);
      }
    });
  }

  /** Project > Show Project Folder (in Finder). */
  async function showProjectFolder() {
    if (!project?.dir) {
      status = {
        text: "This project hasn't been saved yet, so it has no folder.",
        kind: "info",
      };
      return;
    }
    try {
      await backend.showProjectFolder(project.dir);
    } catch (e) {
      showFileError(e);
    }
  }

  /** Stops the running build, upload, test or simulation. */
  async function stopTask() {
    if (!running) return;
    status = { text: "Stopping…", kind: "info" };
    await backend.cancelTask();
  }

  // GTKWave layouts are saved from GTKWave while Logix is in the
  // background; copy them into the project when the user comes back.
  $effect(() => {
    const onFocus = async () => {
      const p = project;
      if (!p?.dir) return;
      try {
        const lines = await backend.saveGtkwaveLayouts(buildKey(p), p.dir);
        if (lines.length > 0) log(...lines);
      } catch {
        // Best effort; the next run tries again.
      }
    };
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  });

  /** Shows a file at a 1-based line (from a link in the output). */
  function jumpTo(file: string, line: number) {
    if (!project || !findFile(project, file)) return;
    activeFile = file;
    editor?.reveal(file, line);
  }

  async function openProject() {
    if (!(await confirmLeave())) return;
    try {
      const doc = await backend.openProjectDialog();
      if (doc) {
        loadDoc(doc);
        finish(`Opened ${doc.dir}.`);
      }
    } catch (e) {
      commandFailed(e);
    }
  }

  async function save(): Promise<boolean> {
    if (!project) return false;
    if (!project.dir) return saveAs();
    const p = project;
    const dir = project.dir;
    const files = filesForSave(p);
    const board = p.board;
    const variant = { ...p.variant };
    try {
      await during("Saving…", () =>
        backend.saveProject(dir, files, [...p.removed], board, variant),
      );
      markSaved(p, dir, p.name, files, board, variant);
      fileError = null;
      finish(`Saved ${dir}.`);
      return true;
    } catch (e) {
      commandFailed(e);
      return false;
    }
  }

  async function saveAs(): Promise<boolean> {
    if (!project) return false;
    const p = project;
    const files = filesForSave(p);
    const board = p.board;
    const variant = { ...p.variant };
    const suggested = p.dir ? `${p.name} copy` : p.name;
    try {
      const doc = await backend.saveProjectAsDialog(
        p.dir,
        copySource(p),
        suggested,
        files,
        [...p.removed],
        board,
        variant,
      );
      if (!doc || !doc.dir) return false;
      // Keep the editor (and its undo history) as is.
      markSaved(p, doc.dir, doc.name, files, board, variant);
      fileError = null;
      finish(`Saved ${doc.dir}.`);
      return true;
    } catch (e) {
      commandFailed(e);
      return false;
    }
  }

  /** Runs one file operation at a time, ignoring repeats while busy. */
  async function exclusive(action: () => Promise<unknown>) {
    if (busy) return;
    busy = true;
    try {
      await action();
    } finally {
      busy = false;
    }
  }

  function onMenu(id: MenuId) {
    const variantChoice = parseMenuId(id);
    if (variantChoice) {
      setVariant(variantChoice.dim, variantChoice.value);
      return;
    }
    // Builds run alongside file operations (e.g. saving while building).
    const tasks: Partial<Record<MenuId, Task>> = {
      verify: "build",
      upload: "upload",
      "run-tests": "test",
      simulate: "simulate",
      "show-diagram": "graph",
    };
    const task = tasks[id];
    if (task) {
      runTask(task);
      return;
    }
    if (id === "auto-format") {
      autoFormat();
      return;
    }
    if (id === "toggle-output") {
      setConsoleVisible(!consoleVisible);
      return;
    }
    if (id === "stop") {
      stopTask();
      return;
    }
    if (id === "show-folder") {
      showProjectFolder();
      return;
    }
    if (id === "devices") {
      showDevices();
      return;
    }
    if (id === "board-info") {
      if (currentBoardInfo) boardInfoOpen = true;
      return;
    }
    if (id === "settings") {
      if (prefs) settingsOpen = true;
      return;
    }
    if (id === "go-to-definition") {
      editor?.goToDefinition();
      return;
    }
    if (id === "toggle-comment") {
      editor?.toggleComment();
      return;
    }
    if (id === "select-all") {
      selectAll();
      return;
    }
    if (id === "output-select-all") {
      selectOutput();
      return;
    }
    if (id === "clear-output") {
      consoleLines = [];
      return;
    }
    // The editor's right-click menu, in a testbench (as on its tab).
    if (id === "run-this-test" || id === "simulate-this-test") {
      if (project && isTestbench(activeFile)) {
        runTask(id === "run-this-test" ? "test" : "simulate", activeFile);
      }
      return;
    }
    if (id === "recents-changed") {
      loadRecents();
      return;
    }
    const actions: Record<
      Exclude<
        MenuId,
        | "verify"
        | "upload"
        | "run-tests"
        | "simulate"
        | "auto-format"
        | "toggle-output"
        | "stop"
        | "show-folder"
        | "show-diagram"
        | "devices"
        | "board-info"
        | "settings"
        | "go-to-definition"
        | "toggle-comment"
        | "select-all"
        | "output-select-all"
        | "clear-output"
        | "recents-changed"
        | "run-this-test"
        | "simulate-this-test"
      >,
      () => Promise<unknown>
    > = {
      new: newProject,
      "close-project": closeProject,
      open: openProject,
      save,
      "save-as": saveAs,
      "new-file": async () => startNewFile(),
      examples: async () => showExamples(),
    };
    // The task ids were handled above.
    exclusive(actions[id as keyof typeof actions]);
  }

  // ---- Select All and the output pane's menu ----

  /** Selects the text of the output pane (for Copy). */
  function selectOutput() {
    const el = consoleEl;
    if (!el) return;
    el.focus();
    const range = document.createRange();
    range.selectNodeContents(el);
    const selection = window.getSelection();
    selection?.removeAllRanges();
    selection?.addRange(range);
  }

  /**
   * Edit > Select All (Cmd+A): all of what has the focus, not the whole
   * window: a text field, the output pane, or else the editor.
   */
  function selectAll() {
    const focused = document.activeElement;
    if (focused instanceof HTMLInputElement || focused instanceof HTMLTextAreaElement) {
      focused.select();
    } else if (consoleEl && focused === consoleEl) {
      selectOutput();
    } else {
      editor?.selectAll();
    }
  }

  function onOutputKey(e: KeyboardEvent) {
    // Cmd+A also comes as the Edit menu's item; this covers keyboards
    // without the menu (e.g. the end-to-end tests).
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "a") {
      e.preventDefault();
      selectOutput();
    }
  }

  function onOutputMenu(e: MouseEvent) {
    e.preventDefault();
    consoleEl?.focus();
    backend.showOutputMenu();
  }

  function onCloseRequested() {
    exclusive(async () => {
      if (await confirmLeave()) await backend.closeWindow();
    });
  }

  $effect(() => {
    const unlisten = [
      backend.onToolOutput((line) => log(line)),
      backend.onLspClosed(onLspClosed),
      backend.onOpenRecent(openRecent),
      backend.onMenu(onMenu),
      backend.onCloseRequested(onCloseRequested),
    ];
    return () => unlisten.forEach((p) => p.then((stop) => stop()));
  });

  /** Starts with the last project, or the start screen if there's none. */
  async function openStartupProject() {
    try {
      const doc = await backend.openLastProject();
      if (doc) {
        loadDoc(doc);
        log(`Opened ${doc.dir}`);
        return;
      }
    } catch (e) {
      // E.g. the folder was moved or deleted; start fresh instead.
      log(`${toUserError(e).details} Showing the start screen.`);
    }
    // No project: the start screen.
  }

  onMount(async () => {
    await loadPrefs();
    try {
      homeBoard = await backend.lastBoard();
      homeVariant = await backend.lastVariant();
    } catch {
      // The board picker then starts empty.
    }
    await openStartupProject();
    try {
      info = await backend.appInfo();
      log(`${info.name} ${info.version} (${info.platform})`);
    } catch (e) {
      log(`Backend error: ${e}`);
      return;
    }
    if (!info.platformSupported) {
      setupError = "This platform is not supported yet.";
      return;
    }
    runSetup();
  });

  const stageText: Record<SetupStage, string> = {
    checking: "Checking FPGA tools…",
    downloadingEngine: "Downloading Logix Engine…",
    installingEnginePackages:
      "Installing FPGA tools. The first run can take a few minutes…",
    ready: "",
  };
</script>

<svelte:window onclick={() => (tabMenu = null)} />

{#if settingsOpen && prefs}
  <PreferencesDialog
    initial={prefs}
    onSave={savePrefs}
    onBrowse={(current) => backend.chooseFolderDialog(current)}
    onClose={() => (settingsOpen = false)}
  />
{/if}

{#if devicesOpen}
  <DevicesDialog
    {devices}
    error={devicesError}
    busy={devicesBusy}
    board={currentBoard}
    {boardName}
    onRefresh={refreshDevices}
    onPick={(id) => {
      devicesOpen = false;
      setBoard(id);
    }}
    onClose={() => (devicesOpen = false)}
  />
{/if}

{#if boardInfoOpen && currentBoardInfo}
  <BoardInfoDialog
    board={currentBoardInfo}
    {boards}
    selected={currentBoard}
    onPick={(id) => {
      boardInfoOpen = false;
      setBoard(id);
    }}
    onClose={() => (boardInfoOpen = false)}
  />
{/if}

{#if devicePicker}
  <DevicePicker
    boardName={boardName(devicePicker.board)}
    devices={devicePicker.devices}
    onChoose={chooseUploadDevice}
    onClose={() => (devicePicker = null)}
  />
{/if}

{#if diagram}
  <DiagramDialog
    svg={diagram.svg}
    modules={project ? projectModules(project) : ["main"]}
    module={diagram.module}
    busy={running === "graph"}
    onModule={showDiagram}
    onClose={() => (diagram = null)}
  />
{/if}

{#if examplesOpen}
  <ExamplesDialog
    {boards}
    {examples}
    initialBoard={pendingBoard ?? currentBoard}
    onlyBoard={pendingBoard}
    recents={pendingBoard ? boardRecents : []}
    onOpenRecent={openBoardRecent}
    onOpen={openExample}
    onClose={() => closeExamples(true)}
  />
{/if}

{#if tabMenu}
  <div
    class="context-menu"
    role="menu"
    style:left="{tabMenu.x}px"
    style:top="{tabMenu.y}px"
  >
    <button
      role="menuitem"
      disabled={!project || tabMenu.file === mainFile(project)}
      onclick={() => fromTabMenu(startRename)}>Rename…</button
    >
    <button
      role="menuitem"
      disabled={!project || tabMenu.file === mainFile(project)}
      onclick={() => fromTabMenu(askDeleteFile)}>Delete…</button
    >
    {#if isTestbench(tabMenu.file)}
      <hr />
      <button
        role="menuitem"
        disabled={!canRun}
        onclick={() => fromTabMenu((f) => runTask("test", f))}
        >Run This Test</button
      >
      <button
        role="menuitem"
        disabled={!canRun}
        onclick={() => fromTabMenu((f) => runTask("simulate", f))}
        >Simulate</button
      >
    {/if}
  </div>
{/if}

<div class="layout">
  <header class="toolbar">
    <button
      title="Verify: build the design"
      class:busy={running === "build"}
      disabled={!canRun}
      onclick={() => runTask("build")}><Icon name="verify" /></button
    >
    <button
      title="Upload: build the design and program the board"
      class:busy={running === "upload"}
      disabled={!canRun}
      onclick={() => runTask("upload")}><Icon name="upload" /></button
    >
    <span class="toolbar-gap"></span>
    <button
      title="Run tests: run all the testbenches"
      class:busy={running === "test"}
      disabled={!canRun || !menuState.testbenches}
      onclick={() => runTask("test")}><Icon name="test" /></button
    >
    <button
      title="Simulate: run the open testbench and show its signals in GTKWave"
      class:busy={running === "simulate"}
      disabled={!canRun || !menuState.simulate}
      onclick={() => runTask("simulate")}><Icon name="simulate" /></button
    >
    <button
      class="secondary"
      title="Stop: stop the running build, upload, test or simulation"
      disabled={!running}
      onclick={stopTask}><Icon name="stop" size={14} /></button
    >
    <BoardPicker
      {boards}
      recent={recentBoards}
      value={pendingBoard ?? currentBoard}
      disabled={boards.length === 0}
      placeholder={boards.length ? "Select board…" : "Loading boards…"}
      onPick={setBoard}
    />
    {#if currentBoardInfo && !pendingBoard}
      <button
        class="board-info"
        title="Board info: its FPGA, how it's programmed, and the boards with the same FPGA"
        aria-label="Board info"
        onclick={() => (boardInfoOpen = true)}>ⓘ</button
      >
    {/if}
    {#if currentBoardInfo && currentBoardInfo.dimensions.length > 0 && !pendingBoard}
      <VariantPanel
        board={currentBoardInfo}
        variant={currentVariant}
        disabled={running !== null}
        onChoose={setVariant}
      />
    {/if}
    {#if ready && currentBoard}
      {@const boardConnected = isConnected(currentBoard)}
      <button
        class="board-link"
        class:connected={boardConnected}
        title={`${boardConnected
          ? `A connected device may be the ${boardName(currentBoard)}. Upload checks.`
          : `${boardName(currentBoard)} isn't connected (no matching USB device found).`} Click to see the connected devices.`}
        onclick={showDevices}
        >{boardConnected ? "Connected" : "Not connected"}</button
      >
    {/if}
  </header>

  {#if devEngine}
    <div class="banner dev-engine" role="status">
      <span
        >Using a local Logix Engine, for diagnosis: {devEngine}. Start Logix
        normally to use the regular engine.</span
      >
    </div>
  {/if}

  {#if project && projectBoardsError}
    <div class="banner error custom-boards-error" role="alert">
      <span>{projectBoardsError} Its boards aren't offered until it's fixed.</span>
      {#if activeFile !== CUSTOM_BOARDS_FILE}
        <button onclick={() => (activeFile = CUSTOM_BOARDS_FILE)}>Show File</button>
      {/if}
    </div>
  {/if}

  {#if fileError}
    <div class="banner error" role="alert">
      <span>{fileError}</span>
      <button onclick={() => (fileError = null)}>Dismiss</button>
    </div>
  {/if}

  {#if setupError}
    <div class="banner error" role="alert">
      <span>{setupError}</span>
      {#if info?.platformSupported}
        <button onclick={runSetup}>Retry</button>
      {/if}
    </div>
  {/if}

  <div class="split" class:dragging bind:this={splitEl}>
    <main class="editor">
      {#if welcome}
        <div class="welcome" aria-label="Start">
          <h2>Welcome to Logix IDE</h2>
          <p class="motto">"The Arduino experience, for FPGAs"</p>
          {#if setupError}
            <p>
              Logix needs its FPGA tools to work. See the message above, and
              try again.
            </p>
          {:else if !ready}
            <p>
              Getting the FPGA tools ready… The first start takes several
              minutes.
            </p>
          {/if}
          <div class="start-actions">
            <button onclick={() => exclusive(newProject)}>
              <span class="start-icon"><Icon name="new" size={26} /></span>
              <span class="start-title">New Project</span>
              <span class="start-hint">An empty design for your board</span>
            </button>
            <button onclick={() => exclusive(openProject)}>
              <span class="start-icon"><Icon name="open" size={26} /></span>
              <span class="start-title">Open Project…</span>
              <span class="start-hint">A project folder on this computer</span>
            </button>
            <button disabled={examples.length === 0} onclick={showExamples}>
              <span class="start-icon"><Icon name="example" size={26} /></span>
              <span class="start-title">From Example…</span>
              <span class="start-hint">Blinky and more, ready to build</span>
            </button>
          </div>
          {#if recents.length > 0}
            <div class="recents">
              <h3>Recent projects</h3>
              <ul>
                {#each recents.slice(0, 5) as dir (dir)}
                  {@const name = dir.split(/[\\/]/).pop() ?? dir}
                  <li>
                    <button class="recent" title={dir} onclick={() => openRecent(dir)}>
                      <Icon name="folder" size={16} />
                      <span class="recent-name">{name}</span>
                      <span class="recent-path">{dir.slice(0, -name.length - 1)}</span>
                    </button>
                  </li>
                {/each}
              </ul>
            </div>
          {/if}
        </div>
      {/if}
      <div class="tabs" style:display={welcome ? "none" : null}>
        {#if project}
          {#each project.files as f (f.name)}
            {#if naming?.mode === "rename" && naming.file === f.name}
              <!-- svelte-ignore a11y_autofocus -->
              <input
                class="tab-input"
                bind:value={nameInput}
                onkeydown={onNameKey}
                onblur={cancelNaming}
                spellcheck="false"
                autocomplete="off"
                autofocus
              />
            {:else}
              <button
                class="tab"
                class:active={f.name === activeFile}
                class:has-error={fileProblems.get(f.name) === "error"}
                class:has-warning={fileProblems.get(f.name) === "warning"}
                title={roleTooltip(fileRole(f.name, mainFile(project), pinExtension), pinExtension)}
                onclick={() => selectFile(f.name)}
                ondblclick={() => startRename(f.name)}
                oncontextmenu={(e) => openTabMenu(e, f.name)}
              >
                {#if fileRole(f.name, mainFile(project), pinExtension) === "testbench"}<span
                    class="tab-icon"><Icon name="test" size={12} /></span
                  >{:else if fileRole(f.name, mainFile(project), pinExtension) === "pins"}<span
                    class="tab-icon"><Icon name="chip" size={12} /></span
                  >{/if}{f.name}{#if isFileDirty(project, f)}<span
                    class="dot"
                    title="Unsaved changes">●</span
                  >{/if}
              </button>
            {/if}
          {/each}
          {#if naming?.mode === "new"}
            <!-- svelte-ignore a11y_autofocus -->
            <input
              class="tab-input"
              placeholder="name.v"
              bind:value={nameInput}
              onkeydown={onNameKey}
              onblur={cancelNaming}
              spellcheck="false"
              autocomplete="off"
              autofocus
            />
          {/if}
          <button class="tab-add" title="New file" onclick={startNewFile}
            >+</button
          >
        {/if}
      </div>
      {#if nameError}
        <div class="name-error" role="alert">{nameError}</div>
      {/if}
      <div class="code" style:display={welcome ? "none" : null}>
        <Editor
          bind:this={editor}
          onChange={onEdit}
          onContextMenu={() => backend.showEditorMenu()}
          {dark}
          {lsp}
          projectDir={project?.dir ?? null}
        />
      </div>
    </main>

    <!-- Shown with the output hidden too, so it can be dragged back up. -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="divider"
      style:height="{DIVIDER_HEIGHT}px"
      title={consoleVisible
        ? "Drag to resize; drag to the bottom to hide the output"
        : "Drag up to show the output"}
      onpointerdown={onDividerDown}
      onpointermove={onDividerMove}
      onpointerup={onDividerUp}
      onpointercancel={onDividerUp}
    ></div>

    <section
      class="console-area"
      style:height={consoleVisible && !collapsing ? `${consoleHeight}px` : undefined}
    >
      {#if consoleVisible}
      <!-- Focusable, so Select All and Copy can act on the output (and the
           keyboard can scroll it). -->
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <div
        class="console"
        style:display={collapsing ? "none" : null}
        bind:this={consoleEl}
        tabindex="0"
        role="log"
        aria-label="Output"
        onkeydown={onOutputKey}
        oncontextmenu={onOutputMenu}
      >
        {#each consoleLines as line, i (i)}
          <div class="line">
            {#each line as seg, j (j)}{#if seg.link && project && findFile(project, seg.link.file)}<a
                  class="file-link"
                  href="#{seg.link.file}:{seg.link.line}"
                  style={seg.style || undefined}
                  title="Go to {seg.link.file} line {seg.link.line}"
                  onclick={(e) => {
                    // A link (not a button), so its text can be selected and
                    // copied with the rest of the output.
                    e.preventDefault();
                    jumpTo(seg.link!.file, seg.link!.line);
                  }}>{seg.text}</a
                >{:else if seg.style}<span style={seg.style}
                  >{seg.text}</span
                >{:else}{seg.text}{/if}{/each}
          </div>
        {/each}
      </div>
      {/if}
      <!-- The status line: at the bottom of the window, below the output. -->
      <div
        class="status-bar"
        class:ok={!isBusy && !activity && status?.kind === "ok"}
        class:error={!isBusy && status?.kind === "error"}
      >
        {#if isBusy}
          <span class="spinner" aria-label="Busy"></span>
        {/if}
        <span class="status-text" role="status">{statusText}</span>
        {#if running && taskStep && !activity}
          <span class="setup-detail"
            >{taskStep.message}{taskStep.percent != null
              ? ` ${taskStep.percent}%`
              : ""}</span
          >
          {#if taskStep.percent != null}
            <progress value={taskStep.percent / 100} max="1"></progress>
          {/if}
        {:else if settingUp && !activity && !running}
          {#if setupDetail}
            <span class="setup-detail">{setupDetail}</span>
          {/if}
          <progress value={downloadFraction ?? undefined} max="1"></progress>
        {/if}
        {#if consoleVisible}
          <button title="Clear the output" onclick={() => (consoleLines = [])}
            >Clear</button
          >
        {/if}
        <button
          title="{consoleVisible ? 'Hide' : 'Show'} the output"
          onclick={() => setConsoleVisible(!consoleVisible)}
          >{consoleVisible ? "Hide ▾" : "Show ▴"}</button
        >
      </div>
    </section>
  </div>
</div>

<style>
  .layout {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .split {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .split.dragging {
    cursor: row-resize;
    user-select: none;
  }
  .divider {
    flex: none;
    cursor: row-resize;
    background: var(--border);
  }
  .divider:hover,
  .split.dragging .divider {
    background: var(--accent);
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: var(--toolbar-bg);
    color: var(--toolbar-fg);
  }
  .toolbar button {
    display: grid;
    place-items: center;
    padding: 0;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    border: none;
    background: var(--accent-strong);
    color: var(--toolbar-fg);
    font-size: 16px;
  }
  /* Whether the selected board is plugged in: a dot and a word. A button
     (it opens Connected Devices), so it must override `.toolbar button`,
     the round icon buttons: hence `.toolbar .board-link`. */
  /* The ⓘ next to the board: Board Info. */
  .toolbar .board-info {
    width: auto;
    height: auto;
    margin-left: -4px;
    padding: 2px 6px;
    font-size: 16px;
    line-height: 1;
    color: rgba(255, 255, 255, 0.75);
    border: none;
    border-radius: 4px;
    background: transparent;
    cursor: pointer;
  }
  .toolbar .board-info:hover {
    background: rgba(255, 255, 255, 0.12);
  }
  .toolbar .board-link {
    display: flex;
    align-items: center;
    gap: 6px;
    width: auto;
    height: auto;
    margin-left: 0;
    padding: 2px 6px;
    font-size: 12px;
    font-family: inherit;
    line-height: inherit;
    color: rgba(255, 255, 255, 0.65);
    white-space: nowrap;
    border: none;
    border-radius: 4px;
    background: transparent;
    cursor: pointer;
  }
  .toolbar .board-link:hover {
    background: rgba(255, 255, 255, 0.12);
  }
  .board-link::before {
    content: "";
    width: 8px;
    height: 8px;
    border-radius: 50%;
    border: 1.5px solid rgba(255, 255, 255, 0.65);
  }
  .toolbar .board-link.connected {
    color: #ffffff;
  }
  .board-link.connected::before {
    border-color: #7ee08a;
    background: #7ee08a;
  }
  .banner {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
    background: var(--banner-bg);
    border-bottom: 1px solid var(--border);
  }
  .banner span {
    flex: 1;
  }
  .banner.dev-engine {
    background: var(--warning-mark);
    color: #1f2328;
    overflow-wrap: anywhere;
  }
  .banner.error {
    background: var(--banner-error-bg);
    color: var(--banner-error-fg);
    border-bottom-color: transparent;
  }
  .banner.error button {
    padding: 4px 14px;
    border: 1px solid currentColor;
    border-radius: 4px;
    background: transparent;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .editor {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .tabs {
    display: flex;
    border-bottom: 1px solid var(--border);
    background: var(--tabs-bg);
  }
  .tab {
    padding: 6px 14px;
    border: none;
    border-right: 1px solid var(--border);
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
    white-space: nowrap;
  }
  /* A testbench's flask (as Run Tests) or a pin file's pin, in the text color. */
  .tab-icon {
    display: inline-flex;
    vertical-align: -1px;
    margin-right: 5px;
    opacity: 0.8;
  }
  .tab.active {
    background: var(--bg);
    color: var(--fg);
    margin-bottom: -1px;
    border-top: 2px solid var(--accent);
    padding-top: 4px;
  }
  .tab-input {
    width: 140px;
    margin: 3px 6px;
    padding: 2px 6px;
    font: inherit;
    font-size: 13px;
    border: 1px solid var(--accent);
    border-radius: 3px;
    background: var(--bg);
    color: var(--fg);
  }
  .tab-add {
    padding: 4px 12px;
    border: none;
    background: transparent;
    color: var(--muted);
    font-size: 16px;
    cursor: pointer;
  }
  .tab-add:hover,
  .tab:hover {
    color: var(--fg);
  }
  .name-error {
    padding: 4px 12px;
    font-size: 12px;
    color: var(--banner-error-fg);
    background: var(--banner-error-bg);
  }
  .context-menu {
    position: fixed;
    z-index: 10;
    display: flex;
    flex-direction: column;
    min-width: 140px;
    padding: 4px 0;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
  }
  .context-menu button {
    padding: 5px 14px;
    border: none;
    background: transparent;
    color: var(--fg);
    font: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }
  .context-menu button:hover:not(:disabled) {
    background: var(--accent);
    color: #fff;
  }
  .context-menu hr {
    margin: 4px 0;
    border: none;
    border-top: 1px solid var(--border);
  }
  .toolbar-gap {
    width: 8px;
  }
  /* Same as the main actions, set slightly apart. */
  .toolbar button.secondary {
    margin-left: 4px;
  }
  .file-link {
    color: inherit;
    text-decoration: underline;
    cursor: pointer;
  }
  .file-link:hover {
    color: var(--accent);
  }
  .context-menu button:disabled {
    color: var(--muted);
    cursor: default;
  }
  .dot {
    margin-left: 6px;
    color: var(--accent);
  }
  .code {
    flex: 1;
    min-height: 0;
  }
  .welcome {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    /* Top-aligned, so the space above the title equals the space between
       it and the cards (--welcome-gap) at any window height. */
    --welcome-gap: 56px;
    justify-content: flex-start;
    padding: var(--welcome-gap) 24px 24px;
    text-align: center;
    overflow: auto;
  }
  .welcome h2 {
    margin: 0 0 6px;
    font-size: 26px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .welcome .motto {
    margin: 0;
    color: var(--muted);
    font-size: 15px;
  }
  .welcome p {
    max-width: 460px;
    margin: 16px 0 0;
    line-height: 1.5;
  }
  .start-actions {
    display: flex;
    gap: 12px;
    margin-top: var(--welcome-gap);
  }
  .start-actions button {
    width: 156px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: 12px 14px 14px;
    font: inherit;
    text-align: left;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--bg);
    color: var(--fg);
    cursor: pointer;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
    transition:
      border-color 0.12s,
      box-shadow 0.12s,
      transform 0.12s;
  }
  .start-actions button:hover:not(:disabled) {
    border-color: var(--accent);
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.1);
    transform: translateY(-1px);
  }
  .start-actions button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .start-actions button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .start-icon {
    display: flex;
    margin-bottom: 6px;
    color: var(--accent);
  }
  .start-title {
    font-size: 14px;
    font-weight: 600;
  }
  .start-hint {
    font-size: 12px;
    line-height: 1.4;
    color: var(--muted);
  }
  .recents {
    margin-top: 40px;
    width: 492px;
    max-width: 100%;
    text-align: left;
  }
  .recents h3 {
    margin: 0 0 8px;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--muted);
  }
  .recents ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .recent {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 8px;
    border: none;
    border-radius: 6px;
    background: none;
    color: var(--accent);
    cursor: pointer;
    font: inherit;
    text-align: left;
  }
  .recent:hover {
    background: var(--tabs-bg);
  }
  .recent-name {
    font-weight: 600;
    color: var(--fg);
  }
  .recent-path {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--muted);
    font-size: 12px;
  }

  .console-area {
    flex: none;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .status-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    background: var(--tabs-bg);
    border-top: 1px solid var(--border);
    font-size: 12px;
  }
  .status-bar.ok {
    background: var(--status-ok-bg);
    color: var(--status-ok-fg);
  }
  .status-bar.error {
    background: var(--banner-error-bg);
    color: var(--banner-error-fg);
  }
  /* Busy indicator, as in Arduino's progress notifications. */
  .spinner {
    flex: none;
    width: 11px;
    height: 11px;
    border: 2px solid currentColor;
    border-right-color: transparent;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    opacity: 0.8;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .status-text {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .setup-detail {
    flex: none;
  }
  .status-bar progress {
    flex: none;
    width: 160px;
  }
  .status-bar button {
    padding: 1px 10px;
    border: 1px solid currentColor;
    border-radius: 3px;
    background: transparent;
    color: inherit;
    font: inherit;
    cursor: pointer;
    opacity: 0.8;
  }
  .toolbar button:disabled {
    opacity: 0.45;
  }
  .toolbar button:not(:disabled) {
    cursor: pointer;
  }
  .toolbar button.busy {
    opacity: 1;
    box-shadow: 0 0 0 2px var(--toolbar-fg);
  }
  .tab.has-error {
    box-shadow: inset 0 -2px 0 var(--error-mark);
  }
  .tab.has-warning {
    box-shadow: inset 0 -2px 0 var(--warning-mark);
  }
  .console:focus {
    outline: none;
  }
  .console {
    flex: 1;
    min-height: 0;
    box-sizing: border-box;
    overflow: auto;
    padding: 8px 12px;
    background: var(--console-bg);
    color: var(--console-fg);
    font-family: ui-monospace, Menlo, Consolas, monospace;
    font-size: 12px;
  }
  .line {
    white-space: pre-wrap;
  }
</style>
