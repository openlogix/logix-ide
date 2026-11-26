<!--
  The Serial Monitor (Tools > Serial Monitor, the bottom pane's second tab),
  as Arduino IDE's. Logix Engine does the work (its serial tools): it finds
  the board's serial port from the board's data, keeps the session, pauses
  it while Upload writes the board and reopens it after an unplug. This
  only opens the port, shows what arrives and sends what the user types.
-->
<script lang="ts">
  import { onDestroy, onMount, untrack } from "svelte";
  import { backend, type SerialChoice, type SerialEvent } from "./backend";
  import { serialLines, type SerialLine } from "./serial";

  interface Props {
    /** The toolbar's board: its UART is the port opened by itself. */
    board: string | null;
    /** The project's boards folder, for its own boards' data. */
    boardsFolder: string | null;
    boardsSaved: boolean;
    /** The project's speed (bits per second). */
    baud: number;
    /** The user chose another speed: saved with the project. */
    onBaud: (baud: number) => void;
    /** Whether the tab is shown: the port opens the first time it is. */
    active: boolean;
  }
  let { board, boardsFolder, boardsSaved, baud, onBaud, active }: Props = $props();

  const BAUDS = [9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600];
  const ENDINGS = [
    { value: "\n", label: "Newline" },
    { value: "\r\n", label: "CR + LF" },
    { value: "\r", label: "CR" },
    { value: "", label: "Nothing" },
  ];

  // Per-user conveniences, kept across runs.
  function stored(key: string, fallback: string): string {
    try {
      return localStorage.getItem(key) ?? fallback;
    } catch {
      return fallback;
    }
  }
  function store(key: string, value: string) {
    try {
      localStorage.setItem(key, value);
    } catch {
      // Not kept: fine.
    }
  }
  let ending = $state(stored("logix.serial.ending", "\n"));
  let liveKeys = $state(stored("logix.serial.liveKeys", "false") === "true");
  let timestamps = $state(stored("logix.serial.timestamps", "false") === "true");
  $effect(() => store("logix.serial.ending", ending));
  $effect(() => store("logix.serial.liveKeys", String(liveKeys)));
  $effect(() => store("logix.serial.timestamps", String(timestamps)));
  /** The port the user chose for a board, such as an external USB to
   * serial adapter wired to it: opened by itself next time, when it's
   * there. Per user (a port's name is this computer's), by board ID. */
  const chosenKey = (b: string | null) => `logix.serial.port.${b ?? ""}`;

  let choices = $state<SerialChoice[]>([]);
  /** The port open (or being opened) now. */
  let chosen = $state<SerialChoice | null>(null);
  let open = $state(false);
  let connecting = $state(false);
  /** A line for the user above the output, such as why nothing is open. */
  let note = $state<{ text: string; warn: boolean } | null>(null);
  let lines = $state<SerialLine[]>([]);
  let input = $state("");
  let outEl: HTMLDivElement | undefined = $state();
  /** What the user closed: nothing reopens it by itself until Connect. */
  let userClosed = $state(false);
  /** The board and speed the last try to open a port was for: tried once
   * each, so a board whose port can't be chosen by itself isn't tried again
   * and again (only Connect or a choice tries again). */
  let triedFor: string | null = null;

  /** Whether the output is scrolled to its end (within a few pixels): new
   * text then keeps it there; scrolled up to read, it stays put (the user's
   * report, 2026-11-20). */
  function atEnd(): boolean {
    return !outEl || outEl.scrollHeight - outEl.scrollTop - outEl.clientHeight < 8;
  }
  function follow(wasAtEnd: boolean) {
    if (!wasAtEnd) return;
    queueMicrotask(() => {
      if (outEl) outEl.scrollTop = outEl.scrollHeight;
    });
  }
  function addText(text: string) {
    const wasAtEnd = atEnd();
    // Every line keeps its time; Timestamps only shows or hides them.
    lines = serialLines(lines, text, new Date());
    follow(wasAtEnd);
  }
  function addNote(text: string) {
    const wasAtEnd = atEnd();
    lines = serialLines(lines, null, new Date(), text);
    follow(wasAtEnd);
  }

  /** Empties the monitor: the status bar's Clear, while this tab is shown. */
  export function clear() {
    lines = [];
  }

  /**
   * A port's hover text, a line each: what it is (Logix Engine's
   * description, such as "The Alhambra II's serial port" or "The Alhambra
   * II's programming port"), its name, its device and its USB interface.
   */
  function portTip(c: SerialChoice): string {
    return [
      c.description || `A serial port of ${c.who}`,
      c.port,
      `Device: ${c.label}`,
      c.interface != null ? `USB interface ${c.interface}` : null,
    ]
      .filter(Boolean)
      .join("\n");
  }

  function label(c: SerialChoice): string {
    const where = c.interface != null ? `, interface ${c.interface}` : "";
    return `${c.port} (${c.who}${where})`;
  }

  /** Finds the ports, then opens the board's UART, if the engine knows it. */
  async function connect(choice?: SerialChoice) {
    if (connecting) return;
    connecting = true;
    userClosed = false;
    try {
      choices = await backend.serialPorts(boardsFolder, boardsSaved, board);
      const remembered = stored(chosenKey(board), "");
      const pick =
        choice ??
        choices.find((c) => remembered !== "" && c.port === remembered) ??
        choices.find((c) => c.boardUart) ??
        (choices.length === 1 ? choices[0] : undefined);
      if (!pick) {
        chosen = null;
        open = false;
        note = choices.length
          ? { text: "Choose the board's serial port.", warn: false }
          : {
              text: board
                ? "No serial port found: is the board plugged in? Its USB cable must carry data."
                : "No serial port found: plug in a board.",
              warn: true,
            };
        return;
      }
      await backend.serialOpen(pick.port, baud, pick.device);
      chosen = pick;
      open = true;
      note = null;
      addNote(`Opened ${label(pick)} at ${baud} baud.`);
    } catch (e) {
      open = false;
      note = { text: String((e as { message?: string })?.message ?? e), warn: true };
    } finally {
      connecting = false;
    }
  }

  async function disconnect() {
    userClosed = true;
    open = false;
    try {
      await backend.serialClose();
    } catch {
      // Closed anyway.
    }
    if (chosen) addNote(`Closed ${chosen.port}.`);
    note = { text: "Closed. Connect opens it again.", warn: false };
  }

  async function send(text: string) {
    if (!open || text === "") return;
    try {
      await backend.serialWrite(text);
    } catch (e) {
      note = { text: String((e as { message?: string })?.message ?? e), warn: true };
    }
  }
  function sendLine() {
    const text = input + ending;
    input = "";
    send(text);
  }
  function onKey(e: KeyboardEvent) {
    if (liveKeys) {
      if (e.key === "Enter") {
        e.preventDefault();
        send(ending || "\r");
      } else if (e.key.length === 1 && !e.metaKey && !e.ctrlKey) {
        e.preventDefault();
        send(e.key);
      }
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      sendLine();
    }
  }

  function onEvent(ev: SerialEvent) {
    if (ev.kind === "read") {
      if (ev.text) addText(ev.text);
      for (const m of ev.events) addNote(m);
      open = ev.open;
      if (chosen && ev.port && ev.port !== chosen.port) chosen = { ...chosen, port: ev.port };
      if (ev.open) note = null;
      else if (ev.events.length) note = { text: ev.events[ev.events.length - 1], warn: true };
    } else {
      // The session ended (such as the engine restarting for an update):
      // open it again, unless the user closed it.
      open = false;
      addNote(ev.message);
      if (!userClosed) setTimeout(() => active && !open && connect(chosen ?? undefined), 2000);
    }
  }

  let unlisten: (() => void) | null = null;
  onMount(() => {
    backend.onSerial(onEvent).then((u) => (unlisten = u));
  });
  onDestroy(() => {
    unlisten?.();
    backend.serialClose().catch(() => {});
  });

  // Opens when the tab is first shown, and again for another board or
  // speed: once each (triedFor).
  $effect(() => {
    const key = `${board}|${baud}`;
    if (!active || userClosed || key === triedFor) return;
    const sameBoard = triedFor?.startsWith(`${board}|`) ?? false;
    triedFor = key;
    untrack(() => connect(sameBoard ? (chosen ?? undefined) : undefined));
  });

  function choose(e: Event) {
    const port = (e.target as HTMLSelectElement).value;
    const c = choices.find((x) => x.port === port);
    if (!c) return;
    // The board's own UART needs no memory: it's opened by itself anyway.
    store(chosenKey(board), c.boardUart ? "" : c.port);
    connect(c);
  }
</script>

<div class="monitor">
  <div class="bar">
    <!-- The port once: a menu when there's a choice, else its name. -->
    <span class="port" title={open ? "Open" : "Closed"}>
      <span class="dot" class:on={open}></span>
      {#if choices.length > 1 || (!chosen && choices.length)}
        <!-- Just the port names; the device and interface on hover. -->
        <select
          aria-label="Serial port"
          title={chosen ? portTip(chosen) : "Choose the board's serial port"}
          value={chosen?.port ?? ""}
          onchange={choose}
        >
          {#if !chosen}<option value="" disabled>Choose a port…</option>{/if}
          {#each choices.filter((c) => !c.other) as c (c.port)}
            <option value={c.port} title={portTip(c)}>{c.port}</option>
          {/each}
          {#if choices.some((c) => c.other)}
            <!-- Ports Logix Engine tied to no device: still the user's to choose. -->
            <optgroup label="Other serial ports">
              {#each choices.filter((c) => c.other) as c (c.port)}
                <option value={c.port} title={portTip(c)}>{c.port}</option>
              {/each}
            </optgroup>
          {/if}
        </select>
      {:else}
        <span title={chosen ? portTip(chosen) : undefined}>{chosen ? chosen.port : "No port"}</span>
      {/if}
    </span>
    {#if open}
      <button onclick={disconnect} title="Close the port, so other programs can open it">Disconnect</button>
    {:else}
      <button onclick={() => connect(chosen ?? undefined)} disabled={connecting}
        >{connecting ? "Connecting…" : "Connect"}</button
      >
    {/if}
    <span class="spacer"></span>
    <label class="field"
      >Baud
      <select
        aria-label="Baud rate"
        value={String(baud)}
        onchange={(e) => onBaud(Number((e.target as HTMLSelectElement).value))}
      >
        {#each BAUDS as b (b)}<option value={String(b)}>{b}</option>{/each}
      </select>
    </label>
    <label class="field"
      >Enter sends
      <select aria-label="Line ending" bind:value={ending}>
        {#each ENDINGS as e (e.label)}<option value={e.value}>{e.label}</option>{/each}
      </select>
    </label>
    <label class="check"><input type="checkbox" bind:checked={liveKeys} /> Send each key</label>
    <label class="check"><input type="checkbox" bind:checked={timestamps} /> Timestamps</label>
  </div>
  {#if note}
    <div class="note" class:warn={note.warn} role="status">{note.text}</div>
  {/if}
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div class="out" bind:this={outEl} role="log" aria-label="Serial Monitor output" tabindex="0">
    {#each lines as l, i (i)}
      <div class="line" class:system={l.kind === "note"}>
        {#if timestamps && l.time}<span class="time">{`${l.time}  `}</span>{/if}{l.text}
      </div>
    {/each}
  </div>
  <div class="send">
    <input
      aria-label="Message to send"
      placeholder={liveKeys
        ? "Each key goes to the board as you type it"
        : "Type a message, then Enter to send it to the board"}
      bind:value={input}
      onkeydown={onKey}
      disabled={!open}
    />
    {#if !liveKeys}
      <button onclick={sendLine} disabled={!open}>Send</button>
    {/if}
  </div>
</div>

<style>
  .monitor {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--console-bg);
    color: var(--console-fg);
  }
  .bar,
  .send {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 4px 10px;
    font-size: 12px;
    background: var(--bg);
    color: var(--fg);
    border-bottom: 1px solid var(--border);
  }
  .send {
    border-top: 1px solid var(--border);
    border-bottom: 0;
  }
  .spacer {
    flex: 1;
  }
  .port {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--muted);
  }
  .dot.on {
    background: var(--status-ok-fg);
  }
  .field,
  .check {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--muted);
  }
  select,
  button,
  input {
    font: inherit;
  }
  .note {
    font-size: 12px;
    padding: 3px 10px;
    background: var(--banner-bg);
    color: var(--fg);
  }
  .note.warn {
    background: var(--warning-mark);
    color: #1f2328;
  }
  .out {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 6px 12px;
    font-family: ui-monospace, Menlo, Consolas, monospace;
    font-size: 12px;
    outline: none;
  }
  .line {
    white-space: pre-wrap;
    word-break: break-word;
    min-height: 1.2em;
  }
  .line.system,
  .time {
    color: var(--muted);
    font-style: italic;
  }
  .time {
    font-style: normal;
  }
  .send input {
    flex: 1;
    min-width: 160px;
    font-family: ui-monospace, Menlo, Consolas, monospace;
    padding: 3px 6px;
  }
</style>
