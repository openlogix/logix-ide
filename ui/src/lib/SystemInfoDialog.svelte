<!--
  Help > System Info…: the report for bug reports, with Copy. The layout
  every Open Logix product uses (the website repo's `CLAUDE.md`, System info
  and the version tree): Logix IDE's facts, the developer settings when any
  is set, then the version tree, with the engine's own tree under Logix IDE
  (built in `system_info.rs`). The tree is always fully open; click an
  entry (or press Enter or Space on it) to show its details under it, and
  again to hide them (the user's request, 2026-11-17: the hover popup was
  hard to read). Copy gives the text form, always the whole tree.
-->
<script lang="ts">
  import { backend, type SystemInfo, type VersionEntry } from "./backend";

  // `refresh` changes whenever what the report shows may have changed (setup
  // ended, an update ran): the open dialog then reads it again by itself.
  let { onClose, refresh = 0 }: { onClose: () => void; refresh?: number } = $props();

  let info = $state<SystemInfo | null>(null);
  let error = $state<string | null>(null);
  let copied = $state(false);
  // The entry whose details are shown, by its path of names ("Logix IDE/
  // Logix Engine"), so a refresh that adds entries keeps the same one.
  let selected = $state<string | null>(null);

  $effect(() => {
    void refresh;
    backend
      .systemInfo()
      .then((i) => {
        info = i;
        error = null;
      })
      .catch((e) => (error = (e as { message?: string }).message ?? String(e)));
  });

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

{#snippet entry(e: VersionEntry, parent: string)}
  {@const path = parent === "" ? e.name : `${parent}/${e.name}`}
  {@const shown = selected === path}
  <li>
    <button
      class="row"
      class:selected={shown}
      aria-expanded={shown}
      onclick={() => (selected = shown ? null : path)}
    >
      <span class="name">{e.name}</span>
      <span class="version">{e.version}</span>
      {#if e.channel}<span class="channel">· {e.channel}</span>{/if}
      {#if e.overriddenBy}<span class="override">dev override</span>{/if}
      {#if e.problem}<span class="problem">{e.problem}</span>{/if}
    </button>
    {#if shown}
      <div class="details" role="region" aria-label="{e.name} details">
        <div class="kind">{kindLabel[e.kind]}</div>
        {#each e.notes as note (note)}<div class="note">{note}</div>{/each}
        {#if e.details.length > 0}
          <dl>
            {#each e.details as d, i (i)}<dt>{d.label}</dt><dd>{d.value}</dd>{/each}
          </dl>
        {/if}
      </div>
    {/if}
    {#if e.children.length > 0}
      <ul>
        {#each e.children as child, i (i)}{@render entry(child, path)}{/each}
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
        <ul class="tree">{@render entry(info.versions, "")}</ul>
      {/if}
    </div>
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
    /* Label and value on one text line, though the value's font differs. */
    align-items: baseline;
    margin: 0;
    font-size: 12px;
    line-height: 1.4;
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
  .row:hover,
  .row.selected {
    background: var(--tabs-bg);
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
  .problem {
    font-size: 12px;
    color: var(--error-mark);
  }
  .details {
    /* Indented as the entry's children are (the tree's ul: 14 + 6 px). */
    margin: 2px 0 6px 20px;
    padding: 8px 12px;
    border: 1px solid var(--border);
    border-radius: 6px;
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
