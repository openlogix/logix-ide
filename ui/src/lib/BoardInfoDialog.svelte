<!--
  Tools > Board Info… (and the ⓘ next to the board): a board's FPGA, its pin
  file kind, the ways it can be programmed, and the other boards with the
  same FPGA, such as to find a board like the user's. A board in that list
  shows its own info (Back returns), and Select This Board picks the board
  shown. Everything comes from Logix Engine's catalog (`get_catalog`).
-->
<script lang="ts">
  import { untrack } from "svelte";
  import type { Board } from "./backend";
  import { boardsWithFpga } from "./boards";
  import { summary } from "./variants";

  let {
    board,
    boards,
    selected,
    onPick,
    onClose,
  }: {
    /** The board to describe first. */
    board: Board;
    /** Every board offered, for the boards with the same FPGA. */
    boards: Board[];
    /** The toolbar's board, marked in the lists. */
    selected: string | null;
    /** Select This Board: select the board shown. */
    onPick: (id: string) => void;
    onClose: () => void;
  } = $props();

  /** The boards shown, the current one last (Back goes to the one before). */
  let shownIds = $state<string[]>(untrack(() => [board.id]));
  const current = $derived(
    boards.find((b) => b.id === shownIds[shownIds.length - 1]) ?? board,
  );
  const same = $derived(boardsWithFpga(boards, current));

  function show(id: string) {
    shownIds = [...shownIds, id];
  }

  function back() {
    if (shownIds.length > 1) shownIds = shownIds.slice(0, -1);
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
    aria-label="Board Info"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
  >
    <header>
      <h2>Board: {current.description}</h2>
      <span class="spacer"></span>
      <button class="close" title="Close" onclick={onClose}>✕</button>
    </header>
    <div class="body">
      {#if current.custom}
        <p class="muted">One of this project's own boards, from custom-boards.json.</p>
      {/if}
      <table>
        <tbody>
          <tr><th scope="row">FPGA family</th><td>{current.archLabel}</td></tr>
          {#if current.fpga}
            <tr><th scope="row">FPGA</th><td>{current.fpga.partNumber}</td></tr>
            {#if current.fpga.size}
              <tr><th scope="row">Size</th><td>{current.fpga.size}</td></tr>
            {/if}
          {/if}
          {#if current.pinExtension}
            <tr><th scope="row">Pin file</th><td>.{current.pinExtension}</td></tr>
          {/if}
          <tr><th scope="row">Board ID</th><td>{current.id}</td></tr>
        </tbody>
      </table>

      <h3>Ways to program it</h3>
      <ul>
        {#each current.configurations as c, i (i)}
          <li>
            {summary(current, c.when)}{i === 0 ? " (default)" : ""}{c.tested ? "" : " (not yet tried on a board)"}
            {#if c.userNotes}<span class="muted note">{c.userNotes}</span>{/if}
          </li>
        {/each}
      </ul>

      <h3>Boards with this FPGA</h3>
      <ul>
        {#each same as b (b.id)}
          {@const marks = [b.id === current.id ? "shown" : "", b.id === selected ? "selected" : ""].filter(Boolean)}
          <li class:shown={b.id === current.id}>
            {#if b.id === current.id}<span>{b.description}</span>{:else}<button
                class="board-pick"
                title="Show the {b.description}"
                onclick={() => show(b.id)}>{b.description}</button
              >{/if}{#if marks.length > 0}{" "}<span class="muted">({marks.join(", ")})</span>{/if}
          </li>
        {/each}
      </ul>
    </div>
    <footer>
      {#if shownIds.length > 1}
        <button onclick={back} title="The board shown before">Back</button>
      {/if}
      <span class="spacer"></span>
      {#if current.id === selected}
        <span class="muted">The selected board</span>
      {:else}
        <button class="primary" onclick={() => onPick(current.id)}>Select This Board</button>
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
    width: min(560px, 100%);
    max-height: min(480px, 100%);
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
    gap: 8px;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
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
  .close {
    border: none;
  }
  .body {
    overflow: auto;
    padding: 4px 16px 14px;
  }
  table {
    border-collapse: collapse;
    margin: 6px 0 4px;
    font-size: 13px;
  }
  th {
    padding: 3px 16px 3px 0;
    color: var(--muted);
    font-weight: normal;
    text-align: left;
    vertical-align: top;
    white-space: nowrap;
  }
  td {
    padding: 3px 0;
  }
  h3 {
    margin: 14px 0 4px;
    font-size: 13px;
    font-weight: 600;
  }
  ul {
    margin: 0;
    padding-left: 20px;
    font-size: 13px;
  }
  li {
    padding: 2px 0;
  }
  .board-pick {
    padding: 0;
    border: none;
    background: none;
    color: var(--accent);
    font: inherit;
    text-decoration: underline;
    cursor: pointer;
  }
  .note {
    display: block;
  }
  /* The board shown: in bold, not a link. */
  li.shown {
    font-weight: 600;
  }
  footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 16px;
    border-top: 1px solid var(--border);
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
  .muted {
    color: var(--muted);
    font-size: 12px;
  }
</style>
