<!--
  Tools > Supported FPGAs…: every FPGA Logix Engine supports, by family (the
  engine's order), with how many boards have each, so users can tell which
  chips Logix supports even without a board for them, and find the FPGA ID
  a board file of their own names. Families start collapsed, their headings
  counting FPGAs and boards; a search shows the matches in every family.
  Everything comes from the engine's catalog (`get_catalog`).
-->
<script lang="ts">
  import { backend, type Board, type FpgaFamily } from "./backend";
  import { count, fpgaGroups } from "./fpgas";

  let {
    boards,
    onClose,
  }: {
    /** The boards offered (the catalog's and the project's), for the counts. */
    boards: Board[];
    onClose: () => void;
  } = $props();

  let families = $state<FpgaFamily[] | null>(null);
  let error = $state<string | null>(null);
  let query = $state("");
  let expanded = $state<Set<string>>(new Set());
  /** The FPGA whose ID was just copied. */
  let copied = $state<string | null>(null);

  $effect(() => {
    backend
      .getFpgaFamilies()
      .then((f) => (families = f))
      .catch((e) => (error = (e as { message?: string }).message ?? String(e)));
  });

  const groups = $derived(families ? fpgaGroups(families, boards, query) : []);
  const searching = $derived(query.trim() !== "");
  const total = $derived(families?.reduce((n, f) => n + f.fpgas.length, 0) ?? 0);

  function toggle(id: string) {
    const next = new Set(expanded);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    expanded = next;
  }

  async function copy(id: string) {
    try {
      await backend.copyText(id);
      copied = id;
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
    aria-label="Supported FPGAs"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
  >
    <header>
      <h2>Supported FPGAs</h2>
      <span class="spacer"></span>
      <button class="close" title="Close" onclick={onClose}>✕</button>
    </header>
    <div class="body">
      {#if error}
        <p class="error">{error}</p>
      {:else if families === null}
        <p class="muted">Loading…</p>
      {:else}
        <p class="muted intro">
          The {total} FPGAs Logix supports, by family. A board file of your own names its FPGA by
          its ID.
        </p>
        <!-- svelte-ignore a11y_autofocus -->
        <input
          type="search"
          placeholder="Search FPGAs"
          aria-label="Search FPGAs"
          bind:value={query}
          autofocus
        />
        {#if groups.length === 0}
          <p class="muted">No FPGA matches “{query.trim()}”.</p>
        {/if}
        <ul class="families" aria-label="FPGA families">
          {#each groups as g (g.id)}
            {@const shown = searching || expanded.has(g.id)}
            <li>
              <button
                class="family-toggle"
                aria-expanded={shown}
                disabled={searching}
                onclick={() => toggle(g.id)}
                ><span class="chevron" aria-hidden="true">{shown ? "▾" : "▸"}</span>
                <span class="family-label">{g.label}</span>
                <span class="muted">{count(g.fpgaCount, "FPGA")} · {count(g.boardCount, "board")}</span></button
              >
              {#if shown}
                <table>
                  <thead>
                    <tr><th>FPGA</th><th>Size</th><th>FPGA ID</th><th></th><th class="num">Boards</th></tr>
                  </thead>
                  <tbody>
                    {#each g.fpgas as f (f.id)}
                      <tr>
                        <td>{f.partNumber}</td>
                        <td>{f.size}</td>
                        <td class="id"><code>{f.id}</code></td>
                        <td
                          ><button
                            class="copy"
                            title="Copy the FPGA ID"
                            aria-label="Copy {f.id}"
                            onclick={() => copy(f.id)}>{copied === f.id ? "Copied" : "Copy"}</button
                          ></td
                        >
                        <td class="num" title={f.boards.join("\n")}>{f.boards.length}</td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              {/if}
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
    width: min(680px, 100%);
    height: min(560px, 100%);
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
    font: inherit;
    font-size: 13px;
    color: var(--fg);
    cursor: pointer;
  }
  .close {
    padding: 3px 12px;
    border: none;
    background: var(--bg);
  }
  .body {
    overflow: auto;
    padding: 8px 16px 14px;
    font-size: 13px;
  }
  .intro {
    margin: 4px 0 8px;
  }
  input[type="search"] {
    box-sizing: border-box;
    width: 100%;
    margin-bottom: 8px;
    padding: 4px 8px;
    font: inherit;
    font-size: 13px;
    color: var(--fg);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 4px;
  }
  .families {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .family-toggle {
    display: flex;
    align-items: baseline;
    gap: 6px;
    width: 100%;
    padding: 5px 4px;
    border: none;
    border-radius: 4px;
    background: none;
    text-align: left;
  }
  .family-toggle:hover:not(:disabled) {
    background: var(--hover, rgba(127, 127, 127, 0.12));
  }
  .family-toggle:disabled {
    cursor: default;
  }
  .chevron {
    width: 10px;
    color: var(--muted);
  }
  .family-label {
    font-weight: 600;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    margin: 2px 0 8px;
  }
  th {
    padding: 3px 8px 3px 20px;
    color: var(--muted);
    font-weight: normal;
    text-align: left;
    border-bottom: 1px solid var(--border);
  }
  td {
    padding: 3px 8px 3px 20px;
  }
  th + th,
  td + td {
    padding-left: 8px;
  }
  .num {
    text-align: right;
  }
  .id {
    white-space: nowrap;
  }
  code {
    font-size: 12px;
  }
  .copy {
    padding: 0 6px;
    font-size: 11px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg);
  }
  .muted {
    color: var(--muted);
    font-size: 12px;
  }
  .error {
    color: var(--error-mark, #c62828);
  }
</style>
