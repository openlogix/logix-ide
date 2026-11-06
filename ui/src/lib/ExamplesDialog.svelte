<!--
  Browse the examples by board and open one. With `onlyBoard` (after a
  board change in the toolbar), it's "Project options for <board>", one
  list of choices (the user's design, 2026-11-06): the current project on
  the new board (selected by default, with a note on what the change
  means), the recent projects for that board (`recents`), and the board's
  example projects. The main button says what the selected choice does:
  Change the Board, Open Project, or New Project (an example or Bare
  Minimum starts a new, unsaved project). Cancel, Esc and ✕ cancel the
  board change.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import type { Board, Example } from "./backend";
  import { groupExamples, type ExampleEntry } from "./examples";
  import { boardChangeNote, groupByArch } from "./boards";
  import { expandGroup, expanded, toggleGroup } from "./expandedGroups.svelte";
  import Icon from "./Icon.svelte";

  let {
    boards,
    examples,
    initialBoard,
    onlyBoard = null,
    recents = [],
    onOpenRecent = () => {},
    currentProject = null,
    onChangeBoard = () => {},
    onOpen,
    onClose,
  }: {
    boards: Board[];
    examples: Example[];
    /** Board to show first (e.g. the project's board). */
    initialBoard: string | null;
    /** Show only this board's examples (the board was just chosen). */
    onlyBoard?: string | null;
    /** Recent project folders for `onlyBoard`, listed first. */
    recents?: string[];
    onOpenRecent?: (dir: string) => void;
    /**
     * With `onlyBoard`: the project open now, offered first, to move to the
     * new board (its name, its board, and its pin file, for the note).
     */
    currentProject?: { name: string; board: Board | null; pinFile: string | null } | null;
    /** Change the board of the current project to `onlyBoard`. */
    onChangeBoard?: () => void;
    onOpen: (example: ExampleEntry) => void;
    /** Closed without a choice: after a board change, it's cancelled. */
    onClose: () => void;
  } = $props();

  let query = $state("");
  // svelte-ignore state_referenced_locally
  let selectedBoard = $state<string | null>(initialBoard);
  /** Selected example as "board/name". */
  let selectedKey = $state<string | null>(null);
  /** Selected recent project folder (instead of an example). */
  let selectedRecent = $state<string | null>(null);
  /**
   * The current project on the new board is selected. Nothing is selected
   * at first (the user's decision: changing the board, especially to
   * another FPGA or family, should be a choice, not a default).
   */
  let selectedCurrent = $state(false);
  const folderName = (dir: string) => dir.split(/[\\/]/).pop() ?? dir;
  /** Recent projects shown: all of them, or those matching the search. */
  const shownRecents = $derived(
    recents.filter((d) =>
      folderName(d).toLowerCase().includes(query.trim().toLowerCase()),
    ),
  );

  const key = (e: Example) => `${e.board}/${e.name}`;
  const groups = $derived(
    groupExamples(boards, examples, query).filter(
      (g) => !onlyBoard || g.board.id === onlyBoard,
    ),
  );
  /** The boards column, by FPGA architecture (collapsible groups). */
  const archGroups = $derived(groupByArch(groups, (g) => g.board));
  const searching = $derived(query.trim() !== "");

  // Show the initial board's group expanded (once, when opened).
  {
    // svelte-ignore state_referenced_locally
    const first = boards.find((b) => b.id === initialBoard);
    if (first) expandGroup(first.archLabel);
  }

  const onlyBoardInfo = $derived(boards.find((b) => b.id === onlyBoard) ?? null);
  const onlyBoardName = $derived(onlyBoardInfo?.description ?? onlyBoard);
  /** What moving the current project to the new board means: short points. */
  const changeNote = $derived(
    currentProject && onlyBoardInfo
      ? boardChangeNote(currentProject.board, onlyBoardInfo, currentProject.pinFile)
      : [],
  );
  /** The selected board's group, or the first one if it's filtered out. */
  const group = $derived(
    groups.find((g) => g.board.id === selectedBoard) ?? groups[0] ?? null,
  );
  /**
   * The clicked entry, or else the board's only example if it has
   * just one (or Bare Minimum if it has none).
   */
  const selectedExample = $derived.by(() => {
    if (selectedRecent || selectedCurrent) return null;
    const clicked = group?.examples.find((e) => key(e) === selectedKey);
    // Project options select nothing by themselves.
    if (clicked || !group || onlyBoard) return clicked ?? null;
    const real = group.examples.filter((e) => !e.bare);
    if (real.length === 1 && group.count === 1) return real[0];
    return group.count === 0 && group.examples.length === 1
      ? group.examples[0]
      : null;
  });

  function selectBoard(id: string) {
    selectedBoard = id;
    selectedKey = null;
    selectedRecent = null;
    selectedCurrent = false;
  }

  function selectExample(k: string) {
    selectedKey = k;
    selectedRecent = null;
    selectedCurrent = false;
  }

  function selectRecent(dir: string) {
    selectedRecent = dir;
    selectedCurrent = false;
  }

  function selectCurrent() {
    selectedCurrent = true;
    selectedRecent = null;
    selectedKey = null;
  }

  const canOpen = $derived(
    selectedCurrent || selectedRecent !== null || selectedExample !== null,
  );
  /**
   * The main button: what the selected choice does. An example, like Bare
   * Minimum, starts a new, unsaved project.
   */
  const openLabel = $derived(
    selectedCurrent
      ? "Change the Board"
      : selectedRecent !== null
        ? "Open Project"
        : selectedExample !== null || !onlyBoard
          ? "New Project"
          : "Choose an Option",
  );

  function open() {
    if (selectedCurrent) onChangeBoard();
    else if (selectedRecent) onOpenRecent(selectedRecent);
    else if (selectedExample) onOpen(selectedExample);
  }

  /**
   * Keys count only once the dialog is open: the Enter that picked a board
   * in the board list opens it and reaches it too, and with a choice
   * selected by default it would act at once.
   */
  let armed = false;
  onMount(() => {
    const timer = setTimeout(() => (armed = true));
    return () => clearTimeout(timer);
  });

  function onKey(e: KeyboardEvent) {
    if (!armed) return;
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    } else if (e.key === "Enter" && canOpen) {
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
      <h2>
        {onlyBoard ? `Project options for ${onlyBoardName}` : "Examples"}
      </h2>
      <!-- svelte-ignore a11y_autofocus -->
      <input
        type="search"
        placeholder={onlyBoard ? "Search projects" : "Search boards and examples"}
        bind:value={query}
        spellcheck="false"
        autocomplete="off"
        autofocus
      />
      <button
        class="close"
        title={onlyBoard ? "Cancel: keep the board as it was" : "Close"}
        onclick={onClose}>✕</button
      >
    </header>


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

      <ul class="examples" aria-label={onlyBoard ? "Project options" : "Examples"}>
        {#if onlyBoard && currentProject}
          <li class="section">Current project</li>
          <li>
            <button
              class="current"
              class:selected={selectedCurrent}
              onclick={selectCurrent}
              ondblclick={onChangeBoard}
            >
              <span class="example-name"
                >Change the board of “{currentProject.name}” to {onlyBoardName}</span
              >
              <!-- The change, then its implications, as list items (spans: a
                   button can't hold a list). -->
              {#if changeNote.length > 0}
                <span class="example-desc note">
                  {#each changeNote as point, i (i)}<span
                      class="point"
                      class:change={point.kind === "change"}
                      class:risky={point.kind === "risky"}>{point.text}</span
                    >{/each}
                </span>
              {/if}
            </button>
          </li>
        {/if}
        {#if shownRecents.length > 0}
          <li class="section">Recent projects</li>
          {#each shownRecents as dir (dir)}
            <li>
              <button
                class="recent"
                class:selected={selectedRecent === dir}
                title={dir}
                onclick={() => selectRecent(dir)}
                ondblclick={() => onOpenRecent(dir)}
              >
                <Icon name="folder" size={16} />
                <span class="example-name">{folderName(dir)}</span>
                <span class="example-desc">{dir.slice(0, -folderName(dir).length - 1)}</span>
              </button>
            </li>
          {/each}
        {/if}
        {#if onlyBoard}
          <li class="section">Example projects</li>
        {/if}
        {#if group}
          {#each group.examples as ex (ex.name)}
            <li>
              <button
                class:selected={selectedExample !== null &&
                  key(ex) === key(selectedExample)}
                onclick={() => selectExample(key(ex))}
                ondblclick={() => onOpen(ex)}
              >
                <span class="example-name">{ex.name}</span>
                <!-- The summary, and the notes of the selected example: a
                     bullet each (spans as list items: a button can't hold
                     a list). -->
                <span class="example-desc note">
                  <span class="point">{ex.summary}</span>
                  {#if selectedExample !== null && key(ex) === key(selectedExample)}
                    {#each ex.notes as n, i (i)}<span class="point">{n}</span>{/each}
                  {/if}
                </span>
              </button>
            </li>
          {/each}
        {/if}
      </ul>
    </div>

    <footer>
      <button class="secondary" onclick={onClose}>Cancel</button>
      <button class="primary" disabled={!canOpen} onclick={open}
        >{openLabel}</button
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
  .examples button.recent {
    flex-direction: row;
    align-items: center;
    gap: 8px;
  }
  .recent :global(svg) {
    flex: none;
    color: var(--accent);
  }
  .recent.selected :global(svg) {
    color: inherit;
  }
  .recent .example-desc {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* Section headings stand out from the entries, like the board picker's
     group headings (the user's request). */
  .section {
    margin-top: 6px;
    padding: 6px 14px 5px;
    background: var(--group-bg);
    color: var(--group-fg);
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }
  .section:first-child {
    margin-top: 0;
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
  /* Project options: the selection's green background covers only the
     choice's title, a band across the row; its note below stays as it is
     (the user's request). A recent project is one line: all of it. */
  .single li button.selected:not(.recent) {
    background: transparent;
    color: inherit;
  }
  .single li button.selected:not(.recent):hover {
    background: var(--banner-bg);
  }
  .single .selected:not(.recent) .example-name {
    margin: -6px -14px 0;
    padding: 6px 14px 4px;
    background: var(--accent);
    color: #fff;
  }
  .single .selected:not(.recent) .example-desc {
    color: var(--muted);
  }
  /* The current-project choice's note: short bullet points. */
  .note {
    display: block;
    margin-top: 2px;
    padding-left: 18px;
  }
  .note .point {
    display: list-item;
    list-style: disc;
  }
  /* The first point: the change itself. */
  .note .point.change {
    color: var(--fg);
  }
  /* A risky consequence of changing the board: reddish (the user's request). */
  .note .point.risky {
    color: var(--error-mark);
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
