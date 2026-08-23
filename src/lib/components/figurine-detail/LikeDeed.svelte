<script lang="ts">
  /**
   * Heart like. Canonical key is the figurine UUID (same as archive / wishlist).
   * The URL handle is only an alias so leftover slugs cannot resurrect on reload.
   */
  import { onMount, getContext } from 'svelte';
  import { t } from '$lib/i18n';
  import { savedFigurines } from '$lib/stores/saved-figurines.svelte';

  let { extraClass = '' }: { extraClass?: string } = $props();

  const ctx = getContext<App.FigurineDetailContext>('figurine-detail');
  let figId = $derived(ctx.figurine.id);
  let handle = $derived(ctx.id);
  let aliases = $derived(handle && handle !== figId ? [handle] : []);
  let liked = $derived(savedFigurines.has(figId) || savedFigurines.hasAny(aliases));
  let lockUntil = 0;

  onMount(() => {
    savedFigurines.load();
  });

  function onClick(event: MouseEvent) {
    event.preventDefault();
    event.stopPropagation();
    const now = Date.now();
    if (now < lockUntil) return;
    lockUntil = now + 400;
    void savedFigurines.set(figId, !liked, aliases);
    ctx.analyticsClient?.cta('wishlist');
  }
</script>

<button
  type="button"
  class="deed {extraClass}"
  class:deed--on={liked}
  onclick={onClick}
  aria-pressed={liked}
  aria-label={liked ? $t('detailLiked') : $t('detailLike')}
>
  <span class="deed-ico" aria-hidden="true">
    <svg width="15" height="15" viewBox="0 0 14 14" fill="none" aria-hidden="true">
      <path
        d="M7 12.5C7 12.5 1 8.5 1 4.5C1 2.5 2.5 1 4.5 1C5.5 1 6.5 1.8 7 3C7.5 1.8 8.5 1 9.5 1C11.5 1 13 2.5 13 4.5C13 8.5 7 12.5 7 12.5Z"
        fill={liked ? 'currentColor' : 'none'}
        stroke="currentColor"
        stroke-width="1.15"
        stroke-linejoin="round"
      />
    </svg>
  </span>
  <span class="deed-label">{liked ? $t('detailLiked') : $t('detailLike')}</span>
</button>
