<script lang="ts">
  /**
   * Desktop-only row under the specimen title: similar, passport, story.
   * Mobile keeps CatalogDeeds + PlateGestures under the film.
   */
  import { getContext } from 'svelte';
  import { figurineHref } from '$lib/figurineHref';
  import { t } from '$lib/i18n';

  const ctx = getContext<App.FigurineDetailContext>('figurine-detail');
  let similarHref = $derived(`/commission?source=${encodeURIComponent(ctx.id)}`);
</script>

<div class="catalog-title-deeds">
  <a
    class="deed catalog-title-deed"
    href={similarHref}
    onclick={() => ctx.analyticsClient?.cta('create_similar')}
  >
    <span class="deed-ico" aria-hidden="true">
      <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="5" cy="4.2" r="1.6"/>
        <path d="M2.4 10.6c.3-1.8 1.4-2.8 2.6-2.8s2.3 1 2.6 2.8"/>
        <circle cx="9.4" cy="4.6" r="1.35"/>
        <path d="M11.8 10.6c-.2-1.4-.9-2.2-1.9-2.4"/>
      </svg>
    </span>
    <span class="deed-label">{$t('commissionCreateSimilarShort')}</span>
  </a>

  <a
    class="deed catalog-title-deed"
    href="{figurineHref(ctx.figurine)}/passport"
    onclick={() => ctx.analyticsClient?.cta('passport')}
  >
    <span class="deed-ico" aria-hidden="true">
      <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
        <rect x="3" y="1.6" width="8" height="10.8" rx="0.8"/>
        <path d="M5.2 4.4h3.6M5.2 6.6h3.6M5.2 8.8h2.2"/>
      </svg>
    </span>
    <span class="deed-label">{$t('detailOpenPassport')}</span>
  </a>

  <button
    type="button"
    class="deed catalog-title-deed"
    onclick={() => ctx.openStoryModal()}
    disabled={ctx.storySaving}
    aria-label={ctx.storySaving ? $t('figurineStorySaving') : $t('figurineStoryShare')}
  >
    <span class="deed-ico" aria-hidden="true">
      {#if ctx.storySaving}
        <span class="deed-spin"></span>
      {:else}
        <svg width="14" height="14" viewBox="0 0 13 13" fill="none" stroke="currentColor" stroke-width="1.35">
          <rect x="2" y="1.5" width="9" height="10" rx="1.2"/>
          <path d="M4.1 8.5 6 6.8l1.3 1.1 1.6-2 1.1 1.4"/>
          <circle cx="4.6" cy="4.1" r="0.65" fill="currentColor" stroke="none"/>
        </svg>
      {/if}
    </span>
    <span class="deed-label">{ctx.storySaving ? $t('figurineStorySaving') : $t('figurineStoryShare')}</span>
  </button>
</div>
