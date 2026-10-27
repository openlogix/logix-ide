<!--
  Browse Apio's examples by board and open one. With `onlyBoard` (after a
  board change in the toolbar), only that board's examples are shown and
  closing keeps the current files.
-->
<script lang="ts">
  import type { Board, Example } from "./backend";
  import { groupExamples, type ExampleEntry } from "./examples";
  import { archLabel, groupByArch } from "./boards";
  import { expandGroup, expanded, toggleGroup } from "./expandedGroups.svelte";

  let {
    boards,
    examples,
    initialBoard,
    onlyBoard = null,
    onOpen,
    onClose,
  }: {
    boards: Board[];
    examples: Example[];
    /** Board to show first (e.g. the project's board). */
    initialBoard: string | null;
    /** Show only this board's examples (the board was just chosen). */
    onlyBoard?: string | null;

    onOpen: (example: ExampleEntry) => void;
    onClose: () => void;
  } = $props();

  let query = $state("");
  // svelte-ignore state_referenced_locally
  let selectedBoard = $state<string | null>(initialBoard);
  /** Selected example as "board/name". */
  let selectedKey = $state<string | null>(null);

  const key = (e: Example) => `${e.board}/${e.name}`;
  const groups = $derived(
    groupExamples(boards, examples, query).filter(
      (g) => !onlyBoard || g.board.id === onlyBoard,
    ),
  );
  /** The boards column, by FPGA architecture (collapsible groups). */
  const archGroups = $derived(groupByArch(groups, (g) => g.board.arch));
  const searching = $derived(query.trim() !== "");

  // Show the initial board's group expanded (once, when opened).
  {
    // svelte-ignore state_referenced_locally
    const first = boards.find((b) => b.id === initialBoard);
    if (first) expandGroup(archLabel(first.arch));
  }

  const onlyBoardName = $derived(
    boards.find((b) => b.id === onlyBoard)?.description ?? onlyBoard,
  );
  /** The selected board's group, or the first one if it's filtered out. */
  const group = $derived(
    groups.find((g) => g.board.id === selectedBoard) ?? groups[0] ?? null,
  );
  /**
   * The clicked entry, or else the board's only example from Apio if it has
   * just one (or BareMinimum if it has none).
   */
  const selectedExample = $derived.by(() => {
    const clicked = group?.examples.find((e) => key(e) === selectedKey);
    if (clicked || !group) return clicked ?? null;
    const fromApio = group.examples.filter((e) => !e.bare);
    if (fromApio.length === 1 && group.count === 1) return fromApio[0];
    return group.count === 0 && group.examples.length === 1
      ? group.examples[0]
      : null;
  });

  function selectBoard(id: string) {
    selectedBoard = id;
    selectedKey = null;
  }

  function open() {
    if (selectedExample) onOpen(selectedExample);
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    } else if (e.key === "Enter" && selectedExample) {
      e.preventDefault();
      open();
    }
  }

  // Scroll the initially selected board into view.
  function scrollIntoViewIfSelected(node: HTMLElement, selected: boolean) {
    if (selected) node.scrollIntoView({ block: "center" });
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
    aria-label="Examples"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
  >
    <header>
      <h2>{onlyBoard ? `Examples for ${onlyBoardName}` : "Examples"}</h2>
      <!-- svelte-ignore a11y_autofocus -->
      <input
        type="search"
        placeholder={onlyBoard ? "Search examples" : "Search boards and examples"}
        bind:value={query}
        spellcheck="false"
        autocomplete="off"
        autofocus
      />
      <button
        class="close"
        title={onlyBoard ? "Keep current files (Esc)" : "Close (Esc)"}
        onclick={onClose}>✕</button
      >
    </header>

    {#if onlyBoard}
      <p class="hint">
        Start from one of the board's examples, or keep your current files.
      </p>
    {/if}

    <div class="body" class:single={onlyBoard}>
      {#if !onlyBoard}
      <ul class="boards" aria-label="Boards">
        {#each archGroups as ag (ag.label)}
          {@const shown = searching || expanded.has(ag.label)}
          <li class="arch">
            <button
              class="arch-toggle"
              aria-expanded={shown}
              onclick={() => toggleGroup(ag.label)}
              ><svg class="chevron" class:open={shown} viewBox="0 0 12 12" width="12" height="12" aria-hidden="true"
                  ><path d="M4 2.5 L8 6 L4 9.5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" /></svg
                >
              {ag.label}
              <span class="arch-count">{ag.items.length}</span></button
            >
          </li>
          {#each shown ? ag.items : [] as g (g.board.id)}
            <li>
              <button
                class:selected={g.board.id === group?.board.id}
                onclick={() => selectBoard(g.board.id)}
                use:scrollIntoViewIfSelected={g.board.id === initialBoard}
              >
                <span class="board-name">{g.board.description}</span>
                <span class="count">{g.count}</span>
              </button>
            </li>
          {/each}
        {:else}
          <li class="empty">No matches</li>
        {/each}
      </ul>
      {/if}

      <ul class="examples" aria-label="Examples">
        {#if group}
          {#each group.examples as ex (ex.name)}
            <li>
              <button
                class:selected={selectedExample !== null &&
                  key(ex) === key(selectedExample)}
                onclick={() => (selectedKey = key(ex))}
                ondblclick={() => onOpen(ex)}
              >
                <span class="example-name">{ex.name}</span>
                <span class="example-desc">{ex.description}</span>
              </button>
            </li>
          {/each}
        {/if}
      </ul>
    </div>

    <footer>
      <button class="secondary" onclick={onClose}
        >{onlyBoard ? "Keep Current Files" : "Cancel"}</button
      >
      <button class="primary" disabled={!selectedExample} onclick={open}
        >Open</button
      >
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
    height: min(560px, 100%);
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
  .body {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(200px, 2fr) 3fr;
  }
  ul {
    margin: 0;
    padding: 4px 0;
    list-style: none;
    overflow: auto;
  }
  .body.single {
    grid-template-columns: 1fr;
  }
  .hint {
    margin: 0;
    padding: 8px 16px;
    font-size: 13px;
    color: var(--muted);
    border-bottom: 1px solid var(--border);
  }
  .boards {
    border-right: 1px solid var(--border);
    background: var(--tabs-bg);
  }
  .arch {
    position: sticky;
    top: -4px;
    z-index: 1;
    background: var(--tabs-bg);
  }
  .boards .arch-toggle {
    align-items: center;
    gap: 6px;
    padding: 8px 14px 4px;
    color: var(--muted);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .boards .arch-toggle:hover {
    background: transparent;
    color: var(--fg);
  }
  .chevron {
    flex: none;
    transition: transform 0.1s;
  }
  .chevron.open {
    transform: rotate(90deg);
  }
  .arch-count {
    margin-left: auto;
    font-weight: 400;
  }
  li button {
    display: flex;
    width: 100%;
    gap: 8px;
    padding: 6px 14px;
    border: none;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  li button:hover {
    background: var(--banner-bg);
  }
  li button.selected {
    background: var(--accent);
    color: #fff;
  }
  .board-name {
    flex: 1;
  }
  .count {
    color: var(--muted);
    font-size: 12px;
  }
  .selected .count {
    color: inherit;
  }
  .examples button {
    flex-direction: column;
    gap: 2px;
  }
  .example-name {
    font-weight: 600;
  }
  .example-desc {
    font-size: 12px;
    color: var(--muted);
  }
  .selected .example-desc {
    color: inherit;
  }
  .empty {
    padding: 8px 14px;
    color: var(--muted);
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 10px 16px;
    border-top: 1px solid var(--border);
  }
  .close {
    border: none;
    background: none;
    color: var(--muted);
    font-size: 15px;
    cursor: pointer;
  }
  /* Arduino-style buttons, as in Settings: outlined secondary, teal primary. */
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
