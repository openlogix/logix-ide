<!--
  Help > System Info…: the report for bug reports, with Copy. The layout
  every Open Logix product uses (the website repo's `CLAUDE.md`, System info
  and the version tree): Logix IDE's facts, the developer settings when any
  is set, then the version tree, with the engine's own tree under Logix IDE
  (built in `system_info.rs`). Click an entry to open or close it; hover it
  (or reach it with the keyboard) for its details. Copy gives the text
  form, always the whole tree.
-->
<script lang="ts">
  import { backend, type SystemInfo, type VersionEntry } from "./backend";

  let { onClose }: { onClose: () => void } = $props();

  let info = $state<SystemInfo | null>(null);
  let error = $state<string | null>(null);
  let copied = $state(false);
  // Closed entries, by their path in the tree ("0", "0.1", ...). The top
  // two levels start open, the rest closed.
  let closed = $state(new Set<string>());

  $effect(() => {
    backend
      .systemInfo()
      .then((i) => {
        closed = new Set(deepPaths(i.versions, "0", 0));
        info = i;
      })
      .catch((e) => (error = (e as { message?: string }).message ?? String(e)));
  });

  /** The paths of entries below the top two levels that have children. */
  function deepPaths(e: VersionEntry, path: string, depth: number): string[] {
    const own = depth >= 2 && e.children.length > 0 ? [path] : [];
    return own.concat(e.children.flatMap((c, i) => deepPaths(c, `${path}.${i}`, depth + 1)));
  }

  function toggle(path: string) {
    const next = new Set(closed);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    closed = next;
  }

  // The details popup of the entry under the mouse or keyboard focus,
  // placed against the window (so the scrolling body doesn't cut it off):
  // below the row, or above it when there's no room below.
  let tip = $state<{ e: VersionEntry; left: number; top?: number; bottom?: number } | null>(
    null,
  );
  function showTip(e: VersionEntry, target: HTMLElement) {
    const r = target.getBoundingClientRect();
    const left = r.left + 22;
    tip =
      window.innerHeight - r.bottom > 240
        ? { e, left, top: r.bottom + 2 }
        : { e, left, bottom: window.innerHeight - r.top + 2 };
  }

  const kindLabel: Record<VersionEntry["kind"], string> = {
    program: "Open Logix program",
    package: "Open Logix package",
    third_party: "Third party, bundled unchanged",
  };

  async function copy() {
    if (info === null) return;
    try {
      await backend.copyText(info.text);
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

{#snippet entry(e: VersionEntry, path: string)}
  {@const open = e.children.length > 0 && !closed.has(path)}
  <li>
    <button
      class="row"
      class:leaf={e.children.length === 0}
      aria-expanded={e.children.length > 0 ? open : undefined}
      onclick={() => e.children.length > 0 && toggle(path)}
      onmouseenter={(ev) => showTip(e, ev.currentTarget)}
      onmouseleave={() => (tip = null)}
      onfocus={(ev) => showTip(e, ev.currentTarget)}
      onblur={() => (tip = null)}
    >
      <span class="twisty">{e.children.length > 0 ? (open ? "▼" : "▶") : ""}</span>
      <span class="name">{e.name}</span>
      <span class="version">{e.version}</span>
      {#if e.channel}<span class="channel">· {e.channel}</span>{/if}
      {#if e.overriddenBy}<span class="override">dev override</span>{/if}
    </button>
    {#if open}
      <ul>
        {#each e.children as child, i (i)}{@render entry(child, `${path}.${i}`)}{/each}
      </ul>
    {/if}
  </li>
{/snippet}

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="overlay" onclick={onClose}>
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-label="System Info"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
  >
    <header>
      <h2>System Info</h2>
      <span class="spacer"></span>
      <button class="close" title="Close" onclick={onClose}>✕</button>
    </header>
    <div class="body">
      {#if error}
        <p class="error">{error}</p>
      {:else if info === null}
        <p class="muted">Collecting…</p>
      {:else}
        <div class="facts">
          <div class="product">{info.versions.name} {info.versions.version}</div>
          {#each info.facts as f (f.label)}
            <div>{f.label}: <span class="value">{f.value}</span></div>
          {/each}
        </div>
        {#if info.developerSettings.length > 0}
          <h3>Developer settings</h3>
          <dl class="settings">
            {#each info.developerSettings as s (s.name)}<dt>{s.name}</dt><dd>{s.value}</dd>{/each}
          </dl>
        {/if}
        <h3>Versions</h3>
        <ul class="tree">{@render entry(info.versions, "0")}</ul>
      {/if}
    </div>
    {#if tip}
      <div
        class="details"
        role="tooltip"
        style:left="{tip.left}px"
        style:top={tip.top === undefined ? null : `${tip.top}px`}
        style:bottom={tip.bottom === undefined ? null : `${tip.bottom}px`}
      >
        <div class="kind">{kindLabel[tip.e.kind]}</div>
        {#each tip.e.notes as note (note)}<div class="note">{note}</div>{/each}
        {#if tip.e.details.length > 0}
          <dl>
            {#each tip.e.details as d (d.label)}<dt>{d.label}</dt><dd>{d.value}</dd>{/each}
          </dl>
        {/if}
      </div>
    {/if}
    <footer>
      <span class="muted">{copied ? "Copied." : "Paste it into a bug report."}</span>
      <span class="spacer"></span>
      <button class="primary" disabled={info === null} onclick={copy}>Copy</button>
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
    width: min(720px, 100%);
    max-height: min(640px, 100%);
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
    border-top: 1px solid var(--border);
  }
  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }
  h3 {
    margin: 14px 0 4px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
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
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .body {
    overflow: auto;
    padding: 12px 16px 16px;
    user-select: text;
  }
  .facts {
    display: grid;
    gap: 2px;
    font-size: 13px;
    color: var(--muted);
  }
  .product {
    font-size: 14px;
    font-weight: 600;
    color: var(--fg);
    margin-bottom: 2px;
  }
  .value,
  dd,
  .version {
    font-family: ui-monospace, Menlo, Consolas, monospace;
    font-size: 12px;
  }
  .value {
    color: var(--fg);
    overflow-wrap: anywhere;
  }
  dl {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: 3px 14px;
    margin: 0;
    font-size: 12px;
  }
  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
  .settings {
    padding: 8px 12px;
    border: 1px solid var(--warning-mark);
    border-radius: 6px;
  }
  .settings dt {
    font-family: ui-monospace, Menlo, Consolas, monospace;
    color: var(--warning-mark);
  }
  .tree,
  .tree ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .tree ul {
    margin-left: 14px;
    padding-left: 6px;
    border-left: 1px dashed var(--border);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 3px 6px;
    border: none;
    border-radius: 4px;
    background: transparent;
    text-align: left;
    white-space: nowrap;
  }
  .row:hover {
    background: var(--tabs-bg);
  }
  .row.leaf {
    cursor: default;
  }
  .twisty {
    width: 10px;
    flex: none;
    font-size: 9px;
    color: var(--muted);
  }
  .name {
    font-weight: 600;
  }
  .version,
  .channel {
    color: var(--muted);
  }
  .channel {
    font-size: 12px;
  }
  .override {
    padding: 0 6px;
    font-size: 11px;
    border: 1px solid var(--warning-mark);
    border-radius: 99px;
    color: var(--warning-mark);
  }
  .details {
    position: fixed;
    z-index: 30;
    width: max-content;
    max-width: min(480px, 80vw);
    padding: 8px 12px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.25);
    pointer-events: none;
    white-space: normal;
  }
  .kind {
    margin-bottom: 6px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
  }
  .note {
    margin-bottom: 4px;
    font-size: 12px;
  }
  .muted {
    color: var(--muted);
    font-size: 12px;
  }
  .error {
    color: var(--error-mark);
  }
</style>
