<script lang="ts">
  /**
   * Почему оплата идёт через Etsy. Один текст на диалог «Приобрести» и на
   * последний шаг заявки «Хочу похожую»: два пересказа одного объяснения
   * разошлись бы на первой правке.
   */
  import { t } from '$lib/i18n';

  let { compact = false }: { compact?: boolean } = $props();
</script>

{#snippet body()}
  <p class="why-intro">{$t('etsyWhyIntro')}</p>
  <ul class="why-list">
    <li><strong>{$t('etsyWhyMoneyLead')}</strong> {$t('etsyWhyMoney')}</li>
    <li>{$t('etsyWhyTrack')}</li>
    <li>{$t('etsyWhyRest')}</li>
  </ul>
{/snippet}

{#if compact}
  <details class="etsy-why etsy-why--compact">
    <summary>{$t('etsyWhyTitle')}</summary>
    {@render body()}
  </details>
{:else}
  <section class="etsy-why">
    <h4 class="why-title">{$t('etsyWhyTitle')}</h4>
    {@render body()}
  </section>
{/if}

<style>
  .etsy-why {
    color: #34251c;
    font-size: 0.9rem;
    line-height: 1.55;
  }
  .why-title {
    margin: 0 0 0.4rem;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 1.05rem;
    font-weight: 600;
  }
  summary {
    cursor: pointer;
    font-family: Georgia, 'Fraunces', serif;
    font-weight: 600;
    color: #6f3b24;
  }
  summary:focus-visible {
    outline: 2px solid #c65f3c;
    outline-offset: 2px;
  }
  .why-intro {
    margin: 0 0 0.5rem;
  }
  .etsy-why--compact .why-intro {
    margin-top: 0.5rem;
  }
  .why-list {
    margin: 0;
    padding-left: 1.1rem;
  }
  .why-list li + li {
    margin-top: 0.4rem;
  }
</style>
