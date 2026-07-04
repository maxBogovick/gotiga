<script lang="ts">
  import type { MarkTone } from '$lib/types/api';

  // Line-icon set for the 3 private mark tones, drawn in the same 14×14
  // stroke style as the neighbouring gallery-heart/gallery-lens icons (not
  // text glyphs — those read as unclear tofu at 14px in some fonts).
  // 'bookmark' is the idle "leave a mark" invitation shown before any tone
  // is picked.
  let { tone, active = false }: { tone: MarkTone | 'bookmark'; active?: boolean } = $props();
</script>

{#if tone === 'touched'}
  <!-- teardrop — "this moved me" -->
  <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
    <path
      d="M7 1.4C7 1.4 3.3 6.5 3.3 9.2C3.3 11.3 4.9 13 7 13C9.1 13 10.7 11.3 10.7 9.2C10.7 6.5 7 1.4 7 1.4Z"
      fill={active ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="1.15" stroke-linejoin="round"
    />
  </svg>
{:else if tone === 'mesmerized'}
  <!-- sparkle — "captivated" -->
  <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
    <path
      d="M7 1L8.3 5.7L13 7L8.3 8.3L7 13L5.7 8.3L1 7L5.7 5.7Z"
      fill={active ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="1.1" stroke-linejoin="round"
    />
  </svg>
{:else if tone === 'desired'}
  <!-- flame — "want one like this" -->
  <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
    <path
      d="M7 1.2C8.3 3 9.8 5 9.8 7.3C9.8 9.4 8.6 11 7 11C5.4 11 4.2 9.4 4.2 7.3C4.2 6.2 4.7 5.2 5.3 4.3C5.5 5 5.9 5.6 6.3 5.9C6.1 4.7 6.4 2.8 7 1.2Z"
      fill={active ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="1.1" stroke-linejoin="round"
    />
  </svg>
{:else}
  <!-- bookmark ribbon — universal "save/mark this", distinct from the heart -->
  <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
    <path
      d="M4 1.5H10C10.55 1.5 11 1.95 11 2.5V12.5L7 10L3 12.5V2.5C3 1.95 3.45 1.5 4 1.5Z"
      fill="none" stroke="currentColor" stroke-width="1.15" stroke-linejoin="round"
    />
  </svg>
{/if}
