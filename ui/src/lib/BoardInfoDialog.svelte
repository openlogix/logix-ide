<!--
  Tools > Board Info… (and the ⓘ next to the board): a board's FPGA, its pin
  file kind, the ways it can be programmed, and the other boards with the
  same FPGA, which the user can select, such as to find a board like theirs.
  Everything comes from Logix Engine's catalog (`get_catalog`).
-->
<script lang="ts">
  import type { Board } from "./backend";
  import { sameFpgaBoards } from "./boards";
  import { summary } from "./variants";

  let {
    board,
    boards,
    selected,
    onPick,
    onClose,
  }: {
    /** The board to describe. */
    board: Board;
    /** Every board offered, for the boards with the same FPGA. */
    boards: Board[];
    /** The toolbar's board, marked in the lists. */
    selected: string | null;
    /** A board's name was clicked: select that board. */
    onPick: (id: string) => void;
    onClose: () => void;
  } = $props();

  const same = $derived(sameFpgaBoards(boards, board));

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
      <h2>{board.description}</h2>
      <span class="spacer"></span>
      <button class="close" title="Close" onclick={onClose}>✕</button>
    </header>
    <div class="body">
      {#if board.custom}
        <p class="muted">One of this project's own boards, from custom-boards.json.</p>
      {/if}
      <table>
        <tbody>
          <tr><th scope="row">FPGA family</th><td>{board.archLabel}</td></tr>
          {#if board.fpga}
            <tr><th scope="row">FPGA</th><td>{board.fpga.partNumber}</td></tr>
            {#if board.fpga.size}
              <tr><th scope="row">Size</th><td>{board.fpga.size}</td></tr>
            {/if}
          {/if}
          {#if board.pinExtension}
            <tr><th scope="row">Pin file</th><td>.{board.pinExtension}</td></tr>
          {/if}
          <tr><th scope="row">Board ID</th><td>{board.id}</td></tr>
        </tbody>
      </table>

      <h3>Ways to program it</h3>
      <ul>
        {#each board.configurations as c, i (i)}
          <li>
            {summary(board, c.when)}{i === 0 ? " (default)" : ""}{c.tested ? "" : " (not yet tried on a board)"}
            {#if c.userNotes}<span class="muted note">{c.userNotes}</span>{/if}
          </li>
        {/each}
      </ul>

      <h3>Other boards with this FPGA</h3>
      {#if same.length === 0}
        <p class="muted">None in Logix's board list.</p>
      {:else}
        <ul>
          {#each same as b (b.id)}
            <li>
              <button class="board-pick" title="Select the {b.description}" onclick={() => onPick(b.id)}
                >{b.description}</button
              >{#if b.id === selected}{" "}<span class="muted">(selected)</span>{/if}
            </li>
          {/each}
        </ul>
      {/if}
    </div>
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
  .muted {
    color: var(--muted);
    font-size: 12px;
  }
</style>
