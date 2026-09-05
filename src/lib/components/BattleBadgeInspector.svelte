<script lang="ts">
  /**
   * Стол одного значка — стоимости или силы.
   *
   * Раньше это было окошко с формой и двумя числами, и каждая новая ручка
   * втискивалась в ту же строку: цвет, потом второй цвет, потом плотность,
   * величина, толщина. Инспектор собирает их так, как их держат в руках:
   * сперва ФОРМА (есть ли подложка вообще), потом её ЦВЕТ и сколько его,
   * потом ЦИФРА — цвет, величина, толщина, — и только в конце МЕСТО, которое
   * обычно ставят мышью и правят числами лишь для точности.
   *
   * У каждой ручки есть обратный ход к домашнему, и он показан словом, а не
   * крестиком: у поля выбора цвета нет состояния «не выбрано», а у ползунка
   * нет «не трогали», и без слова хранитель не отличил бы назначенное им
   * самим от совпавшего с домашним.
   *
   * Читает инспектор РАЗРЕШЁННУЮ рамку (`frame` — уже с надетым нарядом),
   * пишет — в ту, которую отдал вызывающий (`write()`): это может быть чин,
   * а может быть наряд расы, и знать об этом инспектору нечего.
   */
  import { t } from '$lib/i18n';
  import type { TranslationKey } from '$lib/i18n';
  import {
    BADGE_SHAPES,
    BADGE_FIELDS,
    badgeAt,
    BADGE_SCALE_MIN,
    BADGE_SCALE_MAX,
    BADGE_WEIGHTS,
    BADGE_FILL_NONE,
    badgeInk,
    BADGE_HOME,
    badgeExtent,
    badgePlate,
    badgeScale,
    badgeShape,
    badgeText,
    badgeSpot,
    badgeUnfilled,
    badgeWeight,
    fillJoin,
    fillParts,
    DEFAULT_ASPECT,
    statMark,
    statLabel,
  } from '$lib/battles';
  import BattleIcon from '$lib/components/BattleIcon.svelte';
  import type { BattleBadgeShape, BattleFrame } from '$lib/types/api';
  import type { BadgeKind, FrameOverride } from '$lib/battles';

  let {
    kind,
    frame,
    write,
    onEditStart,
    onArtUpload,
    onArtStore,
    onclose,
  }: {
    kind: BadgeKind;
    /** Рамка, какой её видит карта, — для чтения. */
    frame: BattleFrame;
    /** Куда писать. Функция, а не объект: цель зависит от того, что открыто на
     *  столе, и вычислять её надо в мгновение записи. */
    write: () => BattleFrame | FrameOverride;
    onEditStart?: () => void;
    /** Две двери за картинкой жетона — с диска и со склада. Обе приходят
     *  сверху и обе НЕОБЯЗАТЕЛЬНЫ: инспектор открывается и там, где склада
     *  нет вовсе, и тогда он просто не показывает того, чего не может. */
    onArtUpload?: (apply: (url: string) => void) => void;
    onArtStore?: (apply: (url: string) => void) => void;
    onclose: () => void;
  } = $props();

  let keys = $derived(BADGE_FIELDS[kind]);

  // Домашнее — то, что значок носит, когда ему ничего не назначили. У силы
  // оно своё (кайма карты), у стоимости и у здоровья одно на двоих: здоровье
  // заменяет монету в клетке боя и носит её краски, пока хранитель не развёл
  // их сам. Это единственное место, где разницу приходится назвать: сама
  // карта держит её в откатах `var(--badge-fill, …)`, но пипетке нужно, от
  // чего оттолкнуться.
  let houseFill = $derived(kind === 'power' ? frame.border : frame.ink);

  /** Чей наряд этот значок донашивает, пока ему не назначили своего. У
   *  здоровья это стоимость; у стоимости и силы — никто. Нужно затем, что
   *  ПУСТОЕ у них значит разное: у здоровья «как у стоимости», у остальных
   *  «как в раме», и обратный ход обязан называться своим именем. */
  let under = $derived(BADGE_HOME[kind]);
  /** Назначено ли ЗДЕСЬ, а не донашивается. Читается сырое поле, а не
   *  разрешённое: разрешённое у здоровья всегда что-нибудь да вернёт. */
  const own = (key: 'shape' | 'fill' | 'ink' | 'plate') =>
    ((frame[BADGE_FIELDS[kind][key]] as string) ?? '').trim();
  let ownSize = $derived((frame[keys.size] as number) || 0);
  let ownWeight = $derived((frame[keys.weight] as number) || 0);
  let ownSpot = $derived(
    (frame[keys.x] as number | null) != null || (frame[keys.y] as number | null) != null,
  );

  let shape = $derived(badgeShape(frame, kind));
  let chosenFill = $derived(badgeText(frame, kind, 'fill'));
  let chosenInk = $derived(badgeText(frame, kind, 'ink'));
  let parts = $derived(fillParts(chosenFill, houseFill));
  let unfilled = $derived(!!chosenFill && badgeUnfilled(chosenFill));
  let autoInk = $derived(
    unfilled || !chosenFill ? frame.ink : badgeInk(chosenFill, frame),
  );
  let plate = $derived(badgePlate(frame, kind));
  let size = $derived(badgeScale(frame, kind));
  /** Сколько значок занимает — по нарисованному, а не по кружку: без формы
   *  это цифра, и место ей отмеряется по ней. */
  let extent = $derived(badgeExtent(frame, kind));
  let weight = $derived(badgeWeight(frame, kind));
  let spot = $derived(badgeAt(frame, kind));

  function set<K extends keyof BattleFrame>(key: K, value: BattleFrame[K]) {
    (write() as BattleFrame)[key] = value;
  }

  /** Плотность правится, даже когда заливка «как в раме»: тронув ползунок,
   *  хранитель назвал цвет — домашний, — и дальше он уже его собственный.
   *  Иначе первый же сдвиг ползунка не делал бы ничего. */
  function setAlpha(alpha: number) {
    set(keys.fill, fillJoin(parts.hex, alpha));
  }

  const SHAPE_TITLES: Record<BattleBadgeShape, TranslationKey> = {
    circle: 'adminBattlesBadgeShapeCircle',
    square: 'adminBattlesBadgeShapeSquare',
    diamond: 'adminBattlesBadgeShapeDiamond',
    hex: 'adminBattlesBadgeShapeHex',
    shield: 'adminBattlesBadgeShapeShield',
    none: 'adminBattlesBadgeShapeNone',
  };
</script>

<!-- Снимок для отмены — ОДИН на нажатие, перехватом, как у боковой колонки
     стола: иначе пипетка или ползунок, которые ведут мышью, положили бы в
     стопку по снимку на каждый оттенок и каждый пиксель хода. -->
<div class="bi" onpointerdowncapture={() => onEditStart?.()}>
  <header class="bi-head">
    <!-- Знак и слово берутся по одному имени: стол значка называет ровно то
         же, что вычеканено на самом кружке, которым он правит. -->
    <span class="bi-title">
      <BattleIcon name={statMark(kind)} size={12} weight={1.35} />
      {$t(statLabel(kind))}
    </span>
    <button type="button" class="bi-close" onclick={onclose} aria-label={$t('adminBattlesFrameClose')}
      >×</button
    >
  </header>

  {#if onArtUpload || onArtStore}
    <!-- Первой, потому что жетон СИЛЬНЕЕ формы и заливки: хранитель, который
         выбирает форму под уже надетым жетоном, иначе не понял бы, отчего на
         карте ничего не меняется. По той же причине обе секции ниже при
         надетом жетоне гаснут — тем же приёмом, каким гаснет заливка у
         значка без формы. -->
    <section class="bi-part">
      <span class="bi-label">{$t('adminBattlesBadgePlate')}</span>
      <div class="bi-row">
        {#if plate}
          <img class="bi-plate" src={plate} alt="" />
        {/if}
        {#if onArtUpload}
          <button type="button" class="bi-door" onclick={() => onArtUpload?.((url) => set(keys.plate, url))}
            >{$t('adminBattlesFrameArtUpload')}</button
          >
        {/if}
        {#if onArtStore}
          <button type="button" class="bi-door" onclick={() => onArtStore?.((url) => set(keys.plate, url))}
            >{$t('adminAssetsPick')}</button
          >
        {/if}
        {#if plate}
          <button type="button" class="bi-reset" onclick={() => set(keys.plate, '')}
            >{$t(under && own('plate') ? 'adminBattlesBadgeAsCost' : 'adminBattlesBadgePlateNone')}</button
          >
        {/if}
      </div>
      <details class="bi-hint-fold">
        <summary>{$t('adminBattlesHintOpen')}</summary>
        <p class="bi-hint">{$t('adminBattlesBadgePlateHint')}</p>
      </details>
    </section>
  {/if}

  <section class="bi-part" class:bi-part--off={!!plate}>
    <span class="bi-label">{$t('adminBattlesBadgeShape')}</span>
    <div class="bi-shapes" role="radiogroup" aria-label={$t('adminBattlesBadgeShape')}>
      {#each BADGE_SHAPES as s (s)}
        <button
          type="button"
          class="bi-shape bi-shape--{s}"
          class:active={shape === s}
          title={$t(SHAPE_TITLES[s])}
          role="radio"
          aria-checked={shape === s}
          onclick={() => set(keys.shape, s)}
        >{#if s === 'none'}<span class="bi-shape-num">7</span>{/if}</button>
      {/each}
      {#if under && own('shape')}
        <button type="button" class="bi-reset" onclick={() => set(keys.shape, '' as BattleBadgeShape)}
          >{$t('adminBattlesBadgeAsCost')}</button
        >
      {/if}
    </div>
  </section>

  <section class="bi-part" class:bi-part--off={shape === 'none' || !!plate}>
    <span class="bi-label">{$t('adminBattlesBadgeFill')}</span>
    <div class="bi-row">
      <input
        type="color"
        aria-label={$t('adminBattlesBadgeFill')}
        value={parts.hex}
        oninput={(e) => set(keys.fill, fillJoin(e.currentTarget.value, parts.alpha))}
      />
      <input
        type="text"
        class="bi-hex"
        spellcheck="false"
        aria-label={$t('adminBattlesBadgeFill')}
        value={chosenFill || houseFill}
        onchange={(e) => set(keys.fill, e.currentTarget.value.trim())}
      />
      {#if under ? own('fill') : chosenFill}
        <button type="button" class="bi-reset" onclick={() => set(keys.fill, '')}
          >{$t(under ? 'adminBattlesBadgeAsCost' : 'adminBattlesBadgeFillHouse')}</button
        >
      {/if}
    </div>
    <div class="bi-row">
      <input
        type="range"
        min="0"
        max="100"
        step="1"
        aria-label={$t('adminBattlesBadgeFillAlpha')}
        value={parts.alpha}
        oninput={(e) => setAlpha(Number(e.currentTarget.value))}
      />
      <span class="bi-num">{parts.alpha}%</span>
      {#if parts.alpha > 0}
        <button type="button" class="bi-reset" onclick={() => set(keys.fill, BADGE_FILL_NONE)}
          >{$t('adminBattlesBadgeFillNone')}</button
        >
      {/if}
    </div>
  </section>

  <section class="bi-part">
    <span class="bi-label">{$t('adminBattlesBadgeInk')}</span>
    <div class="bi-row">
      <input
        type="color"
        aria-label={$t('adminBattlesBadgeInk')}
        value={chosenInk || autoInk}
        oninput={(e) => set(keys.ink, e.currentTarget.value)}
      />
      <input
        type="text"
        class="bi-hex"
        spellcheck="false"
        aria-label={$t('adminBattlesBadgeInk')}
        value={chosenInk || autoInk}
        onchange={(e) => set(keys.ink, e.currentTarget.value.trim())}
      />
      {#if under ? own('ink') : chosenInk}
        <button type="button" class="bi-reset" onclick={() => set(keys.ink, '')}
          >{$t(under ? 'adminBattlesBadgeAsCost' : 'adminBattlesBadgeInkAuto')}</button
        >
      {/if}
    </div>

    <div class="bi-row">
      <span class="bi-sub">{$t('adminBattlesBadgeWeight')}</span>
      <div class="bi-weights" role="radiogroup" aria-label={$t('adminBattlesBadgeWeight')}>
        {#each BADGE_WEIGHTS as w (w)}
          <button
            type="button"
            class="bi-weight"
            class:active={weight === w}
            style="font-weight:{w}"
            role="radio"
            aria-checked={weight === w}
            onclick={() => set(keys.weight, weight === w ? 0 : w)}
          >7</button>
        {/each}
      </div>
    </div>
  </section>

  <!-- Своя секция, а не строка под «Цифрой»: величина тянет ВЕСЬ значок —
       подложку вместе с цифрой, — и стоять ей под заголовком про цифру
       значило бы обещать, что подложка останется прежней. -->
  <section class="bi-part">
    <span class="bi-label">{$t('adminBattlesBadgeSize')}</span>
    <div class="bi-row">
      <input
        type="range"
        min={BADGE_SCALE_MIN}
        max={BADGE_SCALE_MAX}
        step="0.05"
        aria-label={$t('adminBattlesBadgeSize')}
        value={size}
        oninput={(e) => set(keys.size, Number(e.currentTarget.value))}
      />
      <span class="bi-num">{size.toFixed(2)}×</span>
      {#if under ? ownSize > 0 : size !== 1}
        <button type="button" class="bi-reset" onclick={() => set(keys.size, under ? 0 : 1)}
          >{under ? $t('adminBattlesBadgeAsCost') : '1×'}</button
        >
      {/if}
    </div>
  </section>

  <section class="bi-part">
    <span class="bi-label">{$t('adminBattlesBadgePlace')}</span>
    <div class="bi-row">
      <label class="bi-field">
        X <input
          type="number" min="0" max="100" step="1"
          value={Math.round(spot.x)}
          oninput={(e) => set(keys.x, badgeSpot(Number(e.currentTarget.value), spot.y, frame.aspect || DEFAULT_ASPECT, extent).x)}
        />
      </label>
      <label class="bi-field">
        Y <input
          type="number" min="0" max="100" step="1"
          value={Math.round(spot.y)}
          oninput={(e) => set(keys.y, badgeSpot(spot.x, Number(e.currentTarget.value), frame.aspect || DEFAULT_ASPECT, extent).y)}
        />
      </label>
      <!-- `null`, а не ноль: ноль — верхний левый угол карты, настоящее место,
           и вернуть им «не назначено» значило бы отправить значок в угол под
           видом возврата. -->
      {#if under && ownSpot}
        <button
          type="button"
          class="bi-reset"
          onclick={() => {
            set(keys.x, null as unknown as number);
            set(keys.y, null as unknown as number);
          }}>{$t('adminBattlesBadgeAsCost')}</button
        >
      {/if}
    </div>
  </section>
</div>

<style>
  /* Ширина названа в пикселях, а не в cqi: инспектор — это стол, а не часть
     карты, и на карте, увеличенной вчетверо, он не должен расти вчетверо
     вместе с ней. */
  .bi {
    display: flex;
    flex-direction: column;
    gap: 0.5em;
    width: 15rem;
    /* Выше экрана панель не бывает: у самой мелкой карты она всё равно своей
       величины, и на невысоком окне низ её иначе просто недостижим. */
    max-height: 80vh;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: 0.55em 0.65em 0.7em;
    font-family: Inter, system-ui, sans-serif;
    font-size: 0.7rem;
    color: var(--ink, #34251c);
    background: var(--paper, #f8f1e7);
    border: 1px solid color-mix(in oklab, var(--ink) 30%, transparent);
    box-shadow: 0 6px 22px rgba(0, 0, 0, 0.22);
    white-space: nowrap;
  }

  .bi-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-bottom: 0.35em;
    border-bottom: 1px solid color-mix(in oklab, var(--ink) 18%, transparent);
  }

  .bi-title {
    display: inline-flex;
    align-items: center;
    gap: 0.4em;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.07em;
  }

  .bi-close {
    padding: 0 0.2em;
    font: inherit;
    font-size: 1.1em;
    line-height: 1;
    color: inherit;
    background: none;
    border: none;
    cursor: pointer;
  }

  .bi-part {
    display: flex;
    flex-direction: column;
    gap: 0.32em;
  }

  /* Снятая форма не прячет цвет — сняли, вернули, и он на месте, — но и не
     притворяется, что он сейчас что-то красит. */
  .bi-part--off {
    opacity: 0.45;
  }

  .bi-label {
    font-size: 0.9em;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    opacity: 0.65;
  }

  .bi-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4em 0.35em;
  }

  .bi-sub {
    width: 4.2em;
    opacity: 0.8;
  }

  .bi-row input[type='color'] {
    flex: none;
    width: 1.7em;
    height: 1.5em;
    padding: 0;
    background: none;
    border: 1px solid color-mix(in oklab, var(--ink) 25%, transparent);
    cursor: pointer;
  }

  .bi-row input[type='range'] {
    flex: 1;
    min-width: 0;
    accent-color: var(--ink);
  }

  .bi-hex,
  .bi-field input {
    min-width: 0;
    padding: 0.2em 0.35em;
    font: inherit;
    color: var(--ink);
    background: color-mix(in oklab, var(--paper) 90%, var(--ink) 10%);
    border: 1px solid color-mix(in oklab, var(--ink) 22%, transparent);
  }

  /* Полю цвета нужен свой минимум: без него соседняя кнопка отката съедает
     его до «#99», и запись, ради которой поле и стоит, не читается. */
  .bi-hex {
    flex: 1;
    min-width: 5.5em;
  }

  .bi-field input {
    width: 3.4em;
  }

  .bi-field {
    display: flex;
    align-items: center;
    gap: 0.3em;
  }

  .bi-num {
    min-width: 3em;
    text-align: right;
    font-variant-numeric: tabular-nums;
    opacity: 0.75;
  }

  /* Дверь на склад и дверь к диску — те же кнопки, что и «вернуть домашнее»,
     и это не лень: обе не назначают значение, а ведут за ним. Отличать их
     обводкой значило бы обещать разницу, которой нет. */
  .bi-door,
  .bi-reset {
    padding: 0.15em 0.4em;
    font: inherit;
    font-size: 0.92em;
    color: var(--ink);
    background: none;
    border: 1px solid color-mix(in oklab, var(--ink) 22%, transparent);
    cursor: pointer;
  }

  /* Надетый жетон показан САМ, а не именем файла: имена у них со склада
     машинные, и по ним нельзя узнать бляху, которую выбирали глазами. */
  .bi-plate {
    width: 1.9em;
    height: 1.9em;
    object-fit: contain;
    /* Клетка под прозрачным: жетон вырезан по альфе, и на сплошном фоне
       светлая бляха выглядела бы обрезанной по своей коробке. */
    background:
      repeating-conic-gradient(
        color-mix(in oklab, var(--ink) 12%, transparent) 0% 25%,
        transparent 0% 50%
      )
      0 0 / 0.5em 0.5em;
  }

  .bi-hint-fold {
    margin: 0.35em 0 0;
  }
  .bi-hint-fold summary {
    font-size: 0.78em;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    opacity: 0.6;
    cursor: pointer;
  }
  .bi-hint-fold .bi-hint {
    margin-top: 0.35em;
  }

  .bi-hint {
    margin: 0;
    font-size: 0.88em;
    line-height: 1.35;
    opacity: 0.6;
  }

  .bi-shapes,
  .bi-weights {
    display: flex;
    gap: 0.3em;
  }

  .bi-shape {
    display: grid;
    place-items: center;
    width: 1.5em;
    height: 1.5em;
    padding: 0;
    color: var(--paper);
    background: color-mix(in oklab, var(--ink) 55%, transparent);
    border: 1px solid transparent;
    cursor: pointer;
  }

  .bi-shape.active {
    background: var(--ink);
    outline: 1px solid var(--ink);
    outline-offset: 1px;
  }

  .bi-shape--circle {
    border-radius: 50%;
  }

  .bi-shape--square {
    border-radius: 12%;
  }

  .bi-shape--diamond {
    clip-path: polygon(50% 0, 100% 50%, 50% 100%, 0 50%);
  }

  .bi-shape--hex {
    clip-path: polygon(25% 0%, 75% 0%, 100% 50%, 75% 100%, 25% 100%, 0% 50%);
  }

  .bi-shape--shield {
    clip-path: polygon(50% 0%, 100% 18%, 100% 55%, 50% 100%, 0% 55%, 0% 18%);
  }

  /* «Без формы» показано тем, что оно и есть: одна цифра без подложки. */
  .bi-shape--none {
    color: var(--ink);
    background: none;
    border: 1px dashed color-mix(in oklab, var(--ink) 40%, transparent);
  }

  .bi-shape--none.active {
    background: none;
    border-style: solid;
  }

  .bi-shape-num {
    font-size: 0.85em;
    line-height: 1;
  }

  .bi-weight {
    width: 1.7em;
    height: 1.6em;
    padding: 0;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.95em;
    line-height: 1;
    color: var(--ink);
    background: none;
    border: 1px solid color-mix(in oklab, var(--ink) 18%, transparent);
    cursor: pointer;
  }

  .bi-weight.active {
    color: var(--paper);
    background: var(--ink);
  }
</style>
