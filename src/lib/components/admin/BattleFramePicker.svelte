<script lang="ts">
  // Ящик нарядов: выдвижным списком или разложенный гардеробом.
  //
  // Был полосой квадратиков в четыре с половиной сантиметра, уезжавшей вбок:
  // с двумя нарядами это читалось, с дюжиной — уже нет, а имя под квадратиком
  // обрезалось на третьей букве. Наряд выбирают по двум вещам сразу — по лицу
  // и по имени, — поэтому здесь строка: лицо слева, имя целиком справа, и всё
  // это в списке, который раскрывается над столом и не занимает места, пока
  // его не спросили.
  //
  // Само лицо — это картинка рамки на её же бумаге и в её же кайме: у собранной
  // из частей узнаваем угол или притолока, у прочих — целая фотография. Наряд
  // без единой картинки всё равно узнаётся по цвету бумаги, поэтому квадратик
  // рисуется всегда.
  import { t } from '$lib/i18n';
  import type { BattleFramePreset } from '$lib/types/api';

  let {
    presets,
    chosen = $bindable(null),
    onchoose,
    onforget,
    allowNone = false,
    disabled = false,
    label,
    layout = 'drawer',
    size = 'compact',
  } = $props<{
    presets: BattleFramePreset[];
    /** Имя выбранного наряда. Связка, а не событие: у стола битв три места, где
     *  наряд берут, и каждое помнит свой выбор само. */
    chosen?: string | null;
    /** Что делать, когда наряд выбрали. У чина — надеть сразу: список для того
     *  и раскрывали. У расы и карты — ничего: там ещё выбирают, на что надеть. */
    onchoose?: (preset: BattleFramePreset) => void;
    /** Даётся только там, где наряд можно забыть, — крестик у строки. */
    onforget?: (preset: BattleFramePreset) => void;
    /** Пустая строка сверху: «не выбрано». Нужна там, где выбор — это ещё не
     *  действие, и его можно отменить. */
    allowNone?: boolean;
    disabled?: boolean;
    label?: string;
    /** `drawer` — строка, которую раскрывают. `rack` — все наряды сразу. */
    layout?: 'drawer' | 'rack';
    /** `desk` — табличка на столе рамок: имя крупнее, список шире, забыть
     *  видно сразу, а не при наведении. */
    size?: 'compact' | 'desk';
  }>();

  let desk = $derived(size === 'desk');

  let open = $state(false);
  /** Строка под стрелками. Отдельно от выбранной: по списку ходят, ничего не
   *  надевая, пока не нажали Enter. */
  let active = $state(-1);
  let listBox = $state<HTMLElement | null>(null);

  let taken = $derived(
    presets.find((p: BattleFramePreset) => p.id === chosen) ?? null,
  );

  /** Лицо наряда: его картинка, если она есть, — угол или притолока у собранной
   *  из частей, целая фотография у прочих. */
  function face(preset: BattleFramePreset): string {
    const art =
      preset.frame.frameMode === 'sliced'
        ? preset.frame.cornerImage?.trim() ||
          preset.frame.sideImageH?.trim() ||
          preset.frame.sideImageV?.trim()
        : preset.frame.frameImage?.trim();
    return art ? `url("${art}")` : 'none';
  }

  function swatchStyle(preset: BattleFramePreset): string {
    return `background-color:${preset.frame.paper}; border-color:${preset.frame.border}; background-image:${face(preset)}`;
  }

  function unfold() {
    if (disabled) return;
    open = true;
    active = presets.findIndex((p: BattleFramePreset) => p.id === chosen);
  }

  function pick(preset: BattleFramePreset) {
    chosen = preset.id;
    open = false;
    onchoose?.(preset);
  }

  function clear() {
    chosen = null;
    open = false;
  }

  /** Стрелки по списку, Enter — надеть, Escape — закрыть. Строка под стрелками
   *  подтягивается в видимую часть сама: ящик прокручивается, и ходить по нему
   *  вслепую было бы то же, что не ходить вовсе. */
  function keys(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      open = false;
      return;
    }
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      if (!open) {
        unfold();
        return;
      }
      const step = event.key === 'ArrowDown' ? 1 : -1;
      const last = presets.length - 1;
      active = active < 0 ? (step > 0 ? 0 : last) : active + step;
      if (active < 0) active = last;
      if (active > last) active = 0;
      listBox
        ?.querySelector(`[data-row="${active}"]`)
        ?.scrollIntoView({ block: 'nearest' });
      return;
    }
    if (event.key === 'Enter' || event.key === ' ') {
      if (!open) {
        event.preventDefault();
        unfold();
        return;
      }
      if (active >= 0 && presets[active]) {
        event.preventDefault();
        pick(presets[active]);
      }
    }
  }
</script>

{#if layout === 'rack'}
  <div
    role="listbox"
    aria-label={label ?? $t('adminBattlesPresetChoose')}
    class="grid gap-2.5"
    style="grid-template-columns: repeat(auto-fill, minmax(4.4rem, 1fr));"
  >
    {#if !presets.length}
      <p class="col-span-full text-[11px] italic text-[#8a6a55]">
        {$t('adminBattlesPresetsEmpty')}
      </p>
    {/if}
    {#if allowNone && presets.length}
      <button
        role="option"
        aria-selected={!chosen}
        onclick={clear}
        {disabled}
        class="group flex flex-col gap-1 text-left disabled:opacity-40"
      >
        <span
          class="w-full border border-dashed {!chosen
            ? 'border-[#c65f3c]'
            : 'border-[#34251c]/25'}"
          style="aspect-ratio: 5 / 7;"
        ></span>
        <span
          class="text-[10px] leading-snug italic {!chosen
            ? 'text-[#c65f3c]'
            : 'text-[#8a6a55]'}">{$t('adminBattlesPresetNone')}</span
        >
      </button>
    {/if}
    {#each presets as preset (preset.id)}
      <div class="relative group">
        <button
          role="option"
          aria-selected={preset.id === chosen}
          onclick={() => pick(preset)}
          {disabled}
          title={preset.name}
          class="w-full flex flex-col gap-1 text-left disabled:opacity-40"
        >
          <!-- Обводка тенью, а не рамкой: рамка на выбранной плитке шире на
               пиксель, и весь ряд разъезжается от одного нажатия. -->
          <span
            class="w-full border bg-center bg-contain bg-no-repeat {preset.id ===
            chosen
              ? 'shadow-[0_0_0_2px_#c65f3c]'
              : ''}"
            style="aspect-ratio: 5 / 7; {swatchStyle(preset)}"
          ></span>
          <span
            class="text-[10px] leading-snug break-words {preset.id === chosen
              ? 'text-[#c65f3c]'
              : 'text-[#5f4636]'}">{preset.name}</span
          >
        </button>
        {#if onforget}
          <button
            onclick={() => onforget(preset)}
            title={$t('adminBattlesPresetForget')}
            class="absolute top-0.5 right-0.5 w-5 h-5 flex items-center justify-center text-[11px] leading-none bg-[#f8f1e7] text-[#8f2f22] opacity-0 group-hover:opacity-100 focus:opacity-100 hover:bg-[#c65f3c]/12"
            >×</button
          >
        {/if}
      </div>
    {/each}
  </div>
{:else}
<div class="relative" onkeydown={keys} role="presentation">
  <button
    onclick={() => (open ? (open = false) : unfold())}
    {disabled}
    title={label ?? $t('adminBattlesPresetChoose')}
    aria-haspopup="listbox"
    aria-expanded={open}
    class="w-full flex items-center gap-2 text-left bg-transparent border border-[#34251c]/20 hover:border-[#34251c]/35 disabled:opacity-40 {desk
      ? 'px-2 py-2'
      : 'px-1.5 py-1.5'}"
  >
    {#if taken}
      <span
        class="flex-shrink-0 border bg-center bg-contain bg-no-repeat {desk
          ? 'w-9 h-[3.15rem]'
          : 'w-8 h-11'}"
        style={swatchStyle(taken)}
      ></span>
      <span
        class="flex-1 min-w-0 truncate {desk
          ? 'text-[1.15rem] leading-tight'
          : 'text-xs'}"
        style={desk
          ? "font-family: 'Cormorant Garamond', Georgia, serif;"
          : undefined}>{taken.name}</span
      >
    {:else}
      <span
        class="flex-shrink-0 border border-dashed border-[#34251c]/25 {desk
          ? 'w-9 h-[3.15rem]'
          : 'w-8 h-11'}"
      ></span>
      <span
        class="flex-1 min-w-0 italic text-[#8a6a55] {desk
          ? 'text-sm'
          : 'text-xs'}">{$t('adminBattlesPresetNone')}</span
      >
    {/if}
    <span class="flex-shrink-0 px-1 text-[10px] text-[#8a6a55]"
      >{open ? '▴' : '▾'}</span
    >
  </button>

  {#if open}
    <!-- Занавес. Нажатие мимо списка закрывает его — и не доходит до карты,
         которая под ним и слушает каждое нажатие как взятие детали. -->
    <div
      class="fixed inset-0 z-40"
      role="presentation"
      onclick={() => (open = false)}
    ></div>
    <ul
      bind:this={listBox}
      role="listbox"
      aria-label={label ?? $t('adminBattlesPresetChoose')}
      class="absolute z-50 left-0 right-0 mt-1 max-h-[19rem] overflow-y-auto bg-[#f8f1e7] border border-[#34251c]/25 shadow-[0_10px_30px_rgba(52,37,28,0.22)]"
    >
      {#if !presets.length}
        <li class="px-3 py-3 text-[11px] italic text-[#8a6a55]">
          {$t('adminBattlesPresetsEmpty')}
        </li>
      {/if}
      {#if allowNone && presets.length}
        <li>
          <button
            onclick={clear}
            class="w-full flex items-center gap-2 px-1.5 py-1.5 text-left text-[11px] italic text-[#8a6a55] hover:bg-[#34251c]/6"
          >
            <span
              class="flex-shrink-0 w-8 h-11 border border-dashed border-[#34251c]/25"
            ></span>
            {$t('adminBattlesPresetNone')}
          </button>
        </li>
      {/if}
      {#each presets as preset, i (preset.id)}
        <li class="relative group border-t border-[#34251c]/8 first:border-t-0">
          <button
            data-row={i}
            role="option"
            aria-selected={preset.id === chosen}
            onclick={() => pick(preset)}
            onpointerenter={() => (active = i)}
            class="w-full flex items-center gap-2 px-1.5 py-1.5 {onforget
              ? 'pr-8'
              : ''} text-left {i === active
              ? 'bg-[#34251c]/8'
              : ''} {preset.id === chosen ? 'text-[#c65f3c]' : ''}"
          >
            <span
              class="flex-shrink-0 w-8 h-11 border bg-center bg-contain bg-no-repeat"
              style={swatchStyle(preset)}
            ></span>
            <span class="flex-1 min-w-0 truncate {desk ? 'text-sm' : 'text-xs'}"
              >{preset.name}</span
            >
          </button>
          {#if onforget}
            <button
              onclick={(e) => {
                e.stopPropagation();
                onforget(preset);
              }}
              title={$t('adminBattlesPresetForget')}
              class="absolute top-1/2 right-1 -translate-y-1/2 w-6 h-6 flex items-center justify-center text-[13px] leading-none text-[#8f2f22] hover:bg-[#c65f3c]/12 {desk
                ? 'opacity-100'
                : 'opacity-0 group-hover:opacity-100 focus:opacity-100'}"
              >×</button
            >
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</div>
{/if}
