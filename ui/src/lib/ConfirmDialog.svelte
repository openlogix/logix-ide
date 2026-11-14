<!--
  A question in Logix IDE's own style, in place of the operating system's
  message box (the user's request, 2026-11-14: Windows' box looked harsh):
  a title, a lead sentence, what happens as a short list, an optional note,
  then Cancel and the button that agrees. Used for the driver questions
  (Upload and Tools > Connected Devices). It sits above any other dialog;
  Escape cancels and Enter agrees.
-->
<script lang="ts">
  import type { ConfirmContent } from "./devices";

  let { content, onAnswer }: { content: ConfirmContent; onAnswer: (yes: boolean) => void } = $props();

  let okButton = $state<HTMLButtonElement | null>(null);
  $effect(() => {
    okButton?.focus();
  });

  /** Captured first, so the dialog below (such as Connected Devices) doesn't also close. */
  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      onAnswer(false);
    }
  }
</script>

<svelte:window onkeydowncapture={onKey} />

<div class="overlay">
  <div class="dialog" role="alertdialog" aria-modal="true" aria-labelledby="confirm-title" aria-describedby="confirm-lead">
    <div class="content">
      <div class="icon" aria-hidden="true">
        <svg viewBox="0 0 24 24" width="22" height="22"
          ><circle cx="12" cy="12" r="10" fill="none" stroke="currentColor" stroke-width="2" /><path
            d="M12 11v6M12 7.5v.01"
            stroke="currentColor"
            stroke-width="2.4"
            stroke-linecap="round"
          /></svg
        >
      </div>
      <div class="text">
        <h2 id="confirm-title">{content.title}</h2>
        <p id="confirm-lead" class="lead">{content.lead}</p>
        {#if content.steps.length > 0}
          <p class="what">What happens</p>
          <ul>
            {#each content.steps as step (step)}
              <li>{step}</li>
            {/each}
          </ul>
        {/if}
        {#if content.note}
          <p class="note">{content.note}</p>
        {/if}
      </div>
    </div>
    <footer>
      <button onclick={() => onAnswer(false)}>Cancel</button>
      <button class="primary" bind:this={okButton} onclick={() => onAnswer(true)}>{content.okLabel}</button>
    </footer>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 30;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
    background: rgba(0, 0, 0, 0.35);
  }
  .dialog {
    width: min(480px, 100%);
    max-height: min(560px, 100%);
    overflow: auto;
    background: var(--bg);
    color: var(--fg);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.3);
  }
  .content {
    display: flex;
    gap: 14px;
    padding: 20px 22px 8px;
  }
  .icon {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: var(--banner-bg);
    color: var(--accent);
  }
  h2 {
    margin: 6px 0 8px;
    font-size: 16px;
    font-weight: 600;
  }
  p {
    margin: 0 0 10px;
    font-size: 14px;
    line-height: 1.45;
  }
  .what {
    margin-bottom: 4px;
    font-size: 13px;
    font-weight: 600;
  }
  ul {
    margin: 0 0 12px;
    padding-left: 18px;
    font-size: 13px;
    line-height: 1.45;
  }
  li {
    margin: 3px 0;
  }
  .note {
    color: var(--muted);
    font-size: 13px;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 10px 22px 18px;
  }
  button {
    padding: 6px 16px;
    font: inherit;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    color: var(--fg);
    cursor: pointer;
  }
  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
    font-weight: 600;
  }
  button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
</style>
