<!--
  The toolbar's board picker, like Arduino IDE 2's board selector: a button
  with the board's name that opens a searchable list, the connected boards
  first, then all boards by FPGA architecture (collapsible groups, shared
  with the Examples window). Unlike a native list, picking the current
  board is reported too (Logix then offers its examples).
-->
<script lang="ts">
  import { tick } from "svelte";
  import type { Board } from "./backend";
  import { archLabel, boardGroups, type BoardGroup } from "./boards";
  import { expandGroup, expanded, toggleGroup } from "./expandedGroups.svelte";

  let {
    boards,
    connected,
    value,
    disabled = false,
    placeholder,
    onPick,
  }: {
    boards: Board[];
    /** Ids of the boards plugged in now. */
    connected: string[];
    /** The selected board's id. */
    value: string | null;
    disabled?: boolean;
    /** The button's text when there's no board (e.g. "Loading boards…"). */
    placeholder: string;
    /** A board was picked (possibly the current one). */
    onPick: (id: string) => void;
  } = $props();

  let open = $state(false);
  let query = $state("");
  /** The highlighted entry, as "<group index>:<board id>". */
  let active = $state<string | null>(null);
  let root: HTMLElement;
  let button: HTMLButtonElement;
  let input = $state<HTMLInputElement | null>(null);
  let list = $state<HTMLElement | null>(null);

  const groups = $derived(boardGroups(boards, connected, query));
  /** Whether a group's boards are shown (all of them while searching). */
  const isOpen = (g: BoardGroup) =>
    !g.collapsible || query.trim() !== "" || expanded.has(g.label);
  /** The shown entries in order (a connected board appears twice). */
  const entries = $derived(
    groups.flatMap((g, gi) =>
      isOpen(g) ? g.boards.map((b) => ({ key: `${gi}:${b.id}`, board: b })) : [],
    ),
  );
  const label = $derived(
    boards.find((b) => b.id === value)?.description ??
      (value ? `${value} (unknown)` : placeholder),
  );

  async function show() {
    if (disabled) return;
    open = true;
    query = "";
    const current = boards.find((b) => b.id === value);
    if (current) expandGroup(archLabel(current.arch));
    await tick();
    // Start at the current board, in its architecture's group.
    const here = entries.filter((e) => e.board.id === value);
    active = (here[here.length - 1] ?? entries[0])?.key ?? null;
    input?.focus();
    await tick();
    reveal("center");
  }

  function hide(refocus = true) {
    open = false;
    if (refocus) button.focus();
  }

  function pick(id: string) {
    hide();
    onPick(id);
  }

  /** Scrolls the highlighted entry into view. */
  function reveal(block: ScrollLogicalPosition = "nearest") {
    list
      ?.querySelector(`[data-key="${CSS.escape(active ?? "")}"]`)
      ?.scrollIntoView({ block });
  }

  async function move(step: number) {
    if (entries.length === 0) return;
    const i = entries.findIndex((e) => e.key === active);
    const next = i < 0 ? 0 : Math.min(Math.max(i + step, 0), entries.length - 1);
    active = entries[next].key;
    await tick();
    reveal();
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      move(1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      move(-1);
    } else if (e.key === "PageDown" || e.key === "PageUp") {
      e.preventDefault();
      move(e.key === "PageDown" ? 10 : -10);
    } else if (e.key === "Enter") {
      e.preventDefault();
      const entry = entries.find((x) => x.key === active);
      if (entry) pick(entry.board.id);
    } else if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      hide();
    } else if (e.key === "Tab") {
      hide(false);
    }
  }

  // Searching highlights the first match.
  $effect(() => {
    void query;
    if (open) active = entries[0]?.key ?? null;
  });

  function toggle(g: BoardGroup) {
    toggleGroup(g.label);
    // Keep the highlight on a shown entry.
    if (!entries.some((e) => e.key === active)) active = entries[0]?.key ?? null;
  }

  function onWindowDown(e: MouseEvent) {
    if (open && !root.contains(e.target as Node)) hide(false);
  }
</script>

<svelte:window onmousedown={onWindowDown} />

<div class="picker" bind:this={root}>
  <button
    bind:this={button}
    class="board-button"
    {disabled}
    title="Board: {label}"
    aria-haspopup="listbox"
    aria-expanded={open}
    onclick={() => (open ? hide() : show())}
  >
    <span class="name">{label}</span>
    <svg class="arrow" viewBox="0 0 12 12" width="12" height="12" aria-hidden="true"
      ><path
        d="M2.5 4.5 L6 8 L9.5 4.5"
        fill="none"
        stroke="currentColor"
        stroke-width="1.8"
        stroke-linecap="round"
        stroke-linejoin="round"
      /></svg
    >
  </button>

  {#if open}
    <div class="panel">
      <input
        bind:this={input}
        bind:value={query}
        type="search"
        placeholder="Search boards"
        spellcheck="false"
        autocomplete="off"
        role="combobox"
        aria-controls="board-list"
        aria-expanded="true"
        aria-activedescendant={active ? `board-${active}` : undefined}
        onkeydown={onKey}
      />
      <ul id="board-list" role="listbox" aria-label="Boards" bind:this={list}>
        {#each groups as g, gi (g.label)}
          {@const shown = isOpen(g)}
          <li class="group" role="presentation">
            {#if g.collapsible}
              <!-- Keeps the focus in the search box. -->
              <button
                class="group-toggle"
                aria-expanded={shown}
                onmousedown={(e) => e.preventDefault()}
                onclick={() => toggle(g)}
                ><svg class="chevron" class:open={shown} viewBox="0 0 12 12" width="12" height="12" aria-hidden="true"
                  ><path d="M4 2.5 L8 6 L4 9.5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" /></svg
                >
                {g.label}
                <span class="count">{g.boards.length}</span></button
              >
            {:else}
              {g.label}
            {/if}
          </li>
          {#each shown ? g.boards : [] as b (b.id)}
            {@const key = `${gi}:${b.id}`}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <li
              id="board-{key}"
              data-key={key}
              role="option"
              aria-selected={b.id === value}
              class:active={key === active}
              onmousemove={() => (active = key)}
              onclick={() => pick(b.id)}
            >
              <span class="check" aria-hidden="true"
                >{b.id === value ? "✓" : ""}</span
              >
              {b.description}
            </li>
          {/each}
        {:else}
          <li class="empty" role="presentation">No matching boards</li>
        {/each}
      </ul>
    </div>
  {/if}
</div>

<style>
  .picker {
    position: relative;
    margin-left: 12px;
  }
  .board-button {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 220px;
    max-width: 360px;
    height: 28px;
    padding: 0 10px;
    border: 1px solid rgba(0, 0, 0, 0.15);
    border-radius: 5px;
    background: #ffffff;
    color: #1f2328;
    font: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }
  .board-button:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .arrow {
    flex: none;
    color: #57606a;
  }
  .panel {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    z-index: 30;
    display: flex;
    flex-direction: column;
    width: 380px;
    max-height: min(480px, 70vh);
    background: var(--bg);
    color: var(--fg);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.25);
    overflow: hidden;
  }
  input {
    margin: 8px;
    padding: 5px 8px;
    font: inherit;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg);
    color: var(--fg);
  }
  input:focus {
    outline: 2px solid var(--accent);
    outline-offset: -1px;
  }
  ul {
    margin: 0;
    padding: 0 0 6px;
    list-style: none;
    overflow: auto;
  }
  .group {
    position: sticky;
    top: 0;
    padding: 6px 12px 4px;
    background: var(--tabs-bg);
    color: var(--muted);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .group-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 0;
    border: none;
    background: none;
    color: inherit;
    font: inherit;
    letter-spacing: inherit;
    text-transform: inherit;
    cursor: pointer;
  }
  .chevron {
    flex: none;
    transition: transform 0.1s;
  }
  .chevron.open {
    transform: rotate(90deg);
  }
  .count {
    margin-left: auto;
    font-weight: 400;
  }
  [role="option"] {
    display: flex;
    gap: 6px;
    padding: 5px 12px;
    font-size: 13px;
    cursor: pointer;
  }
  [role="option"].active {
    background: var(--accent);
    color: #ffffff;
  }
  .check {
    width: 12px;
    flex: none;
  }
  .empty {
    padding: 8px 12px;
    color: var(--muted);
    font-size: 13px;
  }
</style>
