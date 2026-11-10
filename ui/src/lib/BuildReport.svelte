<!--
  Project > Build Report… (also the status bar's Build Report link after
  Verify or Upload): the last build's resources and clocks, as Logix
  Engine's `build` gives them (its recipe "Show problems in the editor",
  docs/tool-api.md). Each resource by the toolchain's own name, which
  people search for and quote, with the engine's short description; the
  used ones by default, since most are unused, especially on Xilinx FPGAs.
-->
<script lang="ts">
  import { backend, type BuildReport } from "./backend";
  import { mhzText, percentText, reportRows, reportText, unusedCount } from "./buildReport";

  let {
    report,
    title,
    showUnused = $bindable(false),
    onClose,
  }: {
    report: BuildReport;
    /** What was built, e.g. "blinky · iCESugar v1.5 · 10:42". */
    title: string;
    /** Show unused resources too; kept by the app between builds. */
    showUnused?: boolean;
    onClose: () => void;
  } = $props();

  let filter = $state("");
  let copied = $state(false);
  let error = $state<string | null>(null);

  const rows = $derived(reportRows(report, showUnused, filter));
  const unused = $derived(unusedCount(report));

  async function copy() {
    try {
      await backend.copyText(reportText(`Build Report: ${title}`, rows, report));
      copied = true;
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
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
    aria-label="Build Report"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
  >
    <header>
      <h2>Build Report</h2>
      <span class="muted title">{title}</span>
      <span class="spacer"></span>
      <button class="close" title="Close" onclick={onClose}>✕</button>
    </header>
    <div class="tools">
      <label>
        <input type="checkbox" bind:checked={showUnused} />
        Show unused resources ({unused})
      </label>
      <span class="spacer"></span>
      <input
        class="filter"
        type="search"
        placeholder="Filter"
        aria-label="Filter resources"
        bind:value={filter}
      />
    </div>
    <div class="body">
      <table class="resources" aria-label="Resources">
        <thead>
          <tr>
            <th>Resource</th>
            <th>Description</th>
            <th class="num">Used</th>
            <th class="num">Available</th>
            <th class="num">%</th>
          </tr>
        </thead>
        <tbody>
          {#each rows as r (r.name)}
            <tr class:unused={r.used === 0}>
              <td class="name">{r.name}</td>
              <td>{r.description ?? ""}</td>
              <td class="num">{r.used.toLocaleString()}</td>
              <td class="num">{r.available.toLocaleString()}</td>
              <td class="num" class:invalid={r.percent === null}
                title={r.percent === null ? "The tools reported numbers that don't fit" : undefined}
                >{percentText(r)}</td
              >
            </tr>
          {:else}
            <tr><td colspan="5" class="muted">No resources match.</td></tr>
          {/each}
        </tbody>
      </table>
      {#if report.clocks.length > 0}
        <table class="clocks" aria-label="Clocks">
          <thead>
            <tr><th>Clock</th><th class="num">Max speed</th><th>Net</th></tr>
          </thead>
          <tbody>
            {#each report.clocks as c (c.net)}
              <tr>
                <td class="name">{c.name}</td>
                <td class="num">{mhzText(c.fmaxMhz)}</td>
                <td class="net">{c.net}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>
    <footer>
      <span class="muted">{error ?? (copied ? "Copied." : "")}</span>
      <span class="spacer"></span>
      <button class="primary" onclick={copy}>Copy</button>
      <button onclick={onClose}>Close</button>
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
    width: min(820px, 100%);
    max-height: min(640px, 100%);
    background: var(--bg);
    color: var(--fg);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.3);
    overflow: hidden;
  }
  header,
  footer,
  .tools {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 16px;
  }
  header {
    border-bottom: 1px solid var(--border);
  }
  .tools {
    padding-bottom: 4px;
    font-size: 13px;
  }
  footer {
    border-top: 1px solid var(--border);
  }
  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }
  .title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
  .close {
    border: none;
  }
  .primary {
    background: var(--accent, #00796b);
    border-color: var(--accent, #00796b);
    color: #fff;
  }
  .filter {
    width: 180px;
    padding: 3px 8px;
    font: inherit;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg);
    color: var(--fg);
  }
  .body {
    overflow: auto;
    padding: 4px 16px 12px;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
    user-select: text;
  }
  .clocks {
    margin-top: 16px;
  }
  th,
  td {
    padding: 4px 8px;
    border-bottom: 1px solid var(--border);
    text-align: left;
    vertical-align: top;
  }
  th {
    position: sticky;
    top: 0;
    background: var(--bg);
    font-weight: 600;
  }
  .num {
    text-align: right;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }
  .name,
  .net {
    font-family: ui-monospace, Menlo, Consolas, monospace;
    font-size: 12px;
    white-space: nowrap;
  }
  .unused {
    color: var(--muted);
  }
  .invalid {
    color: var(--muted);
    font-style: italic;
  }
  .muted {
    color: var(--muted);
    font-size: 12px;
  }
</style>
