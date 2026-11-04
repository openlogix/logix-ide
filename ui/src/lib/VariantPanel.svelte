<script lang="ts">
  // The toolbar's variant summary (e.g. "FT232H adapter · RAM"), for boards
  // with choices, and the small panel it opens to change them: one row per
  // dimension. Values that don't exist with the other current values are
  // greyed out; choosing one changes the others too (`choose` in
  // variants.ts). The same choices are in Tools > <dimension>.
  import type { Board, Variant } from "./backend";
  import { configurationOf, isAvailable, resolve, summary } from "./variants";

  let {
    board,
    variant,
    disabled = false,
    onChoose,
  }: {
    board: Board;
    variant: Variant;
    disabled?: boolean;
    onChoose: (dim: string, value: string) => void;
  } = $props();

  let open = $state(false);
  let root: HTMLElement | undefined = $state();

  const when = $derived(resolve(board, variant));
  const configuration = $derived(configurationOf(board, variant));

  function onWindowPointer(e: PointerEvent) {
    if (open && root && !root.contains(e.target as Node)) open = false;
  }
  function onKey(e: KeyboardEvent) {
    if (open && e.key === "Escape") {
      e.stopPropagation();
      open = false;
    }
  }
</script>

<svelte:window onpointerdown={onWindowPointer} onkeydown={onKey} />

<div class="variant" bind:this={root}>
  <button
    class="summary"
    title={`${board.description}: ${summary(board, variant)}. Click to change how the board is programmed.`}
    aria-haspopup="dialog"
    aria-expanded={open}
    {disabled}
    onclick={() => (open = !open)}
  >
    {summary(board, variant)}
    {#if configuration && !configuration.tested}<span class="tag">untested</span>{/if}
    <span class="caret" aria-hidden="true">▾</span>
  </button>
  {#if open}
    <div class="panel" role="dialog" aria-label={`${board.description} options`}>
      {#each board.dimensions as dim (dim.id)}
        <div class="row" role="radiogroup" aria-label={dim.label}>
          <span class="label">{dim.label}</span>
          <div class="values">
            {#each dim.values as v (v.id)}
              {@const available = isAvailable(board, variant, dim.id, v.id)}
              <button
                role="radio"
                aria-checked={when[dim.id] === v.id}
                class:checked={when[dim.id] === v.id}
                class:unavailable={!available}
                title={(v.help ?? v.label) +
                  (available ? "" : ` Not available with the current choices: choosing it changes them.`)}
                onclick={() => onChoose(dim.id, v.id)}>{v.label}</button
              >
            {/each}
          </div>
        </div>
      {/each}
      {#if configuration && !configuration.tested}
        <p class="note">
          This way of programming the board hasn't been tried on a real board yet. If it works (or
          doesn't), please report it.
        </p>
      {/if}
      {#if configuration?.userNotes}
        <p class="note">{configuration.userNotes}</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .variant {
    position: relative;
    margin-left: 6px;
  }
  .summary {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 8px;
    border: 1px solid rgba(255, 255, 255, 0.35);
    border-radius: 4px;
    background: transparent;
    color: var(--toolbar-fg);
    font: inherit;
    font-size: 12px;
    white-space: nowrap;
    cursor: pointer;
  }
  .summary:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.12);
  }
  .caret {
    font-size: 10px;
    opacity: 0.8;
  }
  .tag {
    padding: 0 5px;
    border-radius: 3px;
    background: var(--warning-mark);
    color: #1f2328;
    font-size: 10px;
  }
  .panel {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    z-index: 20;
    min-width: 260px;
    max-width: 380px;
    padding: 10px 12px;
    background: var(--bg);
    color: var(--fg);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.25);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 0;
  }
  .label {
    min-width: 80px;
    color: var(--muted);
    font-size: 12px;
  }
  .values {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .values button {
    padding: 3px 8px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg);
    color: var(--fg);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .values button.checked {
    border-color: var(--accent);
    background: var(--accent);
    color: #ffffff;
  }
  .values button.unavailable {
    opacity: 0.45;
  }
  .note {
    margin: 8px 0 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.4;
  }
</style>
