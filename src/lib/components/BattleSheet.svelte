<script lang="ts">
  // Лист карты — разворот рядом с лицом, не бланк на весь экран.
  // Карта слева якорем, паспорт справа вплотную: ширина по содержимому.
  // Ноль — это число, на листе его видно.
  //
  // Одет лист в хром КОМНАТЫ (`chamber-frame`, `chamber-hall`), а не в бумагу
  // дома: карта носит золочёную резьбу, и пергаментный бланк вокруг неё был
  // рамой в раме — витрина спорила с тем, что в ней стоит. Оправа надета
  // девятью кусками (`border-image`), потому что разворот шире, чем высок, а
  // растянутая целиком картинка увела бы резьбу по одной оси.
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import { t, lang } from '$lib/i18n';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import BattleIcon from '$lib/components/BattleIcon.svelte';
  import {
    abilityCopy,
    cardCopy,
    channelLabelKey,
    frameForCard,
    frameName,
    headerCopy,
    kindLabelKey,
    pricesOf,
    statMark,
    statLabel,
    type MarkedStat,
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

  /** Шапка — СПИСОК слов, а не строка с точками, вписанными в разметку между
   *  `{#if}`: разделитель, набранный руками, однажды встанет там, где слова
   *  нет, и лист напечатает «Тело · · Скромная». */
  let kicker = $derived(
    [kindWord, channelWord, head.race, head.type, rank]
      .map((word) => word?.trim() ?? '')
      .filter(Boolean),
  );

  /** Число на листе называется СЛОТОМ, а не парой «ключ слова + картинка»:
   *  слово и знак берутся по одному имени (`statLabel`/`statMark`), и лист,
   *  назвавший их порознь, однажды напишет «Оберег» над щитом брони.
   *
   *  `major` — не размер, а ГЛАВЕНСТВО: сила, здоровье и мана решают партию, а
   *  оберег и лечение у большинства карт нули. Сказано здесь, рядом с числом, а
   *  не классом в разметке, потому что плашку рисует один сниппет: развилка
   *  «крупная ли эта» в разметке была бы вторым списком главных чисел. */
  type Figure = { slot: MarkedStat; value: number; major?: boolean };
  /** Четыре числа удара. Ноль здесь тоже число: на листе «не бьёт с шага»
   *  видно, а не пропадает. */
  let blow = $derived<Figure[]>([
    { slot: 'cost', value: card.cost },
    { slot: 'power', value: card.power, major: true },
    { slot: 'reach', value: card.reach },
    { slot: 'step', value: card.step },
  ]);

  /** Всё тело, которым играют. Миниатюра прячет нули; лист называет каждое. */
  let body = $derived<Figure[]>([
    { slot: 'health', value: card.health, major: true },
    { slot: 'mana', value: card.mana, major: true },
    { slot: 'armor', value: card.armor },
    { slot: 'ward', value: card.ward },
    { slot: 'mend', value: card.mend },
    { slot: 'speed', value: card.speed },
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

<!-- Волосяная черта с ромбом посередине. Один и тот же росчерк стоит и над
     байкой, и на плите под картой: связь половин разворота — это повторённый
     мотив, а не одинаковая темнота фона. -->
{#snippet flourish()}
  <span class="flourish" aria-hidden="true"><span class="pip"></span></span>
{/snippet}

<!-- Плашка числа. Одна на удар и на характеристики: два отрисовщика одной
     плашки — это две плашки, которые однажды разойдутся кеглем. -->
{#snippet plate(fig: Figure)}
  <li class="plate" class:plate--major={fig.major}>
    <span class="mark"><BattleIcon name={statMark(fig.slot)} size="1em" weight={1.3} /></span>
    <b>{fig.value}</b>
    <span class="word">{$t(statLabel(fig.slot))}</span>
  </li>
{/snippet}

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
    <span class="frame" aria-hidden="true"></span>

    <button
      type="button"
      class="dismiss"
      onclick={onclose}
      aria-label={$t('battlesTakenClose')}
      title={$t('battlesTakenClose')}><span aria-hidden="true">×</span></button
    >

    <div class="spread">
      <!-- Карта стоит в витрине, а не лежит на тёмном поле: та же кайма, что у
           плашек справа, и цена — плита той же витрины, а не подвал листа. -->
      <div class="plinth" class:plinth--bare={!taking}>
        <div class="face">
          <BattleCard {card} {frames} owned={true} transition={false} interactive={false} />
        </div>

        {#if taking}
          <div class="purse">
            {@render flourish()}
            {#if owned}
              <p class="purse-word">{$t('battlesYours')}</p>
            {:else if signedIn}
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
            {:else}
              {#each prices as price (price.coin)}
                <p class="price">
                  <span class="amount">{price.amount}</span>
                  <span class="coin">
                    {price.coin === 'dust' ? $t('battlesCoinDust') : $t('battlesCoinFeed')}
                  </span>
                </p>
              {/each}
              <a class="take take--link" href={loginHref}>{$t('battlesSignInToTake')}</a>
            {/if}

            {#if complaint}
              <p class="fault">{complaint}</p>
            {/if}
          </div>
        {/if}
      </div>

      <div class="prose">
        <header class="mast">
          {#if card.raceIconUrl}
            <img class="race" src={card.raceIconUrl} alt="" width="36" height="36" />
          {/if}
          <div class="mast-copy">
            <h2 id="battle-sheet-title" class="name">{face.title || $t('battlesKindUnit')}</h2>
            {#if kicker.length}
              <p class="kind">
                {#each kicker as word, i (`${word}-${i}`)}
                  {#if i}<span class="sep" aria-hidden="true"></span>{/if}<span>{word}</span>
                {/each}
              </p>
            {/if}
          </div>
        </header>

        {#if face.effect}
          <!-- Голос карты. Ему отведено СВОЁ место между росчерками: строкой в
               общем потоке он читался ещё одной подписью, а это единственное на
               листе, что говорит не числом. -->
          <div class="voice">
            {@render flourish()}
            <p class="lead">{face.effect}</p>
            {@render flourish()}
          </div>
        {:else}
          {@render flourish()}
        {/if}

        {#if blow.length}
          <section class="block">
            <h3 class="block-title">{$t('battlesSheetBlow')}</h3>
            <ul class="plates plates--blow">
              {#each blow as fig (fig.slot)}{@render plate(fig)}{/each}
            </ul>
          </section>
        {/if}

        <section class="block">
          <h3 class="block-title">{$t('battlesSheetParams')}</h3>
          <ul class="plates plates--body">
            {#each body as fig (fig.slot)}{@render plate(fig)}{/each}
          </ul>
        </section>

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

        <footer class="foot">
          {#if face.lore}
            <p class="note">{face.lore}</p>
          {/if}
          {#if href}
            <a class="work" {href}>{workName || $t('battlesWorkLink')}</a>
          {/if}
        </footer>
      </div>
    </div>
  </div>
</div>

<style>
  .veil {
    position: fixed;
    inset: 0;
    z-index: 90;
    display: grid;
    place-items: center;
    padding: 1.25rem 1rem;
    overflow-y: auto;
    background: radial-gradient(ellipse at 50% 45%, rgba(20, 13, 8, 0.78), rgba(6, 4, 3, 0.92));
    backdrop-filter: blur(3px);
    cursor: default;
  }

  .leaf {
    position: relative;
    width: fit-content;
    max-width: min(66rem, calc(100vw - 2rem));
    margin: auto;
    padding: 2.15rem 1.85rem;
    color: #e6dcc8;
    background:
      radial-gradient(ellipse at 50% 6%, rgba(150, 96, 40, 0.16), transparent 60%),
      linear-gradient(180deg, rgba(10, 7, 5, 0.88), rgba(8, 5, 4, 0.95)),
      #0b0806 url('/battles/chamber/chamber-hall.jpg?v=2') center / cover no-repeat;
    box-shadow: 0 26px 70px rgba(0, 0, 0, 0.72);
  }

  /* Оправа — та же, что у комнаты этюда (`chamber-frame`), надетая девятью
     кусками: лист карты и стол боя — одна комната, и второй рисованной рамы,
     которую пришлось бы держать в согласии с первой, у дома нет. Полоса задана
     двумя числами в тех же долях, что и разрез (150/130 из 1024), иначе резьба
     поехала бы по одной оси.
     Живёт она СЛОЕМ, а не каймой самого листа, ровно затем, чтобы её можно было
     притушить: резьба в полную силу спорила с картой — глаз видел сперва раму,
     потом разворот, и только потом то, ради чего его открыли. Потушена
     фильтром, а не второй картинкой: картинок у дома одна. */
  .frame {
    position: absolute;
    inset: 0;
    z-index: 2;
    pointer-events: none;
    border: 2.15rem solid transparent;
    border-width: 2.15rem 1.85rem;
    border-image: url('/battles/chamber/chamber-frame.png?v=2') 150 130 / 2.15rem 1.85rem / 0 stretch;
    filter: brightness(0.66) saturate(0.6) contrast(0.96);
    /* Волосяная кайма по внутреннему краю оправы: она и есть то, что делает
       обе половины разворота одной вещью в одной витрине. */
    box-shadow: inset 0 0 0 1px rgba(196, 160, 96, 0.32);
  }

  /* Верх тише остального: там имя, и золото над ним не должно спорить с ним за
     первый взгляд. */
  .leaf::after {
    content: '';
    position: absolute;
    inset: 0 0 auto 0;
    z-index: 3;
    height: 2.6rem;
    pointer-events: none;
    background: linear-gradient(180deg, rgba(6, 4, 3, 0.5), rgba(6, 4, 3, 0));
  }

  .leaf:focus {
    outline: none;
  }

  /* Крестик — не кнопка поверх резьбы, а гнездо той же ковки (`chamber-socket`),
     с тремя внятными состояниями: лежит, отзывается, вдавлен. */
  .dismiss {
    position: absolute;
    top: 1.35rem;
    right: 1.05rem;
    z-index: 5;
    display: grid;
    place-items: center;
    width: 2.6rem;
    height: 2.6rem;
    padding: 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 1.5rem;
    line-height: 1;
    /* Чернила тёмные: середина гнезда — светлое зеркало, и золотой крестик на
       нём не читался вовсе. */
    color: #241708;
    background: url('/battles/chamber/chamber-socket.png?v=2') center / contain no-repeat;
    border: none;
    text-shadow: 0 1px 0 #1a1208;
    cursor: pointer;
    transition:
      filter 0.15s ease,
      transform 0.1s ease;
  }

  .dismiss:hover,
  .dismiss:focus-visible {
    color: #120b04;
    filter: brightness(1.3) drop-shadow(0 0 7px rgba(230, 180, 90, 0.45));
    outline: none;
  }

  .dismiss:active {
    transform: translateY(1px);
    filter: brightness(0.85);
  }

  .spread {
    display: grid;
    grid-template-columns: 17.5rem minmax(0, 28rem);
    gap: 1.1rem 1.5rem;
    align-items: stretch;
  }

  .plinth {
    display: flex;
    flex-direction: column;
    width: 17.5rem;
    padding: 0.5rem;
    background: linear-gradient(180deg, rgba(52, 36, 22, 0.55), rgba(14, 10, 7, 0.72));
    border: 1px solid rgba(196, 160, 96, 0.42);
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.5);
  }

  /* Без плиты цены (сцена боя открывает лист, чтобы прочесть, а не взять)
     витрина жмётся к карте: пустой ящик под ней держался ровно на плите. */
  .plinth--bare {
    align-self: start;
  }

  .face {
    filter: drop-shadow(0 8px 20px rgba(0, 0, 0, 0.55));
  }

  .purse {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    justify-content: center;
    gap: 0.35rem 0.9rem;
    /* `auto` — к подножию витрины: обе половины разворота одной высоты (справа
       подпись прижата к низу так же), и плита, стоящая сразу под картой,
       оставляла под собой пустой ящик в треть карты высотой. */
    margin-top: auto;
    padding-top: 0.55rem;
    text-align: center;
  }

  /* Паспорт тянется на всю высоту карты, а подпись прижата к низу: у короткой
     карты (ни черт, ни умений) правая половина иначе обрывалась посередине, и
     под ней стояло поле пустоты в треть разворота. */
  .prose {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .mast {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    margin: 0 2.6rem 0.7rem 0;
  }

  .mast-copy {
    min-width: 0;
  }

  .race {
    flex: 0 0 auto;
    width: 2.4rem;
    height: 2.4rem;
    object-fit: cover;
    border: 1px solid rgba(196, 160, 96, 0.5);
    border-radius: 20%;
  }

  .name {
    margin: 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: clamp(1.7rem, 3vw, 2.3rem);
    font-weight: 400;
    line-height: 1.08;
    color: #f6e9c8;
    text-shadow:
      0 2px 0 #150e07,
      0 0 26px rgba(230, 170, 40, 0.28);
  }

  .kind {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.35rem 0.55rem;
    margin: 0.34rem 0 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.64rem;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #c9ad78;
  }

  /* Разделитель — тот же ромб, что в росчерке и на плите: точка была не мелка,
     а безымянна, и строка рассыпалась на слова без общего мотива. */
  .sep {
    width: 0.22rem;
    height: 0.22rem;
    background: rgba(201, 169, 97, 0.75);
    transform: rotate(45deg);
  }

  .flourish {
    display: flex;
    align-items: center;
    gap: 0.55rem;
  }

  .flourish::before,
  .flourish::after {
    content: '';
    flex: 1 1 auto;
    height: 1px;
    background: linear-gradient(90deg, rgba(196, 160, 96, 0.04), rgba(224, 190, 118, 0.62));
  }

  .flourish::after {
    background: linear-gradient(90deg, rgba(224, 190, 118, 0.62), rgba(196, 160, 96, 0.04));
  }

  .pip {
    flex: 0 0 auto;
    width: 0.34rem;
    height: 0.34rem;
    background: #d3ae66;
    box-shadow: 0 0 7px rgba(230, 180, 90, 0.4);
    transform: rotate(45deg);
  }

  .voice {
    margin: 0 0 1.1rem;
  }

  .lead {
    margin: 0.85rem 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 1.06rem;
    line-height: 1.55;
    font-style: italic;
    color: #e6d3a8;
  }

  .block {
    margin: 0 0 1rem;
  }

  .block-title {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    margin: 0 0 0.5rem;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.7rem;
    font-weight: 500;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: #e8d5a0;
    text-shadow: 0 1px 0 #150e07;
  }

  .block-title::before {
    content: '';
    flex: 0 0 auto;
    width: 0.26rem;
    height: 0.26rem;
    background: rgba(201, 169, 97, 0.8);
    transform: rotate(45deg);
  }

  .plates {
    display: grid;
    gap: 0.4rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .plates--blow {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }

  /* Главные числа занимают по две доли, второстепенные — по одной: игрок должен
     видеть здоровье и ману раньше оберега, у которого у большинства карт ноль. */
  .plates--body {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }

  .plates--body .plate--major {
    grid-column: span 2;
  }

  /* Плашка числа: тёмный камень в золотой кайме. Кайма двойная — внешняя
     золотая и внутренняя чёрная, — потому что одиночная золотая нить на тёмном
     поле смотрится нарисованной поверх, а не врезанной. */
  .plate {
    display: grid;
    grid-template-columns: auto auto;
    justify-content: center;
    align-items: center;
    column-gap: 0.4rem;
    row-gap: 0.1rem;
    padding: 0.5rem 0.3rem 0.45rem;
    text-align: center;
    background: linear-gradient(180deg, rgba(52, 36, 22, 0.6), rgba(14, 10, 7, 0.78));
    border: 1px solid rgba(196, 160, 96, 0.34);
    box-shadow:
      inset 0 0 0 1px rgba(0, 0, 0, 0.5),
      inset 0 10px 20px rgba(0, 0, 0, 0.35);
  }

  .plate--major {
    padding: 0.6rem 0.3rem 0.5rem;
    background: linear-gradient(180deg, rgba(74, 52, 30, 0.66), rgba(18, 12, 8, 0.8));
    border-color: rgba(214, 180, 110, 0.6);
  }

  /* Знак — не украшение подписи, а второе имя числа, и меряется он ЧИСЛОМ, а не
     словом под ним: у слова кегль вдвое меньше, и знак при нём терялся. */
  .mark {
    display: inline-flex;
    font-size: 0.95rem;
    color: #c9a961;
  }

  .plate--major .mark {
    font-size: 1.2rem;
    color: #dcb86e;
  }

  .mark :global(svg) {
    display: block;
  }

  .plate b {
    font-family: Georgia, 'Fraunces', serif;
    font-size: 1.2rem;
    font-weight: 400;
    font-variant-numeric: tabular-nums;
    line-height: 1;
    color: #f2e0b6;
    text-shadow: 0 1px 0 #150e07;
  }

  .plate--major b {
    font-size: 1.65rem;
    color: #fbeec6;
  }

  .word {
    grid-column: 1 / -1;
    font-family: 'Inter', system-ui, sans-serif;
    font-size: 0.53rem;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #b39d72;
  }

  .plate--major .word {
    font-size: 0.58rem;
    color: #c9b285;
  }

  .gifts {
    margin: 0;
    padding: 0.5rem 0.7rem;
    list-style: none;
    background: linear-gradient(180deg, rgba(40, 28, 18, 0.5), rgba(12, 8, 6, 0.66));
    border: 1px solid rgba(196, 160, 96, 0.3);
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.45);
  }

  .gift {
    display: flex;
    flex-direction: column;
    gap: 0.12rem;
    padding: 0.4rem 0;
    border-bottom: 1px solid rgba(196, 160, 96, 0.18);
  }

  .gift:first-child {
    padding-top: 0;
  }

  .gift:last-child {
    padding-bottom: 0;
    border-bottom: none;
  }

  .gift-name {
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.98rem;
    color: #f0e0bc;
  }

  .gift-other {
    margin-left: 0.4em;
    font-size: 0.78em;
    color: #b79c6e;
  }

  .gift-text {
    font-family: 'Inter', system-ui, sans-serif;
    font-size: 0.8rem;
    line-height: 1.45;
    color: #cdbc9c;
  }

  .gift--voice .gift-text {
    font-family: Georgia, 'Fraunces', serif;
    font-style: italic;
  }

  .purse-word {
    margin: 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.68rem;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: #c9ad78;
  }

  .price {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    justify-content: center;
    gap: 0.35rem 0.55rem;
    margin: 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.64rem;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #c9ad78;
  }

  .amount {
    font-family: Georgia, 'Fraunces', serif;
    font-size: 1.1rem;
    letter-spacing: 0;
    color: #f6e7bd;
    text-shadow: 0 1px 0 #150e07;
  }

  .take {
    padding: 0.2rem 0.7rem 0.18rem;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.66rem;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #e8d5a0;
    background: linear-gradient(180deg, rgba(70, 48, 26, 0.75), rgba(24, 16, 10, 0.85));
    border: 1px solid rgba(196, 160, 96, 0.5);
    text-shadow: 0 1px 0 #1a1208;
    cursor: pointer;
  }

  .take:hover:not(:disabled) {
    color: #fff1c4;
    border-color: rgba(232, 200, 120, 0.75);
  }

  .take:active:not(:disabled) {
    transform: translateY(1px);
  }

  .take:disabled {
    color: #8f7c5a;
    border-color: rgba(196, 160, 96, 0.22);
    cursor: default;
  }

  .take--link {
    text-decoration: none;
  }

  .fault {
    flex-basis: 100%;
    margin: 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.82rem;
    color: #e08060;
  }

  .foot {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.5rem 1.2rem;
    /* `auto` — прижим к низу; отступ сверху приходит от нижнего поля
       последнего блока, поэтому его здесь нет и быть не может. */
    margin: auto 0 0;
    padding: 0.75rem 0 0;
    border-top: 1px solid rgba(196, 160, 96, 0.28);
  }

  .note {
    flex: 1 1 14rem;
    margin: 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.9rem;
    font-style: italic;
    line-height: 1.5;
    color: #bfa87e;
  }

  .work {
    margin-left: auto;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.82rem;
    color: #a08b64;
    text-decoration: none;
    border-bottom: 1px solid transparent;
  }

  .work:hover {
    color: #f0e0bc;
    border-bottom-color: rgba(196, 160, 96, 0.5);
  }

  @media (max-width: 56rem) {
    .leaf {
      /* Ширина считается от ПОЛЯ завесы, а не от окна: у завесы свой отступ в
         1rem с каждой стороны, и лист, померенный по `100vw`, вылезал из неё
         правым краем — текст уходил под резьбу. */
      width: 100%;
      padding: 1.7rem 1.45rem;
    }

    .frame {
      border-width: 1.7rem 1.45rem;
      border-image-width: 1.7rem 1.45rem;
    }

    .spread {
      grid-template-columns: 1fr;
      gap: 1rem;
    }

    .plinth {
      width: min(17.5rem, 100%);
      margin: 0 auto;
    }

    .plates--blow {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .work {
      margin-left: 0;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .veil {
      backdrop-filter: none;
    }

    .dismiss,
    .take {
      transition: none;
    }
  }
</style>
