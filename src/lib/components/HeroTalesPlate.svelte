<script lang="ts">
  /**
   * A door to the shelf, in the hero's row of doors.
   *
   * Shows the one tale the room shows large — its photograph and its title —
   * and nothing else. It fetches its own shelf rather than widening the home
   * page's load, and when the shelf is bare it renders nothing at all: an
   * empty door is worse than no door.
   */
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import { api } from '$lib/api';
  import { t, lang } from '$lib/i18n';
  import { leafCopy, leafCoverUrl, leafHref } from '$lib/gazette';
  import { leadTale } from '$lib/tales';
  import type { GazetteLeaf } from '$lib/types/api';
  import AppImage from '$lib/components/AppImage.svelte';

  let tales = $state<GazetteLeaf[]>([]);
  let lead = $derived(leadTale(tales));
  let copy = $derived(lead ? leafCopy(lead, $lang) : null);
  let cover = $derived(lead ? leafCoverUrl(lead) : '');

  // One door among several: if the shelf cannot be read, this door simply is
  // not there. `getTales` throws so the prerenderer cannot ship an empty shelf
  // silently (see api.ts); here the honest answer really is "render nothing".
  onMount(async () => {
    try {
      tales = await api.getTales();
    } catch {
      tales = [];
    }
  });
</script>

{#if lead && copy}
  <a class="tales-door" href={leafHref(lead, 'home_tales')} in:fade={{ duration: 500 }}>
    <span class="tales-face" aria-hidden="true">
      {#if cover}
        <AppImage src={cover} alt="" class="tales-img" sizes="64px" />
      {:else}
        <span class="tales-glyph">✦</span>
      {/if}
    </span>
    <span class="tales-copy">
      <b>{copy.title}</b>
      <span>{$t('homeDoorTalesHint')}</span>
    </span>
  </a>
{/if}

<style>
  /* Deliberately the same grid, rules and type as the neighbouring doors —
     this is one more door in the row, not a widget pasted beside them. */
  .tales-door {
    display: grid;
    grid-template-columns: 64px minmax(0, 1fr);
    gap: 14px;
    align-items: center;
    width: 100%;
    padding: 12px 0;
    text-decoration: none;
    color: inherit;
  }

  .tales-door:focus-visible {
    outline: 2px solid rgba(198, 95, 60, 0.56);
    outline-offset: 3px;
  }

  .tales-face {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 64px;
    height: 64px;
    overflow: hidden;
    background: #1a120e;
    border: 1px solid var(--border, #d8c6b1);
  }
  .tales-face :global(.app-image-wrap),
  .tales-face :global(img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
    filter: sepia(0.35) contrast(0.94);
    opacity: 0.82;
    transition: opacity 0.2s ease;
  }
  .tales-door:hover .tales-face :global(img) { opacity: 1; }

  .tales-glyph {
    font-size: 20px;
    color: var(--copper, #c65f3c);
    opacity: 0.7;
  }

  .tales-copy {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .tales-copy b {
    font-family: 'Cormorant Garamond', serif;
    font-size: 22px;
    font-weight: 500;
    line-height: 1.15;
    color: var(--ink, #34251c);
    transition: color 0.18s ease;
    /* The neighbouring doors carry two-word labels; a tale's title is a
       sentence. Two lines rather than an ellipsis — a title cut mid-word is
       a worse invitation than a title that takes the room it needs. */
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
  }
  .tales-door:hover .tales-copy b,
  .tales-door:focus-visible .tales-copy b { color: var(--copper, #c65f3c); }

  .tales-copy span {
    font-size: 12px;
    line-height: 1.35;
    color: var(--muted, #5f4636);
  }
</style>
