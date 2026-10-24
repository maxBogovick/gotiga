<script lang="ts">
  /**
   * Стол надписей на обложке книги: название на матовой табличке и имя автора.
   *
   * Обложка стоит крупно слева, табличку берут и переносят мышью прямо на ней
   * (тот же жест, что у меток карты), справа — всё, что правится у выбранной
   * надписи: текст, шрифт, толщина, кегль, высота строки, цвет, ширина,
   * место и сама табличка. Обложка здесь — та же `TaleBookFace`, что на полке:
   * предпросмотр, считающий иначе, однажды соврал бы.
   *
   * Правится черновик; в базу он уходит одной кнопкой своей ручкой
   * (`adminSetTaleBookCoverText`): автосохранение листа надписей не трогает.
   */
  import { t } from '$lib/i18n';
  import { portal } from '$lib/actions/portal';
  import { api } from '$lib/api';
  import { SITE_FONTS } from '$lib/fonts';
  import TaleBookFace from '$lib/components/TaleBookFace.svelte';
  import { selectOnFocus, blurOnWheel } from '$lib/utils/fields';
  import { COVER_LIMITS, defaultLabel, labelOf, type BookCoverKey } from '$lib/bookCover';
  import type { BookCoverAlign, BookCoverLabel, BookCoverPlate, BookCoverText } from '$lib/types/api';

  let {
    taleId,
    bookCover,
    title,
    author,
    value,
    onsaved,
    onclose,
  }: {
    taleId: string;
    bookCover: string;
    /** Название на языке, который сейчас правится на столе. */
    title: string;
    /** Автор с листа; надпись без своего текста печатает его. */
    author: string;
    /** Сохранённое. */
    value: BookCoverText | null;
    onsaved: (text: BookCoverText | null) => void;
    onclose: () => void;
  } = $props();

  const copy = (v: BookCoverText | null | undefined): BookCoverText =>
    JSON.parse(JSON.stringify(v ?? {})) as BookCoverText;

  // Черновик начинается с сохранённого и дальше живёт сам: прежнее значение приходит только при открытии.
  // svelte-ignore state_referenced_locally
  let draft = $state<BookCoverText>(copy(value));
  // svelte-ignore state_referenced_locally
  let saved = $state(JSON.stringify(copy(value)));
  let selected = $state<BookCoverKey | null>(null);
  let busy = $state(false);
  let error = $state('');

  let dirty = $derived(JSON.stringify(draft) !== saved);
  let label = $derived(selected ? labelOf(draft, selected) : null);

  const KEYS: BookCoverKey[] = ['title', 'author'];
  const present = (key: BookCoverKey) => !!draft[key];

  /** Запись надписи целиком: на сервер уходят все числа, а не разница. */
  function write(key: BookCoverKey, next: BookCoverLabel) {
    draft = { ...draft, [key]: next };
  }

  function patch(change: Partial<Omit<BookCoverLabel, 'plate'>>) {
    if (!selected || !label) return;
    write(selected, { ...label, ...change });
  }

  function patchPlate(change: Partial<BookCoverPlate>) {
    if (!selected || !label) return;
    write(selected, { ...label, plate: { ...label.plate, ...change } });
  }

  function add(key: BookCoverKey) {
    write(key, defaultLabel(key));
    selected = key;
  }

  function remove(key: BookCoverKey) {
    const next = { ...draft };
    delete next[key];
    draft = next;
    if (selected === key) selected = null;
  }

  function move(key: BookCoverKey, x: number, y: number) {
    const own = labelOf(draft, key);
    if (own) write(key, { ...own, x, y });
  }

  /** Число из поля, прижатое к пределам; чужое и пустое не пишется. */
  function num(e: Event & { currentTarget: HTMLInputElement }, min: number, max: number): number | null {
    const n = e.currentTarget.valueAsNumber;
    return Number.isFinite(n) ? Math.min(max, Math.max(min, n)) : null;
  }

  function setNum(field: 'size' | 'lineHeight' | 'spacing' | 'width' | 'weight') {
    const [min, max] = COVER_LIMITS[field];
    return (e: Event & { currentTarget: HTMLInputElement }) => {
      const n = num(e, min, max);
      if (n !== null) patch({ [field]: n } as Partial<BookCoverLabel>);
    };
  }

  function setPlateNum(field: 'blur' | 'radius' | 'pad' | 'opacity') {
    const [min, max] = field === 'opacity' ? [0, 1] : COVER_LIMITS[field];
    return (e: Event & { currentTarget: HTMLInputElement }) => {
      const n = num(e, min, max);
      if (n !== null) patchPlate({ [field]: n } as Partial<BookCoverPlate>);
    };
  }

  function setPlace(axis: 'x' | 'y') {
    return (e: Event & { currentTarget: HTMLInputElement }) => {
      const n = num(e, 0, 100);
      if (n !== null) patch({ [axis]: n });
    };
  }

  /** Цвет из текстового поля: только `#rrggbb`, иначе поле вернётся к прежнему. */
  function setHex(apply: (hex: string) => void, current: string) {
    return (e: Event & { currentTarget: HTMLInputElement }) => {
      const v = e.currentTarget.value.trim();
      const hex = /^#[0-9a-fA-F]{6}$/.test(v) ? v.toLowerCase() : null;
      if (hex) apply(hex);
      else e.currentTarget.value = current;
    };
  }

  const ALIGNS: { id: BookCoverAlign; glyph: string; key: 'adminBattlesReadyAlignLeft' | 'adminBattlesReadyAlignCenter' | 'adminBattlesReadyAlignRight' }[] = [
    { id: 'left', glyph: '⇤', key: 'adminBattlesReadyAlignLeft' },
    { id: 'center', glyph: '↔', key: 'adminBattlesReadyAlignCenter' },
    { id: 'right', glyph: '⇥', key: 'adminBattlesReadyAlignRight' },
  ];

  const fmt = (n: number, digits = 1) => String(Math.round(n * 10 ** digits) / 10 ** digits);

  async function save() {
    busy = true;
    error = '';
    try {
      const text = draft.title || draft.author ? draft : null;
      await api.adminSetTaleBookCoverText(taleId, text);
      saved = JSON.stringify(draft);
      onsaved(text ? copy(text) : null);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function revert() {
    draft = JSON.parse(saved) as BookCoverText;
    selected = null;
  }

  function close() {
    if (dirty && !confirm($t('adminTalesCoverCloseUnsaved'))) return;
    onclose();
  }

  function keydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      close();
    }
  }

  function leaving(e: BeforeUnloadEvent) {
    if (dirty) e.preventDefault();
  }

  let plateOn = $derived(label?.plate.on ?? false);
</script>

<svelte:window onkeydown={keydown} onbeforeunload={leaving} />

<div class="veil" use:portal role="presentation" onpointerdown={(e) => e.target === e.currentTarget && close()}>
  <div class="desk" role="dialog" aria-modal="true" aria-label={$t('adminTalesCoverHeading')}>
    <header class="top">
      <h3>{$t('adminTalesCoverHeading')}</h3>
      {#if dirty}<span class="dirty">{$t('adminTalesCoverUnsaved')}</span>{/if}
      <span class="grow"></span>
      <button type="button" class="btn" onclick={revert} disabled={!dirty || busy}>{$t('adminTalesCoverRevert')}</button>
      <button type="button" class="btn btn--main" onclick={save} disabled={!dirty || busy}>
        {busy ? '…' : $t('adminTalesCoverSave')}
      </button>
      <button type="button" class="x" onclick={close} aria-label={$t('adminTalesCoverClose')}>×</button>
    </header>
    {#if error}<p class="err">{error}</p>{/if}

    <div class="body">
      <div class="stage">
        <div class="book">
          <TaleBookFace
            {title}
            {bookCover}
            {author}
            coverText={draft}
            sizes="520px"
            eager
            editing={{
              selected,
              onpick: (key) => (selected = key),
              onmove: move,
            }}
          />
        </div>
        <p class="hint">{$t('adminTalesCoverHint')}</p>
      </div>

      <div class="side">
        <div class="adds">
          {#each KEYS as key (key)}
            {#if !present(key)}
              <button type="button" class="btn" onclick={() => add(key)}>
                ✚ {$t(key === 'title' ? 'adminTalesCoverAddTitle' : 'adminTalesCoverAddAuthor')}
              </button>
            {/if}
          {/each}
        </div>

        {#if present('title') || present('author')}
          <div class="tabs" role="tablist">
            {#each KEYS as key (key)}
              {#if present(key)}
                <button
                  type="button"
                  role="tab"
                  aria-selected={selected === key}
                  class="tab"
                  class:on={selected === key}
                  onclick={() => (selected = key)}
                >{$t(key === 'title' ? 'adminTalesCoverTitle' : 'adminTalesCoverAuthor')}</button>
              {/if}
            {/each}
          </div>
        {/if}

        {#if label && selected}
          {@const key = selected}
          <section class="part">
            <span class="lab">{$t('adminTalesCoverText')}</span>
            <input
              type="text"
              class="in"
              maxlength="120"
              value={label.text}
              placeholder={key === 'title' ? title : author}
              oninput={(e) => patch({ text: e.currentTarget.value })}
            />
            <p class="note">{$t(key === 'title' ? 'adminTalesCoverTitleTextHint' : 'adminTalesCoverAuthorTextHint')}</p>
          </section>

          <section class="part">
            <span class="lab">{$t('adminTalesCoverFont')}</span>
            <div class="row">
              <select class="in grow" value={label.font} onchange={(e) => patch({ font: e.currentTarget.value })}>
                {#each SITE_FONTS as font (font.id)}
                  <option value={font.id}>{font.name}</option>
                {/each}
              </select>
              <button
                type="button"
                class="tog"
                class:on={label.italic}
                aria-pressed={label.italic}
                title={$t('adminBattlesReadyItalic')}
                onclick={() => patch({ italic: !label.italic })}
              ><i>{$t('adminBattlesReadyItalicLetter')}</i></button>
            </div>
          </section>

          <section class="part">
            <span class="lab">{$t('adminTalesCoverWeight')}</span>
            <div class="row">
              <input type="range" min="100" max="900" step="100" value={label.weight} oninput={setNum('weight')} />
              <span class="val">{label.weight}</span>
            </div>
          </section>

          <section class="part">
            <span class="lab">{$t('adminTalesCoverSize')}</span>
            <div class="row">
              <input type="range" min={COVER_LIMITS.size[0]} max={COVER_LIMITS.size[1]} step="0.1" value={label.size} oninput={setNum('size')} />
              <span class="val">{fmt(label.size)}</span>
            </div>
          </section>

          <section class="part">
            <span class="lab">{$t('adminTalesCoverLineHeight')}</span>
            <div class="row">
              <input type="range" min={COVER_LIMITS.lineHeight[0]} max={COVER_LIMITS.lineHeight[1]} step="0.05" value={label.lineHeight} oninput={setNum('lineHeight')} />
              <span class="val">{fmt(label.lineHeight, 2)}</span>
            </div>
          </section>

          <section class="part">
            <span class="lab">{$t('adminTalesCoverSpacing')}</span>
            <div class="row">
              <input type="range" min={COVER_LIMITS.spacing[0]} max={COVER_LIMITS.spacing[1]} step="0.01" value={label.spacing} oninput={setNum('spacing')} />
              <span class="val">{fmt(label.spacing, 2)}</span>
            </div>
          </section>

          <section class="part">
            <span class="lab">{$t('adminTalesCoverColor')}</span>
            <div class="row">
              <input type="color" aria-label={$t('adminTalesCoverColor')} value={label.color} oninput={(e) => patch({ color: e.currentTarget.value })} />
              <input type="text" class="in hex" spellcheck="false" value={label.color} onchange={setHex((hex) => patch({ color: hex }), label.color)} />
            </div>
          </section>

          <section class="part">
            <span class="lab">{$t('adminTalesCoverShadow')}</span>
            <div class="row">
              <input type="range" min="0" max="1" step="0.05" value={label.shadow ?? 0} oninput={(e) => patch({ shadow: Number(e.currentTarget.value) })} />
              <span class="val">{Math.round((label.shadow ?? 0) * 100)}%</span>
            </div>
          </section>

          <section class="part">
            <span class="lab">{$t('adminTalesCoverWidth')}</span>
            <div class="row">
              <input type="range" min={COVER_LIMITS.width[0]} max={COVER_LIMITS.width[1]} step="1" value={label.width} oninput={setNum('width')} />
              <span class="val">{Math.round(label.width)}%</span>
            </div>
            <div class="row" role="radiogroup" aria-label={$t('adminTalesCoverAlign')}>
              {#each ALIGNS as one (one.id)}
                <button
                  type="button"
                  class="tog"
                  class:on={label.align === one.id}
                  role="radio"
                  aria-checked={label.align === one.id}
                  title={$t(one.key)}
                  onclick={() => patch({ align: one.id })}
                >{one.glyph}</button>
              {/each}
            </div>
          </section>

          <section class="part">
            <span class="lab">{$t('adminTalesCoverPlace')}</span>
            <div class="row">
              <label class="fld">X <input type="number" min="0" max="100" step="0.5" value={fmt(label.x)} onfocus={selectOnFocus} onwheel={blurOnWheel} oninput={setPlace('x')} /></label>
              <label class="fld">Y <input type="number" min="0" max="100" step="0.5" value={fmt(label.y)} onfocus={selectOnFocus} onwheel={blurOnWheel} oninput={setPlace('y')} /></label>
            </div>
          </section>

          <section class="part plate">
            <label class="check">
              <input type="checkbox" checked={label.plate.on} onchange={(e) => patchPlate({ on: e.currentTarget.checked })} />
              <span class="lab">{$t('adminTalesCoverPlateOn')}</span>
            </label>
            {#if plateOn}
              <div class="row">
                <input type="color" aria-label={$t('adminTalesCoverPlate')} value={label.plate.color} oninput={(e) => patchPlate({ color: e.currentTarget.value })} />
                <input type="text" class="in hex" spellcheck="false" value={label.plate.color} onchange={setHex((hex) => patchPlate({ color: hex }), label.plate.color)} />
              </div>
              <span class="lab">{$t('adminTalesCoverPlateOpacity')}</span>
              <div class="row">
                <input type="range" min="0" max="1" step="0.01" value={label.plate.opacity} oninput={setPlateNum('opacity')} />
                <span class="val">{Math.round(label.plate.opacity * 100)}%</span>
              </div>
              <span class="lab">{$t('adminTalesCoverPlateBlur')}</span>
              <div class="row">
                <input type="range" min={COVER_LIMITS.blur[0]} max={COVER_LIMITS.blur[1]} step="0.1" value={label.plate.blur} oninput={setPlateNum('blur')} />
                <span class="val">{fmt(label.plate.blur)}</span>
              </div>
              <span class="lab">{$t('adminTalesCoverPlateRadius')}</span>
              <div class="row">
                <input type="range" min={COVER_LIMITS.radius[0]} max={COVER_LIMITS.radius[1]} step="0.1" value={label.plate.radius} oninput={setPlateNum('radius')} />
                <span class="val">{fmt(label.plate.radius)}</span>
              </div>
              <span class="lab">{$t('adminTalesCoverPlatePad')}</span>
              <div class="row">
                <input type="range" min={COVER_LIMITS.pad[0]} max={COVER_LIMITS.pad[1]} step="0.1" value={label.plate.pad} oninput={setPlateNum('pad')} />
                <span class="val">{fmt(label.plate.pad)}</span>
              </div>
            {/if}
          </section>

          <div class="row foot">
            <button type="button" class="btn" onclick={() => write(key, { ...defaultLabel(key), x: label.x, y: label.y, text: label.text })}>
              {$t('adminTalesCoverReset')}
            </button>
            <button type="button" class="btn btn--danger" onclick={() => remove(key)}>{$t('adminTalesCoverRemove')}</button>
          </div>
        {:else if present('title') || present('author')}
          <p class="note">{$t('adminTalesCoverPick')}</p>
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .veil {
    position: fixed;
    inset: 0;
    /* Выше нижней панели разделов (190), иначе она закрывает низ окна. */
    z-index: 400;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 20px;
    background: rgba(32, 20, 14, 0.55);
  }
  .desk {
    display: flex;
    flex-direction: column;
    width: min(1040px, 100%);
    /* Высота окна задана, а не «по содержимому»: иначе длинная колонка
       настроек растягивала бы окно за экран, и прокручивать было бы нечего. */
    height: calc(100dvh - 40px);
    max-height: 860px;
    background: #f8f1e7;
    color: #34251c;
    border: 1px solid #d8c6b1;
    box-shadow: 0 18px 60px rgba(0, 0, 0, 0.4);
    font: 13px/1.35 Inter, system-ui, sans-serif;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 16px;
    border-bottom: 1px solid #d8c6b1;
  }
  .top h3 { margin: 0; font: 600 14px/1.2 Inter, system-ui, sans-serif; }
  .grow { flex: 1; }
  .dirty { font-size: 11px; color: #c65f3c; }
  .x { font-size: 22px; line-height: 1; padding: 0 6px; background: none; border: 0; cursor: pointer; color: inherit; }
  .err { margin: 0; padding: 8px 16px; color: #8f2f22; background: rgba(198, 95, 60, 0.12); }

  .body {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 320px;
    gap: 24px;
    grid-template-rows: minmax(0, 1fr);
    flex: 1;
    padding: 18px 16px 0;
    min-height: 0;
    overflow: hidden;
  }
  .stage {
    min-height: 0;
    overflow-y: auto;
    padding-bottom: 20px; display: grid; justify-items: center; align-content: start; gap: 12px; }
  .book {
    /* Обложка 2:3 той же ширины, что на развороте; не выше окна. */
    width: min(100%, 440px, calc((100vh - 200px) * 2 / 3));
    aspect-ratio: 2 / 3;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.35);
  }
  .hint, .note { margin: 0; font-size: 11.5px; color: #6f5848; max-width: 46ch; }

  .side {
    display: grid;
    align-content: start;
    gap: 14px;
    min-width: 0;
    min-height: 0;
    overflow-y: auto;
    padding: 0 6px 24px 0;
  }
  .adds, .tabs, .row { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; }
  .part { display: grid; gap: 5px; }
  .lab { font-size: 10px; letter-spacing: 0.1em; text-transform: uppercase; color: #8a6a55; }
  .in, .fld input {
    min-width: 0;
    padding: 5px 7px;
    font: inherit;
    color: inherit;
    background: #fffaf2;
    border: 1px solid #d8c6b1;
  }
  .in { width: 100%; box-sizing: border-box; }
  select.in { width: auto; }
  .hex { flex: 1; width: auto; font-family: ui-monospace, monospace; }
  .fld { display: flex; align-items: center; gap: 5px; }
  .fld input { width: 70px; }
  input[type='range'] { flex: 1; min-width: 0; accent-color: #c65f3c; }
  input[type='color'] { width: 34px; height: 28px; padding: 0; border: 1px solid #d8c6b1; background: none; cursor: pointer; }
  .val { min-width: 3.2em; text-align: right; font-variant-numeric: tabular-nums; color: #6f5848; }
  .check { display: flex; align-items: center; gap: 8px; cursor: pointer; }
  .plate { padding-top: 10px; border-top: 1px solid #e4d6c3; }

  .btn {
    padding: 5px 11px;
    font: inherit;
    color: inherit;
    background: #fffaf2;
    border: 1px solid #d8c6b1;
    cursor: pointer;
  }
  .btn:disabled { opacity: 0.45; cursor: default; }
  .btn--main { color: #fff; background: #c65f3c; border-color: #a84a2b; }
  .btn--danger { color: #8f2f22; }
  .tab, .tog {
    padding: 5px 11px;
    font: inherit;
    color: inherit;
    background: #fffaf2;
    border: 1px solid #d8c6b1;
    cursor: pointer;
  }
  .tog { padding: 4px 9px; }
  .tab.on, .tog.on { color: #fff; background: #6f3b24; border-color: #6f3b24; }
  .foot { justify-content: space-between; padding-top: 6px; }

  @media (max-width: 860px) {
    .body { grid-template-columns: minmax(0, 1fr); grid-template-rows: none; overflow-y: auto; }
    .stage, .side { overflow: visible; }
  }
</style>
