<!-- Debug: choose the nets to watch and drive, the first time a project is
     debugged (no nets in its debug.toml), and from Project > Choose Debug
     Nets…, with the file's choice ticked. The nets are Logix Engine's
     list_nets; the choice is saved in debug.toml (debug_file.rs), which
     keeps whatever else the file says. -->
<script lang="ts">
  import { arrowNav } from "./arrowNav";
  import type { DebugConfig, DebugNet } from "./backend";

  let {
    nets,
    config,
    onChoose,
    onClose,
  }: {
    /** The design's nets, sorted by path. */
    nets: DebugNet[];
    /** What the project's debug.toml already says. */
    config: DebugConfig;
    onChoose: (clock: string, watch: string[], drive: string[]) => void;
    onClose: () => void;
  } = $props();

  /** The clocks to offer: 1-bit inputs, those named like a clock first. */
  const clocks = $derived(
    nets
      .filter((n) => n.kind === "input" && n.width === 1)
      .sort((a, b) => Number(isClockName(b)) - Number(isClockName(a))),
  );

  function isClockName(n: DebugNet): boolean {
    return /(^|[._])(clk|clock)/i.test(n.path.split(".").pop() ?? "");
  }

  /** Debug files name the clock by its port name in the top module. */
  function portName(path: string): string {
    return path.split(".").slice(1).join(".");
  }

  // The initial choice comes from the props, once (the dialog edits it).
  // svelte-ignore state_referenced_locally
  let clock = $state(config.clock || (clocks[0] ? portName(clocks[0].path) : ""));
  // svelte-ignore state_referenced_locally
  let watch = $state(new Set(config.watch));
  // svelte-ignore state_referenced_locally
  let drive = $state(new Set(config.drive));

  /** What a debug file may do with each net comes from list_nets
   *  (`can_watch`, `can_drive`, `why_not`): Logix Engine knows, and refuses
   *  the rest at build time (its `docs/debugger.md`). Every net is listed,
   *  and what it can't do is grayed out, with why as its hover text. */

  function toggle(set: Set<string>, path: string): Set<string> {
    const next = new Set(set);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    return next;
  }

  const shown = $derived(nets.filter((n) => portName(n.path) !== clock));
  const chosen = $derived(watch.size + drive.size > 0);

  function choose() {
    const keep = (s: Set<string>, can: (n: DebugNet) => boolean) =>
      nets.filter((n) => s.has(n.path) && can(n)).map((n) => n.path);
    onChoose(
      clock,
      keep(watch, (n) => n.can_watch),
      keep(drive, (n) => n.can_drive),
    );
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="overlay" onclick={onClose}>
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-label="Choose the Nets to Debug"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
  >
    <header>
      <h2>Debug which nets?</h2>
      <span class="spacer"></span>
      <button class="close" title="Cancel" onclick={onClose}>✕</button>
    </header>
    <div class="body">
      <p class="muted">
        Watch shows a net's value in the Debugger window; Drive lets you set
        it from there. Debug saves your choice in the project's debug.toml;
        to change it later, choose Project &gt; Choose Debug Nets….
      </p>
      {#if nets.length === 0}
        <p>The design has no named nets to debug.</p>
      {:else}
        <label class="clock">
          Clock
          <select bind:value={clock}>
            {#each clocks as n (n.path)}
              <option value={portName(n.path)}>{portName(n.path)}</option>
            {/each}
          </select>
        </label>
        <table use:arrowNav={{ items: "input" }}>
          <thead>
            <tr><th>Net</th><th>Bits</th><th>Watch</th><th>Drive</th></tr>
          </thead>
          <tbody>
            {#each shown as n (n.path)}
              <tr>
                <td class="name">{n.path}<span class="kind">{n.kind}</span></td>
                <td>{n.width}</td>
                <!-- The title is on the cell: a disabled checkbox shows none. -->
                <td title={n.can_watch ? undefined : n.why_not}>
                  <input
                    type="checkbox"
                    aria-label={`Watch ${n.path}`}
                    disabled={!n.can_watch}
                    checked={n.can_watch && watch.has(n.path)}
                    onchange={() => (watch = toggle(watch, n.path))}
                  />
                </td>
                <td title={n.can_drive ? undefined : n.why_not}>
                  <input
                    type="checkbox"
                    aria-label={`Drive ${n.path}`}
                    disabled={!n.can_drive}
                    checked={n.can_drive && drive.has(n.path)}
                    onchange={() => (drive = toggle(drive, n.path))}
                  />
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>
    <footer>
      <button onclick={onClose}>Cancel</button>
      <button class="primary" disabled={!chosen || !clock} onclick={choose}>Debug</button>
    </footer>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 20;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
    background: rgba(0, 0, 0, 0.35);
  }
  .dialog {
    display: flex;
    flex-direction: column;
    width: min(620px, 100%);
    max-height: min(560px, 100%);
    background: var(--bg);
    color: var(--fg);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.3);
    overflow: hidden;
  }
  header,
  footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 16px;
  }
  header {
    border-bottom: 1px solid var(--border);
  }
  footer {
    justify-content: flex-end;
    border-top: 1px solid var(--border);
  }
  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }
  .spacer {
    flex: 1;
  }
  button {
    padding: 3px 12px;
    font: inherit;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg);
    color: var(--fg);
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .primary {
    font-weight: 600;
  }
  .close {
    border: none;
  }
  .body {
    overflow: auto;
    padding: 4px 16px 14px;
    font-size: 13px;
  }
  .clock {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 6px 0 10px;
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  th {
    text-align: left;
    font-weight: 600;
    color: var(--muted);
    font-size: 12px;
  }
  th,
  td {
    padding: 5px 6px;
    border-bottom: 1px solid var(--border);
  }
  .name {
    font-family: var(--mono, monospace);
  }
  .kind {
    margin-left: 8px;
    color: var(--muted);
    font-family: inherit;
    font-size: 11px;
  }
  .muted {
    color: var(--muted);
    font-size: 12px;
  }
  input:disabled {
    opacity: 0.35;
  }
</style>
