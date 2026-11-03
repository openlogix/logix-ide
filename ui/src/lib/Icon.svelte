<!-- Small line icons (toolbar, start screen), drawn in the current text color. -->
<script lang="ts">
  export type IconName =
    | "verify"
    | "upload"
    | "test"
    | "simulate"
    | "stop"
    | "new"
    | "open"
    | "example"
    | "folder"
    | "chip";

  let { name, size = 18 }: { name: IconName; size?: number } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="2"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if name === "verify"}
    <!-- Check mark -->
    <polyline points="5 12.5 10 17.5 19 7" />
  {:else if name === "upload"}
    <!-- Arrow to the right -->
    <line x1="5" y1="12" x2="19" y2="12" />
    <polyline points="13 6 19 12 13 18" />
  {:else if name === "test"}
    <!-- Lab flask -->
    <path d="M9 3h6" />
    <path
      d="M10 3v6.5L4.8 18.2A1.8 1.8 0 0 0 6.4 21h11.2a1.8 1.8 0 0 0 1.6-2.8L14 9.5V3"
    />
    <path d="M7.2 15h9.6" />
  {:else if name === "stop"}
    <!-- Solid square -->
    <rect x="6" y="6" width="12" height="12" rx="1.5" fill="currentColor" />
  {:else if name === "simulate"}
    <!-- Digital waveform -->
    <polyline points="2 17 6 17 6 7 11 7 11 17 15 17 15 7 19 7 19 17 22 17" />
  {:else if name === "new"}
    <!-- Page with a plus -->
    <path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z" />
    <polyline points="14 3 14 8 19 8" />
    <line x1="12" y1="11" x2="12" y2="17" />
    <line x1="9" y1="14" x2="15" y2="14" />
  {:else if name === "open"}
    <!-- Open folder -->
    <path d="M3 19V6a2 2 0 0 1 2-2h4l2 2h6a2 2 0 0 1 2 2v2" />
    <path d="M3 19l2.6-7.2A2 2 0 0 1 7.5 10.5H21l-2.8 7.7a2 2 0 0 1-1.9 1.3H3z" />
  {:else if name === "example"}
    <!-- Open book -->
    <path d="M2 5h6a4 4 0 0 1 4 4v11a3 3 0 0 0-3-3H2z" />
    <path d="M22 5h-6a4 4 0 0 0-4 4v11a3 3 0 0 1 3-3h7z" />
  {:else if name === "chip"}
    <!-- A chip with pins on both sides: a pin file (not a push pin, which
         reads as "pin this tab") -->
    <rect x="7" y="4" width="10" height="16" rx="1.5" />
    <line x1="3" y1="8" x2="7" y2="8" />
    <line x1="3" y1="12" x2="7" y2="12" />
    <line x1="3" y1="16" x2="7" y2="16" />
    <line x1="17" y1="8" x2="21" y2="8" />
    <line x1="17" y1="12" x2="21" y2="12" />
    <line x1="17" y1="16" x2="21" y2="16" />
  {:else if name === "folder"}
    <!-- Closed folder -->
    <path d="M3 6a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
  {/if}
</svg>
