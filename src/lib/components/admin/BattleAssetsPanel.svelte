<script lang="ts">
  // Склад деталей рамки и листы, с которых они срезаны.
  //
  // Две картины в одной вкладке. Обычная — полка: слева листы, справа детали,
  // с фильтром по роли и поиском по имени. Вторая включается только на время
  // разреза: предложение сервера, где видно ВСЁ найденное, включая отброшенные
  // подписи, — потому что проверить разрез можно только увидев его целиком.
  //
  // Предложение ничего не пишет на диск: превью приходят как data:-картинки,
  // и лист можно перечитывать сколько угодно, не оставляя за собой файлов.
  import { api } from '$lib/api';
  import { t } from '$lib/i18n';
  import BattleSplitBoard from '$lib/components/admin/BattleSplitBoard.svelte';
  import type {
    BattleAsset,
    BattleAssetPick,
    BattleAssetRole,
    BattleAssetSheet,
    BattleSheetCut,
    BattleSliceSettings,
    BattleSplitRect,
  } from '$lib/types/api';

  let { flash } = $props<{ flash: (message: string, ms?: number) => void }>();

  const ROLES: BattleAssetRole[] = ['corner', 'sideH', 'sideV', 'accent', 'art', 'motion', 'other'];
  const ROLE_LABEL: Record<BattleAssetRole, string> = {
    corner: 'adminAssetsRoleCorner',
    sideH: 'adminAssetsRoleSideH',
    sideV: 'adminAssetsRoleSideV',
    accent: 'adminAssetsRoleAccent',
    art: 'adminAssetsRoleArt',
    motion: 'adminAssetsRoleMotion',
    other: 'adminAssetsRoleOther',
  };
  // Придержано на удар, как и на полке карт: три перетащенные подряд детали —
  // это один порядок, а не три.
  const REORDER_MS = 600;

  let sheets = $state<BattleAssetSheet[]>([]);
  let assets = $state<BattleAsset[]>([]);
  /** 'all' · 'loose' · id листа. */
  let scope = $state<string>('all');
  let role = $state<BattleAssetRole | ''>('');
  let query = $state('');
  let loading = $state(false);
  let busy = $state(false);

  // ── Разрез ───────────────────────────────────────────────────────────────
  let cutSheet = $state<BattleAssetSheet | null>(null);
  let cut = $state<BattleSheetCut | null>(null);
  let knobs = $state<BattleSliceSettings | null>(null);
  let knobsOpen = $state(false);
  // Подписи показаны по умолчанию: проверить разрез можно только увидев всё
  // найденное. Но на листе, где буквы дают полсотни плиток из восьмидесяти,
  // до настоящих деталей приходится долго листать — поэтому их можно убрать,
  // и убранное всегда сосчитано рядом, чтобы это не было тихой пропажей.
  let showCaptions = $state(true);
  /** Номер детали → имя, роль и нарисованные на ней рамки. */
  let picks = $state<
    Record<number, { name: string; role: BattleAssetRole; rects?: BattleSplitRect[] }>
  >({});

  // ── Разделочная доска ────────────────────────────────────────────────────
  //
  // Открывается и над плиткой разбора, и над готовой деталью склада. Разница
  // только в том, куда деваются рамки: у первой они уедут вместе с отбором,
  // у второй режутся сразу.
  let board = $state<{
    kind: 'pick' | 'asset';
    key: number | string;
    image: string;
    title: string;
    width: number;
    height: number;
    initial: BattleSplitRect[];
  } | null>(null);

  let openSheet = $derived(sheets.find((s) => s.id === scope) ?? null);
  let chosenCount = $derived(Object.keys(picks).length);
  let captionCount = $derived(cut?.parts.filter((p) => p.isText && !picks[p.index]).length ?? 0);
  let shownParts = $derived(
    cut ? cut.parts.filter((p) => showCaptions || !p.isText || picks[p.index]) : [],
  );
  // Порядок таскается только тогда, когда список показан целиком: перетащить
  // деталь внутри отфильтрованного списка — значит записать порядок, которого
  // хранитель не видел.
  let orderable = $derived(!role && !query.trim() && scope !== 'all');

  async function load() {
    loading = true;
    try {
      sheets = await api.adminListBattleAssetSheets();
      await loadAssets();
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      loading = false;
    }
  }

  async function loadAssets() {
    assets = await api.adminListBattleAssets({
      sheetId: scope === 'all' ? undefined : scope,
      role: role || undefined,
      q: query,
    });
  }

  async function reload() {
    try {
      await loadAssets();
    } catch (e) {
      flash(String(e), 6000);
    }
  }

  function pickScope(next: string) {
    scope = next;
    reload();
  }

  // ── Лист ─────────────────────────────────────────────────────────────────

  async function addSheet(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = '';
    if (!file) return;
    busy = true;
    try {
      const sheet = await api.adminAddBattleAssetSheet(file);
      sheets = [sheet, ...sheets];
      scope = sheet.id;
      await loadAssets();
      // Свежий лист незачем разглядывать — его затем и принесли, чтобы резать.
      await beginCut(sheet);
    } catch (e) {
      flash(String(e), 8000);
    } finally {
      busy = false;
    }
  }

  async function renameSheet(sheet: BattleAssetSheet) {
    const name = prompt($t('adminAssetsNewName'), sheet.name);
    if (name == null || !name.trim()) return;
    try {
      const saved = await api.adminRenameBattleAssetSheet(sheet.id, name.trim());
      sheets = sheets.map((s) => (s.id === saved.id ? saved : s));
      flash($t('adminAssetsSaved'));
    } catch (e) {
      flash(String(e), 6000);
    }
  }

  async function removeSheet(sheet: BattleAssetSheet) {
    if (!confirm($t('adminAssetsDeleteSheetConfirm'))) return;
    try {
      await api.adminDeleteBattleAssetSheet(sheet.id);
      sheets = sheets.filter((s) => s.id !== sheet.id);
      if (scope === sheet.id) scope = 'loose';
      await loadAssets();
      flash($t('adminAssetsDeleted'));
    } catch (e) {
      flash(String(e), 6000);
    }
  }

  // ── Россыпь ──────────────────────────────────────────────────────────────

  async function addLoosePart(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const files = [...(input.files ?? [])];
    input.value = '';
    if (!files.length) return;
    busy = true;
    try {
      for (const file of files) {
        const stem = file.name.replace(/\.[^.]+$/, '');
        await api.adminAddBattleAsset(file, stem);
      }
      scope = 'loose';
      await loadAssets();
      flash($t('adminAssetsSaved'));
    } catch (e) {
      flash(String(e), 8000);
    } finally {
      busy = false;
    }
  }

  // ── Деталь ───────────────────────────────────────────────────────────────

  async function savePart(asset: BattleAsset, patch: { name?: string; role?: BattleAssetRole }) {
    try {
      const saved = await api.adminSaveBattleAsset(asset.id, patch);
      assets = assets.map((a) => (a.id === saved.id ? saved : a));
    } catch (e) {
      flash(String(e), 6000);
      await reload();
    }
  }

  function renamePart(asset: BattleAsset, event: Event) {
    const value = (event.currentTarget as HTMLInputElement).value.trim();
    if (!value || value === asset.name) return;
    savePart(asset, { name: value });
  }

  async function removePart(asset: BattleAsset) {
    if (!confirm($t('adminAssetsDeletePartConfirm'))) return;
    try {
      await api.adminDeleteBattleAsset(asset.id);
      assets = assets.filter((a) => a.id !== asset.id);
      sheets = sheets.map((s) =>
        s.id === asset.sheetId ? { ...s, partCount: Math.max(0, s.partCount - 1) } : s,
      );
      flash($t('adminAssetsDeleted'));
    } catch (e) {
      flash(String(e), 6000);
    }
  }

  let dragFrom = $state<number | null>(null);
  let dragOver = $state<number | null>(null);
  let orderTimer: ReturnType<typeof setTimeout> | null = null;

  function onDrop(to: number) {
    const from = dragFrom;
    dragFrom = null;
    dragOver = null;
    if (from == null || from === to) return;
    const next = [...assets];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    assets = next;
    if (orderTimer) clearTimeout(orderTimer);
    orderTimer = setTimeout(async () => {
      try {
        await api.adminReorderBattleAssets(assets.map((a) => a.id));
        flash($t('adminAssetsReordered'));
      } catch (e) {
        flash(String(e), 6000);
        await reload();
      }
    }, REORDER_MS);
  }

  // ── Разрез ───────────────────────────────────────────────────────────────

  async function beginCut(sheet: BattleAssetSheet) {
    cutSheet = sheet;
    cut = null;
    // Настройки прошлого разреза, если лист уже резали: ручки стоят там, где
    // их оставили. Иначе — пусто, и сервер подставит замеренные умолчания.
    let remembered: Partial<BattleSliceSettings> = {};
    if (sheet.settings) {
      try {
        remembered = JSON.parse(sheet.settings);
      } catch {
        remembered = {};
      }
    }
    await runCut(remembered);
  }

  async function runCut(settings: Partial<BattleSliceSettings>) {
    if (!cutSheet) return;
    busy = true;
    try {
      const result = await api.adminSliceBattleAssetSheet(cutSheet.id, settings);
      cut = result;
      knobs = { ...result.settings };
      // Отмечено всё, что не подпись. Хранитель снимает лишнее, а не
      // расставляет тридцать галочек.
      const next: Record<number, { name: string; role: BattleAssetRole }> = {};
      for (const part of result.parts) {
        if (part.isText) continue;
        next[part.index] = { name: String(part.index).padStart(2, '0'), role: part.role };
      }
      picks = next;
    } catch (e) {
      flash(String(e), 8000);
      cut = null;
    } finally {
      busy = false;
    }
  }

  function togglePick(index: number, roleGuess: BattleAssetRole) {
    if (picks[index]) {
      const next = { ...picks };
      delete next[index];
      picks = next;
    } else {
      picks = {
        ...picks,
        [index]: { name: String(index).padStart(2, '0'), role: roleGuess },
      };
    }
  }

  function pickAll(all: boolean) {
    if (!cut) return;
    if (!all) {
      picks = {};
      return;
    }
    const next: Record<number, { name: string; role: BattleAssetRole }> = {};
    for (const part of cut.parts) {
      next[part.index] = picks[part.index] ?? {
        name: String(part.index).padStart(2, '0'),
        role: part.role,
      };
    }
    picks = next;
  }

  async function saveChosen() {
    if (!cutSheet || !cut || !knobs) return;
    const chosen: BattleAssetPick[] = cut.parts
      .filter((part) => picks[part.index])
      .map((part) => ({
        index: part.index,
        name: picks[part.index].name,
        role: picks[part.index].role,
        rects: picks[part.index].rects ?? [],
        // Форма, при которой хранитель на неё смотрел: номер значит одно и то
        // же только при тех же настройках, и сервер это проверит.
        width: part.width,
        height: part.height,
      }));
    if (!chosen.length) return;
    busy = true;
    try {
      const saved = await api.adminCutBattleAssetSheet(cutSheet.id, knobs, chosen);
      flash(`${$t('adminAssetsSaved')}: ${saved.length}`);
      sheets = await api.adminListBattleAssetSheets();
      scope = cutSheet.id;
      cutSheet = null;
      cut = null;
      await loadAssets();
    } catch (e) {
      flash(String(e), 10000);
    } finally {
      busy = false;
    }
  }

  /**
   * Кусок разбора — крупно. Превью в сетке нарочно мелкое (их восемьдесят в
   * одном ответе), а обводить по нему нельзя, поэтому картинка берётся
   * отдельно и только для того куска, над которым сейчас работают.
   */
  async function openBoardOnPart(index: number) {
    if (!cutSheet || !knobs) return;
    busy = true;
    try {
      const full = await api.adminBattleSheetPart(cutSheet.id, knobs, index);
      board = {
        kind: 'pick',
        key: index,
        image: full.image,
        title: picks[index]?.name ?? String(index),
        width: full.width,
        height: full.height,
        initial: picks[index]?.rects ?? [],
      };
    } catch (e) {
      flash(String(e), 8000);
    } finally {
      busy = false;
    }
  }

  /** Готовая деталь уже лежит файлом — её и показываем, без запроса. */
  function openBoardOnAsset(asset: BattleAsset) {
    board = {
      kind: 'asset',
      key: asset.id,
      image: asset.url,
      title: asset.name,
      width: asset.width,
      height: asset.height,
      initial: [],
    };
  }

  async function boardDone(rects: BattleSplitRect[]) {
    if (!board) return;
    if (board.kind === 'pick') {
      // Ничего не режем: рамки поедут на сервер вместе с отбором, когда
      // хранитель сохранит разбор. До тех пор их можно править и стирать.
      const index = board.key as number;
      picks = { ...picks, [index]: { ...picks[index], rects } };
      board = null;
      return;
    }
    busy = true;
    try {
      const saved = await api.adminSplitBattleAsset(board.key as string, rects);
      flash(`${$t('adminAssetsSaved')}: ${saved.length}`);
      board = null;
      sheets = await api.adminListBattleAssetSheets();
      await loadAssets();
    } catch (e) {
      flash(String(e), 10000);
    } finally {
      busy = false;
    }
  }

  function closeCut() {
    cutSheet = null;
    cut = null;
  }

  function sizeOf(w: number, h: number) {
    return `${w}×${h}`;
  }

  $effect(() => {
    load();
  });
</script>

{#if cutSheet}
  <!-- ── Предложение разреза ───────────────────────────────────────────── -->
  <div class="flex-1 overflow-y-auto p-6 min-w-0">
    <div class="flex flex-wrap items-baseline gap-4 mb-4">
      <button
        onclick={closeCut}
        class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] hover:text-[#c65f3c]"
      >{$t('adminAssetsBack')}</button>
      <h3 class="text-lg" style="font-family: 'Cormorant Garamond', Georgia, serif;">
        {cutSheet.name}
      </h3>
      {#if cut}
        <span class="text-[11px] text-[#8a6a55]">
          {sizeOf(cut.width, cut.height)} ·
          {cut.source === 'alpha'
            ? $t('adminAssetsSourceAlpha')
            : $t('adminAssetsSourceBackground')}
        </span>
      {/if}
    </div>

    {#if knobs}
      <div class="max-w-4xl mb-5 border border-[#34251c]/12">
        <button
          onclick={() => (knobsOpen = !knobsOpen)}
          class="w-full flex items-center justify-between px-3 py-2 text-[10px] uppercase tracking-[0.16em] text-[#5f4636]"
        >
          <span>{$t('adminAssetsSettings')}</span>
          <span>{knobsOpen ? '−' : '+'}</span>
        </button>
        {#if knobsOpen}
          <div class="px-3 pb-4 border-t border-[#34251c]/10">
            <details class="my-3">
              <summary
                class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
                >{$t('adminBattlesHintOpen')}</summary
              >
              <p class="max-w-[62ch] mt-2 text-xs leading-relaxed text-[#5f4636]">
                {$t('adminAssetsSettingsHint')}
              </p>
            </details>
            <div class="flex flex-wrap gap-x-6 gap-y-3">
              <label class="block w-40">
                <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">
                  {$t('adminAssetsAlphaThreshold')}
                </span>
                <input
                  type="number"
                  min="0"
                  max="255"
                  bind:value={knobs.alphaThreshold}
                  class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                />
              </label>
              <label class="block w-40">
                <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">
                  {$t('adminAssetsBgValue')}
                </span>
                <input
                  type="number"
                  min="0"
                  max="1"
                  step="0.01"
                  bind:value={knobs.bgValue}
                  class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                />
              </label>
              <label class="block w-40">
                <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">
                  {$t('adminAssetsBgSat')}
                </span>
                <input
                  type="number"
                  min="0"
                  max="1"
                  step="0.01"
                  bind:value={knobs.bgSat}
                  class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                />
              </label>
              <label class="block w-40">
                <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">
                  {$t('adminAssetsMergeGap')}
                </span>
                <input
                  type="number"
                  min="0"
                  max="40"
                  bind:value={knobs.mergeGap}
                  class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                />
              </label>
              <label class="block w-40">
                <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">
                  {$t('adminAssetsMinArea')}
                </span>
                <input
                  type="number"
                  min="0"
                  bind:value={knobs.minArea}
                  class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                />
              </label>
              <label class="block w-40">
                <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">
                  {$t('adminAssetsTextMaxH')}
                </span>
                <input
                  type="number"
                  min="0"
                  bind:value={knobs.textMaxH}
                  class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                />
              </label>
              <label class="block w-40">
                <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">
                  {$t('adminAssetsTextColor')}
                </span>
                <input
                  type="number"
                  min="0"
                  max="1"
                  step="0.01"
                  bind:value={knobs.textColor}
                  class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                />
              </label>
              <label class="block w-40">
                <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">
                  {$t('adminAssetsPad')}
                </span>
                <input
                  type="number"
                  min="0"
                  max="40"
                  bind:value={knobs.pad}
                  class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                />
              </label>
              <label class="block w-40">
                <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">
                  {$t('adminAssetsFeather')}
                </span>
                <input
                  type="number"
                  min="0"
                  max="8"
                  bind:value={knobs.feather}
                  class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                />
              </label>
              <label class="block w-40">
                <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">
                  {$t('adminAssetsBleed')}
                </span>
                <input
                  type="number"
                  min="0"
                  max="12"
                  bind:value={knobs.bleed}
                  class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                />
              </label>
              <label class="flex items-center gap-2 self-end pb-1.5 text-xs text-[#5f4636]">
                <input type="checkbox" bind:checked={knobs.keepText} />
                {$t('adminAssetsKeepText')}
              </label>
            </div>
            <ul class="max-w-[62ch] mt-4 space-y-1.5 text-[11px] leading-relaxed text-[#8a6a55]">
              <li>{$t('adminAssetsAlphaThresholdHint')}</li>
              <li>{$t('adminAssetsBgHint')}</li>
              <li>{$t('adminAssetsMergeGapHint')}</li>
              <li>{$t('adminAssetsTextHint')}</li>
              <li>{$t('adminAssetsBleedHint')}</li>
            </ul>
            <button
              onclick={() => knobs && runCut(knobs)}
              disabled={busy}
              class="mt-4 px-4 py-2 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 disabled:opacity-40"
            >{busy ? $t('adminAssetsCutting') : $t('adminAssetsRecut')}</button>
          </div>
        {/if}
      </div>
    {/if}

    {#if cut}
      <div class="flex flex-wrap items-center gap-4 mb-3 text-[11px] text-[#8a6a55]">
        <span>{$t('adminAssetsFound')}: <b class="tabular-nums">{cut.parts.length}</b></span>
        <span>{$t('adminAssetsChosen')}: <b class="tabular-nums">{chosenCount}</b></span>
        <button onclick={() => pickAll(true)} class="hover:text-[#c65f3c]">
          {$t('adminAssetsSelectAll')}
        </button>
        <button onclick={() => pickAll(false)} class="hover:text-[#c65f3c]">
          {$t('adminAssetsSelectNone')}
        </button>
        <label class="flex items-center gap-1.5">
          <input type="checkbox" bind:checked={showCaptions} />
          {$t('adminAssetsShowCaptions')}
          <span class="tabular-nums">({captionCount})</span>
        </label>
        <button
          onclick={saveChosen}
          disabled={busy || !chosenCount}
          class="ml-auto px-4 py-2 text-[10px] uppercase tracking-[0.16em] bg-[#34251c] text-[#f8f1e7] disabled:opacity-40"
        >{$t('adminAssetsSaveChosen')}</button>
      </div>
      <details class="mb-5">
        <summary
          class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
          >{$t('adminBattlesHintOpen')}</summary
        >
        <p class="max-w-[62ch] mt-2 text-[11px] leading-relaxed text-[#8a6a55]">
          {$t('adminAssetsCaptionsHint')}
        </p>
      </details>

      <div class="grid gap-3" style="grid-template-columns: repeat(auto-fill, minmax(11rem, 1fr));">
        {#each shownParts as part (part.index)}
          <div
            class="border {picks[part.index]
              ? 'border-[#c65f3c]/60'
              : 'border-[#34251c]/12'} {part.isText && !picks[part.index] ? 'opacity-55' : ''}"
          >
            <button
              onclick={() => togglePick(part.index, part.role)}
              class="flex items-center justify-center w-full h-32 p-2 tile"
              title={sizeOf(part.width, part.height)}
            >
              <img
                src={part.preview}
                alt=""
                class="max-w-full max-h-full object-contain {part.isText && !picks[part.index]
                  ? 'line-through-tile'
                  : ''}"
              />
            </button>
            <div class="flex items-center gap-2 px-2 py-1 border-t border-[#34251c]/10">
              <input
                type="checkbox"
                checked={!!picks[part.index]}
                onchange={() => togglePick(part.index, part.role)}
              />
              <span class="text-[10px] tabular-nums text-[#8a6a55]">{part.index}</span>
              <span class="ml-auto text-[10px] tabular-nums text-[#8a6a55]">
                {sizeOf(part.width, part.height)}
              </span>
            </div>
            {#if picks[part.index]}
              <div class="px-2 pb-2 space-y-1">
                <input
                  bind:value={picks[part.index].name}
                  maxlength="80"
                  class="w-full px-1.5 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                />
                <select
                  bind:value={picks[part.index].role}
                  class="w-full px-1.5 py-1 text-[11px] bg-transparent border border-[#34251c]/15 outline-none"
                >
                  {#each ROLES as r (r)}
                    <option value={r}>{$t(ROLE_LABEL[r] as never)}</option>
                  {/each}
                </select>
                <button
                  onclick={() => openBoardOnPart(part.index)}
                  disabled={busy}
                  class="w-full px-1.5 py-1 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 disabled:opacity-40"
                >{$t('adminAssetsSplit')}</button>
                {#if picks[part.index].rects?.length}
                  <p class="text-[10px] leading-snug text-[#c65f3c]">
                    {picks[part.index].rects?.length}
                    {$t('adminAssetsSplitMarked')} · {$t('adminAssetsSplitPending')}
                  </p>
                {/if}
              </div>
            {:else if part.isText}
              <p class="px-2 pb-2 text-[10px] uppercase tracking-[0.16em] text-[#c65f3c]/70">
                {$t('adminAssetsCaption')}
              </p>
            {/if}
          </div>
        {/each}
      </div>
    {:else if busy}
      <p class="text-xs italic text-[#5f4636]">{$t('adminAssetsCutting')}</p>
    {/if}
  </div>
{:else}
  <!-- ── Полка ──────────────────────────────────────────────────────────── -->
  <div class="flex-1 flex min-h-0">
    <div class="w-64 shrink-0 overflow-y-auto border-r border-[#34251c]/10 p-4">
      <div class="flex gap-2 mb-4">
        <label
          class="flex-1 px-2 py-1.5 text-[10px] uppercase tracking-[0.16em] text-center border border-[#34251c]/20 cursor-pointer"
        >
          {busy ? $t('adminAssetsUploading') : $t('adminAssetsAddSheet')}
          <input type="file" accept="image/*" class="hidden" onchange={addSheet} disabled={busy} />
        </label>
        <label
          class="flex-1 px-2 py-1.5 text-[10px] uppercase tracking-[0.16em] text-center border border-[#34251c]/20 cursor-pointer"
        >
          {$t('adminAssetsAddPart')}
          <input
            type="file"
            accept="image/*"
            multiple
            class="hidden"
            onchange={addLoosePart}
            disabled={busy}
          />
        </label>
      </div>

      <button
        onclick={() => pickScope('all')}
        class="block w-full text-left py-1.5 text-xs {scope === 'all'
          ? 'text-[#c65f3c]'
          : 'hover:text-[#c65f3c]'}"
      >{$t('adminAssetsAllParts')}</button>
      <button
        onclick={() => pickScope('loose')}
        class="block w-full text-left py-1.5 text-xs {scope === 'loose'
          ? 'text-[#c65f3c]'
          : 'hover:text-[#c65f3c]'}"
      >{$t('adminAssetsLoose')}</button>

      <p class="mt-5 mb-2 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">
        {$t('adminAssetsSheets')}
      </p>
      {#if !sheets.length}
        <p class="text-[11px] italic leading-relaxed text-[#8a6a55]">{$t('adminAssetsNoSheets')}</p>
      {:else}
        <ul class="border-t border-[#34251c]/10">
          {#each sheets as sheet (sheet.id)}
            <li class="border-b border-[#34251c]/10">
              <button
                onclick={() => pickScope(sheet.id)}
                class="block w-full text-left py-2 {scope === sheet.id
                  ? 'text-[#c65f3c]'
                  : 'hover:text-[#c65f3c]'}"
              >
                <span class="block text-xs leading-snug">{sheet.name}</span>
                <span class="block mt-0.5 text-[10px] tabular-nums text-[#8a6a55]">
                  {sheet.partCount}&nbsp;{$t('adminAssetsParts')} · {sizeOf(
                    sheet.width,
                    sheet.height,
                  )}
                </span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <div class="flex-1 overflow-y-auto p-6 min-w-0">
      {#if openSheet}
        <div class="flex flex-wrap items-start gap-4 mb-5">
          <img
            src={openSheet.sourceUrl}
            alt=""
            class="w-40 border border-[#34251c]/12 tile"
          />
          <div class="min-w-0">
            <h3 class="text-lg" style="font-family: 'Cormorant Garamond', Georgia, serif;">
              {openSheet.name}
            </h3>
            <p class="max-w-[52ch] mt-1 text-[11px] leading-relaxed text-[#8a6a55]">
              {$t('adminAssetsSheetIntro')}
            </p>
            <div class="flex flex-wrap gap-2 mt-3">
              <button
                onclick={() => openSheet && beginCut(openSheet)}
                disabled={busy}
                class="px-4 py-2 text-[10px] uppercase tracking-[0.16em] bg-[#34251c] text-[#f8f1e7] disabled:opacity-40"
              >{$t('adminAssetsCut')}</button>
              <button
                onclick={() => openSheet && renameSheet(openSheet)}
                class="px-4 py-2 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20"
              >{$t('adminAssetsRename')}</button>
              <button
                onclick={() => openSheet && removeSheet(openSheet)}
                class="px-4 py-2 text-[10px] uppercase tracking-[0.16em] border border-[#8f2f22]/30 text-[#8f2f22]"
              >{$t('adminBattlesDelete')}</button>
            </div>
          </div>
        </div>
      {:else if scope === 'loose'}
        <details class="mb-5">
          <summary
            class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
            >{$t('adminBattlesHintOpen')}</summary
          >
          <p class="max-w-[62ch] mt-2 text-xs leading-relaxed text-[#5f4636]">
            {$t('adminAssetsLooseHint')}
          </p>
        </details>
      {/if}

      <div class="flex flex-wrap items-center gap-3 mb-4">
        <select
          bind:value={role}
          onchange={reload}
          class="px-2 py-1.5 text-[11px] bg-transparent border border-[#34251c]/15 outline-none"
        >
          <option value="">{$t('adminAssetsAllRoles')}</option>
          {#each ROLES as r (r)}
            <option value={r}>{$t(ROLE_LABEL[r] as never)}</option>
          {/each}
        </select>
        <input
          bind:value={query}
          oninput={reload}
          placeholder={$t('adminAssetsSearch')}
          class="px-2 py-1.5 w-56 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
        />
        <span class="text-[11px] tabular-nums text-[#8a6a55]">{assets.length}</span>
      </div>

      {#if loading}
        <p class="text-xs italic text-[#5f4636]">…</p>
      {:else if !assets.length}
        <p class="text-xs italic text-[#5f4636]">{$t('adminAssetsEmpty')}</p>
      {:else}
        <div
          class="grid gap-3"
          style="grid-template-columns: repeat(auto-fill, minmax(11rem, 1fr));"
        >
          {#each assets as asset, i (asset.id)}
            <div
              draggable={orderable}
              ondragstart={() => (dragFrom = i)}
              ondragover={(e) => {
                e.preventDefault();
                dragOver = i;
              }}
              ondragleave={() => (dragOver = dragOver === i ? null : dragOver)}
              ondrop={(e) => {
                e.preventDefault();
                onDrop(i);
              }}
              class="border {dragOver === i ? 'border-[#c65f3c]' : 'border-[#34251c]/12'}"
              role="listitem"
            >
              <div class="flex items-center justify-center h-32 p-2 tile">
                <img src={asset.url} alt={asset.name} class="max-w-full max-h-full object-contain" />
              </div>
              <div class="px-2 py-1 border-t border-[#34251c]/10">
                <input
                  value={asset.name}
                  onblur={(e) => renamePart(asset, e)}
                  maxlength="80"
                  class="w-full px-1 py-0.5 text-xs bg-transparent border border-transparent outline-none hover:border-[#34251c]/15 focus:border-[#34251c]/35"
                />
                <div class="flex items-center gap-2 mt-1">
                  <select
                    value={asset.role}
                    onchange={(e) =>
                      savePart(asset, {
                        role: (e.currentTarget as HTMLSelectElement).value as BattleAssetRole,
                      })}
                    class="flex-1 min-w-0 px-1 py-0.5 text-[11px] bg-transparent border border-[#34251c]/15 outline-none"
                  >
                    {#each ROLES as r (r)}
                      <option value={r}>{$t(ROLE_LABEL[r] as never)}</option>
                    {/each}
                  </select>
                  <button
                    onclick={() => openBoardOnAsset(asset)}
                    title={$t('adminAssetsSplit')}
                    class="px-1 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] hover:text-[#c65f3c]"
                  >{$t('adminAssetsSplit')}</button>
                  <button
                    onclick={() => removePart(asset)}
                    title={$t('adminBattlesDelete')}
                    class="px-1 text-[#8f2f22]/70 hover:text-[#8f2f22]"
                  >×</button>
                </div>
                <p class="mt-1 text-[10px] tabular-nums text-[#8a6a55]">
                  {sizeOf(asset.width, asset.height)}
                  {#if scope === 'all'}
                    · {asset.sheetName ?? $t('adminAssetsNoSheet')}
                  {/if}
                </p>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>
{/if}

{#if board}
  <!-- Ключ здесь несущий: доска берёт `initial` один раз, при рождении. Без
       ключа переход к другому куску не пересоздал бы её, и на новой картинке
       остались бы рамки, нарисованные на прошлой. -->
  {#key board.key}
    <BattleSplitBoard
      image={board.image}
      title={board.title}
      width={board.width}
      height={board.height}
      initial={board.initial}
      {busy}
      onDone={boardDone}
      onClose={() => (board = null)}
    />
  {/key}
{/if}

<style>
  /* Детали прозрачны — на пергаменте их край не виден. Клетка показывает,
     где кончается рисунок, тем же приёмом, что и всякий редактор картинок. */
  .tile {
    background-image:
      linear-gradient(45deg, #e4d7c4 25%, transparent 25%),
      linear-gradient(-45deg, #e4d7c4 25%, transparent 25%),
      linear-gradient(45deg, transparent 75%, #e4d7c4 75%),
      linear-gradient(-45deg, transparent 75%, #e4d7c4 75%);
    background-size: 14px 14px;
    background-position:
      0 0,
      0 7px,
      7px -7px,
      -7px 0;
    background-color: #f2e8da;
  }

  .line-through-tile {
    opacity: 0.75;
  }
</style>
