<!-- Small line icons for the toolbar, drawn in the current text color. -->
<script lang="ts">
  export type IconName = "verify" | "upload" | "test" | "simulate" | "stop";

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
  {/if}
</svg>
