<!--
  File > Settings (Cmd+,), laid out like the Arduino IDE's Preferences:
  label and control rows, Cancel / OK at the bottom right. Changes apply on
  OK.
-->
<script lang="ts">
  import type { Preferences, Theme } from "./backend";

  let {
    initial,
    onSave,
    onBrowse,
    onClose,
  }: {
    initial: Preferences;
    /** Saves; resolves with an error message, or null if saved. */
    onSave: (prefs: Preferences) => Promise<string | null>;
    /** Asks for a folder, starting at `current`. */
    onBrowse: (current: string) => Promise<string | null>;
    onClose: () => void;
  } = $props();

  // svelte-ignore state_referenced_locally
  let projectsDir = $state(initial.projectsDir);
  // svelte-ignore state_referenced_locally
  let fontSize = $state(initial.editorFontSize);
  // svelte-ignore state_referenced_locally
  let autoScale = $state(initial.uiScale === null);
  // svelte-ignore state_referenced_locally
  let scale = $state(initial.uiScale ?? 100);
  // svelte-ignore state_referenced_locally
  let theme = $state<Theme>(initial.theme);
  // svelte-ignore state_referenced_locally
  let lintOnVerify = $state(initial.lintOnVerify);
  // svelte-ignore state_referenced_locally
  let liveChecking = $state(initial.liveChecking);
  // svelte-ignore state_referenced_locally
  let hoverInfo = $state(initial.hoverInfo);

  let error = $state<string | null>(null);
  let saving = $state(false);

  async function browse() {
    const dir = await onBrowse(projectsDir);
    if (dir) projectsDir = dir;
  }

  async function ok() {
    saving = true;
    error = await onSave({
      projectsDir: projectsDir.trim(),
      editorFontSize: Math.round(Number(fontSize)),
      uiScale: autoScale ? null : Math.round(Number(scale)),
      theme,
      lintOnVerify,
      liveChecking,
      hoverInfo,
    });
    saving = false;
    if (!error) onClose();
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    } else if (e.key === "Enter" && !(e.target instanceof HTMLButtonElement)) {
      e.preventDefault();
      ok();
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
    aria-label="Settings"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
  >
    <header>
      <h2>Settings</h2>
      <button class="close" title="Close (Esc)" onclick={onClose}>✕</button>
    </header>

    <div class="rows">
      <label for="pref-dir">Projects folder location:</label>
      <div class="control">
        <input
          id="pref-dir"
          class="wide"
          bind:value={projectsDir}
          spellcheck="false"
          autocomplete="off"
        />
        <button class="secondary" onclick={browse}>Browse</button>
      </div>

      <label for="pref-font">Editor font size:</label>
      <div class="control">
        <input
          id="pref-font"
          class="number"
          type="number"
          min="8"
          max="32"
          bind:value={fontSize}
        />
      </div>

      <span class="label">Interface scale:</span>
      <div class="control">
        <label class="check">
          <input type="checkbox" bind:checked={autoScale} />
          Automatic
        </label>
        <input
          class="number"
          type="number"
          min="50"
          max="300"
          step="10"
          aria-label="Interface scale in percent"
          disabled={autoScale}
          bind:value={scale}
        />
        <span class:muted={autoScale}>%</span>
      </div>

      <label for="pref-theme">Theme:</label>
      <div class="control">
        <select id="pref-theme" bind:value={theme}>
          <option value="system">System</option>
          <option value="light">Light</option>
          <option value="dark">Dark</option>
        </select>
      </div>

      <span></span>
      <div class="control">
        <label class="check">
          <input type="checkbox" bind:checked={lintOnVerify} />
          Show lint warnings on Verify
        </label>
      </div>

      <span></span>
      <div class="control">
        <label class="check">
          <input type="checkbox" bind:checked={liveChecking} />
          Check code while typing
        </label>
      </div>

      <span></span>
      <div class="control sub">
        <label class="check" class:muted={!liveChecking}>
          <input
            type="checkbox"
            bind:checked={hoverInfo}
            disabled={!liveChecking}
          />
          Show info when hovering
        </label>
      </div>
    </div>

    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}

    <footer>
      <button class="secondary" onclick={onClose}>Cancel</button>
      <button class="primary" disabled={saving} onclick={ok}>OK</button>
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
    width: min(640px, 100%);
    background: var(--bg);
    color: var(--fg);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.3);
  }
  header {
    display: flex;
    align-items: center;
    padding: 14px 20px 6px;
  }
  h2 {
    flex: 1;
    margin: 0;
    font-size: 16px;
    font-weight: 600;
  }
  .close {
    border: none;
    background: none;
    color: var(--muted);
    font-size: 15px;
    cursor: pointer;
  }
  .rows {
    display: grid;
    grid-template-columns: max-content 1fr;
    align-items: center;
    gap: 14px 16px;
    padding: 14px 20px;
  }
  .rows > label,
  .label {
    font-size: 13px;
  }
  .control {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }
  input,
  select {
    padding: 5px 8px;
    font: inherit;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg);
    color: var(--fg);
  }
  input:focus,
  select:focus {
    outline: 2px solid var(--accent);
    outline-offset: -1px;
  }
  .wide {
    flex: 1;
    min-width: 0;
  }
  .number {
    width: 64px;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .sub {
    padding-left: 24px;
  }
  .check input {
    padding: 0;
    accent-color: var(--accent);
  }
  .muted {
    color: var(--muted);
  }
  .error {
    margin: 0 20px;
    font-size: 13px;
    color: var(--error-mark);
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 12px 20px 18px;
  }
  /* Arduino-style buttons: outlined secondary, teal primary. */
  button.secondary,
  button.primary {
    min-width: 84px;
    padding: 6px 16px;
    font: inherit;
    font-size: 13px;
    font-weight: 600;
    letter-spacing: 0.02em;
    border-radius: 4px;
    cursor: pointer;
  }
  button.secondary {
    border: 1px solid var(--accent);
    background: transparent;
    color: var(--accent);
  }
  button.primary {
    border: 1px solid var(--accent);
    background: var(--accent);
    color: #ffffff;
  }
  button:disabled {
    opacity: 0.6;
    cursor: default;
  }
</style>
