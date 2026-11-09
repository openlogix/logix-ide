<!--
  Tools > Board Explorer… (and the ⓘ next to the board): the FPGA families,
  FPGAs and boards Logix Engine knows (the project's own boards included)
  and Logix's examples, and how they relate (the user's design, 2026-11-08:
  one regular model, every item linking to the others). The left side lists
  one kind at a time (tabs), grouped by family like the board picker; the
  right side shows the selected item's facts and links, Back returning to
  the one before. Select This Board picks a board, Open Example opens an
  example. The model and the words are in `explorer.ts`.
-->
<script lang="ts">
  import { untrack } from "svelte";
  import { backend, type Board, type Example, type FpgaFamily } from "./backend";
  import { expanded, expandGroup, toggleGroup } from "./expandedGroups.svelte";
  import {
    KINDS,
    buildModel,
    detailsOf,
    familyOfRef,
    listGroups,
    sameRef,
    type Kind,
    type Ref,
  } from "./explorer";

  let {
    boards,
    examples,
    initial,
    selected,
    onPickBoard,
    onOpenExample,
    onClose,
  }: {
    /** The boards offered (the catalog's and the project's). */
    boards: Board[];
    examples: Example[];
    /** The item shown first, such as the toolbar's board; null for none. */
    initial: Ref | null;
    /** The toolbar's board, marked in its details. */
    selected: string | null;
    /** Select This Board. */
    onPickBoard: (id: string) => void;
    /** Open Example. */
    onOpenExample: (example: Example) => void;
    onClose: () => void;
  } = $props();

  let families = $state<FpgaFamily[] | null>(null);
  let error = $state<string | null>(null);
  let query = $state("");
  /** The items shown, the current one last (Back goes to the one before). */
  let history = $state<Ref[]>(untrack(() => (initial ? [initial] : [])));
  let tab = $state<Kind>(untrack(() => initial?.kind ?? "board"));

  $effect(() => {
    backend
      .getFpgaFamilies()
      .then((f) => (families = f))
      .catch((e) => (error = (e as { message?: string }).message ?? String(e)));
  });

  const model = $derived(buildModel(families ?? [], boards, examples));
  const current = $derived(history.length ? history[history.length - 1] : null);
  const details = $derived(current && families ? detailsOf(model, current) : null);
  const groups = $derived(families ? listGroups(model, tab, query) : []);
  const searching = $derived(query.trim() !== "");
  const currentBoard = $derived(current?.kind === "board" ? model.boards.get(current.id) : undefined);
  const currentExample = $derived(current?.kind === "example" ? model.examples.get(current.id) : undefined);

  // The current item's group starts expanded, in the list of its kind.
  $effect(() => {
    if (!families || !current) return;
    const f = familyOfRef(model, current);
    if (f && current.kind !== "family") untrack(() => expandGroup(f.label));
  });

  /** Shows an item: its details, and its kind's list. */
  function show(ref: Ref) {
    if (sameRef(ref, current)) return;
    history = [...history, ref];
    tab = ref.kind;
  }

  function back() {
    if (history.length < 2) return;
    history = history.slice(0, -1);
    tab = history[history.length - 1].kind;
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
  }

  /** Keeps the selected item in view in the list. */
  function scrollIfSelected(node: HTMLElement, isSelected: boolean) {
    if (isSelected) node.scrollIntoView({ block: "nearest" });
    return {
      update(now: boolean) {
        if (now) node.scrollIntoView({ block: "nearest" });
      },
    };
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
    aria-label="Board Explorer"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
  >
    <header>
      <h2>Board Explorer</h2>
      <!-- svelte-ignore a11y_autofocus -->
      <input
        type="search"
        placeholder="Search {KINDS.find((k) => k.kind === tab)?.label.toLowerCase()}"
        aria-label="Search"
        bind:value={query}
        spellcheck="false"
        autocomplete="off"
        autofocus
      />
      <button class="close" title="Close" onclick={onClose}>✕</button>
    </header>

    <div class="body">
      <div class="side">
        <div class="kinds" role="tablist" aria-label="What to list">
          {#each KINDS as k (k.kind)}
            <button role="tab" aria-selected={tab === k.kind} class:active={tab === k.kind} onclick={() => (tab = k.kind)}
              >{k.label}</button
            >
          {/each}
        </div>
        <ul class="list" aria-label={KINDS.find((k) => k.kind === tab)?.label}>
          {#if error}
            <li class="empty error">{error}</li>
          {:else if families === null}
            <li class="empty">Loading…</li>
          {:else}
            {#each groups as g (g.key)}
              {@const open = g.label === null || searching || expanded.has(g.label)}
              {#if g.label !== null}
                <li class="group">
                  <button class="group-toggle" aria-expanded={open} onclick={() => toggleGroup(g.label!)}
                    ><svg
                      class="chevron"
                      class:open
                      viewBox="0 0 12 12"
                      width="12"
                      height="12"
                      aria-hidden="true"
                      ><path
                        d="M4 2.5 L8 6 L4 9.5"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="1.8"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                      /></svg
                    >
                    <span class="group-label">{g.label}</span>
                    <span class="count">{g.items.length}</span></button
                  >
                </li>
              {/if}
              {#each open ? g.items : [] as item (item.ref.id)}
                {@const isSel = sameRef(item.ref, current)}
                <li>
                  <button class="item" class:selected={isSel} onclick={() => show(item.ref)} use:scrollIfSelected={isSel}>
                    <span class="item-label">{item.label}</span>
                    <span class="hint">{item.hint}</span>
                  </button>
                </li>
              {/each}
            {:else}
              <li class="empty">No matches</li>
            {/each}
          {/if}
        </ul>
      </div>

      <div class="details" aria-label="Details">
        {#if details}
          <div class="kind-label">{details.kindLabel}</div>
          <h3>{details.title}</h3>
          <table class="facts">
            <tbody>
              {#each details.facts as f (f.label)}
                <tr>
                  <th scope="row">{f.label}</th>
                  <td>
                    {#if f.link}<button class="link" onclick={() => show(f.link!)}>{f.value}</button
                      >{:else if f.code}<code>{f.value}</code>{:else}{f.value}{/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
          {#each details.sections as s (s.title)}
            <h4 class="section">{s.title}</h4>
            {#if s.kind === "lines"}
              <ul class="lines">
                {#each s.lines as line, i (i)}<li>{line}</li>{/each}
              </ul>
            {:else if s.kind === "links"}
              {#if s.links.length === 0}
                <p class="muted">{s.empty}</p>
              {:else}
                <ul class="links">
                  {#each s.links as l (l.ref.kind + l.ref.id)}
                    <li>
                      <button class="link" onclick={() => show(l.ref)}>{l.label}</button>
                      {#if l.hint}<span class="muted">{l.hint}</span>{/if}
                    </li>
                  {/each}
                </ul>
              {/if}
            {:else if s.rows.length === 0}
              <p class="muted">{s.empty}</p>
            {:else}
              <table class="resources">
                <thead><tr><th>Port</th><th>Type</th><th>Pins</th><th>Notes</th></tr></thead>
                <tbody>
                  {#each s.rows as r (r.name)}
                    <tr><td><code>{r.name}</code></td><td>{r.type}</td><td>{r.pins}</td><td>{r.notes}</td></tr>
                  {/each}
                </tbody>
              </table>
            {/if}
          {/each}
        {:else if families !== null}
          <p class="muted placeholder">Select a family, FPGA, board or example to see its details.</p>
        {/if}
      </div>
    </div>

    <footer>
      {#if history.length > 1}
        <button class="secondary" onclick={back} title="The item shown before">Back</button>
      {/if}
      <span class="spacer"></span>
      <button class="secondary" onclick={onClose}>Close</button>
      {#if currentBoard}
        <button
          class="primary"
          disabled={currentBoard.id === selected}
          title={currentBoard.id === selected ? "It's the selected board" : undefined}
          onclick={() => onPickBoard(currentBoard!.id)}>Select This Board</button
        >
      {:else if currentExample}
        <button class="primary" onclick={() => onOpenExample(currentExample!)}>Open Example</button>
      {/if}
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
    width: min(980px, 100%);
    height: min(640px, 100%);
    background: var(--bg);
    color: var(--fg);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.3);
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
  }
  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }
  header input {
    flex: 1;
    padding: 5px 8px;
    font: inherit;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg);
    color: var(--fg);
  }
  .close {
    border: none;
    background: none;
    color: var(--muted);
    font-size: 15px;
    cursor: pointer;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(240px, 2fr) 3fr;
  }
  .side {
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-right: 1px solid var(--border);
    background: var(--tabs-bg);
  }
  /* The kinds, as tabs. */
  .kinds {
    display: flex;
    border-bottom: 1px solid var(--border);
  }
  .kinds button {
    flex: 1;
    padding: 7px 4px 6px;
    border: none;
    border-bottom: 2px solid transparent;
    background: none;
    color: var(--muted);
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .kinds button:hover {
    color: var(--fg);
  }
  .kinds button.active {
    border-bottom-color: var(--accent);
    color: var(--fg);
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  /* Family headings, as in the board picker. */
  .group {
    position: sticky;
    top: 0;
    z-index: 1;
    margin-top: 1px;
  }
  .group-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 6px 12px 4px;
    border: none;
    background: var(--group-bg);
    color: var(--group-fg);
    font: inherit;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    text-align: left;
    cursor: pointer;
  }
  .group-label {
    flex: 1;
    min-width: 0;
  }
  .chevron {
    flex: none;
    transition: transform 0.1s;
  }
  .chevron.open {
    transform: rotate(90deg);
  }
  .count {
    font-weight: 400;
  }
  .item {
    display: flex;
    align-items: baseline;
    gap: 8px;
    width: 100%;
    padding: 5px 12px 5px 30px;
    border: none;
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }
  .item:hover {
    background: var(--banner-bg);
  }
  .item.selected {
    background: var(--accent);
    color: #fff;
  }
  .item-label {
    flex: 1;
    min-width: 0;
  }
  .hint {
    color: var(--muted);
    font-size: 12px;
    white-space: nowrap;
  }
  .item.selected .hint {
    color: inherit;
  }
  .empty {
    padding: 8px 14px;
    color: var(--muted);
  }
  .details {
    min-height: 0;
    overflow: auto;
    padding: 12px 18px 16px;
    font-size: 13px;
  }
  .kind-label {
    color: var(--muted);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  h3 {
    margin: 2px 0 8px;
    font-size: 16px;
    font-weight: 600;
  }
  .facts {
    border-collapse: collapse;
  }
  .facts th {
    padding: 3px 16px 3px 0;
    color: var(--muted);
    font-weight: normal;
    text-align: left;
    vertical-align: top;
    white-space: nowrap;
  }
  .facts td {
    padding: 3px 0;
  }
  /* Section headings, as the Examples window's. */
  .section {
    margin: 14px 0 6px;
    padding: 5px 10px 4px;
    background: var(--group-bg);
    color: var(--group-fg);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .lines {
    padding-left: 20px;
    list-style: disc;
  }
  .lines li,
  .links li {
    padding: 2px 0;
  }
  .links li {
    display: flex;
    gap: 8px;
    align-items: baseline;
  }
  .link {
    padding: 0;
    border: none;
    background: none;
    color: var(--accent);
    font: inherit;
    text-align: left;
    text-decoration: underline;
    cursor: pointer;
  }
  .resources {
    width: 100%;
    border-collapse: collapse;
  }
  .resources th {
    padding: 3px 8px 3px 0;
    color: var(--muted);
    font-weight: normal;
    text-align: left;
    border-bottom: 1px solid var(--border);
  }
  .resources td {
    padding: 3px 8px 3px 0;
    vertical-align: top;
  }
  code {
    font-size: 12px;
  }
  .muted {
    color: var(--muted);
    font-size: 12px;
  }
  .placeholder {
    margin-top: 24px;
    text-align: center;
  }
  .error {
    color: var(--error-mark);
  }
  footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 16px;
    border-top: 1px solid var(--border);
  }
  .spacer {
    flex: 1;
  }
  /* Arduino-style buttons, as in the Examples window. */
  footer button {
    min-width: 84px;
    padding: 6px 16px;
    font: inherit;
    font-size: 13px;
    font-weight: 600;
    letter-spacing: 0.02em;
    border: 1px solid var(--accent);
    border-radius: 4px;
    cursor: pointer;
  }
  footer button.secondary {
    background: transparent;
    color: var(--accent);
  }
  footer button.primary {
    background: var(--accent);
    color: #ffffff;
  }
  footer button:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
