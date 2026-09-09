<script lang="ts">
  // Стол новичка: пять решений вместо ста пятидесяти.
  //
  // Правит он ТОТ ЖЕ `BattleFrame`, что и полный стол. Сборщик, пишущий во
  // что-то своё, был бы вторым форматом рамы и разошёлся бы с первым на первой
  // же правке. Здесь он просто заполняет те же поля выборами, а не
  // перетаскиванием.
  //
  // Наполняется БИБЛИОТЕКОЙ ДОМА: «выбери уголок» из пустоты — не выбор.
  import { onMount } from 'svelte';
  import { t } from '$lib/i18n';
  import { api } from '$lib/api';
  import { defaultSlices } from '$lib/battles';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import type { BattleAsset, BattleCard as BattleCardDto, BattleFrame } from '$lib/types/api';

  interface Props {
    frame: BattleFrame;
    sample: BattleCardDto;
    /** Позвать отмену перед правкой — та же, что у полного стола. */
    mark: () => void;
    /** Уйти на полный стол. Дорога односторонняя, и об этом сказано словами. */
    onadvanced: () => void;
  }
  let { frame = $bindable(), sample, mark, onadvanced }: Props = $props();

  let corners = $state<BattleAsset[]>([]);
  let sidesH = $state<BattleAsset[]>([]);
  let sidesV = $state<BattleAsset[]>([]);
  let papers = $state<BattleAsset[]>([]);
  let loading = $state(true);

  /** Бумаги дома — на случай, если библиотека пуста: цвет всегда есть, даже
   *  когда картинок ещё не выложили. */
  const PAPERS = ['#f8f1e7', '#f3e9db', '#efe3cd', '#e6cfb2', '#eee2cb'];
  const INKS = ['#34251c', '#4a3325', '#2b1d16'];
  const BORDERS = ['#d8c6b1', '#c9b291', '#b59a76', '#8a6a55'];

  onMount(async () => {
    const all = await api.getStudioLibrary().catch(() => []);
    corners = all.filter((a) => a.role === 'corner');
    sidesH = all.filter((a) => a.role === 'sideH');
    sidesV = all.filter((a) => a.role === 'sideV');
    papers = all.filter((a) => a.role === 'art' || a.role === 'other');
    loading = false;
  });

  /** Резьба включается вместе с первой же выбранной деталью: рама в режиме
   *  «покрашена» не покажет ни уголка, и человек решит, что выбор не работает.
   *  Врезки — те же десять процентов, которыми полный стол открывает `sliced`:
   *  полоса нулевой ширины это деталь нулевого размера. */
  function carve() {
    if (frame.frameMode !== 'sliced') {
      frame.frameMode = 'sliced';
      if (!frame.insetTop && !frame.insetRight && !frame.insetBottom && !frame.insetLeft) {
        frame.insetTop = 10;
        frame.insetRight = 10;
        frame.insetBottom = 10;
        frame.insetLeft = 10;
      }
    }
    if (!frame.slices) frame.slices = defaultSlices();
  }

  function pickCorner(url: string) {
    mark();
    carve();
    frame.cornerImage = url;
  }

  function pickSide(url: string, vertical: boolean) {
    mark();
    carve();
    if (vertical) frame.sideImageV = url;
    else frame.sideImageH = url;
  }

  function pickPaperImage(url: string) {
    mark();
    frame.paperImage = url;
  }

  function pickPaper(colour: string) {
    mark();
    frame.paper = colour;
    frame.paperImage = '';
  }
</script>

<div class="flex h-full min-h-0 flex-col gap-4 overflow-y-auto p-4 lg:flex-row">
  <!-- Карта — не предпросмотр сборщика, а та же самая карта: выбор виден
       сразу и ровно таким, каким он окажется на полке. -->
  <div class="flex shrink-0 items-start justify-center lg:w-[340px]">
    <div style="width: 300px">
      <!-- Лицом вверх: `owned` по умолчанию ложь, и карта легла бы рубашкой,
           то есть выбор уголка был бы не виден вовсе. -->
      <BattleCard card={sample} frames={[frame]} owned={true} />
    </div>
  </div>

  <div class="min-w-0 flex-1 space-y-6">
    {#if loading}
      <p class="text-sm text-[#8a6a55]">{$t('studioLoading')}</p>
    {:else}
      <section>
        <h3 class="text-[11px] uppercase tracking-[0.18em] text-[#8a6a55]">
          {$t('studioBuildPaper')}
        </h3>
        <div class="mt-2 flex flex-wrap gap-2">
          {#each PAPERS as colour (colour)}
            <button
              onclick={() => pickPaper(colour)}
              style="background: {colour}"
              class="h-8 w-10 border {frame.paper === colour && !frame.paperImage
                ? 'border-[#c65f3c]'
                : 'border-[#34251c]/20'}"
              aria-label={colour}
            ></button>
          {/each}
          {#each papers as one (one.id)}
            <button
              onclick={() => pickPaperImage(one.url)}
              title={one.name}
              class="h-8 w-10 border bg-cover bg-center {frame.paperImage === one.url
                ? 'border-[#c65f3c]'
                : 'border-[#34251c]/20'}"
              style="background-image: url({one.url})"
              aria-label={one.name}
            ></button>
          {/each}
        </div>
      </section>

      <section>
        <h3 class="text-[11px] uppercase tracking-[0.18em] text-[#8a6a55]">
          {$t('studioBuildCorners')}
        </h3>
        {#if corners.length === 0}
          <p class="mt-2 text-xs text-[#b0a08e]">{$t('studioBuildNothingYet')}</p>
        {:else}
          <div class="mt-2 grid grid-cols-5 gap-2 sm:grid-cols-8">
            {#each corners as one (one.id)}
              <button
                onclick={() => pickCorner(one.url)}
                title={one.name}
                class="aspect-square border bg-[#fdf9f3] p-1 {frame.cornerImage === one.url
                  ? 'border-[#c65f3c]'
                  : 'border-[#34251c]/12'}"
              >
                <img src={one.url} alt={one.name} class="h-full w-full object-contain" />
              </button>
            {/each}
          </div>
        {/if}
      </section>

      <section>
        <h3 class="text-[11px] uppercase tracking-[0.18em] text-[#8a6a55]">
          {$t('studioBuildSides')}
        </h3>
        {#if sidesH.length === 0 && sidesV.length === 0}
          <p class="mt-2 text-xs text-[#b0a08e]">{$t('studioBuildNothingYet')}</p>
        {:else}
          <div class="mt-2 grid grid-cols-5 gap-2 sm:grid-cols-8">
            {#each sidesH as one (one.id)}
              <button
                onclick={() => pickSide(one.url, false)}
                title={one.name}
                class="aspect-square border bg-[#fdf9f3] p-1 {frame.sideImageH === one.url
                  ? 'border-[#c65f3c]'
                  : 'border-[#34251c]/12'}"
              >
                <img src={one.url} alt={one.name} class="h-full w-full object-contain" />
              </button>
            {/each}
            {#each sidesV as one (one.id)}
              <button
                onclick={() => pickSide(one.url, true)}
                title={one.name}
                class="aspect-square border bg-[#fdf9f3] p-1 {frame.sideImageV === one.url
                  ? 'border-[#c65f3c]'
                  : 'border-[#34251c]/12'}"
              >
                <img src={one.url} alt={one.name} class="h-full w-full object-contain" />
              </button>
            {/each}
          </div>
        {/if}
      </section>

      <section>
        <h3 class="text-[11px] uppercase tracking-[0.18em] text-[#8a6a55]">
          {$t('studioBuildColours')}
        </h3>
        <div class="mt-2 flex flex-wrap items-center gap-4">
          <div class="flex items-center gap-2">
            <span class="text-[10px] uppercase tracking-[0.14em] text-[#b0a08e]"
              >{$t('studioBuildInk')}</span
            >
            {#each INKS as colour (colour)}
              <button
                onclick={() => {
                  mark();
                  frame.ink = colour;
                }}
                style="background: {colour}"
                class="h-6 w-6 border {frame.ink === colour
                  ? 'border-[#c65f3c]'
                  : 'border-[#34251c]/20'}"
                aria-label={colour}
              ></button>
            {/each}
          </div>
          <div class="flex items-center gap-2">
            <span class="text-[10px] uppercase tracking-[0.14em] text-[#b0a08e]"
              >{$t('studioBuildBorder')}</span
            >
            {#each BORDERS as colour (colour)}
              <button
                onclick={() => {
                  mark();
                  frame.border = colour;
                }}
                style="background: {colour}"
                class="h-6 w-6 border {frame.border === colour
                  ? 'border-[#c65f3c]'
                  : 'border-[#34251c]/20'}"
                aria-label={colour}
              ></button>
            {/each}
          </div>
        </div>
      </section>

      <!-- Дверь на полный стол. Односторонняя, и сказано об этом до нажатия:
           раму с двенадцатью орнаментами пятью ползунками не показать, а
           сборщик, делающий вид, что может, потерял бы работу молча. -->
      <section class="border-t border-[#d8c6b1] pt-4">
        <button
          onclick={onadvanced}
          class="border border-[#34251c]/25 px-4 py-2 text-xs uppercase tracking-[0.16em] hover:bg-[#34251c]/5"
          >{$t('studioBuildAdvanced')}</button
        >
        <p class="mt-2 max-w-md text-[11px] leading-relaxed text-[#8a6a55]">
          {$t('studioBuildAdvancedNote')}
        </p>
      </section>
    {/if}
  </div>
</div>
