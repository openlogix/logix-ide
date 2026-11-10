<!--
  Tools > Connected Devices: the USB devices connected, each a collapsible
  group (open when it may be a board), with the boards it may be as bullet
  lists in two collapsible groups: Matches (its most likely boards, open)
  and Other possible matches (weaker ones, closed). Board names select the
  board.
-->
<script lang="ts">
  import type { Devices } from "./backend";
  import { SvelteSet } from "svelte/reactivity";
  import type { UsbDevice } from "./backend";
  import { deviceBoardGroups } from "./devices";

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
    /** A board name was clicked: select that board. */
    onPick: (board: string) => void;
    onClose: () => void;
  } = $props();

  /**
   * The groups the user opened or closed, as "<device id>", "<device
   * id>:matches" or "<device id>:others": each is the opposite of its
   * default.
   */
  const toggled = new SvelteSet<string>();
  function toggle(key: string) {
    if (toggled.has(key)) toggled.delete(key);
    else toggled.add(key);
  }
  /** Whether a group is open: its default, unless the user toggled it. */
  const isOpen = (key: string, byDefault: boolean) => byDefault !== toggled.has(key);

  /** The groups of a device: the boards are sorted by name among others. */
  function groupsOf(d: UsbDevice) {
    const { matches, others } = deviceBoardGroups(d, board);
    return {
      matches,
      others: [...others].sort((a, b) => boardName(a).localeCompare(boardName(b))),
    };
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
          <ul class="devices">
            {#each devices.usb as d (d.id)}
              {@const g = groupsOf(d)}
              {@const fits = g.matches.length + g.others.length > 0}
              {@const open = isOpen(d.id, fits)}
              <li class="device">
                <button class="toggle device-toggle" aria-expanded={open} onclick={() => toggle(d.id)}
                  >{@render chevron(open)}<span class="name">{d.label}</span>{#if d.serialNumber}{" "}<span class="muted">· serial {d.serialNumber}</span>{/if}</button
                >
                {#if open}
                  <div class="groups">
                    {#if !fits}
                      <p class="muted">Not a board Logix IDE knows.</p>
                    {/if}
                    {#each [{ key: "matches", label: "Matches", list: g.matches, byDefault: true }, { key: "others", label: "Other possible matches", list: g.others, byDefault: false }] as group (group.key)}
                      {#if group.list.length > 0}
                        {@const groupOpen = isOpen(`${d.id}:${group.key}`, group.byDefault)}
                        <button
                          class="toggle"
                          aria-expanded={groupOpen}
                          onclick={() => toggle(`${d.id}:${group.key}`)}
                          >{@render chevron(groupOpen)}{group.label}<span class="count">{group.list.length}</span></button
                        >
                        {#if groupOpen}
                          <ul class="boards">
                            {#each group.list as id (id)}
                              <li>
                                <button
                                  class="board-pick"
                                  title="Select the {boardName(id)}"
                                  onclick={() => onPick(id)}>{boardName(id)}</button
                                >{#if id === board}{" "}<span class="muted">(selected)</span>{/if}
                              </li>
                            {/each}
                          </ul>
                        {/if}
                      {/if}
                    {/each}
                  </div>
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

{#snippet chevron(open: boolean)}<svg
    class="chevron"
    class:open
    viewBox="0 0 12 12"
    width="12"
    height="12"
    aria-hidden="true"
    ><path
      d="M4 2.5 L8 6 L4 9.5"
      fill="none"
      stroke="currentColor"
      stroke-width="1.8"
      stroke-linecap="round"
      stroke-linejoin="round"
    /></svg
  >{/snippet}

<style>
  /* A collapsible group's heading: an arrow and its title. */
  .toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 3px 0;
    border: none;
    background: none;
    color: inherit;
    font: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }
  .device-toggle .name {
    font-weight: 600;
  }
  .chevron {
    flex: none;
    transition: transform 0.1s;
  }
  .chevron.open {
    transform: rotate(90deg);
  }
  .count {
    margin-left: 6px;
    color: var(--muted);
    font-size: 12px;
  }
  .groups {
    padding-left: 18px;
  }
  .groups p {
    margin: 2px 0;
  }
  ul.boards {
    margin: 0 0 4px;
    padding-left: 36px;
    list-style: disc;
  }
  ul.boards li {
    display: list-item;
    padding: 1px 0;
    border: none;
  }
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
