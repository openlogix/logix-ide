<!-- Simulate: choose the testbench when the project has several and none is
     open or was simulated before (simulateChoice.ts). -->
<script lang="ts">
  import { arrowNav } from "./arrowNav";

  let {
    testbenches,
    onChoose,
    onClose,
  }: {
    /** The project's testbenches. */
    testbenches: string[];
    onChoose: (testbench: string) => void;
    onClose: () => void;
  } = $props();

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
    aria-label="Choose a Testbench"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
  >
    <header>
      <h2>Simulate which testbench?</h2>
      <span class="spacer"></span>
      <button class="close" title="Cancel" onclick={onClose}>✕</button>
    </header>
    <div class="body">
      <p class="muted">
        This project has {testbenches.length} testbenches. Choose the one to
        simulate; Simulate runs it again next time. To simulate another one,
        open it first.
      </p>
      <ul use:arrowNav={{ items: "button" }}>
        {#each testbenches as name (name)}
          <li>
            <span class="text name">{name}</span>
            <button onclick={() => onChoose(name)}>Simulate</button>
          </li>
        {/each}
      </ul>
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
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 0;
    border-bottom: 1px solid var(--border);
    font-size: 13px;
  }
  .text {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 2px;
  }
  .muted {
    color: var(--muted);
    font-size: 12px;
  }
</style>
