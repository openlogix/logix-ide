<script lang="ts">
  import { onMount, tick, untrack } from "svelte";
  import {
    backend,
    toUserError,
    type AppInfo,
    type Board,
    type ConnectedBoard,
    type Example,
    type Devices,
    type Diagnostic,
    type Preferences,
    type MenuId,
    type ProjectDoc,
    type Task,
    type SetupEvent,
    type SetupStage,
  } from "./lib/backend";
  import Editor from "./lib/Editor.svelte";
  import { setHoverEnabled, startLanguageServer } from "./lib/lsp";
  import type { LSPClient } from "@codemirror/lsp-client";
  import ExamplesDialog from "./lib/ExamplesDialog.svelte";
  import BoardPicker from "./lib/BoardPicker.svelte";
  import DiagramDialog from "./lib/DiagramDialog.svelte";
  import DevicesDialog from "./lib/DevicesDialog.svelte";
  import PreferencesDialog from "./lib/PreferencesDialog.svelte";
  import { deviceSummaryLines, isBoardNotFound } from "./lib/devices";
  import Icon from "./lib/Icon.svelte";
  import {
    addFile,
    buildKey,
    copySource,
    deleteFile,
    filesForSave,
    findFile,
    fromDoc,
    isHdlSource,
    isTestbench,
    isUntouchedStarter,
    isDirty,
    isFileDirty,
    MAIN_FILE,
    mainFile,
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
  import { clampConsoleHeight, DEFAULT_CONSOLE_HEIGHT } from "./lib/layout";

  const MAX_CONSOLE_LINES = 5000;
  const DIVIDER_HEIGHT = 6;
  const CONSOLE_HEIGHT_KEY = "logix.consoleHeight";

  // Editor/console split. The console height is remembered between runs.
  let splitEl = $state<HTMLElement | null>(null);
  let consoleHeight = $state(loadConsoleHeight());
  let dragging = $state(false);

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
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    e.preventDefault();
  }

  function onDividerMove(e: PointerEvent) {
    if (!dragging || !splitEl) return;
    const bottom = splitEl.getBoundingClientRect().bottom;
    fitConsoleHeight(bottom - e.clientY - DIVIDER_HEIGHT / 2);
  }

  function onDividerUp() {
    if (!dragging) return;
    dragging = false;
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

  /** Shows the output, e.g. when a build starts or something fails. */
  function showOutput() {
    setConsoleVisible(true);
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

  const ready = $derived(stage === "ready");
  const downloadFraction = $derived(
    download ? progressFraction(download.received, download.total) : null,
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

  function onSetupEvent(event: SetupEvent) {
    switch (event.kind) {
      case "stage":
        stage = event.stage;
        if (event.stage !== "downloadingApio") download = null;
        // A long first-run install: give the user a project meanwhile.
        if (event.stage !== "checking" && event.stage !== "ready")
          ensureProject();
        break;
      case "download":
        download = { received: event.received, total: event.total };
        break;
      case "log":
        log(event.line);
        break;
    }
  }

  async function runSetup() {
    setupError = null;
    stage = "checking";
    try {
      await backend.setupApio(onSetupEvent);
      // The "ready" event comes through another channel and may arrive
      // after this returns; don't wait for it.
      stage = "ready";
      // Boards and examples first, so "Ready." comes last.
      await loadCatalog();
      await ensureProject();
      startLsp();
      log("Ready.");
    } catch (e) {
      const err = toUserError(e);
      setupError = err.message;
      log(`Error: ${err.details}`);
      showOutput();
      ensureProject();
    }
  }

  /**
   * Starts a new project if there's none yet. At startup without a last
   * project, this waits until the tools are ready, so the board's starter
   * (blinky) shows right away instead of a placeholder that's replaced a
   * moment later. A long install or a failed setup gets the placeholder.
   */
  async function ensureProject() {
    if (project) return;
    await exclusive(async () => {
      if (project) return;
      try {
        loadDoc(
          await during("Creating a new project…", () =>
            backend.newProject(null),
          ),
        );
      } catch (e) {
        showFileError(e);
      }
    });
  }

  // ---- Boards and examples ----

  let boards = $state<Board[]>([]);
  let examples = $state<Example[]>([]);
  let examplesOpen = $state(false);

  async function loadCatalog() {
    try {
      const [boardList, exampleList] = await Promise.all([
        backend.getBoards(),
        backend.getExamples(),
      ]);
      boards = boardList;
      examples = exampleList.examples;
      log(examplesSummary(exampleList.examples.length, exampleList.hidden));
    } catch (e) {
      showFileError(e);
      return;
    }
    // A new project made before the tools were ready (e.g. at startup) has
    // the generic starter; give it its board's starter now.
    const p = project;
    if (p && p.board && p.templateDir === null && isUntouchedStarter(p)) {
      await exclusive(() => replaceStarter(p.board!));
    }
  }

  /** Replaces an untouched new project with `board`'s starter project. */
  async function replaceStarter(board: string) {
    try {
      const doc = await during("Preparing the board's starter…", () =>
        backend.newProject(board),
      );
      // Only if the user hasn't started editing meanwhile.
      if (project && isUntouchedStarter(project)) loadDoc(doc);
    } catch (e) {
      showFileError(e);
    }
  }

  // ---- Connected boards (the board list's "Connected" group) ----

  /** Boards that match the connected USB devices, polled while idle. */
  let connected = $state<ConnectedBoard[]>([]);
  let scanning = false;

  async function refreshConnected() {
    // Not while a task runs: Upload uses the USB device.
    if (scanning || !ready || isBusy) return;
    scanning = true;
    try {
      const found = await backend.connectedBoards();
      if (JSON.stringify(found) !== JSON.stringify(connected)) connected = found;
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

  function isConnected(board: string | null | undefined): boolean {
    return board != null && connected.some((c) => c.board === board);
  }

  /**
   * A board chosen in the toolbar while its examples are offered: applied
   * only if the user keeps the current files (opening an example brings
   * its own board), so the "Save changes?" question is only about files.
   */
  let pendingBoard = $state<string | null>(null);

  /**
   * Toolbar board change. As in the Arduino IDE, it never changes the
   * files by itself; the board's examples are offered, with "Keep Current
   * Files" as the default.
   */
  function setBoard(id: string) {
    if (!project || !id) return;
    backend.setLastBoard(id);
    if (examples.length === 0) {
      project.board = id;
      return;
    }
    pendingBoard = id;
    examplesOpen = true;
  }

  function closeExamples(keepFiles: boolean) {
    if (keepFiles && pendingBoard && project) project.board = pendingBoard;
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
        const doc = await during(
          `Opening example ${ex.name}…`,
          () =>
            ex.bare
              ? backend.openBareMinimum(ex.board)
              : backend.openExample(ex.board, ex.name),
          `Opened example ${ex.name}.`,
        );
        loadDoc(doc);
        log(`Opened example ${ex.board}/${ex.name}`);
      } catch (e) {
        showFileError(e);
      }
    });
  }

  // ---- Verify (build) and upload ----

  let running = $state<Task | null>(null);
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

  /**
   * Shows `label` as the current activity while `work` runs, then `done`
   * (if given) as the status.
   */
  async function during<T>(
    label: string,
    work: () => Promise<T>,
    done?: string,
  ): Promise<T> {
    activity = label;
    try {
      const result = await work();
      if (done) status = { text: done, kind: "info" };
      return result;
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


  async function autoFormat() {
    if (!project || formatting) return;
    const p = project;
    const file = activeFile;
    const f = findFile(p, file);
    if (!f) return;
    if (!isHdlSource(file)) {
      status = {
        text: "Auto Format works on Verilog and SystemVerilog files (.v .sv .vh .svh).",
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
      const result = await backend.formatFile(file, before, p.board);
      if (project !== p) return; // Another project was opened meanwhile.
      // Old markers in this file no longer match its lines.
      const others = shownDiagnostics.filter((d) => d.file !== file);
      if (!result.ok || result.text === null) {
        showOutput();
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
  let devices = $state<Devices | null>(null);
  let devicesError = $state<string | null>(null);
  let devicesBusy = $state(false);

  async function refreshDevices() {
    devicesBusy = true;
    try {
      devices = await backend.scanDevices();
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
      log("", ...deviceSummaryLines(await backend.scanDevices()));
    } catch {
      // Only a hint; the upload error is already shown.
    }
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
   * testbench for "test" and "simulate".
   */
  async function runTask(task: Task, testbench: string | null = null) {
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
    status = { text: runningText(task), kind: "info" };
    consoleLines = [];
    showOutput();
    const which = testbench ? ` (${testbench})` : "";
    log(`${VERB[task]} ${p.name}${which} for ${boardName(board)}…`);
    try {
      const outcome = await backend.runTask(
        task,
        testbench,
        buildKey(p),
        copySource(p),
        p.dir,
        filesForSave(p),
        board,
        (line) => log(line),
      );
      if (project !== p) return; // Another project was opened meanwhile.
      if (task === "test") log("", ...testbenchLines(outcome));
      if (
        task === "upload" &&
        !outcome.ok &&
        !outcome.cancelled &&
        outcome.diagnostics.some((d) => isBoardNotFound(d.message))
      ) {
        await logConnectedDevices();
      }
      if (task === "graph" && outcome.ok && outcome.graphSvg) {
        diagram = { svg: outcome.graphSvg, module: testbench ?? "main" };
      }
      showDiagnostics(outcome.diagnostics);
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
    } finally {
      running = null;
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
   * The FPGA tools aren't ready (the first start's install, or a failed
   * setup) and there's nothing of the user's to show: the editor area shows
   * a welcome panel instead of a placeholder design that can't be built yet.
   * Once ready, the board's starter (blinky) replaces it.
   */
  const welcome = $derived(
    !ready && (project === null || isUntouchedStarter(project)),
  );
  /** Whether Verify and Upload can be started now. */
  const canRun = $derived(
    ready &&
      project !== null &&
      !welcome &&
      running === null &&
      activity === null,
  );

  $effect(() => {
    if (project) backend.setDocumentState(project.name, dirty);
  });

  function showFileError(e: unknown) {
    const err = toUserError(e);
    fileError = err.message;
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

  async function newProject() {
    const board = project?.board ?? null;
    if (!(await confirmLeave())) return;
    loadDoc(
      await during(
        "Creating a new project…",
        () => backend.newProject(board),
        "New project.",
      ),
    );
  }

  /** File > Open Recent. */
  function openRecent(dir: string) {
    exclusive(async () => {
      if (!(await confirmLeave())) return;
      try {
        const doc = await during(
          "Opening the project…",
          () => backend.openProjectAt(dir),
          "Opened the project.",
        );
        loadDoc(doc);
        log(`Opened ${doc.dir}`);
      } catch (e) {
        showFileError(e);
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
        log(`Opened ${doc.dir}`);
      }
    } catch (e) {
      showFileError(e);
    }
  }

  async function save(): Promise<boolean> {
    if (!project) return false;
    if (!project.dir) return saveAs();
    const p = project;
    const dir = project.dir;
    const files = filesForSave(p);
    const board = p.board;
    try {
      await during(
        "Saving…",
        () => backend.saveProject(dir, files, [...p.removed], board),
        "Saved.",
      );
      markSaved(p, dir, p.name, files, board);
      fileError = null;
      log(`Saved ${dir}`);
      return true;
    } catch (e) {
      showFileError(e);
      return false;
    }
  }

  async function saveAs(): Promise<boolean> {
    if (!project) return false;
    const p = project;
    const files = filesForSave(p);
    const board = p.board;
    const suggested = p.dir ? `${p.name} copy` : p.name;
    try {
      const doc = await backend.saveProjectAsDialog(
        copySource(p),
        suggested,
        files,
        [...p.removed],
        board,
      );
      if (!doc || !doc.dir) return false;
      // Keep the editor (and its undo history) as is.
      markSaved(p, doc.dir, doc.name, files, board);
      fileError = null;
      log(`Saved ${doc.dir}`);
      return true;
    } catch (e) {
      showFileError(e);
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
        | "settings"
        | "go-to-definition"
        | "toggle-comment"
        | "select-all"
        | "output-select-all"
        | "clear-output"
      >,
      () => Promise<unknown>
    > = {
      new: newProject,
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
      backend.onApioOutput((line) => log(line)),
      backend.onLspClosed(onLspClosed),
      backend.onOpenRecent(openRecent),
      backend.onMenu(onMenu),
      backend.onCloseRequested(onCloseRequested),
    ];
    return () => unlisten.forEach((p) => p.then((stop) => stop()));
  });

  /** Starts with the last project, or a new one if there's none. */
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
      log(`${toUserError(e).details} Starting a new project.`);
    }
    // No project yet: a new one comes once the tools are ready
    // (`ensureProject`).
  }

  onMount(async () => {
    await loadPrefs();
    await openStartupProject();
    try {
      info = await backend.appInfo();
      log(`${info.name} ${info.version} (${info.platform})`);
    } catch (e) {
      log(`Backend error: ${e}`);
      ensureProject();
      return;
    }
    if (!info.platformSupported) {
      setupError = "This platform is not supported yet.";
      ensureProject();
      return;
    }
    runSetup();
  });

  const stageText: Record<SetupStage, string> = {
    checking: "Checking FPGA tools…",
    downloadingApio: "Downloading Apio…",
    installingPackages:
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
    onRefresh={refreshDevices}
    onClose={() => (devicesOpen = false)}
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
    initialBoard={pendingBoard ?? project?.board ?? null}
    onlyBoard={pendingBoard}
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
      title="Verify (⌘R): build the design"
      class:busy={running === "build"}
      disabled={!canRun}
      onclick={() => runTask("build")}><Icon name="verify" /></button
    >
    <button
      title="Upload (⌘U): build and program the board"
      class:busy={running === "upload"}
      disabled={!canRun}
      onclick={() => runTask("upload")}><Icon name="upload" /></button
    >
    <span class="toolbar-gap"></span>
    <button
      title="Run tests (⇧⌘T): run the testbenches"
      class:busy={running === "test"}
      disabled={!canRun}
      onclick={() => runTask("test")}><Icon name="test" /></button
    >
    <button
      title="Simulate (⌥⌘T): run the open testbench and show its signals in GTKWave"
      class:busy={running === "simulate"}
      disabled={!canRun}
      onclick={() => runTask("simulate")}><Icon name="simulate" /></button
    >
    <button
      class="secondary"
      title="Stop (⌘.): stop the running build, upload, test or simulation"
      disabled={!running}
      onclick={stopTask}><Icon name="stop" size={14} /></button
    >
    <BoardPicker
      {boards}
      connected={connected.map((c) => c.board)}
      value={pendingBoard ?? project?.board ?? null}
      disabled={!project || boards.length === 0}
      placeholder={boards.length ? "Select board…" : "Loading boards…"}
      onPick={setBoard}
    />
    {#if ready && project?.board}
      {@const boardConnected = isConnected(project.board)}
      <span
        class="board-link"
        class:connected={boardConnected}
        title={boardConnected
          ? `${boardName(project.board)} is connected.`
          : `${boardName(project.board)} isn't connected (no matching USB device found).`}
        >{boardConnected ? "Connected" : "Not connected"}</span
      >
    {/if}
  </header>

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
  {:else if stage && !ready}
    <div class="banner" role="status">
      <span>
        {stageText[stage]}
        {#if download}
          {formatBytes(download.received)}{download.total
            ? ` of ${formatBytes(download.total)}`
            : ""}
        {/if}
      </span>
      <progress value={downloadFraction ?? undefined} max="1"></progress>
    </div>
  {/if}

  <div class="split" class:dragging bind:this={splitEl}>
    <main class="editor">
      {#if welcome}
        <div class="welcome">
          <h2>Welcome to Logix IDE</h2>
          <p class="motto">"The Arduino experience, for FPGAs"</p>
          {#if setupError}
            <p>
              Logix needs its FPGA tools to work. See the message above, and
              try again.
            </p>
          {:else}
            <p>
              Getting the FPGA tools ready… The first start takes several
              minutes.
            </p>
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
                title={f.name === mainFile(project)
                  ? "The main file (top module 'main')"
                  : "Double-click to rename; right-click for more"}
                onclick={() => selectFile(f.name)}
                ondblclick={() => startRename(f.name)}
                oncontextmenu={(e) => openTabMenu(e, f.name)}
              >
                {f.name}{#if isFileDirty(project, f)}<span
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
          <button class="tab-add" title="New file (⇧⌘N)" onclick={startNewFile}
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
          onContextMenu={(canGoTo) => backend.showEditorMenu(canGoTo)}
          {dark}
          {lsp}
          projectDir={project?.dir ?? null}
        />
      </div>
    </main>

    {#if consoleVisible}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="divider"
      style:height="{DIVIDER_HEIGHT}px"
      title="Drag to resize"
      onpointerdown={onDividerDown}
      onpointermove={onDividerMove}
      onpointerup={onDividerUp}
      onpointercancel={onDividerUp}
    ></div>
    {/if}

    <section
      class="console-area"
      style:height={consoleVisible ? `${consoleHeight}px` : undefined}
    >
      <div
        class="status-bar"
        class:ok={!isBusy && !activity && status?.kind === "ok"}
        class:error={!isBusy && status?.kind === "error"}
      >
        {#if isBusy}
          <span class="spinner" aria-label="Busy"></span>
        {/if}
        <span class="status-text" role="status">{statusText}</span>
        {#if consoleVisible}
          <button title="Clear the output" onclick={() => (consoleLines = [])}
            >Clear</button
          >
        {/if}
        <button
          title="{consoleVisible ? 'Hide' : 'Show'} the output (⌘J)"
          onclick={() => setConsoleVisible(!consoleVisible)}
          >{consoleVisible ? "Hide ▾" : "Show ▴"}</button
        >
      </div>
      {#if consoleVisible}
      <!-- Focusable, so Select All and Copy can act on the output (and the
           keyboard can scroll it). -->
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <div
        class="console"
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
  /* Whether the selected board is plugged in: a dot and a word. */
  .board-link {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-left: 10px;
    font-size: 12px;
    color: rgba(255, 255, 255, 0.65);
    white-space: nowrap;
  }
  .board-link::before {
    content: "";
    width: 8px;
    height: 8px;
    border-radius: 50%;
    border: 1.5px solid rgba(255, 255, 255, 0.65);
  }
  .board-link.connected {
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
  .banner progress {
    width: 200px;
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
    justify-content: center;
    gap: 6px;
    padding: 24px;
    text-align: center;
  }
  .welcome h2 {
    margin: 0;
    font-size: 22px;
    font-weight: 600;
  }
  .welcome .motto {
    margin: 0 0 12px;
    color: var(--accent);
    font-weight: 600;
  }
  .welcome p {
    max-width: 460px;
    margin: 0 0 12px;
    line-height: 1.5;
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
    padding: 3px 12px;
    background: var(--tabs-bg);
    border-bottom: 1px solid var(--border);
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
