<script lang="ts">
  // Склад студии: где берутся детали, из которых собирают раму.
  //
  // Три полки, и они разные не по виду, а по природе:
  //
  //   ВЕРСТАК — картинки в этом браузере. Их не видит никто, они ничего не
  //   стоят и пропадут вместе с почищенным кэшем.
  //   ЯЩИК — то, что отдано дому: переживёт чистку, видно с другого
  //   устройства, считается в 50 МБ.
  //   ЛИСТЫ — то, что ещё не детали. Лист режется НА СЕРВЕРЕ (тем же
  //   `sheet.rs`, что режет листы хозяина), поэтому он всегда в ящике.
  //
  // Разрез — главное, ради чего сюда приходят: рисуют не по детали, а листом,
  // и резать двадцать уголков вручную никто не станет.
  import { onMount } from 'svelte';
  import { t } from '$lib/i18n';
  import { api } from '$lib/api';
  import StudioAsk from '$lib/components/studio/StudioAsk.svelte';
  import '$lib/components/studio/studio-room.css';
  import { authStore } from '$lib/stores/auth.svelte';
  import * as local from '$lib/studio/local';
  import { liveUrl } from '$lib/studio/live';
  import BattleSplitBoard from '$lib/components/admin/BattleSplitBoard.svelte';
  import type {
    BattleAssetPick,
    BattleSheetCut,
    BattleSliceSettings,
    StudioAsset,
    StudioSheet,
    StudioState,
  } from '$lib/types/api';

  const ROLES = ['corner', 'sideH', 'sideV', 'accent', 'art', 'paper', 'other'];

  let studio = $state<StudioState | null>(null);
  let mine = $state<local.LocalAsset[]>([]);
  let shown = $state<Record<string, string>>({});
  let loading = $state(true);
  let busy = $state(false);
  let said = $state<string | null>(null);

  // Разрез
  let sheet = $state<StudioSheet | null>(null);
  let cut = $state<BattleSheetCut | null>(null);
  let picked = $state<Record<number, { name: string; role: string }>>({});
  let board = $state<{ index: number; image: string; width: number; height: number } | null>(null);
  /** Настройки разреза приходят С ОТВЕТОМ: первый разрез дом делает своими,
   *  и крутить есть что только со второго раза. Своих умолчаний здесь нет —
   *  второй список тех же чисел однажды разошёлся бы с настоящим. */
  let settings = $state<BattleSliceSettings | null>(null);

  let token = $derived(authStore.token);

  /** Что выбрасывают. Своим окном, а не системным: необратимое дом
   *  спрашивает сам и спрашивает на пергаменте. */
  let dropping = $state<
    null | { where: 'bench'; one: local.LocalAsset } | { where: 'box'; one: StudioAsset }
  >(null);

  onMount(reload);

  async function reload() {
    loading = true;
    mine = await local.listAssets();
    for (const one of mine) {
      const url = await liveUrl(one.key);
      if (url) shown[one.key] = url;
    }
    if (token) studio = await api.getStudio(token).catch(() => null);
    loading = false;
  }

  function flash(text: string, ms = 5000) {
    said = text;
    setTimeout(() => (said = null), ms);
  }

  // ── Верстак ────────────────────────────────────────────────────────────────

  async function bringLocal(files: FileList | null) {
    if (!files) return;
    for (const file of Array.from(files)) {
      const row = await local.keepAsset(file);
      const url = await liveUrl(row.key);
      if (url) shown[row.key] = url;
    }
    mine = await local.listAssets();
  }

  /** Отдать деталь дому. Место проверяет сервер — врать человеку числом,
   *  которое считаем мы, незачем. */
  async function toBox(one: local.LocalAsset) {
    if (!token) return;
    busy = true;
    try {
      const blob = await local.getBlob(one.key);
      if (!blob) return;
      const file = new File([blob], `${one.name}.webp`, { type: blob.type || 'image/webp' });
      await api.uploadStudioAsset(token, file, one.name, one.role);
      studio = await api.getStudio(token);
      flash($t('studioToBoxDone'));
    } catch (e) {
      flash(String(e));
    } finally {
      busy = false;
    }
  }

  async function dropLocal(one: local.LocalAsset) {
    await local.dropAsset(one.key);
    mine = await local.listAssets();
  }

  // ── Ящик ───────────────────────────────────────────────────────────────────

  async function renameBox(one: StudioAsset, name: string, role: string) {
    if (!token) return;
    await api.renameStudioAsset(token, one.id, name, role).catch((e) => flash(String(e)));
    studio = await api.getStudio(token);
  }

  async function dropBox(one: StudioAsset) {
    if (!token) return;
    await api.deleteStudioAsset(token, one.id).catch((e) => flash(String(e)));
    studio = await api.getStudio(token);
  }

  // ── Листы и разрез ─────────────────────────────────────────────────────────

  async function bringSheet(files: FileList | null) {
    if (!token || !files?.length) return;
    busy = true;
    try {
      sheet = await api.addStudioSheet(token, files[0], files[0].name);
      studio = await api.getStudio(token);
      await propose();
    } catch (e) {
      flash(String(e), 8000);
    } finally {
      busy = false;
    }
  }

  /** Предложить разрез. Ничего не пишет: настройки крутят, пока не сойдётся. */
  async function propose() {
    if (!token || !sheet) return;
    busy = true;
    try {
      cut = await api.sliceStudioSheet(token, sheet.id, settings);
      settings = { ...cut.settings };
      picked = {};
    } catch (e) {
      flash(String(e), 8000);
    } finally {
      busy = false;
    }
  }

  function togglePick(index: number, role: string) {
    if (picked[index]) {
      const next = { ...picked };
      delete next[index];
      picked = next;
    } else {
      picked = { ...picked, [index]: { name: '', role } };
    }
  }

  /** Сохранить отобранное. Каждый отбор несёт размер, при котором на него
   *  смотрели: если настройки успели поменяться, сервер откажет, а не сохранит
   *  другой кусок под именем выбранного. */
  async function keepCut() {
    if (!token || !sheet || !cut) return;
    const picks: BattleAssetPick[] = Object.entries(picked).map(([index, one]) => {
      const part = cut!.parts.find((p) => p.index === Number(index))!;
      return {
        index: Number(index),
        name: one.name || undefined,
        role: one.role as BattleAssetPick['role'],
        width: part.width,
        height: part.height,
        rects: [],
      };
    });
    if (!picks.length) return;
    busy = true;
    try {
      const saved = await api.keepStudioCut(token, sheet.id, settings!, picks);
      studio = await api.getStudio(token);
      picked = {};
      flash($t('studioCutKept').replace('{n}', String(saved.length)));
    } catch (e) {
      flash(String(e), 8000);
    } finally {
      busy = false;
    }
  }

  /** Кусок в полный рост — обводить по мелкому превью нельзя. */
  async function openBoard(index: number) {
    if (!token || !sheet) return;
    busy = true;
    try {
      const full = await api.studioSheetPart(token, sheet.id, settings!, index);
      board = { index, image: full.image, width: full.width, height: full.height };
    } catch (e) {
      flash(String(e), 8000);
    } finally {
      busy = false;
    }
  }

  function mb(bytes: number): string {
    if (bytes < 1024) return `${bytes} ${$t('studioBytes')}`;
    if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} ${$t('studioKb')}`;
    return `${(bytes / 1024 / 1024).toFixed(1)} ${$t('studioMb')}`;
  }
</script>

<svelte:head>
  <title>{$t('studioStore')}</title>
  <meta name="description" content={$t('studioStoreLead')} />
</svelte:head>

<div class="studio-room">
  <div class="page">
  <p class="eyebrow">
    <a href="/studio">{$t('studioBack')}</a>
    <span class="eyebrow-rule"></span>
    <span>{$t('studioEyebrow')}</span>
  </p>
  <h1 class="room-title">{$t('studioStore')}</h1>
  <p class="room-lead">{$t('studioStoreLead')}</p>

  {#if said}<p class="said">{said}</p>{/if}

  {#if loading}
    <p class="empty">{$t('studioLoading')}</p>
  {:else}
    <!-- ── Лист и разрез ─────────────────────────────────────────────────── -->
    <section class="mt-8 border border-[#d8c6b1] bg-[#fdf9f3] p-4">
      <h2 class="shelf-title">{$t('studioSheets')}</h2>
      <p class="mt-1 max-w-2xl text-xs leading-relaxed text-[#8a6a55]">{$t('studioSheetsLead')}</p>

      <label class="btn" style="margin-top:1rem;cursor:pointer">
        {$t('studioBringSheet')}
        <input
          type="file"
          accept="image/*"
          class="hidden"
          onchange={(e) => bringSheet(e.currentTarget.files)}
        />
      </label>
      {#if busy}<span class="ml-3 text-xs text-[#8a6a55]">{$t('studioWorking')}</span>{/if}

      {#if sheet && cut}
        <div class="mt-4 flex flex-wrap items-end gap-3 border-t border-[#d8c6b1] pt-3">
          {#if settings}
            <label class="field">
              {$t('studioSliceThreshold')}
              <input
                type="number"
                bind:value={settings.bgValue}
                min="0"
                max="1"
                step="0.05"
                class="ml-2 w-16 border border-[#34251c]/15 bg-transparent px-1 py-0.5 text-xs"
              />
            </label>
            <label class="field">
              {$t('studioSliceMinSide')}
              <input
                type="number"
                bind:value={settings.minArea}
                min="1"
                class="ml-2 w-20 border border-[#34251c]/15 bg-transparent px-1 py-0.5 text-xs"
              />
            </label>
            <label class="field" style="display:flex;align-items:center;gap:.4rem">
              <input type="checkbox" bind:checked={settings.keepText} class="accent-[#c65f3c]" />
              {$t('studioSliceKeepText')}
            </label>
          {/if}
          <button
            onclick={propose}
            disabled={busy}
            class="btn"
            >{$t('studioSliceAgain')}</button
          >
          <span class="text-[11px] text-[#8a6a55]"
            >{$t('studioSliceFound').replace('{n}', String(cut.parts.length))}</span
          >
          <button
            onclick={keepCut}
            disabled={busy || !Object.keys(picked).length}
            class="btn btn--lit" style="margin-left:auto"
            >{$t('studioKeepPicked').replace('{n}', String(Object.keys(picked).length))}</button
          >
        </div>

        <div class="mt-3 grid grid-cols-4 gap-2 sm:grid-cols-8">
          {#each cut.parts as part (part.index)}
            <div
              class="border p-1 {picked[part.index]
                ? 'border-[#c65f3c] bg-[#c65f3c]/5'
                : 'border-[#34251c]/12'}"
            >
              <button
                onclick={() => togglePick(part.index, part.role)}
                class="block aspect-square w-full"
                title={`${part.width}×${part.height}`}
              >
                <img src={part.preview} alt="" class="h-full w-full object-contain" />
              </button>
              {#if picked[part.index]}
                <select
                  value={picked[part.index].role}
                  onchange={(e) =>
                    (picked = {
                      ...picked,
                      [part.index]: { ...picked[part.index], role: e.currentTarget.value },
                    })}
                  class="mt-1 w-full border border-[#34251c]/15 bg-transparent text-[10px]"
                >
                  {#each ROLES as r (r)}
                    <option value={r}>{$t(`studioRole_${r}` as never)}</option>
                  {/each}
                </select>
              {/if}
              <button
                onclick={() => openBoard(part.index)}
                class="quiet" style="margin-top:.25rem;width:100%"
                >{$t('studioSplit')}</button
              >
            </div>
          {/each}
        </div>
      {/if}
    </section>

    <!-- ── Верстак: свои картинки в этом браузере ────────────────────────── -->
    <section class="mt-8">
      <h2 class="shelf-title">{$t('studioMyPieces')}</h2>
      <p class="mt-1 text-xs text-[#8a6a55]">{$t('studioMyPiecesLead')}</p>
      <label class="btn" style="margin-top:1rem;cursor:pointer">
        {$t('studioBringPieces')}
        <input
          type="file"
          accept="image/*"
          multiple
          class="hidden"
          onchange={(e) => bringLocal(e.currentTarget.files)}
        />
      </label>

      {#if mine.length}
        <div class="mt-3 grid grid-cols-3 gap-3 sm:grid-cols-6">
          {#each mine as one (one.key)}
            <div class="border border-[#34251c]/12 p-1">
              <div class="flex aspect-square items-center justify-center bg-[#fdf9f3]">
                {#if shown[one.key]}
                  <img src={shown[one.key]} alt={one.name} class="max-h-full max-w-full object-contain" />
                {/if}
              </div>
              <p class="mt-1 truncate text-[10px] text-[#6f3b24]" title={one.name}>{one.name}</p>
              <p class="text-[9px] text-[#b0a08e]">{mb(one.bytes)}</p>
              <div class="mt-1 flex justify-between">
                <button
                  onclick={() => toBox(one)}
                  disabled={busy}
                  class="text-[9px] uppercase tracking-[0.14em] text-[#8a6a55] hover:text-[#c65f3c] disabled:opacity-40"
                  >{$t('studioToBox')}</button
                >
                <button
                  onclick={() => (dropping = { where: 'bench', one })}
                  class="text-[9px] uppercase tracking-[0.14em] text-[#8f2f22]/70 hover:text-[#8f2f22]"
                  >{$t('studioDrop')}</button
                >
              </div>
            </div>
          {/each}
        </div>
      {:else}
        <p class="empty">{$t('studioPickEmpty')}</p>
      {/if}
    </section>

    <!-- ── Ящик: то, что отдано дому ─────────────────────────────────────── -->
    <section class="mt-8">
      <div class="flex flex-wrap items-baseline justify-between gap-2">
        <h2 class="shelf-title">{$t('studioBox')}</h2>
        {#if studio}
          <span class="text-[11px] text-[#8a6a55]"
            >{$t('studioBoxUsed')
              .replace('{used}', mb(studio.box.used))
              .replace('{limit}', mb(studio.box.limit))}</span
          >
        {/if}
      </div>
      {#if studio?.box.assets.length}
        <div class="mt-3 grid grid-cols-3 gap-3 sm:grid-cols-6">
          {#each studio.box.assets as one (one.id)}
            <div class="border border-[#34251c]/12 p-1">
              <div class="flex aspect-square items-center justify-center bg-[#fdf9f3]">
                <img src={one.url} alt={one.name} class="max-h-full max-w-full object-contain" />
              </div>
              <input
                value={one.name}
                onblur={(e) => renameBox(one, e.currentTarget.value, one.role)}
                maxlength="80"
                class="mt-1 w-full border border-transparent bg-transparent text-[10px] outline-none hover:border-[#34251c]/15 focus:border-[#34251c]/35"
              />
              <select
                value={one.role}
                onchange={(e) => renameBox(one, one.name, e.currentTarget.value)}
                class="mt-1 w-full border border-[#34251c]/15 bg-transparent text-[10px]"
              >
                {#each ROLES as r (r)}
                  <option value={r}>{$t(`studioRole_${r}` as never)}</option>
                {/each}
              </select>
              <div class="mt-1 flex justify-between">
                <span class="text-[9px] text-[#b0a08e]">{mb(one.bytes)}</span>
                <button
                  onclick={() => (dropping = { where: 'box', one })}
                  class="text-[9px] uppercase tracking-[0.14em] text-[#8f2f22]/70 hover:text-[#8f2f22]"
                  >{$t('studioDrop')}</button
                >
              </div>
            </div>
          {/each}
        </div>
      {:else}
        <p class="empty">{$t('studioBoxEmpty')}</p>
      {/if}
    </section>
  {/if}
  </div>
</div>

<!-- Разделочная доска: там, где автоматика бессильна по существу — два уголка
     нарисованы соприкасающимися, и никакой порог их не разведёт. -->
{#if board}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-[#34251c]/50 p-4">
    <div class="max-h-[90vh] w-full max-w-3xl overflow-y-auto border border-[#d8c6b1] bg-[#f8f1e7] p-4">
      {#key board.index}
        <BattleSplitBoard
          image={board.image}
          title={`${board.index}`}
          width={board.width}
          height={board.height}
          initial={[]}
          {busy}
          onClose={() => (board = null)}
          onDone={async (rects) => {
            if (!token || !sheet || !board) return;
            const part = cut?.parts.find((p) => p.index === board!.index);
            if (!part) return;
            busy = true;
            try {
              await api.keepStudioCut(token, sheet.id, settings!, [
                {
                  index: board.index,
                  role: part.role as BattleAssetPick['role'],
                  width: part.width,
                  height: part.height,
                  rects,
                },
              ]);
              studio = await api.getStudio(token);
              board = null;
              flash($t('studioCutKept').replace('{n}', String(rects.length + 1)));
            } catch (e) {
              flash(String(e), 8000);
            } finally {
              busy = false;
            }
          }}
        />
      {/key}
    </div>
  </div>
{/if}

{#if dropping}
  <StudioAsk
    title={$t('studioDrop')}
    lead={$t('studioDropSure').replace('{name}', dropping.one.name)}
    yes={$t('studioDrop')}
    danger
    onyes={() => {
      const go = dropping;
      dropping = null;
      if (go?.where === 'bench') void dropLocal(go.one);
      else if (go) void dropBox(go.one);
    }}
    onclose={() => (dropping = null)}
  />
{/if}
