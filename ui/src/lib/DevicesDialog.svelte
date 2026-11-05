<!-- Tools > Connected Devices: the USB devices connected. -->
<script lang="ts">
  import type { Devices } from "./backend";
  import { SvelteSet } from "svelte/reactivity";
  import { deviceBoardsParts, otherBoards, usbLabel } from "./devices";

  let {
    devices,
    error,
    busy,
    board,
    boardName,
    onRefresh,
    onPick,
    onClose,
  }: {
    /** The board selected in the toolbar, if any. */
    board: string | null;
    /** A board's name for people, from its id. */
    boardName: (id: string) => string;
    /** Null until the first scan finishes. */
    devices: Devices | null;
    /** Why scanning failed, if it did. */
    error: string | null;
    busy: boolean;
    onRefresh: () => void;
    /** A board name in a device's line was clicked: select that board. */
    onPick: (board: string) => void;
    onClose: () => void;
  } = $props();

  /** Devices whose other boards are listed (by device id). */
  const showingOthers = new SvelteSet<string>();
  function toggleOthers(id: string) {
    if (showingOthers.has(id)) showingOthers.delete(id);
    else showingOthers.add(id);
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
    aria-label="Connected Devices"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
  >
    <header>
      <h2>Connected Devices</h2>
      <span class="spacer"></span>
      <button onclick={onRefresh} disabled={busy}
        >{busy ? "Scanning…" : "Refresh"}</button
      >
      <button class="close" title="Close" onclick={onClose}>✕</button>
    </header>

    <div class="body">
      {#if error}
        <p class="error">{error}</p>
      {:else if !devices}
        <p class="muted">Scanning…</p>
      {:else}
        <h3>USB devices</h3>
        {#if devices.usb.length === 0}
          <p class="muted">
            None. Check that the board is plugged in and powered, and try
            another cable (some USB cables only charge and don't carry data).
          </p>
        {:else}
          <ul>
            {#each devices.usb as d, i (i)}
              {@const parts = deviceBoardsParts(d, board, boardName)}
              {@const others = otherBoards(d, board)}
              <li>
                <span class="name">{usbLabel(d)}</span>
                {#if d.serialNumber}
                  <span class="muted">serial {d.serialNumber}</span>
                {/if}
                {#if parts || others.length > 0}
                  <span class="muted"
                    >{#each parts ?? [] as p, pi (pi)}{#if typeof p === "string"}{p}{:else}<button
                          class="board-pick"
                          title="Select the {boardName(p.board)}"
                          onclick={() => onPick(p.board)}>{boardName(p.board)}</button
                        >{/if}{/each}{#if others.length > 0}{parts ? " It may also be one of " : "It may be one of "}<button
                        class="board-pick"
                        aria-expanded={showingOthers.has(d.id)}
                        title="Boards it matches less well. Click to {showingOthers.has(d.id) ? 'hide' : 'list'} them."
                        onclick={() => toggleOthers(d.id)}
                        >{others.length}{parts ? " other" : ""} {others.length === 1 ? "board" : "boards"}</button
                      >.{/if}</span
                  >
                  {#if showingOthers.has(d.id)}
                    <span class="muted others"
                      >{#each [...others].sort((a, b) => boardName(a).localeCompare(boardName(b))) as o, oi (o)}{oi > 0 ? ", " : ""}<button
                          class="board-pick"
                          title="Select the {boardName(o)}"
                          onclick={() => onPick(o)}>{boardName(o)}</button
                        >{/each}</span
                    >
                  {/if}
                {/if}
              </li>
            {/each}
          </ul>
        {/if}

        {#if devices.unreachable.length > 0}
          <h3>Found but can't be opened</h3>
          <ul>
            {#each devices.unreachable as u, i (i)}
              <li>
                <span class="name">A USB device ({u.vid}:{u.pid})</span>
                <span class="muted">{u.actionDescription}</span>
              </li>
            {/each}
          </ul>
        {/if}


        <p class="muted hint">
          Upload finds the board by its USB ids and name, and asks which one
          when several fit. If your board is listed but Upload can't find it,
          check that the board selected in the toolbar matches it.
        </p>
      {/if}
    </div>
  </div>
</div>

<style>
  /* A board name that selects the board: a link, inline in the line. */
  .board-pick {
    padding: 0;
    border: none;
    background: none;
    color: var(--accent);
    font: inherit;
    text-decoration: underline;
    cursor: pointer;
  }
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
  header button {
    padding: 3px 12px;
    font: inherit;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg);
    color: var(--fg);
    cursor: pointer;
  }
  header button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .close {
    border: none !important;
  }
  .body {
    overflow: auto;
    padding: 4px 16px 14px;
  }
  h3 {
    margin: 14px 0 6px;
    font-size: 13px;
    font-weight: 600;
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px 0;
    border-bottom: 1px solid var(--border);
    font-size: 13px;
  }
  .muted {
    color: var(--muted);
    font-size: 12px;
  }
  .hint {
    margin-top: 14px;
  }
  .error {
    color: var(--error-mark);
  }
</style>
