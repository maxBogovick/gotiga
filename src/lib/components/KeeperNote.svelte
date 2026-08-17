<script lang="ts">
  /**
   * A blotter on the home collection: the visitor writes a note to the keeper.
   * The reel below is not filtered. Results are a handful of plates, not a shop grid.
   * No view-transition-name on thumbs — a duplicate figurine-{id} would abort the morph.
   *
   * This frame is the first lesson. Once it leaves the screen, a loupe appears
   * in the header and recalls the same cabinet — centered under the nav.
   */
  import KeeperFrame from '$lib/components/KeeperFrame.svelte';
  import { keeper } from '$lib/stores/keeper.svelte';
  import type { FigurineListItem } from '$lib/types/api';

  type Props = {
    figurines: FigurineListItem[];
    /** Works currently on the home reel — those plates scroll the hall, the rest leave it. */
    reelIds: string[];
  };

  let { figurines, reelIds }: Props = $props();
  let ceremonialEl = $state<HTMLElement | null>(null);

  $effect(() => {
    keeper.seed(figurines, reelIds);
    return () => {
      keeper.setBlotterOffscreen(false);
    };
  });

  $effect(() => {
    const el = ceremonialEl;
    if (!el || typeof window === 'undefined') return;

    const update = () => {
      const rect = el.getBoundingClientRect();
      const headerH =
        parseFloat(
          getComputedStyle(document.documentElement).getPropertyValue('--site-header-h'),
        ) || 54;
      keeper.setBlotterOffscreen(rect.bottom <= headerH + 8);
    };

    const io = new IntersectionObserver(update, { threshold: [0, 0.01, 0.1, 1] });
    io.observe(el);
    window.addEventListener('scroll', update, { passive: true });
    window.addEventListener('resize', update);
    update();
    return () => {
      io.disconnect();
      window.removeEventListener('scroll', update);
      window.removeEventListener('resize', update);
    };
  });
</script>

{#if figurines.length > 0}
<div class="keeper-slot" bind:this={ceremonialEl} inert={keeper.panelOpen}>
  <KeeperFrame titleId="keeper-note-title" source="home_keeper" />
</div>
{/if}

<style>
  .keeper-slot {
    width: 100%;
    max-width: var(--reel-card-width, 64rem);
    margin: clamp(1.25rem, 3vw, 2.25rem) auto 0;
  }
</style>
