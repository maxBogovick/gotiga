<script lang="ts">
  // The leaf before a card is taken — same veil as the taking ceremony,
  // not a shop modal. The body is readable here: the miniature on the shelf
  // cannot hold it, and "Get it" under a closed book was the old lie.
  import { fade } from 'svelte/transition';
  import { t, lang } from '$lib/i18n';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import {
    bodyPassport,
    BODY_STAT_LABELS,
    cardCopy,
    channelLabelKey,
    kindLabelKey,
    pricesOf,
    workHref,
    type Coin,
  } from '$lib/battles';
  import type { BattleCard as BattleCardDto, BattleFrame } from '$lib/types/api';

  let {
    card,
    frames = null,
    signedIn = false,
    owned = false,
    busy = false,
    canAfford = () => false,
    loginHref = '/login',
    ontake = () => {},
    onclose,
    complaint = null,
    taking = true,
  }: {
    card: BattleCardDto;
    frames?: BattleFrame[] | null;
    signedIn?: boolean;
    owned?: boolean;
    busy?: boolean;
    canAfford?: (coin: Coin, amount: number) => boolean;
    loginHref?: string;
    ontake?: (coin: Coin, amount: number) => void;
    onclose: () => void;
    complaint?: string | null;
    /** На доске лист только читают: ни «Получить», ни «Ваша». */
    taking?: boolean;
  } = $props();

  let face = $derived(cardCopy(card, $lang));
  let prices = $derived(pricesOf(card));
  let href = $derived(workHref(card));
  let passport = $derived(bodyPassport(card));
  let kindWord = $derived($t(kindLabelKey(card.kind)));
  let channelWord = $derived(
    (() => {
      const key = channelLabelKey(card.attackChannel);
      return key ? $t(key) : '';
    })(),
  );

  function onkey(e: KeyboardEvent) {
    if (e.key !== 'Escape') return;
    e.stopImmediatePropagation();
    onclose();
  }
</script>

<svelte:window onkeydown={onkey} />

<!-- Click the dust to close; the sheet itself keeps the pointer. -->
<div
  class="veil"
  role="presentation"
  onclick={onclose}
  transition:fade={{ duration: 250 }}
>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    class="sheet"
    role="dialog"
    aria-modal="true"
    aria-label={face.title}
    tabindex="-1"
    onpointerdown={(e) => e.stopPropagation()}
    onclick={(e) => e.stopPropagation()}
  >
    <div class="face">
      <BattleCard {card} {frames} owned={true} transition={false} interactive={false} />
    </div>

    <div class="side">
      <p class="kind-line">
        {kindWord}{#if channelWord}<span class="sep">·</span>{channelWord}{/if}
      </p>

      <dl class="passport">
        <div>
          <dt>{$t('battlesCostLabel')}</dt>
          <dd>{card.cost}</dd>
        </div>
        <div>
          <dt>{$t('battlesPowerLabel')}</dt>
          <dd>{card.power}</dd>
        </div>
        {#each passport as row (row.field)}
          <div>
            <dt>{$t(BODY_STAT_LABELS[row.field])}</dt>
            <dd>{row.value}</dd>
          </div>
        {/each}
      </dl>

      {#if taking && owned}
        <p class="yours">{$t('battlesYours')}</p>
      {:else if taking && signedIn}
        <div class="prices">
          {#each prices as price (price.coin)}
            <p class="price">
              <span class="amount">{price.amount}</span>
              <span class="coin">
                {price.coin === 'dust' ? $t('battlesCoinDust') : $t('battlesCoinFeed')}
              </span>
              <button
                type="button"
                class="take"
                disabled={busy || !canAfford(price.coin, price.amount)}
                onclick={() => ontake(price.coin, price.amount)}
              >
                {busy
                  ? $t('battlesTaking')
                  : canAfford(price.coin, price.amount)
                    ? $t('battlesTake')
                    : $t('battlesNotEnough')}
              </button>
            </p>
          {/each}
        </div>
      {:else if taking}
        <div class="prices">
          {#each prices as price (price.coin)}
            <p class="price">
              <span class="amount">{price.amount}</span>
              <span class="coin">
                {price.coin === 'dust' ? $t('battlesCoinDust') : $t('battlesCoinFeed')}
              </span>
            </p>
          {/each}
        </div>
        <a class="take take--link" href={loginHref}>{$t('battlesTake')}</a>
        <p class="hint">
          <a href={loginHref}>{$t('battlesSignInToTake')}</a>
        </p>
      {/if}

      {#if complaint}
        <p class="fault">{complaint}</p>
      {/if}

      {#if href}
        <a class="work" href={href}>{card.figurineName || face.title}</a>
      {/if}
    </div>
  </div>
</div>

<style>
  .veil {
    position: fixed;
    inset: 0;
    z-index: 90;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2rem 1.25rem;
    overflow-y: auto;
    background: rgba(52, 37, 28, 0.64);
    backdrop-filter: blur(2px);
    cursor: default;
  }

  .sheet {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: center;
    gap: 1.5rem 1.75rem;
    max-width: 46rem;
  }

  /* 400px: above the shelf query, so lore and traits are the card's own. */
  .face {
    width: min(25rem, 86vw);
    flex: 0 0 auto;
  }

  /* A leaf of paper beside the card — not ghost type over the shelf.
     The card is the thing; this is the note that names its body. */
  .side {
    width: min(15.5rem, 86vw);
    padding: 1.2rem 1.3rem 1.35rem;
    color: #34251c;
    background: #f8f1e7;
    border: 1px solid #d8c6b1;
    box-shadow:
      inset 0 0 0 3px #f8f1e7,
      inset 0 0 0 4px #d8c6b1,
      0 2px 18px rgba(52, 37, 28, 0.22);
    font-family: Georgia, 'Fraunces', serif;
  }

  .kind-line {
    margin: 0 0 0.85rem;
    font-size: 0.68rem;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: #6f3b24;
  }

  .sep {
    margin: 0 0.45em;
    opacity: 0.45;
  }

  .passport {
    display: flex;
    flex-direction: column;
    gap: 0.28rem;
    margin: 0;
  }

  .passport div {
    display: flex;
    align-items: baseline;
    gap: 0.85rem;
  }

  .passport dt {
    flex: 0 0 8.6em;
    margin: 0;
    font-size: 0.7rem;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #6f3b24;
  }

  .passport dd {
    margin: 0;
    font-size: 1.2rem;
    font-variant-numeric: tabular-nums;
    color: #34251c;
  }

  .yours {
    margin: 1.1rem 0 0;
    font-size: 0.72rem;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #8a6a55;
  }

  .prices {
    margin: 1.1rem 0 0;
    padding-top: 0.85rem;
    border-top: 1px solid #d8c6b1;
  }

  .price {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.4rem 0.55rem;
    margin: 0.2rem 0 0;
    font-size: 0.68rem;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: #6f3b24;
  }

  .amount {
    font-size: 1.05rem;
    letter-spacing: 0;
    color: #34251c;
  }

  .coin {
    color: #6f3b24;
  }

  /* The same word as on the shelf label: an annotation, not a shop button. */
  .take {
    padding: 0;
    font: inherit;
    font-size: 0.78rem;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #6f3b24;
    background: none;
    border: none;
    border-bottom: 1px solid rgba(111, 59, 36, 0.4);
    cursor: pointer;
  }

  .take:hover:not(:disabled) {
    color: #c65f3c;
    border-bottom-color: #c65f3c;
  }

  .take:disabled {
    color: #8a6a55;
    border-bottom-color: transparent;
    cursor: default;
  }

  .take--link {
    display: inline-block;
    margin-top: 0.7rem;
    text-decoration: none;
  }

  .hint {
    margin: 0.55rem 0 0;
    font-size: 0.88rem;
    line-height: 1.5;
    color: #5f4636;
  }

  .hint a {
    color: inherit;
    text-decoration: none;
    border-bottom: 1px solid rgba(111, 59, 36, 0.35);
  }

  .work {
    display: inline-block;
    margin-top: 1.15rem;
    font-size: 0.82rem;
    color: #8a6a55;
    text-decoration: none;
    border-bottom: 1px solid transparent;
  }

  .work:hover {
    color: #34251c;
    border-bottom-color: #d8c6b1;
  }

  .fault {
    margin: 0.85rem 0 0;
    font-size: 0.88rem;
    color: #8f2f22;
  }

  @media (prefers-reduced-motion: reduce) {
    .veil {
      backdrop-filter: none;
    }
  }
</style>
