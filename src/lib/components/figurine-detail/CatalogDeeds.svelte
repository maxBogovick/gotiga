<script lang="ts">
  /**
   * Mobile-only deeds under the specimen film: letter, like, passport.
   * Desktop uses CatalogTitleDeeds and CatalogPlateMarks instead.
   */
  import { getContext } from 'svelte';
  import { figurineHref } from '$lib/figurineHref';
  import { t } from '$lib/i18n';
  import LikeDeed from './LikeDeed.svelte';

  const ctx = getContext<App.FigurineDetailContext>('figurine-detail');
</script>

<div class="deed-row catalog-deeds">
  <button
    type="button"
    class="deed deed--primary"
    onclick={() => ctx.openRequestModal(ctx.statusUi.defaultIntent)}
  >
    <span class="deed-ico" aria-hidden="true">
      <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round">
        <rect x="1.5" y="3" width="11" height="8" rx="0.8"/>
        <path d="M1.8 3.4 7 7.4 12.2 3.4"/>
      </svg>
    </span>
    <span class="deed-label">{$t('unifiedOpenRequest')}</span>
  </button>

  <LikeDeed />

  <a
    class="deed"
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
</div>
