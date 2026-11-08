<!--
  Help > System Info…: a plain text report for bug reports, with Copy. As
  the Logix Engine's recipe "Show system info" describes, so it reads the
  same from every frontend: the IDE's own lines, a blank line, then the
  engine's `report` from `get_engine_info` (built in `system_info.rs`).
-->
<script lang="ts">
  import { backend } from "./backend";

  let { onClose }: { onClose: () => void } = $props();

  let text = $state<string | null>(null);
  let error = $state<string | null>(null);
  let copied = $state(false);

  $effect(() => {
    backend
      .systemInfo()
      .then((t) => (text = t))
      .catch((e) => (error = (e as { message?: string }).message ?? String(e)));
  });

  async function copy() {
    if (text === null) return;
    try {
      await backend.copyText(text);
      copied = true;
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
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
    aria-label="System Info"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
  >
    <header>
      <h2>System Info</h2>
      <span class="spacer"></span>
      <button class="close" title="Close" onclick={onClose}>✕</button>
    </header>
    <div class="body">
      {#if error}
        <p class="error">{error}</p>
      {:else if text === null}
        <p class="muted">Collecting…</p>
      {:else}
        <pre class="report">{text}</pre>
      {/if}
    </div>
    <footer>
      <span class="muted">{copied ? "Copied." : "Paste it into a bug report."}</span>
      <span class="spacer"></span>
      <button class="primary" disabled={text === null} onclick={copy}>Copy</button>
      <button onclick={onClose}>Close</button>
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
    width: min(720px, 100%);
    max-height: min(560px, 100%);
    background: var(--bg);
    color: var(--fg);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.3);
    overflow: hidden;
  }
  header,
  footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 16px;
  }
  header {
    border-bottom: 1px solid var(--border);
  }
  footer {
    border-top: 1px solid var(--border);
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
  .primary {
    background: var(--accent, #00796b);
    border-color: var(--accent, #00796b);
    color: #fff;
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .body {
    overflow: auto;
    padding: 8px 16px;
  }
  .report {
    margin: 0;
    font-family: ui-monospace, Menlo, Consolas, monospace;
    font-size: 12px;
    white-space: pre-wrap;
    word-break: break-all;
    user-select: text;
  }
  .muted {
    color: var(--muted);
    font-size: 12px;
  }
  .error {
    color: var(--error-mark);
  }
</style>
