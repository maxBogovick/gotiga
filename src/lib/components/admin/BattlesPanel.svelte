<script lang="ts">
  // The keeper's desk for the card shelf.
  //
  // Two things are edited here and they are deliberately kept apart. A CARD is
  // content: name, effect, corners, price, which work it stands for. A FRAME is
  // design: how a whole rank is dressed. Editing forty cards to change how rank
  // three looks is the mistake every card-authoring tool exists to prevent, so
  // the frames live in their own view and there are exactly five of them.
  //
  // The preview is not a drawing of a card. It is the same `BattleCard.svelte`
  // the shelf renders, fed from the fields being typed — a preview with its own
  // renderer is a preview that eventually lies.
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { t, lang } from '$lib/i18n';
  import { DEFAULT_FRAMES, TIERS, frameName } from '$lib/battles';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import type {
    BattleCard as BattleCardDto,
    BattleCardStatus,
    BattleFrame,
    FigurineListItem,
    SaveBattleCardRequest,
  } from '$lib/types/api';

  const REORDER_MS = 600;

  let view = $state<'cards' | 'frames'>('cards');
  let cards = $state<BattleCardDto[]>([]);
  let frames = $state<BattleFrame[]>([...DEFAULT_FRAMES]);
  let figurines = $state<FigurineListItem[]>([]);
  let loading = $state(true);
  let saving = $state(false);
  let message = $state('');
  let listQuery = $state('');

  // ── The card being written ───────────────────────────────────────────────
  let selectedId = $state<string | null>(null);
  let slug = $state('');
  let status = $state<BattleCardStatus>('draft');
  let tier = $state(1);
  let titleEn = $state('');
  let titleRu = $state('');
  let effectEn = $state('');
  let effectRu = $state('');
  let loreEn = $state('');
  let loreRu = $state('');
  let cost = $state(1);
  let power = $state(1);
  // Svelte coerces a `type="number"` binding to a number as soon as anything is
  // typed, and back to an empty string when the field is cleared — so these are
  // read through `coin()` below rather than as strings.
  let priceDust = $state<string | number>('');
  let priceFeed = $state<string | number>('');
  let figurineId = $state('');
  let artUrl = $state('');
  let focusX = $state(0.5);
  let focusY = $state(0.5);
  let zoom = $state(1);

  // The preview shows what a copy looks like, which is a different thing from
  // the card's rank — so the desk can set it, and it is off by default.
  let previewLevel = $state<number | null>(null);
  let previewDown = $state(false);

  let dragFrom = $state<number | null>(null);
  let dragOver = $state<number | null>(null);
  let reorderTimer: ReturnType<typeof setTimeout> | null = null;
  let flashTimer: ReturnType<typeof setTimeout> | null = null;

  let selectedFig = $derived(figurines.find((f) => f.id === figurineId) ?? null);

  /** An empty price field means "not for this coin", which is not zero. */
  function coin(raw: string | number): number | null {
    const text = String(raw).trim();
    if (text === '') return null;
    const n = Number(text);
    return Number.isFinite(n) ? n : null;
  }

  /** A cleared number field must not travel as "" and 422 the whole save. */
  function whole(raw: number | string, fallback: number): number {
    const n = Number(raw);
    return Number.isFinite(n) ? Math.round(n) : fallback;
  }

  let visible = $derived(
    listQuery.trim()
      ? cards.filter((c) =>
          `${c.titleRu} ${c.titleEn} ${c.figurineName ?? ''}`
            .toLowerCase()
            .includes(listQuery.trim().toLowerCase()),
        )
      : cards,
  );

  // The same choice the server makes in `battle_card_dto`: the keeper's own
  // picture if there is one, the work's face otherwise. Kept in step with it so
  // the preview and the shelf cannot disagree about what a card wears.
  let previewArt = $derived(
    artUrl.trim() ||
      selectedFig?.faceImageLargeUrl ||
      selectedFig?.faceImageUrl ||
      null,
  );

  let preview = $derived<BattleCardDto>({
    id: selectedId ?? 'preview',
    slug: slug || 'preview',
    status,
    tier,
    titleEn,
    titleRu,
    effectEn: effectEn || null,
    effectRu: effectRu || null,
    loreEn: loreEn || null,
    loreRu: loreRu || null,
    cost: whole(cost, 0),
    power: whole(power, 0),
    priceDust: coin(priceDust),
    priceFeed: coin(priceFeed),
    artUrl: previewArt,
    artFocal: JSON.stringify({ x: focusX, y: focusY, zoom }),
    figurineId: figurineId || null,
    figurineName: selectedFig?.name ?? null,
    figurineSlug: selectedFig?.slug ?? null,
    createdAt: '',
    updatedAt: '',
  });

  const STATUS_TONE: Record<BattleCardStatus, string> = {
    draft: 'bg-[#b9a68f]',
    published: 'bg-emerald-600',
    retired: 'bg-[#b0a08e]',
  };

  function flash(text: string, ms = 3000) {
    message = text;
    if (flashTimer) clearTimeout(flashTimer);
    flashTimer = setTimeout(() => (message = ''), ms);
  }

  function titleOf(card: BattleCardDto): string {
    return ($lang === 'ru' ? card.titleRu || card.titleEn : card.titleEn || card.titleRu) || '—';
  }

  async function loadCards() {
    cards = await api.adminListBattleCards();
  }

  function blank() {
    selectedId = null;
    slug = '';
    status = 'draft';
    tier = 1;
    titleEn = '';
    titleRu = '';
    effectEn = '';
    effectRu = '';
    loreEn = '';
    loreRu = '';
    cost = 1;
    power = 1;
    priceDust = '';
    priceFeed = '';
    figurineId = '';
    artUrl = '';
    focusX = 0.5;
    focusY = 0.5;
    zoom = 1;
  }

  function openCard(card: BattleCardDto) {
    selectedId = card.id;
    slug = card.slug;
    status = card.status;
    tier = card.tier;
    titleEn = card.titleEn;
    titleRu = card.titleRu;
    effectEn = card.effectEn ?? '';
    effectRu = card.effectRu ?? '';
    loreEn = card.loreEn ?? '';
    loreRu = card.loreRu ?? '';
    cost = card.cost;
    power = card.power;
    priceDust = card.priceDust == null ? '' : String(card.priceDust);
    priceFeed = card.priceFeed == null ? '' : String(card.priceFeed);
    figurineId = card.figurineId ?? '';
    artUrl = card.artUrlOverride ?? '';
    try {
      const f = JSON.parse(card.artFocal ?? '{}');
      focusX = typeof f.x === 'number' ? f.x : 0.5;
      focusY = typeof f.y === 'number' ? f.y : 0.5;
      zoom = typeof f.zoom === 'number' ? f.zoom : 1;
    } catch {
      focusX = 0.5;
      focusY = 0.5;
      zoom = 1;
    }
  }

  async function save() {
    saving = true;
    try {
      const body: SaveBattleCardRequest = {
        slug: slug.trim() || null,
        status,
        tier,
        titleEn: titleEn.trim(),
        titleRu: titleRu.trim(),
        effectEn: effectEn.trim() || null,
        effectRu: effectRu.trim() || null,
        loreEn: loreEn.trim() || null,
        loreRu: loreRu.trim() || null,
        cost: whole(cost, 0),
        power: whole(power, 0),
        priceDust: coin(priceDust),
        priceFeed: coin(priceFeed),
        artUrl: artUrl.trim() || null,
        artFocal: JSON.stringify({ x: focusX, y: focusY, zoom }),
        figurineId: figurineId || null,
      };
      const saved = await api.adminSaveBattleCard(body, selectedId ?? undefined);
      await loadCards();
      openCard(saved);
      flash($t('adminBattlesSaved'));
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      saving = false;
    }
  }

  async function remove() {
    if (!selectedId || !confirm($t('adminBattlesDeleteConfirm'))) return;
    try {
      await api.adminDeleteBattleCard(selectedId);
      await loadCards();
      blank();
      flash($t('adminBattlesDeleted'));
    } catch (e) {
      flash(String(e), 6000);
    }
  }

  function onDrop(to: number) {
    const from = dragFrom;
    dragFrom = null;
    dragOver = null;
    if (from == null || from === to) return;
    const next = [...cards];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    cards = next;
    // Held for a beat: dragging three cards in a row is one shelf, not three.
    if (reorderTimer) clearTimeout(reorderTimer);
    reorderTimer = setTimeout(async () => {
      try {
        await api.adminReorderBattleCards(cards.map((c) => c.id));
        flash($t('adminBattlesReordered'));
      } catch (e) {
        flash(String(e), 6000);
        await loadCards();
      }
    }, REORDER_MS);
  }

  async function saveFrames() {
    saving = true;
    try {
      const saved = await api.adminSaveBattleFrames({ frames });
      frames = saved.frames;
      flash($t('adminBattlesFramesSaved'));
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      saving = false;
    }
  }

  onMount(async () => {
    try {
      const [, figs, savedFrames] = await Promise.all([
        loadCards(),
        api.getAllFigurines(),
        api.getBattleFrames(),
      ]);
      figurines = figs;
      if (savedFrames.frames.length) frames = savedFrames.frames;
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      loading = false;
      blank();
    }
  });
</script>

<div class="h-full flex flex-col bg-[#f8f1e7] text-[#34251c]">
  <div class="flex items-center gap-3 px-4 py-2 border-b border-[#34251c]/10 text-[10px] uppercase tracking-[0.16em]">
    <div class="flex border border-[#34251c]/15">
      <button
        onclick={() => (view = 'cards')}
        class="px-3 py-1 {view === 'cards' ? 'bg-[#34251c] text-[#f8f1e7]' : ''}"
      >{$t('adminBattlesCardsView')}</button>
      <button
        onclick={() => (view = 'frames')}
        class="px-3 py-1 {view === 'frames' ? 'bg-[#34251c] text-[#f8f1e7]' : ''}"
      >{$t('adminBattlesFramesView')}</button>
    </div>
    {#if message}
      <span class="ml-auto normal-case tracking-normal text-[11px] text-[#6f3b24]">{message}</span>
    {/if}
  </div>

  {#if view === 'frames'}
    <!-- ── Five frames, one per rank ────────────────────────────────────── -->
    <div class="flex-1 overflow-y-auto p-6">
      <p class="max-w-[62ch] mb-6 text-xs leading-relaxed text-[#5f4636]">{$t('adminBattlesFramesHint')}</p>
      <div class="space-y-4">
        {#each frames as frame, i (frame.tier)}
          <div class="flex flex-wrap items-end gap-4 p-3 border border-[#34251c]/12">
            <span class="w-8 text-lg text-[#6f3b24]">{frame.tier}</span>
            <label class="block">
              <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesFrameName')} · EN</span>
              <input bind:value={frames[i].nameEn} class="px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35" />
            </label>
            <label class="block">
              <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesFrameName')} · RU</span>
              <input bind:value={frames[i].nameRu} class="px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35" />
            </label>
            {#each [['paper', $t('adminBattlesFramePaper')], ['ink', $t('adminBattlesFrameInk')], ['border', $t('adminBattlesFrameBorder')]] as [key, label] (key)}
              <label class="block">
                <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{label}</span>
                <input
                  type="color"
                  value={frames[i][key as 'paper' | 'ink' | 'border']}
                  oninput={(e) => (frames[i][key as 'paper' | 'ink' | 'border'] = e.currentTarget.value)}
                  class="w-12 h-7 bg-transparent border border-[#34251c]/15"
                />
              </label>
            {/each}
            <label class="block flex-1 min-w-[14rem]">
              <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">
                {$t('adminBattlesFrameFoil')}
                {#if !frame.foil.trim()}<span class="normal-case tracking-normal italic"> — {$t('adminBattlesFrameNoFoil')}</span>{/if}
              </span>
              <input bind:value={frames[i].foil} placeholder="rgba(198,95,60,0.28)" class="w-full px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35" />
            </label>
          </div>
        {/each}
      </div>
      <button
        onclick={saveFrames}
        disabled={saving}
        class="mt-6 px-4 py-2 text-[10px] uppercase tracking-[0.16em] bg-[#34251c] text-[#f8f1e7] disabled:opacity-40"
      >{$t('adminBattlesFramesSave')}</button>
    </div>
  {:else}
    <div class="flex-1 flex min-h-0">
      <!-- ── The shelf ──────────────────────────────────────────────────── -->
      <aside class="w-64 flex flex-col border-r border-[#34251c]/10">
        <div class="p-3 space-y-2 border-b border-[#34251c]/10">
          <button
            onclick={blank}
            class="w-full px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5"
          >{$t('adminBattlesNew')}</button>
          <input
            bind:value={listQuery}
            placeholder={$t('adminBattlesSearch')}
            class="w-full px-2 py-1.5 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
          />
        </div>
        <div class="flex-1 overflow-y-auto">
          {#if loading}
            <p class="p-3 text-xs text-[#5f4636]">…</p>
          {:else if visible.length === 0}
            <p class="p-3 text-xs italic text-[#5f4636]">{$t('adminBattlesEmpty')}</p>
          {:else}
            <p class="px-3 py-2 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesDragHint')}</p>
            <ul class="pb-4">
              {#each visible as card, i (card.id)}
                <li
                  draggable={!listQuery}
                  ondragstart={() => (dragFrom = i)}
                  ondragover={(e) => { e.preventDefault(); dragOver = i; }}
                  ondragleave={() => { if (dragOver === i) dragOver = null; }}
                  ondrop={(e) => { e.preventDefault(); onDrop(i); }}
                  ondragend={() => { dragFrom = null; dragOver = null; }}
                  class="border-b border-[#34251c]/5 {dragOver === i ? 'bg-[#c65f3c]/10' : ''} {dragFrom === i ? 'opacity-40' : ''}"
                >
                  <button
                    onclick={() => openCard(card)}
                    class="w-full text-left px-3 py-2.5 flex gap-2 items-start hover:bg-[#34251c]/[0.04] {selectedId === card.id ? 'bg-[#34251c]/[0.06]' : ''}"
                  >
                    <span class="mt-1.5 w-1.5 h-1.5 rounded-full flex-shrink-0 {STATUS_TONE[card.status]}"></span>
                    <span class="min-w-0">
                      <span class="block text-[13px] leading-snug truncate" style="font-family: 'Cormorant Garamond', Georgia, serif;">
                        {titleOf(card)}
                      </span>
                      {#if card.figurineName}
                        <span class="block text-[10px] text-[#8a6a55] truncate">{card.figurineName}</span>
                      {/if}
                    </span>
                    <span class="ml-auto text-[10px] text-[#8a6a55]">{card.tier}</span>
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      </aside>

      <!-- ── The card ───────────────────────────────────────────────────── -->
      <div class="flex-1 overflow-y-auto p-6 min-w-0">
        <div class="grid grid-cols-2 gap-4 max-w-3xl">
          <label class="block">
            <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesTitle')} · EN</span>
            <input bind:value={titleEn} maxlength="80" class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35" />
          </label>
          <label class="block">
            <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesTitle')} · RU</span>
            <input bind:value={titleRu} maxlength="80" class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35" />
          </label>

          <label class="block">
            <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesEffect')} · EN</span>
            <textarea bind:value={effectEn} maxlength="160" rows="2" class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35 resize-none"></textarea>
          </label>
          <label class="block">
            <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesEffect')} · RU</span>
            <textarea bind:value={effectRu} maxlength="160" rows="2" class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35 resize-none"></textarea>
          </label>

          <label class="block">
            <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesLore')} · EN</span>
            <textarea bind:value={loreEn} maxlength="400" rows="2" class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35 resize-none"></textarea>
          </label>
          <label class="block">
            <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesLore')} · RU</span>
            <textarea bind:value={loreRu} maxlength="400" rows="2" class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35 resize-none"></textarea>
          </label>
        </div>

        <div class="flex flex-wrap gap-4 mt-6 max-w-3xl">
          <label class="block">
            <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesTier')}</span>
            <select bind:value={tier} class="px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none">
              {#each TIERS as rank (rank)}
                <option value={rank}>{rank} — {frameName(frames[rank - 1] ?? DEFAULT_FRAMES[rank - 1], $lang)}</option>
              {/each}
            </select>
          </label>
          <label class="block w-24">
            <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesCost')}</span>
            <input type="number" min="0" max="20" bind:value={cost} class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none" />
          </label>
          <label class="block w-24">
            <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesPower')}</span>
            <input type="number" min="0" max="99" bind:value={power} class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none" />
          </label>
          <label class="block">
            <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesStatus')}</span>
            <select bind:value={status} class="px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none">
              <option value="draft">{$t('adminBattlesStatusDraft')}</option>
              <option value="published">{$t('adminBattlesStatusPublished')}</option>
              <option value="retired">{$t('adminBattlesStatusRetired')}</option>
            </select>
          </label>
        </div>

        <p class="mt-1 max-w-[62ch] text-[11px] leading-relaxed italic text-[#8a6a55]">{$t('adminBattlesTierHint')}</p>

        <div class="flex flex-wrap items-end gap-4 mt-6 max-w-3xl">
          <label class="block w-32">
            <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesPriceDust')}</span>
            <input type="number" min="0" bind:value={priceDust} class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none" />
          </label>
          <label class="block w-32">
            <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesPriceFeed')}</span>
            <input type="number" min="0" bind:value={priceFeed} class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none" />
          </label>
          <p class="flex-1 min-w-[18rem] text-[11px] leading-relaxed italic text-[#8a6a55]">{$t('adminBattlesPriceHint')}</p>
        </div>

        <div class="flex flex-wrap items-end gap-4 mt-6 max-w-3xl">
          <label class="block min-w-[16rem]">
            <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesWork')}</span>
            <select bind:value={figurineId} class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none">
              <option value="">{$t('adminBattlesWorkNone')}</option>
              {#each figurines as fig (fig.id)}
                <option value={fig.id}>{fig.name}</option>
              {/each}
            </select>
          </label>
          <label class="block flex-1 min-w-[16rem]">
            <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">
              {$t('adminBattlesArtOwn')}
              {#if !artUrl.trim()}<span class="normal-case tracking-normal italic"> — {$t('adminBattlesArtFromWork')}</span>{/if}
            </span>
            <input bind:value={artUrl} placeholder="/static/images/…" class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35" />
          </label>
          {#if artUrl.trim()}
            <button onclick={() => (artUrl = '')} class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5">
              {$t('adminBattlesArtClear')}
            </button>
          {/if}
          <label class="block w-24">
            <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesSlug')}</span>
            <input bind:value={slug} class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none" />
          </label>
        </div>

        <!-- How the photograph sits in the frame. Same normalised shape the
             figurine keyhole focus already uses. -->
        <div class="flex flex-wrap gap-6 mt-6 max-w-3xl">
          {#each [['x', $t('adminBattlesFocusX')], ['y', $t('adminBattlesFocusY')], ['zoom', $t('adminBattlesZoom')]] as [key, label] (key)}
            <label class="block w-44">
              <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{label}</span>
              {#if key === 'zoom'}
                <input type="range" min="1" max="3" step="0.05" bind:value={zoom} class="w-full" />
              {:else if key === 'x'}
                <input type="range" min="0" max="1" step="0.01" bind:value={focusX} class="w-full" />
              {:else}
                <input type="range" min="0" max="1" step="0.01" bind:value={focusY} class="w-full" />
              {/if}
            </label>
          {/each}
        </div>

        <div class="flex items-center gap-3 mt-8">
          <button
            onclick={save}
            disabled={saving}
            class="px-4 py-2 text-[10px] uppercase tracking-[0.16em] bg-[#34251c] text-[#f8f1e7] disabled:opacity-40"
          >{$t('adminBattlesSave')}</button>
          {#if selectedId}
            <button
              onclick={remove}
              class="px-4 py-2 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 text-[#6f3b24] hover:bg-[#c65f3c]/10"
            >{$t('adminBattlesDelete')}</button>
          {/if}
        </div>
      </div>

      <!-- ── The same card the shelf will render ─────────────────────────── -->
      <aside class="w-72 flex-shrink-0 p-5 border-l border-[#34251c]/10 overflow-y-auto">
        <p class="mb-3 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesPreview')}</p>
        <!-- `transition={false}`: the shelf's card owns the shared-element name,
             and two elements carrying it would abort the transition outright. -->
        <BattleCard
          card={preview}
          {frames}
          owned={!previewDown}
          level={previewLevel}
          transition={false}
        />
        <label class="flex items-center gap-2 mt-4 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">
          <input type="checkbox" bind:checked={previewDown} />
          {$t('adminBattlesPreviewDown')}
        </label>
        <label class="block mt-3">
          <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesPreviewLevel')}</span>
          <select
            value={previewLevel ?? ''}
            onchange={(e) => (previewLevel = e.currentTarget.value === '' ? null : Number(e.currentTarget.value))}
            class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none"
          >
            <option value="">{$t('adminBattlesPreviewNone')}</option>
            {#each TIERS as step (step)}
              <option value={step}>{step}</option>
            {/each}
          </select>
        </label>
      </aside>
    </div>
  {/if}
</div>
