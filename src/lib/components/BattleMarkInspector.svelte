<script lang="ts">
  /**
   * Стол одной метки готовой иллюстрации — слова или числа карты.
   *
   * Сначала то, что метка говорит (текст на языке листа, число), потом как
   * она это говорит (шрифт, чернила, величина, ширина строки, выравнивание),
   * и в конце место — его обычно ставят мышью, а числа тут для точности.
   *
   * Устроен как стол значка (`BattleBadgeInspector`): читает разрешённую
   * раму (`frame`), пишет в ту, которую назвал вызывающий (`write()`), — ранг
   * на столе рамок, свой наряд карты на листе карты. Текст и числа пишутся в
   * саму карту, то есть в те же поля, что и форма листа рядом.
   */
  import { t } from '$lib/i18n';
  import { SITE_FONTS } from '$lib/fonts';
  import BattleGlyphControl from '$lib/components/BattleGlyphControl.svelte';
  import { selectOnFocus, blurOnWheel } from '$lib/utils/fields';
  import {
    freeMarkLabel,
    freeMarkOf,
    setFreeMark,
    isFreeStat,
    mostlyCyrillic,
    FREE_MARK_SIZE_MIN,
    FREE_MARK_SIZE_MAX,
    FREE_MARK_WIDTH_MIN,
    FREE_MARK_WIDTH_MAX,
    type FreePrintedSlot,
    type FrameOverride,
    type GlyphSlot,
  } from '$lib/battles';
  import type { BattleCard, BattleFrame, FreeMark, FreeMarkAlign } from '$lib/types/api';

  let {
    slot,
    card = $bindable(),
    frame,
    lang,
    textEditable = false,
    write,
    onEditStart,
    onclose,
  }: {
    slot: FreePrintedSlot;
    card: BattleCard;
    /** Рама, какой её видит карта, — для чтения. */
    frame: BattleFrame;
    /** Язык, на котором набирается текст: тот же, что у листа. */
    lang: 'en' | 'ru';
    /** Слова и числа карты правятся только на листе карты; на столе рамок
     *  карта — образец, и набирать в неё нечего. */
    textEditable?: boolean;
    write: () => BattleFrame | FrameOverride;
    onEditStart?: () => void;
    onclose: () => void;
  } = $props();

  let look = $derived(freeMarkOf(frame, slot));
  let own = $derived<FreeMark>(frame.freeMarks?.[slot] ?? {});
  let stat = $derived(isFreeStat(slot));

  function set(patch: Partial<Record<keyof FreeMark, FreeMark[keyof FreeMark] | undefined>>) {
    setFreeMark(write(), slot, patch);
  }

  /** Пределы чисел — те же, что держит сервер. Он всё равно прижмёт, но поле,
   *  пускающее 40 в «шаг», обещало бы то, чего не будет. */
  const STAT_RANGE: Record<string, [number, number]> = {
    mana: [0, 99],
    armor: [0, 20],
    ward: [0, 20],
    reach: [0, 5],
    step: [0, 3],
    speed: [1, 5],
    mend: [0, 20],
  };

  /** Оба языка сразу, а не тот, что выбран на листе: имя и эффект
   *  обязательны на обоих, и панель, предлагавшая один, оставляла второй
   *  пустым — на полке другой локали карта стояла без имени. */
  const SIDES = ['ru', 'en'] as const;
  type Side = (typeof SIDES)[number];

  const FIELDS = {
    title: { ru: 'titleRu', en: 'titleEn' },
    kind: { ru: 'typeRu', en: 'typeEn' },
    effect: { ru: 'effectRu', en: 'effectEn' },
    lore: { ru: 'loreRu', en: 'loreEn' },
  } as const;

  type TextSlot = keyof typeof FIELDS;
  let textSlot = $derived<TextSlot | null>(slot in FIELDS ? (slot as TextSlot) : null);

  function textOf(side: Side): string {
    if (!textSlot) return '';
    return ((card as unknown as Record<string, string | null>)[FIELDS[textSlot][side]] ?? '') as string;
  }

  function setText(side: Side, value: string) {
    if (!textSlot) return;
    const key = FIELDS[textSlot][side];
    // Имя — строка, у остальных пустое значит «нет» (`null`), как в форме листа.
    (card as unknown as Record<string, string | null>)[key] =
      textSlot === 'title' ? value : value || null;
  }

  function setStat(field: HTMLInputElement) {
    if (!stat || !Number.isFinite(field.valueAsNumber)) return;
    const [min, max] = STAT_RANGE[slot] ?? [0, 99];
    const n = Math.min(max, Math.max(min, Math.round(field.valueAsNumber)));
    // Прижатое — в самом поле, а не только на карте.
    if (String(n) !== field.value) field.value = String(n);
    (card as unknown as Record<string, number>)[slot] = n;
  }

  const ALIGNS: { id: FreeMarkAlign; glyph: string; key: 'adminBattlesReadyAlignLeft' | 'adminBattlesReadyAlignCenter' | 'adminBattlesReadyAlignRight' }[] = [
    { id: 'left', glyph: '⇤', key: 'adminBattlesReadyAlignLeft' },
    { id: 'center', glyph: '↔', key: 'adminBattlesReadyAlignCenter' },
    { id: 'right', glyph: '⇥', key: 'adminBattlesReadyAlignRight' },
  ];
</script>

<!-- Снимок для отмены — один на нажатие, перехватом, как у стола значка. -->
<div class="mi" onpointerdowncapture={() => onEditStart?.()}>
  <header class="mi-head">
    <span class="mi-title">{$t(freeMarkLabel(slot))}</span>
    <button type="button" class="mi-close" onclick={onclose} aria-label={$t('adminBattlesFrameClose')}>×</button>
  </header>

  {#if stat}
    {#if textEditable}
      <section class="mi-part">
        <span class="mi-label">{$t('adminBattlesReadyNumber')}</span>
        <input
          type="number"
          class="mi-input mi-input--num"
          min={(STAT_RANGE[slot] ?? [0, 99])[0]}
          max={(STAT_RANGE[slot] ?? [0, 99])[1]}
          step="1"
          value={(card as unknown as Record<string, number>)[slot] ?? 0}
          onfocus={selectOnFocus}
          onwheel={blurOnWheel}
          oninput={(e) => setStat(e.currentTarget)}
        />
      </section>
    {/if}
      <BattleGlyphControl {frame} slot={slot as GlyphSlot} {write} from={look} />
    {:else if textSlot && textEditable}
      {#each SIDES as side (side)}
        {@const value = textOf(side)}
        <section class="mi-part">
          <span class="mi-label">
            {side === 'ru' ? 'Русский' : 'English'}
            {#if side === lang}<em class="mi-now">{$t('adminBattlesReadyOnCard')}</em>{/if}
          </span>
          {#if textSlot === 'title' || textSlot === 'kind'}
            <input
              type="text"
              class="mi-input"
              class:mi-input--empty={!value.trim()}
              lang={side}
              maxlength={textSlot === 'title' ? 80 : 40}
              {value}
              oninput={(e) => setText(side, e.currentTarget.value)}
            />
          {:else}
            <textarea
              class="mi-input"
              class:mi-input--empty={!value.trim()}
              lang={side}
              rows="3"
              maxlength="400"
              {value}
              oninput={(e) => setText(side, e.currentTarget.value)}
            ></textarea>
          {/if}
          {#if side === 'en' && value.trim() && mostlyCyrillic(value)}
            <p class="mi-warn">{$t('adminBattlesReadyCyrillicHint')}</p>
          {:else if !value.trim() && (textSlot === 'title' || textSlot === 'effect')}
            <p class="mi-warn">{$t('adminBattlesReadyLangRequired')}</p>
          {/if}
        </section>
      {/each}
      {#if textSlot === 'kind'}<p class="mi-hint">{$t('adminBattlesReadyKindHint')}</p>{/if}
    {:else if textEditable}
      <p class="mi-hint">{$t('adminBattlesReadyTraitsHint')}</p>
  {/if}

  <section class="mi-part">
    <span class="mi-label">{$t('adminBattlesReadyFont')}</span>
    <div class="mi-row">
      <select
        class="mi-input"
        value={own.font ?? ''}
        onchange={(e) => set({ font: e.currentTarget.value || undefined })}
      >
        <option value="">{$t('adminBattlesTitleFontDefault')}</option>
        {#each SITE_FONTS as font (font.id)}
          <option value={font.id}>{font.name}</option>
        {/each}
      </select>
      <button
        type="button"
        class="mi-toggle"
        class:active={look.bold}
        aria-pressed={look.bold}
        title={$t('adminBattlesReadyBold')}
        onclick={() => set({ bold: !look.bold })}
      ><b>Ж</b></button>
      <button
        type="button"
        class="mi-toggle"
        class:active={look.italic}
        aria-pressed={look.italic}
        title={$t('adminBattlesReadyItalic')}
        onclick={() => set({ italic: !look.italic })}
      ><i>К</i></button>
    </div>
  </section>

  <section class="mi-part">
    <span class="mi-label">{$t('adminBattlesReadyInk')}</span>
    <div class="mi-row">
      <input
        type="color"
        aria-label={$t('adminBattlesReadyInk')}
        value={look.ink}
        oninput={(e) => set({ ink: e.currentTarget.value })}
      />
      <input
        type="text"
        class="mi-input mi-hex"
        spellcheck="false"
        value={look.ink}
        onchange={(e) => set({ ink: e.currentTarget.value.trim() || undefined })}
      />
      {#if own.ink}
        <button type="button" class="mi-reset" onclick={() => set({ ink: undefined })}
          >{$t('adminBattlesReadyAsCard')}</button
        >
      {/if}
    </div>
  </section>

  <section class="mi-part">
    <span class="mi-label">{$t('adminBattlesReadySize')}</span>
    <div class="mi-row">
      <input
        type="range"
        min={FREE_MARK_SIZE_MIN}
        max={FREE_MARK_SIZE_MAX}
        step="0.05"
        value={look.size}
        oninput={(e) => set({ size: Number(e.currentTarget.value) })}
      />
      <span class="mi-num">{look.size.toFixed(2)}×</span>
      {#if own.size}
        <button type="button" class="mi-reset" onclick={() => set({ size: undefined })}>1×</button>
      {/if}
    </div>
  </section>

  {#if !stat}
    <section class="mi-part">
      <span class="mi-label">{$t('adminBattlesReadyWidth')}</span>
      <div class="mi-row">
        <input
          type="range"
          min={FREE_MARK_WIDTH_MIN}
          max={FREE_MARK_WIDTH_MAX}
          step="1"
          value={look.width || FREE_MARK_WIDTH_MAX}
          oninput={(e) => set({ width: Number(e.currentTarget.value) })}
        />
        <span class="mi-num">{Math.round(look.width)}%</span>
      </div>
      <div class="mi-row" role="radiogroup" aria-label={$t('adminBattlesReadyAlign')}>
        {#each ALIGNS as one (one.id)}
          <button
            type="button"
            class="mi-toggle"
            class:active={look.align === one.id}
            role="radio"
            aria-checked={look.align === one.id}
            title={$t(one.key)}
            onclick={() => set({ align: one.id })}
          >{one.glyph}</button>
        {/each}
      </div>
    </section>
  {/if}

  <section class="mi-part">
    <span class="mi-label">{$t('adminBattlesBadgePlace')}</span>
    <div class="mi-row">
      <label class="mi-field">
        X <input
          type="number" min="0" max="100" step="0.5"
          value={Math.round(look.x * 2) / 2}
          oninput={(e) => {
            const n = e.currentTarget.valueAsNumber;
            if (Number.isFinite(n)) set({ x: Math.min(100, Math.max(0, n)) });
          }}
        />
      </label>
      <label class="mi-field">
        Y <input
          type="number" min="0" max="100" step="0.5"
          value={Math.round(look.y * 2) / 2}
          oninput={(e) => {
            const n = e.currentTarget.valueAsNumber;
            if (Number.isFinite(n)) set({ y: Math.min(100, Math.max(0, n)) });
          }}
        />
      </label>
    </div>
    <p class="mi-hint">{$t('adminBattlesReadyArrows')}</p>
  </section>

  <footer class="mi-foot">
    <button
      type="button"
      class="mi-reset"
      onclick={() => {
        set({ shown: false });
        onclose();
      }}>{$t('adminBattlesReadyHide')}</button
    >
  </footer>
</div>

<style>
  /* Свой стол, а не бумага карты: метку правят и на чёрной иллюстрации, а
     инструмент обязан оставаться читаемым при любой картинке. Ширина в
     пикселях: под лупой стол не растёт вместе с картой. */
  .mi {
    display: flex;
    flex-direction: column;
    gap: 0.55em;
    width: 16rem;
    max-height: 80vh;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: 0.55em 0.65em 0.7em;
    font-family: Inter, system-ui, sans-serif;
    font-size: 0.7rem;
    font-style: normal;
    font-weight: 400;
    text-align: left;
    text-transform: none;
    letter-spacing: normal;
    white-space: normal;
    --paper: #f8f1e7;
    --ink: #34251c;
    color: var(--ink);
    background: var(--paper);
    border: 1px solid color-mix(in oklab, var(--ink) 30%, transparent);
    box-shadow: 0 6px 22px rgba(0, 0, 0, 0.22);
  }

  .mi-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-bottom: 0.35em;
    border-bottom: 1px solid color-mix(in oklab, var(--ink) 18%, transparent);
  }

  .mi-title {
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.07em;
  }

  .mi-close {
    padding: 0 0.2em;
    font: inherit;
    font-size: 1.1em;
    line-height: 1;
    color: inherit;
    background: none;
    border: none;
    cursor: pointer;
  }

  .mi-part {
    display: flex;
    flex-direction: column;
    gap: 0.32em;
  }

  .mi-label {
    font-size: 0.9em;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    opacity: 0.65;
  }

  /* Какой язык сейчас стоит на карте — рядом с его полем. */
  .mi-now {
    margin-left: 0.4em;
    padding: 0 0.35em;
    font-style: normal;
    letter-spacing: 0.04em;
    color: var(--paper);
    background: var(--ink);
  }

  .mi-input--empty {
    border-color: #c65f3c;
  }

  .mi-warn {
    margin: 0;
    font-size: 0.88em;
    line-height: 1.35;
    color: #8f2f22;
  }

  .mi-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4em 0.35em;
  }

  .mi-input,
  .mi-field input {
    min-width: 0;
    padding: 0.25em 0.4em;
    font: inherit;
    font-size: 1.05em;
    color: var(--ink);
    background: color-mix(in oklab, var(--paper) 90%, var(--ink) 10%);
    border: 1px solid color-mix(in oklab, var(--ink) 22%, transparent);
  }

  textarea.mi-input {
    resize: vertical;
    line-height: 1.35;
  }

  select.mi-input {
    flex: 1;
  }

  .mi-input--num {
    width: 5em;
  }

  .mi-hex {
    flex: 1;
    min-width: 5.5em;
  }

  .mi-field {
    display: flex;
    align-items: center;
    gap: 0.3em;
  }

  .mi-field input {
    width: 5em;
  }

  .mi-row input[type='color'] {
    flex: none;
    width: 1.7em;
    height: 1.5em;
    padding: 0;
    background: none;
    border: 1px solid color-mix(in oklab, var(--ink) 25%, transparent);
    cursor: pointer;
  }

  .mi-row input[type='range'] {
    flex: 1;
    min-width: 0;
    accent-color: var(--ink);
  }

  .mi-num {
    min-width: 3em;
    text-align: right;
    font-variant-numeric: tabular-nums;
    opacity: 0.75;
  }

  .mi-toggle {
    min-width: 1.9em;
    height: 1.9em;
    padding: 0 0.35em;
    font: inherit;
    font-size: 1.05em;
    color: var(--ink);
    background: none;
    border: 1px solid color-mix(in oklab, var(--ink) 22%, transparent);
    cursor: pointer;
  }

  .mi-toggle.active {
    color: var(--paper);
    background: var(--ink);
  }

  .mi-reset {
    padding: 0.15em 0.4em;
    font: inherit;
    font-size: 0.92em;
    color: var(--ink);
    background: none;
    border: 1px solid color-mix(in oklab, var(--ink) 22%, transparent);
    cursor: pointer;
  }

  .mi-hint {
    margin: 0;
    font-size: 0.88em;
    line-height: 1.35;
    opacity: 0.6;
  }

  .mi-foot {
    display: flex;
    justify-content: flex-end;
    padding-top: 0.4em;
    border-top: 1px solid color-mix(in oklab, var(--ink) 18%, transparent);
  }
</style>
