<script lang="ts">
  // Кошелёк. Один отрисовщик на два места: шапку карточки заданий и полоску,
  // которая закрепляется сверху, когда карточка ушла вверх. Две реализации
  // трёх чисел однажды разошлись бы, и разошлись бы молча.
  //
  // Числа крупные и табличные — не мода, а причина: цифра одинаковой ширины не
  // прыгает при изменении, и «335» читается как значение, а не как слово в
  // предложении «У вас пыли 335».
  import { t } from '$lib/i18n';

  let {
    dust,
    feed,
    settled = null,
    compact = false,
  }: {
    dust: number;
    feed: number;
    /** Сколько пришло с прошлого раза. Рядом с числом, а не отдельной строкой:
     *  это изменение того же значения, а не отдельная новость. */
    settled?: number | null;
    compact?: boolean;
  } = $props();
</script>

<span class="amount" class:amount--compact={compact}>
  <i class="dot dot--dust" aria-hidden="true"></i><b>{dust}</b>
  <span class="coin">{$t('battleGreetDust')}</span>
</span>
<span class="amount" class:amount--compact={compact}>
  <i class="dot dot--feed" aria-hidden="true"></i><b>{feed}</b>
  <span class="coin">{$t('battleGreetFeed')}</span>
</span>
{#if settled !== null && settled > 0}
  <span class="delta">+{settled}</span>
{/if}

<style>
  .amount {
    display: inline-flex;
    align-items: baseline;
    gap: 0.4rem;
    font-family: 'Inter', system-ui, sans-serif;
    font-size: 0.85rem;
    color: #6f3b24;
  }

  .amount b {
    font-size: 1.45rem;
    font-weight: 500;
    letter-spacing: -0.01em;
    color: #34251c;
    font-variant-numeric: tabular-nums;
  }

  .amount--compact b {
    font-size: 1.05rem;
  }

  .dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 50%;
    align-self: center;
  }

  .dot--dust {
    background: #cbb79c;
  }

  .dot--feed {
    background: #6f3b24;
  }

  .delta {
    font-family: 'Inter', system-ui, sans-serif;
    font-size: 0.78rem;
    color: #c65f3c;
  }

  @media (max-width: 30rem) {
    .coin {
      display: none;
    }
  }
</style>
