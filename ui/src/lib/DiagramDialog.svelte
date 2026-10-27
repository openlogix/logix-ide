<!--
  Shows the design diagram (an SVG drawn by `apio graph`). The SVG is shown
  as an image, so nothing in it can run as code.
-->
<script lang="ts">
  let {
    svg,
    modules,
    module,
    busy,
    onModule,
    onClose,
  }: {
    svg: string;
    /** Modules that can be drawn, "main" first. */
    modules: string[];
    /** The module shown. */
    module: string;
    /** True while another module's diagram is being drawn. */
    busy: boolean;
    onModule: (name: string) => void;
    onClose: () => void;
  } = $props();

  const ZOOMS = [0.5, 0.75, 1, 1.5, 2, 3, 4];
  /** Index into ZOOMS; 1 (index 2) fits the diagram in the window. */
  let zoomIndex = $state(2);
  const zoom = $derived(ZOOMS[zoomIndex]);

  const src = $derived(
    `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`,
  );

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
    aria-label="Diagram"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
  >
    <header>
      <h2>Diagram</h2>
      <label>
        Module
        <select
          value={module}
          disabled={busy}
          onchange={(e) => onModule(e.currentTarget.value)}
        >
          {#each modules as name (name)}
            <option value={name}>{name}</option>
          {/each}
        </select>
      </label>
      {#if busy}<span class="busy">Drawing…</span>{/if}
      <span class="spacer"></span>
      <div class="zoom" role="group" aria-label="Zoom">
        <button
          title="Fit in the window"
          disabled={zoomIndex === 2}
          onclick={() => (zoomIndex = 2)}>Fit</button
        >
        <button
          title="Zoom out"
          disabled={zoomIndex === 0}
          onclick={() => zoomIndex--}>−</button
        >
        <span class="percent">{Math.round(zoom * 100)}%</span>
        <button
          title="Zoom in"
          disabled={zoomIndex === ZOOMS.length - 1}
          onclick={() => zoomIndex++}>+</button
        >
      </div>
      <button class="close" title="Close (Esc)" onclick={onClose}>✕</button>
    </header>

    <div class="canvas" class:fit={zoom === 1}>
      <img
        {src}
        alt="Diagram of module {module}"
        style:width={zoom === 1 ? undefined : `${zoom * 100}%`}
      />
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
    width: min(1100px, 100%);
    height: min(760px, 100%);
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
    gap: 12px;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
  }
  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }
  label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: var(--muted);
  }
  .busy {
    font-size: 12px;
    color: var(--muted);
  }
  .spacer {
    flex: 1;
  }
  .zoom {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .percent {
    min-width: 44px;
    text-align: center;
    font-size: 12px;
    color: var(--muted);
  }
  header button {
    padding: 3px 10px;
    font: inherit;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg);
    color: var(--fg);
    cursor: pointer;
  }
  header button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .close {
    border: none !important;
  }
  /* The diagram is drawn for a white background, also in dark mode. */
  .canvas {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 12px;
    background: #ffffff;
  }
  .canvas.fit {
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .canvas.fit img {
    max-width: 100%;
    max-height: 100%;
  }
  img {
    display: block;
  }
</style>
