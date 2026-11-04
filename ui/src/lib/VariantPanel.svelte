<script lang="ts">
  // The toolbar's variant summary (e.g. "FT232H adapter · RAM"), for boards
  // with choices, and the small panel it opens to change them: one section
  // per dimension, its values as radio buttons and the selected value's
  // help text. Values that don't exist
  // with the other current values are dimmed; choosing one changes the
  // others too (`choose` in variants.ts). The same choices are in
  // Tools > <dimension>.
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

  /** The help text of a dimension's selected value, if it has one. */
  function helpOf(dim: string): string | undefined {
    return board.dimensions.find((d) => d.id === dim)?.values.find((v) => v.id === when[dim])
      ?.help;
  }

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
          <div class="label">{dim.label}</div>
          {#each dim.values as v (v.id)}
            {@const available = isAvailable(board, variant, dim.id, v.id)}
            <label
              class="choice"
              class:unavailable={!available}
              title={(v.help ?? v.label) +
                (available ? "" : " Not available with the current choices: choosing it changes them.")}
            >
              <input
                type="radio"
                name={`variant-${dim.id}`}
                class:unavailable={!available}
                checked={when[dim.id] === v.id}
                onchange={() => onChoose(dim.id, v.id)}
              />
              {v.label}
            </label>
          {/each}
          {#if helpOf(dim.id)}<p class="hint">{helpOf(dim.id)}</p>{/if}
        </div>
      {/each}
      {#if configuration && (!configuration.tested || configuration.userNotes)}
        <!-- About the whole choice, not one dimension: its own section. -->
        <div class="row">
          <div class="label">Note</div>
          {#if !configuration.tested}
            <p class="note">
              This way of programming the board hasn't been tried on a real board yet. If it works
              (or doesn't), please report it.
            </p>
          {/if}
          {#if configuration.userNotes}
            <p class="note">{configuration.userNotes}</p>
          {/if}
        </div>
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
    width: max-content;
    min-width: 200px;
    max-width: min(420px, 90vw);
    padding: 4px 14px 12px;
    background: var(--bg);
    color: var(--fg);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.25);
  }
  /* One section per dimension: its label, then its values as radio buttons. */
  .row {
    padding: 10px 0 6px;
  }
  .row + .row {
    border-top: 1px solid var(--border);
  }
  .label {
    margin-bottom: 4px;
    color: var(--muted);
    font-size: 12px;
  }
  .choice {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 0;
    font-size: 13px;
    white-space: nowrap;
    cursor: pointer;
  }
  /* Drawn here, not by the system: the app's web view ignores accent-color
     and draws the selected radio as a faint dot. */
  .choice input {
    appearance: none;
    display: grid;
    place-content: center;
    flex: none;
    width: 15px;
    height: 15px;
    margin: 0;
    border: 1.5px solid var(--muted);
    border-radius: 50%;
    background: var(--bg);
    cursor: pointer;
  }
  .choice input:checked {
    border-color: var(--accent);
  }
  .choice input:checked::before {
    content: "";
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
  }
  .choice input:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  /* Values that don't go with the other choices: still clickable (choosing
     one changes the others), but dimmed. */
  .choice.unavailable {
    color: var(--muted);
  }
  /* The selected value's help, e.g. what RAM means. */
  .hint {
    max-width: 300px;
    margin: 6px 0 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.4;
  }
  .note {
    max-width: 300px;
    margin: 0;
    color: var(--fg);
    font-size: 12px;
    line-height: 1.4;
  }
</style>
