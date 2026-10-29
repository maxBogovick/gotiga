<script lang="ts">
  /**
   * Главное действие страницы работы: «Приобрести» (работа в наличии, оплата
   * через Etsy) или «Хочу похожую» (прочие статусы). Рядом — статус словами.
   * Один компонент на все пять раскладок.
   */
  import { getContext } from 'svelte';
  import { t } from '$lib/i18n';

  const ctx = getContext<App.FigurineDetailContext>('figurine-detail');

  let available = $derived(ctx.figurine.status === 'available');
  let statusLine = $derived(available ? $t('buyAvailableNote') : ctx.statusLabel(ctx.figurine.status));

  function press() {
    if (available) ctx.openBuyDialog();
    else ctx.openRequestModal('similar');
  }
</script>

<div class="primary-deed">
  <button type="button" class="primary-deed-btn" onclick={press}>
    {available ? $t('buyAction') : $t('buySimilar')}
  </button>
  <span class="primary-deed-status primary-deed-status--{ctx.figurine.status}">{statusLine}</span>
</div>

<style>
  .primary-deed {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem 0.9rem;
    margin: 0.9rem 0 0.2rem;
  }
  .primary-deed-btn {
    min-height: 2.75rem;
    padding: 0 1.6rem;
    border: 1px solid #c65f3c;
    border-radius: 6px;
    color: #fff9f0;
    background: #c65f3c;
    font-size: 0.95rem;
    font-weight: 600;
    cursor: pointer;
  }
  .primary-deed-btn:hover {
    border-color: #6f3b24;
    background: #6f3b24;
  }
  .primary-deed-btn:focus-visible {
    outline: 2px solid #6f3b24;
    outline-offset: 2px;
  }
  .primary-deed-status {
    font-size: 0.85rem;
    color: #6f3b24;
  }
</style>
