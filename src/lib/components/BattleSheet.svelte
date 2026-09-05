<script lang="ts">
  // Лист карты — полный разворот, не бланк рядом с миниатюрой и не RPG-витрина.
  // Расклад с мерки «класса»: имя и голос, удар четырьмя числами, умения
  // с текстом, портрет в своей рамке, внизу всё тело целиком. Ноль — это
  // число, на листе его видно. Золота и тёмной полоски параметров нет.
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import { t, lang } from '$lib/i18n';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import {
    abilityCopy,
    cardCopy,
    channelLabelKey,
    frameForCard,
    frameName,
    headerCopy,
    kindLabelKey,
    pricesOf,
    traitCopy,
    workHref,
    type Coin,
  } from '$lib/battles';
  import type {
    AbilityShape,
    AbilityTrigger,
    AbilityVerb,
    BattleCard as BattleCardDto,
    BattleFrame,
    CardAbility,
  } from '$lib/types/api';
  import type { TranslationKey } from '$lib/i18n';

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
    taking?: boolean;
  } = $props();

  let leaf = $state<HTMLDivElement | null>(null);

  const VERB_LABELS = {
    damage: 'battlesVerbDamage',
    dot: 'battlesVerbDot',
    heal: 'battlesVerbHeal',
    hot: 'battlesVerbHot',
    shield: 'battlesVerbShield',
    zone: 'battlesVerbZone',
    bless: 'battlesVerbBless',
    curse: 'battlesVerbCurse',
    control: 'battlesVerbControl',
    silence: 'battlesVerbSilence',
    disarm: 'battlesVerbDisarm',
    charm: 'battlesVerbCharm',
    veil: 'battlesVerbVeil',
    guard: 'battlesVerbGuard',
    immune: 'battlesVerbImmune',
    thorns: 'battlesVerbThorns',
    move: 'battlesVerbMove',
    summon: 'battlesVerbSummon',
    sacrifice: 'battlesVerbSacrifice',
    cleanse: 'battlesVerbCleanse',
    dispel: 'battlesVerbDispel',
    mana: 'battlesVerbMana',
  } as const satisfies Record<AbilityVerb, TranslationKey>;

  const SHAPE_LABELS = {
    self: 'battlesShapeSelf',
    one: 'battlesShapeOne',
    adjacent: 'battlesShapeAdjacent',
    chain: 'battlesShapeChain',
    line: 'battlesShapeLine',
    radius: 'battlesShapeRadius',
    side: 'battlesShapeSide',
    cell: 'battlesShapeCell',
  } as const satisfies Record<AbilityShape, TranslationKey>;

  const TRIGGER_LABELS = {
    active: 'battlesTriggerActive',
    onPlay: 'battlesTriggerOnPlay',
    onHit: 'battlesTriggerOnHit',
    onDamaged: 'battlesTriggerOnDamaged',
    onDeath: 'battlesTriggerOnDeath',
    turnStart: 'battlesTriggerTurnStart',
    aura: 'battlesTriggerAura',
    once: 'battlesTriggerOnce',
  } as const satisfies Record<AbilityTrigger, TranslationKey>;

  let face = $derived(cardCopy(card, $lang));
  let head = $derived(headerCopy(card, $lang));
  let prices = $derived(pricesOf(card));
  let href = $derived(workHref(card));
  let workName = $derived(card.figurineName?.trim() || '');
  let rank = $derived(frameName(frameForCard(card, frames), $lang));
  let kindWord = $derived($t(kindLabelKey(card.kind)));
  let channelWord = $derived(
    $t(channelLabelKey(card.attackChannel) ?? 'battlesChannelPhysical'),
  );
  let printRules = $derived((card.abilities?.length ?? 0) > 0);
  let traits = $derived(
    (card.traits ?? [])
      .map((row) => traitCopy(row, $lang))
      .filter((row) => row.name || row.text),
  );
  let gifts = $derived.by(() => {
    void $t;
    return (card.abilities ?? []).map((row) => ({
      id: row.id,
      name: abilityCopy(row, $lang).name,
      line: abilityLine(row),
    }));
  });

  type Figure = { label: TranslationKey; value: number };
  /** Четыре числа удара — как четыре кружка на мерке, без золотой оправы.
   *  Ноль здесь тоже число: на листе «не бьёт с шага» видно, а не пропадает. */
  let blow = $derived<Figure[]>([
    { label: 'battlesCostLabel', value: card.cost },
    { label: 'battlesPowerLabel', value: card.power },
    { label: 'battleStatReach', value: card.reach },
    { label: 'battleStatStep', value: card.step },
  ]);

  /** Всё тело, которым играют. Миниатюра прячет нули; лист называет каждое. */
  let body = $derived<Figure[]>([
    { label: 'battlesHealthLabel', value: card.health },
    { label: 'battlesManaLabel', value: card.mana },
    { label: 'battleStatArmour', value: card.armor },
    { label: 'battleStatWard', value: card.ward },
    { label: 'battleStatMend', value: card.mend },
    { label: 'battlesSpeedLabel', value: card.speed },
  ]);

  function abilityLine(row: CardAbility): string {
    const parts = [
      $t(TRIGGER_LABELS[row.trigger]),
      $t(VERB_LABELS[row.verb]),
      $t(SHAPE_LABELS[row.shape]),
    ];
    if (row.amount) parts.push(String(row.amount));
    if (row.range) parts.push(`${$t('battleStatReach')} ${row.range}`);
    if (row.radius) parts.push(`${$t('battlesAbilitySpan')} ${row.radius}`);
    if (row.manaCost) parts.push(`${$t('battlesManaLabel')} ${row.manaCost}`);
    if (row.cooldown) parts.push(`${$t('battlesAbilityCooldown')} ${row.cooldown}`);
    if (row.duration) parts.push(`${$t('battleStatusTurns')} ${row.duration}`);
    return parts.join(' · ');
  }

  onMount(() => {
    leaf?.focus();
  });

  function onkey(e: KeyboardEvent) {
    if (e.key !== 'Escape') return;
    e.stopImmediatePropagation();
    onclose();
  }
</script>

<svelte:window onkeydown={onkey} />

<div
  class="veil"
  role="presentation"
  onclick={onclose}
  transition:fade={{ duration: 250 }}
>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    bind:this={leaf}
    class="leaf"
    role="dialog"
    aria-modal="true"
    aria-labelledby="battle-sheet-title"
    tabindex="-1"
    onpointerdown={(e) => e.stopPropagation()}
    onclick={(e) => e.stopPropagation()}
  >
    <button type="button" class="dismiss" onclick={onclose}>{$t('battlesTakenClose')}</button>

    <header class="mast">
      {#if card.raceIconUrl}
        <img class="race" src={card.raceIconUrl} alt="" width="36" height="36" />
      {/if}
      <div class="mast-copy">
        <p class="kind">
          {kindWord}<span class="sep">·</span>{channelWord}{#if head.race}<span class="sep">·</span
            >{head.race}{/if}{#if head.type}<span class="sep">·</span>{head.type}{/if}
        </p>
        <h2 id="battle-sheet-title" class="name">{face.title || $t('battlesKindUnit')}</h2>
        {#if rank}
          <p class="rarity">{$t('battlesRankLabel')} · {rank}</p>
        {/if}
      </div>
    </header>

    {#if face.effect}
      <p class="lead">{face.effect}</p>
    {/if}

    <div class="spread">
      <div class="prose">
        {#if blow.length}
          <section class="block">
            <h3 class="block-title">{$t('battlesSheetBlow')}</h3>
            <ul class="figures">
              {#each blow as fig (fig.label)}
                <li class="figure">
                  <b>{fig.value}</b>
                  <span>{$t(fig.label)}</span>
                </li>
              {/each}
            </ul>
          </section>
        {/if}

        {#if traits.length}
          <section class="block">
            <h3 class="block-title">{$t('battlesSheetTraits')}</h3>
            <ul class="gifts">
              {#each traits as trait, i (`${trait.name}-${i}`)}
                <li class="gift" class:gift--voice={!printRules}>
                  <span class="gift-name">
                    {trait.name}{#if trait.other}<span class="gift-other">({trait.other})</span>{/if}
                  </span>
                  {#if trait.text}
                    <span class="gift-text">{trait.text}</span>
                  {/if}
                </li>
              {/each}
            </ul>
          </section>
        {/if}

        {#if gifts.length}
          <section class="block">
            <h3 class="block-title">{$t('battlesSheetAbilities')}</h3>
            <ul class="gifts">
              {#each gifts as gift (gift.id)}
                <li class="gift">
                  {#if gift.name}
                    <span class="gift-name">{gift.name}</span>
                  {/if}
                  <span class="gift-text">{gift.line}</span>
                </li>
              {/each}
            </ul>
          </section>
        {/if}

        {#if face.lore}
          <section class="block">
            <h3 class="block-title">{$t('battlesSheetNote')}</h3>
            <p class="note">{face.lore}</p>
          </section>
        {/if}
      </div>

      <div class="face">
        <BattleCard {card} {frames} owned={true} transition={false} interactive={false} />
      </div>
    </div>

    <section class="params">
      <h3 class="block-title">{$t('battlesSheetParams')}</h3>
      <ul class="param-row">
        {#each body as fig (fig.label)}
          <li class="param">
            <b>{fig.value}</b>
            <span>{$t(fig.label)}</span>
          </li>
        {/each}
      </ul>
    </section>

    <footer class="foot">
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
        {#if prices.length}
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
        {/if}
        <a class="take take--link" href={loginHref}>{$t('battlesSignInToTake')}</a>
      {/if}

      {#if complaint}
        <p class="fault">{complaint}</p>
      {/if}

      {#if href}
        <a class="work" {href}>{workName || $t('battlesWorkLink')}</a>
      {/if}
    </footer>
  </div>
</div>

<style>
  .veil {
    position: fixed;
    inset: 0;
    z-index: 90;
    display: grid;
    place-items: center;
    padding: 1.5rem 1.25rem;
    overflow-y: auto;
    background: rgba(52, 37, 28, 0.64);
    backdrop-filter: blur(2px);
    cursor: default;
  }

  .leaf {
    position: relative;
    width: min(72rem, 100%);
    margin: auto;
    padding: 1.6rem 1.7rem 1.45rem;
    color: #34251c;
    background: #f8f1e7;
    border: 1px solid #d8c6b1;
    outline: 1px solid #d8c6b1;
    outline-offset: 5px;
    transform: rotate(-0.6deg);
    box-shadow: 0 18px 50px rgba(52, 37, 28, 0.35);
  }

  .leaf:focus {
    outline: 1px solid #d8c6b1;
    outline-offset: 5px;
  }

  .dismiss {
    position: absolute;
    top: 1.1rem;
    right: 1.3rem;
    padding: 0 0 1px;
    font-family: 'Inter', system-ui, sans-serif;
    font-size: 0.68rem;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: #8a6a55;
    background: none;
    border: none;
    border-bottom: 1px solid rgba(138, 106, 85, 0.35);
    cursor: pointer;
  }

  .dismiss:hover {
    color: #c65f3c;
    border-bottom-color: rgba(198, 95, 60, 0.5);
  }

  .mast {
    display: flex;
    align-items: center;
    gap: 0.85rem;
    margin: 0 3.5rem 0.85rem 0;
  }

  .race {
    flex: 0 0 auto;
    width: 2.25rem;
    height: 2.25rem;
    object-fit: cover;
    border: 1px solid #d8c6b1;
    border-radius: 20%;
  }

  .kind {
    margin: 0 0 0.28rem;
    font-family: 'Inter', system-ui, sans-serif;
    font-size: 0.68rem;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: #6f3b24;
  }

  .sep {
    margin: 0 0.45em;
    opacity: 0.45;
  }

  .name {
    margin: 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: clamp(1.6rem, 3vw, 2.35rem);
    font-weight: 400;
    line-height: 1.12;
  }

  .rarity {
    margin: 0.35rem 0 0;
    font-family: 'Inter', system-ui, sans-serif;
    font-size: 0.62rem;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #8a6a55;
  }

  .lead {
    max-width: 46rem;
    margin: 0 0 1.35rem;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 1.02rem;
    line-height: 1.55;
    font-style: italic;
    color: #5c4838;
  }

  .spread {
    display: grid;
    grid-template-columns: minmax(0, 1fr) min(25rem, 44%);
    gap: 1.6rem 1.8rem;
    align-items: start;
  }

  .block {
    margin: 0 0 1.25rem;
  }

  .block:last-child {
    margin-bottom: 0;
  }

  .block-title {
    margin: 0 0 0.7rem;
    font-family: 'Inter', system-ui, sans-serif;
    font-size: 0.66rem;
    font-weight: 600;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #8a6a55;
  }

  .figures {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.55rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .figure {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.2rem;
    padding: 0.65rem 0.3rem 0.55rem;
    border: 1px solid #d8c6b1;
    text-align: center;
  }

  .figure b {
    font-family: Georgia, 'Fraunces', serif;
    font-size: 1.45rem;
    font-weight: 400;
    font-variant-numeric: tabular-nums;
    line-height: 1;
  }

  .figure span {
    font-family: 'Inter', system-ui, sans-serif;
    font-size: 0.58rem;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #6f3b24;
  }

  .gifts {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .gift {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    padding: 0.55rem 0;
    border-bottom: 1px solid rgba(216, 198, 177, 0.55);
  }

  .gift:first-child {
    padding-top: 0;
  }

  .gift-name {
    font-family: Georgia, 'Fraunces', serif;
    font-size: 1.02rem;
  }

  .gift-other {
    margin-left: 0.4em;
    font-size: 0.78em;
    color: #8a6a55;
  }

  .gift-text {
    font-family: 'Inter', system-ui, sans-serif;
    font-size: 0.82rem;
    line-height: 1.45;
    color: #5c4838;
  }

  .gift--voice .gift-text {
    font-family: Georgia, 'Fraunces', serif;
    font-style: italic;
  }

  .note {
    margin: 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.95rem;
    line-height: 1.55;
    color: #5c4838;
  }

  .face {
    width: 100%;
  }

  .params {
    margin: 1.4rem 0 0;
    padding: 1rem 0 0;
    border-top: 1px solid #d8c6b1;
  }

  .param-row {
    display: grid;
    grid-template-columns: repeat(6, minmax(0, 1fr));
    gap: 0.55rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .param {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.2rem;
    padding: 0.55rem 0.25rem 0.5rem;
    border: 1px solid #d8c6b1;
    text-align: center;
  }

  .param b {
    font-family: Georgia, 'Fraunces', serif;
    font-size: 1.25rem;
    font-weight: 400;
    font-variant-numeric: tabular-nums;
    line-height: 1;
  }

  .param span {
    font-family: 'Inter', system-ui, sans-serif;
    font-size: 0.58rem;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #6f3b24;
  }

  .foot {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.7rem 1.4rem;
    margin: 1.15rem 0 0;
    padding: 0.95rem 0 0;
    border-top: 1px solid #d8c6b1;
  }

  .yours {
    margin: 0;
    font-size: 0.72rem;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #8a6a55;
  }

  .prices {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem 1.2rem;
  }

  .price {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.4rem 0.55rem;
    margin: 0;
    font-size: 0.68rem;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: #6f3b24;
  }

  .amount {
    font-family: Georgia, 'Fraunces', serif;
    font-size: 1.05rem;
    letter-spacing: 0;
    color: #34251c;
  }

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
    text-decoration: none;
  }

  .work {
    margin-left: auto;
    font-family: Georgia, 'Fraunces', serif;
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
    flex-basis: 100%;
    margin: 0;
    font-size: 0.88rem;
    color: #8f2f22;
  }

  @media (max-width: 52rem) {
    .leaf {
      transform: none;
      padding: 1.35rem 1.15rem 1.2rem;
    }

    .spread {
      grid-template-columns: 1fr;
    }

    .face {
      order: -1;
      width: min(25rem, 100%);
      margin: 0 auto;
    }

    .figures {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .param-row {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }

    .work {
      margin-left: 0;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .veil {
      backdrop-filter: none;
    }

    .leaf {
      transform: none;
    }
  }
</style>
