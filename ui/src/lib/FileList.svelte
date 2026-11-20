<!--
  The All files list and Go to File: one list, opened by the tab bar's ▾
  (when the tabs don't fit) or by ⌘P / Ctrl+P (Edit > Go to File…). Type to
  narrow it; Up and Down move, Enter opens, Escape closes. The groups come
  from fileList.ts.
-->
<script lang="ts">
  import { focusOnMount } from "./focus";
  import { fileGroups, shortcutLabel } from "./fileList";

  type Props = {
    /** The project's files, in the tabs' order. */
    files: string[];
    /** The project's history, newest first. */
    history: string[];
    /** The file shown now. */
    current: string;
    main: string;
    pinExtension: string | null;
    /** Where it opens: under the ▾ (its right edge) or centered. */
    top: number;
    right: number | null;
    onPick: (file: string) => void;
    onClose: () => void;
  };
  let { files, history, current, main, pinExtension, top, right, onPick, onClose }: Props =
    $props();

  let query = $state("");
  let selected = $state(0);
  let listEl = $state<HTMLElement>();

  const groups = $derived(fileGroups(files, history, current, main, pinExtension, query));
  const order = $derived(groups.flatMap((g) => g.files));

  // A new query starts at the first file.
  $effect(() => {
    void query;
    selected = 0;
  });

  // Keep the chosen file in view.
  $effect(() => {
    void selected;
    listEl?.querySelector(".chosen")?.scrollIntoView({ block: "nearest" });
  });

  function onKey(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      selected = Math.min(selected + 1, order.length - 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      selected = Math.max(selected - 1, 0);
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (order[selected]) onPick(order[selected]);
    } else if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      onClose();
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="file-list"
  class:centered={right === null}
  style:top="{top}px"
  style:right={right === null ? null : `${right}px`}
  onclick={(e) => e.stopPropagation()}
  onkeydown={(e) => e.stopPropagation()}
>
  <input
    bind:value={query}
    onkeydown={onKey}
    onblur={onClose}
    placeholder="Search files ({shortcutLabel()})"
    aria-label="Search files"
    spellcheck="false"
    autocomplete="off"
    use:focusOnMount
  />
  <div class="files" role="listbox" aria-label="Files" bind:this={listEl}>
    {#each groups as g (g.title)}
      <div class="group">{g.title}</div>
      {#each g.files as f (f)}
        {@const i = order.indexOf(f)}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
          class="file"
          class:chosen={i === selected}
          class:current={f === current}
          role="option"
          aria-selected={i === selected}
          tabindex="-1"
          onmousedown={(e) => e.preventDefault()}
          onmousemove={() => (selected = i)}
          onclick={() => onPick(f)}
        >
          {f}
        </div>
      {/each}
    {:else}
      <div class="none">No file matches.</div>
    {/each}
  </div>
</div>

<style>
  .file-list {
    position: fixed;
    z-index: 10;
    width: min(340px, calc(100vw - 24px));
    max-height: min(420px, calc(100vh - 120px));
    display: flex;
    flex-direction: column;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
  }
  .file-list.centered {
    left: 50%;
    transform: translateX(-50%);
    width: min(440px, calc(100vw - 24px));
  }
  input {
    margin: 8px;
    padding: 5px 8px;
    font: inherit;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg);
    color: var(--fg);
  }
  input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .files {
    overflow-y: auto;
    padding: 0 0 6px;
  }
  /* As the board picker's groups (the user's request, 2026-11-20). */
  .group {
    position: sticky;
    top: 0;
    padding: 6px 12px 4px;
    background: var(--group-bg);
    color: var(--group-fg);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .file {
    margin: 0 4px;
    padding: 4px 8px;
    border-radius: 4px;
    font-size: 13px;
    color: var(--fg);
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .file.current {
    font-weight: 600;
  }
  /* One highlight, as the context menus: the chosen file, which the mouse
     moves too. */
  .file.chosen {
    background: var(--accent);
    color: #fff;
  }
  .none {
    padding: 8px 10px;
    font-size: 13px;
    color: var(--muted);
  }
</style>
