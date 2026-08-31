<script lang="ts">
  // Разделочная доска: один кусок крупно, и на нём пунктирные рамки.
  //
  // Нужна там, где автоматика бессильна по существу, а не по настройке: если
  // два уголка нарисованы соприкасающимися, никакой порог не разведёт их —
  // машина не знает, что задумано двумя вещами. Знает хранитель, и вот место,
  // где он это говорит.
  //
  // Рамок можно нарисовать сколько угодно; каждая станет отдельной деталью.
  // Обводить можно грубо: срез подтягивается к краю непрозрачного внутри
  // рамки — ровно так же, как подтягивается всякий разрез на листе.
  //
  // Рамки живут в ДОЛЯХ куска (0..1), а не в пикселях: доска показывает
  // картинку той величины, какая влезла в экран, и доля значит одно и то же
  // при любом масштабе.
  import { t } from '$lib/i18n';
  import type { BattleAssetRole, BattleSplitRect } from '$lib/types/api';

  let { image, title, width, height, initial, busy, onDone, onClose } = $props<{
    image: string;
    title: string;
    width: number;
    height: number;
    initial: BattleSplitRect[];
    busy?: boolean;
    onDone: (rects: BattleSplitRect[]) => void;
    onClose: () => void;
  }>();

  const ROLES: BattleAssetRole[] = ['corner', 'sideH', 'sideV', 'accent', 'art', 'other'];
  const ROLE_LABEL: Record<BattleAssetRole, string> = {
    corner: 'adminAssetsRoleCorner',
    sideH: 'adminAssetsRoleSideH',
    sideV: 'adminAssetsRoleSideV',
    accent: 'adminAssetsRoleAccent',
    art: 'adminAssetsRoleArt',
    other: 'adminAssetsRoleOther',
  };
  /** Меньше этого по любой стороне — не рамка, а промах мышью. */
  const MIN_SIDE = 0.015;

  // Снимок при рождении — и это правильно: доска открывается под ключом
  // (`{#key}` у вызывающего), поэтому на каждый кусок она рождается заново.
  // svelte-ignore state_referenced_locally
  let rects = $state<BattleSplitRect[]>(initial.map((r: BattleSplitRect) => ({ ...r })));
  let held = $state<number | null>(null);
  let board = $state<HTMLDivElement | null>(null);
  let drag: {
    mode: 'new' | 'move' | 'resize';
    index: number;
    fromX: number;
    fromY: number;
    origin: BattleSplitRect;
  } | null = null;

  function at(event: PointerEvent) {
    const box = board?.getBoundingClientRect();
    if (!box) return { x: 0, y: 0 };
    return {
      x: (event.clientX - box.left) / box.width,
      y: (event.clientY - box.top) / box.height,
    };
  }

  function clamp(value: number) {
    return Math.min(1, Math.max(0, value));
  }

  function startNew(event: PointerEvent) {
    if (event.button !== 0) return;
    const { x, y } = at(event);
    rects = [...rects, { x, y, w: 0, h: 0 }];
    held = rects.length - 1;
    drag = { mode: 'new', index: held, fromX: x, fromY: y, origin: { x, y, w: 0, h: 0 } };
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  function startOn(event: PointerEvent, index: number, mode: 'move' | 'resize') {
    if (event.button !== 0) return;
    event.stopPropagation();
    const { x, y } = at(event);
    held = index;
    drag = { mode, index, fromX: x, fromY: y, origin: { ...rects[index] } };
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  function move(event: PointerEvent) {
    if (!drag) return;
    const { x, y } = at(event);
    const next = [...rects];
    const r = { ...next[drag.index] };
    if (drag.mode === 'new') {
      // Тянуть можно в любую сторону от точки, где нажали.
      r.x = clamp(Math.min(drag.fromX, x));
      r.y = clamp(Math.min(drag.fromY, y));
      r.w = clamp(Math.max(drag.fromX, x)) - r.x;
      r.h = clamp(Math.max(drag.fromY, y)) - r.y;
    } else if (drag.mode === 'move') {
      r.x = clamp(Math.min(drag.origin.x + (x - drag.fromX), 1 - drag.origin.w));
      r.y = clamp(Math.min(drag.origin.y + (y - drag.fromY), 1 - drag.origin.h));
    } else {
      r.w = clamp(drag.origin.w + (x - drag.fromX));
      r.h = clamp(drag.origin.h + (y - drag.fromY));
      if (r.x + r.w > 1) r.w = 1 - r.x;
      if (r.y + r.h > 1) r.h = 1 - r.y;
    }
    next[drag.index] = r;
    rects = next;
  }

  function finish() {
    if (!drag) return;
    const r = rects[drag.index];
    // Промах мышью по пустому месту не должен оставлять после себя рамку в
    // пиксель шириной, которую потом придётся искать и удалять.
    if (r && (r.w < MIN_SIDE || r.h < MIN_SIDE)) {
      const index = drag.index;
      rects = rects.filter((_, i) => i !== index);
      held = null;
    }
    drag = null;
  }

  function drop(index: number) {
    rects = rects.filter((_, i) => i !== index);
    held = null;
  }

  /** Сколько пикселей исходника попадёт в рамку — до подтяжки к рисунку. */
  function pixels(r: BattleSplitRect) {
    return `${Math.round(r.w * width)}×${Math.round(r.h * height)}`;
  }

  function percent(value: number) {
    return `${value * 100}%`;
  }
</script>

<div
  class="fixed inset-0 z-[70] bg-[#34251c]/45"
  role="presentation"
  onclick={onClose}
></div>
<div
  class="fixed inset-4 z-[71] flex bg-[#f8f1e7] border border-[#34251c]/20 shadow-[0_20px_60px_rgba(52,37,28,0.25)]"
  role="dialog"
  aria-label={$t('adminAssetsSplitTitle')}
>
  <div class="flex-1 flex flex-col min-w-0">
    <div class="flex items-baseline gap-3 px-4 py-3 border-b border-[#34251c]/10">
      <h4 class="text-sm" style="font-family: 'Cormorant Garamond', Georgia, serif;">
        {$t('adminAssetsSplitTitle')}
      </h4>
      <span class="text-[11px] text-[#8a6a55]">{title} · {width}×{height}</span>
    </div>
    <!-- Полотно. Клетка — чтобы был виден край прозрачного, иначе обводить
         нечего: у детали нет собственного силуэта на пергаменте. -->
    <div class="flex-1 flex items-center justify-center p-6 min-h-0 overflow-hidden">
      <div
        bind:this={board}
        class="relative select-none tile touch-none"
        style="aspect-ratio: {width} / {height}; max-width: 100%; max-height: 100%;
               width: {width}px;"
        onpointerdown={startNew}
        onpointermove={move}
        onpointerup={finish}
        onpointercancel={finish}
      >
        <img src={image} alt={title} class="block w-full h-full object-contain pointer-events-none" />
        {#each rects as rect, i (i)}
          <div
            class="absolute cursor-move"
            style="left: {percent(rect.x)}; top: {percent(rect.y)};
                   width: {percent(rect.w)}; height: {percent(rect.h)};
                   outline: 2px dashed {held === i ? '#c65f3c' : '#34251c'};
                   outline-offset: -1px;
                   background: {held === i ? 'rgba(198,95,60,0.10)' : 'rgba(52,37,28,0.05)'};"
            role="button"
            tabindex="-1"
            onpointerdown={(e) => startOn(e, i, 'move')}
          >
            <span
              class="absolute -top-5 left-0 px-1 text-[10px] tabular-nums bg-[#f8f1e7] border border-[#34251c]/20"
            >{i + 1}</span>
            <!-- Ручка во внутреннем углу, как у резьбы на столе рамок. -->
            <span
              class="absolute -right-1.5 -bottom-1.5 w-3 h-3 bg-[#f8f1e7] border border-[#c65f3c] cursor-nwse-resize"
              role="button"
              tabindex="-1"
              onpointerdown={(e) => startOn(e, i, 'resize')}
            ></span>
          </div>
        {/each}
      </div>
    </div>
  </div>

  <aside class="w-72 shrink-0 flex flex-col border-l border-[#34251c]/10">
    <div class="px-4 py-3 border-b border-[#34251c]/10">
      <p class="text-[11px] leading-relaxed text-[#5f4636]">{$t('adminAssetsSplitHint')}</p>
    </div>
    <div class="flex-1 overflow-y-auto p-3">
      {#if !rects.length}
        <p class="text-[11px] italic leading-relaxed text-[#8a6a55]">
          {$t('adminAssetsSplitEmpty')}
        </p>
      {:else}
        <ul class="space-y-3">
          {#each rects as rect, i (i)}
            <li
              class="border p-2 {held === i ? 'border-[#c65f3c]' : 'border-[#34251c]/12'}"
              onpointerenter={() => (held = i)}
              role="listitem"
            >
              <div class="flex items-center gap-2 mb-1.5">
                <span class="text-[10px] tabular-nums text-[#8a6a55]">{i + 1}</span>
                <span class="text-[10px] tabular-nums text-[#8a6a55]">{pixels(rect)}</span>
                <button
                  onclick={() => drop(i)}
                  class="ml-auto px-1 text-[#8f2f22]/70 hover:text-[#8f2f22]"
                  title={$t('adminBattlesDelete')}>×</button
                >
              </div>
              <input
                bind:value={rect.name}
                maxlength="80"
                placeholder={$t('adminAssetsNewName')}
                class="w-full px-1.5 py-1 mb-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
              />
              <select
                bind:value={rect.role}
                class="w-full px-1.5 py-1 text-[11px] bg-transparent border border-[#34251c]/15 outline-none"
              >
                <option value={undefined}>{$t('adminAssetsSplitSameRole')}</option>
                {#each ROLES as r (r)}
                  <option value={r}>{$t(ROLE_LABEL[r] as never)}</option>
                {/each}
              </select>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
    <div class="flex items-center gap-2 px-3 py-3 border-t border-[#34251c]/10">
      <button
        onclick={onClose}
        class="px-3 py-2 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20"
      >{$t('adminAssetsCancel')}</button>
      <button
        onclick={() => onDone(rects)}
        disabled={busy || !rects.length}
        class="flex-1 px-3 py-2 text-[10px] uppercase tracking-[0.16em] bg-[#34251c] text-[#f8f1e7] disabled:opacity-40"
      >{$t('adminAssetsSplitDone')}</button>
    </div>
  </aside>
</div>

<style>
  .tile {
    background-image:
      linear-gradient(45deg, #e4d7c4 25%, transparent 25%),
      linear-gradient(-45deg, #e4d7c4 25%, transparent 25%),
      linear-gradient(45deg, transparent 75%, #e4d7c4 75%),
      linear-gradient(-45deg, transparent 75%, #e4d7c4 75%);
    background-size: 18px 18px;
    background-position:
      0 0,
      0 9px,
      9px -9px,
      -9px 0;
    background-color: #f2e8da;
  }
</style>
