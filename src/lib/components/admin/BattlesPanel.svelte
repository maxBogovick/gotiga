<script lang="ts">
  // The keeper's desk for the card shelf.
  //
  // Two things are edited here and they are deliberately kept apart. A CARD is
  // content: name, effect, corners, price, which work it stands for. A FRAME is
  // design: how a whole rank is dressed. Editing forty cards to change how rank
  // three looks is the mistake every card-authoring tool exists to prevent, so
  // the frames live in their own view and there are exactly five of them.
  //
  // The card editor does not draw a form beside a preview — it edits the
  // preview. `draft` is the one live `BattleCard` object; `BattleCard.svelte`
  // (`editable`) mutates its fields directly as the keeper clicks and types on
  // the card itself, and this desk never keeps a second copy of the same data
  // in a pile of loose variables that could drift from what the card shows.
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { t, lang } from '$lib/i18n';
  import {
    DEFAULT_FRAMES,
    FRAME_MODES,
    LAYOUTS,
    TIERS,
    applyInsetDelta,
    frameName,
    parseFocal,
    pickImageFile,
    type InsetKey,
  } from '$lib/battles';
  import { SITE_FONTS } from '$lib/fonts';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import type {
    BattleCard as BattleCardDto,
    BattleCardStatus,
    BattleFrame,
    BattleRace,
    FigurineListItem,
    SaveBattleCardRequest,
  } from '$lib/types/api';

  const REORDER_MS = 600;

  let view = $state<'cards' | 'frames' | 'races'>('cards');
  let cards = $state<BattleCardDto[]>([]);
  let frames = $state<BattleFrame[]>([...DEFAULT_FRAMES]);
  let figurines = $state<FigurineListItem[]>([]);
  let races = $state<BattleRace[]>([]);
  let loading = $state(true);
  let saving = $state(false);
  let message = $state('');
  let listQuery = $state('');

  // ── The card being written — one live object, edited on the card itself ──
  let selectedId = $state<string | null>(null);
  let draft = $state<BattleCardDto>(emptyCard());
  /** Which language the on-card fields read and write. The card shows one
   *  language at a time, same as a reader would see it. */
  let editLang = $state<'en' | 'ru'>(($lang as 'en' | 'ru') ?? 'ru');
  /** Prices only ever show on a card's back — flipping is how the desk gets
   *  at them, the same as anyone else would have to turn the card over. */
  let facedown = $state(false);
  // The preview shows what a copy looks like, which is a different thing from
  // the card's rank — so the desk can set it, and it is off by default.
  let previewLevel = $state<number | null>(null);

  // The frames view edits one rank at a time. Five stacked blocks of sliders is
  // a wall; one rank with its card standing next to it is a workbench.
  let frameIndex = $state(0);
  let uploading = $state(false);

  let dragFrom = $state<number | null>(null);
  let dragOver = $state<number | null>(null);
  let reorderTimer: ReturnType<typeof setTimeout> | null = null;
  let flashTimer: ReturnType<typeof setTimeout> | null = null;

  // The race dictionary, edited in its own view.
  let raceDraftId = $state<string | null>(null);
  let raceNameEn = $state('');
  let raceNameRu = $state('');
  let raceNoteEn = $state('');
  let raceNoteRu = $state('');
  let raceIconUrl = $state('');

  function emptyCard(): BattleCardDto {
    return {
      id: '',
      slug: '',
      status: 'draft',
      tier: 1,
      raceId: null,
      raceNameEn: null,
      raceNameRu: null,
      raceIconUrl: null,
      typeEn: null,
      typeRu: null,
      titleEn: '',
      titleRu: '',
      effectEn: null,
      effectRu: null,
      loreEn: null,
      loreRu: null,
      cost: 1,
      power: 1,
      health: 0,
      mana: 0,
      traits: [],
      priceDust: null,
      priceFeed: null,
      artUrl: null,
      artUrlOverride: null,
      artFocal: null,
      frameOverride: null,
      shelfOrder: null,
      figurineId: null,
      figurineName: null,
      figurineSlug: null,
      createdAt: '',
      updatedAt: '',
    };
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

  /** The "Work" picker's own filter — the collection is flat and alphabetical
   *  by nothing in particular, so past thirty or so entries scrolling to find
   *  one beats typing its name. The card's already-chosen work stays in the
   *  list even against a query that would otherwise hide it, so picking one
   *  and then narrowing the filter never makes the selection vanish. */
  let workQuery = $state('');
  let visibleFigurines = $derived(
    workQuery.trim()
      ? figurines.filter(
          (f) => f.id === draft.figurineId || f.name.toLowerCase().includes(workQuery.trim().toLowerCase()),
        )
      : figurines,
  );

  /** A figurine already wearing a card, spotted before the save round-trip
   *  reports it back as a `UNIQUE figurine_id` conflict — `battle_cards` allows
   *  exactly one card per work. */
  let workTaken = $derived(
    !!draft.figurineId && cards.some((c) => c.figurineId === draft.figurineId && c.id !== selectedId),
  );

  /** Where the picture sits in the frame, read out of the card's own JSON —
   *  the same shape the on-card drag writes, kept here only as the
   *  keyboard-reachable fallback a pointer drag cannot be. */
  let focal = $derived(parseFocal(draft.artFocal));
  function setFocal(patch: Partial<{ x: number; y: number; zoom: number }>) {
    draft.artFocal = JSON.stringify({ ...focal, ...patch });
  }

  /** The card's own picture wins; absent one, it borrows the linked work's
   *  face — the same choice the server makes in `battle_card_dto`. Kept live
   *  here so picking a different work updates the card immediately, the way
   *  it will once saved. */
  $effect(() => {
    const fig = figurines.find((f) => f.id === draft.figurineId) ?? null;
    if (!draft.artUrlOverride) {
      draft.artUrl = fig?.faceImageLargeUrl || fig?.faceImageUrl || null;
    }
  });

  /** The card the frames view dresses: the one being written, else any real
   *  card of that rank, else a stand-in — so the sliders always have a subject. */
  let frameSample = $derived<BattleCardDto>(
    draft.titleRu || draft.titleEn
      ? { ...draft, tier: frames[frameIndex]?.tier ?? 1 }
      : (cards.find((c) => c.tier === (frames[frameIndex]?.tier ?? 1)) ?? {
          ...emptyCard(),
          tier: frames[frameIndex]?.tier ?? 1,
          titleEn: 'The Keeper of the Key',
          titleRu: 'Хранительница Ключа',
          effectRu: 'Вихрь Души: каждое третье заклинание создаёт копию эффекта.',
          effectEn: 'Wind of Soul: every third spell makes a copy of its effect.',
          cost: 5,
          power: 10,
        }),
  );

  /** The race dictionary's own sample, so the icon can be judged on a card
   *  rather than as a bare thumbnail — the same reasoning the frames view
   *  already follows for its own sample. */
  let raceSample = $derived<BattleCardDto>({
    ...emptyCard(),
    tier: 3,
    titleEn: 'The Keeper of the Key',
    titleRu: 'Хранительница Ключа',
    effectEn: 'A sample card, to see the icon in place.',
    effectRu: 'Пример карты — чтобы увидеть иконку на месте.',
    raceNameEn: raceNameEn || raceNameRu ? raceNameEn || raceNameRu : null,
    raceNameRu: raceNameRu || raceNameEn ? raceNameRu || raceNameEn : null,
    raceIconUrl: raceIconUrl.trim() || null,
  });

  /** A frame picture, kept transparent, with the card's ratio taken from it. */
  async function uploadFrameArt() {
    const file = await pickImageFile();
    if (!file) return;
    uploading = true;
    try {
      const art = await api.adminUploadBattleFrameArt(file);
      const frame = frames[frameIndex];
      frame.frameImage = art.url;
      // The picture is stretched over the whole card, so the card must take the
      // picture's proportions or the carving comes out squashed. Set here, once,
      // instead of leaving the keeper to find the slider and guess.
      if (art.width && art.height) frame.aspect = art.width / art.height;
      if (art.hasAlpha) {
        frame.frameMode = 'overlay';
      } else {
        // No hole in it: worn on top it would cover the card completely.
        frame.frameMode = 'behind';
        flash($t('adminBattlesFrameNoAlpha'), 8000);
      }
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      uploading = false;
    }
  }

  /** The paper under the card. An ordinary photograph — no transparency needed. */
  async function uploadPaperArt() {
    const file = await pickImageFile();
    if (!file) return;
    uploading = true;
    try {
      const imported = await api.importMediaWithVariants(file, 'images', 'card-paper');
      frames[frameIndex].paperImage = imported.url;
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      uploading = false;
    }
  }

  /** The reverse — what a card you do not own shows lying in dust. Never the
   *  frame's own picture: the carving is the front's dress and BattleCard
   *  never wears it face down, whatever this is set to. */
  async function uploadBackArt() {
    const file = await pickImageFile();
    if (!file) return;
    uploading = true;
    try {
      const art = await api.adminUploadBattleFrameArt(file);
      frames[frameIndex].backImage = art.url;
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      uploading = false;
    }
  }

  /** A slider set to `value` — mirrored onto the opposite side by the same
   *  amount, same as dragging the inset handle on the card itself. */
  function setInset(kind: InsetKey, value: number) {
    const frame = frames[frameIndex];
    applyInsetDelta(frame, kind, value - (frame[kind] ?? 0));
  }

  // ── The race dictionary ───────────────────────────────────────────────────

  function openRace(race: BattleRace | null) {
    raceDraftId = race?.id ?? null;
    raceNameEn = race?.nameEn ?? '';
    raceNameRu = race?.nameRu ?? '';
    raceNoteEn = race?.noteEn ?? '';
    raceNoteRu = race?.noteRu ?? '';
    raceIconUrl = race?.iconUrl ?? '';
  }

  async function saveRace() {
    saving = true;
    try {
      await api.adminSaveBattleRace(
        {
          nameEn: raceNameEn.trim(),
          nameRu: raceNameRu.trim(),
          noteEn: raceNoteEn.trim() || null,
          noteRu: raceNoteRu.trim() || null,
          iconUrl: raceIconUrl.trim() || null,
        },
        raceDraftId ?? undefined,
      );
      races = await api.getBattleRaces();
      openRace(null);
      flash($t('adminBattlesRaceSaved'));
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      saving = false;
    }
  }

  async function removeRace(race: BattleRace) {
    if (!confirm($t('adminBattlesRaceDeleteConfirm'))) return;
    try {
      await api.adminDeleteBattleRace(race.id);
      races = await api.getBattleRaces();
      await loadCards();
      if (raceDraftId === race.id) openRace(null);
      flash($t('adminBattlesRaceDeleted'));
    } catch (e) {
      flash(String(e), 6000);
    }
  }

  /** The per-card editor's icon click, when the icon belongs to a race
   *  someone else's card also wears: sent here instead of editing it in place. */
  function jumpToRace() {
    const race = races.find((r) => r.id === draft.raceId) ?? null;
    openRace(race);
    view = 'races';
  }

  // ── The card's text and numbers — an ordinary form, not the card face ────
  //
  // These used to be typed straight into the decorative card (see the note at
  // the top of `BattleCard.svelte`): a cost badge the size of a coin with a
  // number-input's spin arrows crammed inside it, a race dropdown clipped to
  // half the header's width. All of it read the same `draft` object the card
  // still previews live, so moving the typing here loses nothing — the card
  // stays the single source of truth, it is just no longer also the keyboard.

  /** Picking a race also updates what the header actually shows — the DTO's
   *  flattened name/icon are normally the server's doing, and the live card
   *  must agree with them before anything is saved. */
  function selectRace(id: string) {
    draft.raceId = id || null;
    const picked = races.find((r) => r.id === id);
    draft.raceNameEn = picked?.nameEn ?? null;
    draft.raceNameRu = picked?.nameRu ?? null;
    draft.raceIconUrl = picked?.iconUrl ?? null;
  }

  const TRAITS_MAX = 8;

  function addTrait() {
    const list = draft.traits ?? [];
    if (list.length >= TRAITS_MAX) return;
    draft.traits = [...list, { nameEn: '', nameRu: '', textEn: '', textRu: '' }];
  }

  function removeTrait(at: number) {
    draft.traits = (draft.traits ?? []).filter((_, i) => i !== at);
  }

  function moveTrait(at: number, by: number) {
    const list = draft.traits ?? [];
    const to = at + by;
    if (to < 0 || to >= list.length) return;
    const next = [...list];
    [next[at], next[to]] = [next[to], next[at]];
    draft.traits = next;
  }

  function priceInput(coin: 'priceDust' | 'priceFeed', raw: string) {
    const text = raw.trim();
    draft[coin] = text === '' ? null : Math.max(0, Math.round(Number(text)));
  }

  /** A number field's whole value is worth more retyped than nudged — this
   *  makes a click-and-type behave like it does everywhere else. */
  function selectOnFocus(e: FocusEvent & { currentTarget: HTMLInputElement }) {
    e.currentTarget.select();
  }

  /** A focused `type="number"` spins its value on an ordinary page-scroll
   *  wheel in Chrome and Firefox — surprising here, where the field sits in a
   *  normally scrollable sidebar. Blurring it hands the scroll back to the
   *  page instead of silently changing cost, power, health or a price. */
  function blurOnWheel(e: WheelEvent & { currentTarget: HTMLInputElement }) {
    e.currentTarget.blur();
  }

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
    draft = emptyCard();
    facedown = false;
  }

  function openCard(card: BattleCardDto) {
    selectedId = card.id;
    draft = {
      ...card,
      traits: (card.traits ?? []).map((t) => ({ ...t })),
    };
    facedown = false;
  }

  async function save() {
    saving = true;
    try {
      const body: SaveBattleCardRequest = {
        slug: draft.slug.trim() || null,
        status: draft.status,
        tier: draft.tier,
        titleEn: draft.titleEn.trim(),
        titleRu: draft.titleRu.trim(),
        effectEn: draft.effectEn?.trim() || null,
        effectRu: draft.effectRu?.trim() || null,
        loreEn: draft.loreEn?.trim() || null,
        loreRu: draft.loreRu?.trim() || null,
        cost: draft.cost,
        power: draft.power,
        health: draft.health,
        mana: draft.mana,
        traits: (draft.traits ?? []).filter((t) => t.nameEn.trim() || t.nameRu.trim()),
        raceId: draft.raceId || null,
        typeEn: draft.typeEn?.trim() || null,
        typeRu: draft.typeRu?.trim() || null,
        priceDust: draft.priceDust,
        priceFeed: draft.priceFeed,
        artUrl: draft.artUrlOverride?.trim() || null,
        artFocal: draft.artFocal,
        frameOverride: draft.frameOverride,
        figurineId: draft.figurineId || null,
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
      const [, figs, savedFrames, savedRaces] = await Promise.all([
        loadCards(),
        api.getAllFigurines(),
        api.getBattleFrames(),
        api.getBattleRaces(),
      ]);
      figurines = figs;
      races = savedRaces;
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
      <button
        onclick={() => (view = 'races')}
        class="px-3 py-1 {view === 'races' ? 'bg-[#34251c] text-[#f8f1e7]' : ''}"
      >{$t('adminBattlesRacesView')}</button>
    </div>
    {#if message}
      <span class="ml-auto normal-case tracking-normal text-[11px] text-[#6f3b24]">{message}</span>
    {/if}
  </div>

  {#if view === 'frames'}
    <!-- ── Five frames, one per rank ────────────────────────────────────── -->
    <div class="flex-1 flex min-h-0">
      <div class="flex-1 overflow-y-auto p-6 min-w-0">
        <p class="max-w-[62ch] mb-5 text-xs leading-relaxed text-[#5f4636]">{$t('adminBattlesFramesHint')}</p>

        <div class="flex border border-[#34251c]/15 w-fit mb-6">
          {#each frames as frame, i (frame.tier)}
            <button
              onclick={() => (frameIndex = i)}
              class="px-3 py-1.5 text-[11px] {frameIndex === i ? 'bg-[#34251c] text-[#f8f1e7]' : 'hover:bg-[#34251c]/5'}"
            >{frame.tier} · {frameName(frame, $lang)}</button>
          {/each}
        </div>

        {#if frames[frameIndex]}
          <div class="max-w-2xl space-y-6">
            <div class="flex flex-wrap items-end gap-4">
              <label class="block">
                <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesFrameName')} · EN</span>
                <input bind:value={frames[frameIndex].nameEn} class="px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35" />
              </label>
              <label class="block">
                <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesFrameName')} · RU</span>
                <input bind:value={frames[frameIndex].nameRu} class="px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35" />
              </label>
              <label class="block">
                <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesFrameLayout')}</span>
                <select bind:value={frames[frameIndex].layout} class="px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none">
                  {#each LAYOUTS as option (option)}
                    <option value={option}>
                      {option === 'corners' ? $t('adminBattlesLayoutCorners') : $t('adminBattlesLayoutPlaque')}
                    </option>
                  {/each}
                </select>
              </label>
            </div>

            <!-- The photograph of a real frame. -->
            <div class="pt-5 border-t border-[#34251c]/10">
              <p class="mb-3 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesFrameArt')}</p>
              <p class="max-w-[62ch] mb-3 text-[11px] leading-relaxed italic text-[#8a6a55]">{$t('adminBattlesFrameArtHint')}</p>
              <div class="flex flex-wrap items-end gap-3">
                <button
                  onclick={uploadFrameArt}
                  disabled={uploading}
                  class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5 disabled:opacity-40"
                >{uploading ? '…' : $t('adminBattlesFrameArtUpload')}</button>
                <label class="block flex-1 min-w-[16rem]">
                  <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">
                    {#if !frames[frameIndex].frameImage.trim()}{$t('adminBattlesFrameArtNone')}{:else}URL{/if}
                  </span>
                  <input bind:value={frames[frameIndex].frameImage} placeholder="/static/frames/…" class="w-full px-2 py-1.5 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35" />
                </label>
                <label class="block">
                  <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesFrameMode')}</span>
                  <select bind:value={frames[frameIndex].frameMode} class="px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none">
                    {#each FRAME_MODES as mode (mode)}
                      <option value={mode}>
                        {mode === 'overlay' ? $t('adminBattlesFrameOverlay') : $t('adminBattlesFrameBehind')}
                      </option>
                    {/each}
                  </select>
                </label>
                {#if frames[frameIndex].frameImage.trim()}
                  <button
                    onclick={() => (frames[frameIndex].frameImage = '')}
                    class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5"
                  >{$t('adminBattlesFrameArtClear')}</button>
                {/if}
              </div>

              <!-- What shows through the hole in a cut-out frame. -->
              <div class="flex flex-wrap items-end gap-3 mt-4">
                <button
                  onclick={uploadPaperArt}
                  disabled={uploading}
                  class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5 disabled:opacity-40"
                >{uploading ? '…' : $t('adminBattlesPaperUpload')}</button>
                <label class="block flex-1 min-w-[16rem]">
                  <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">
                    {#if !frames[frameIndex].paperImage.trim()}{$t('adminBattlesPaperNone')}{:else}URL{/if}
                  </span>
                  <input bind:value={frames[frameIndex].paperImage} placeholder="/static/images/preview/…" class="w-full px-2 py-1.5 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35" />
                </label>
                {#if frames[frameIndex].paperImage.trim()}
                  <button
                    onclick={() => (frames[frameIndex].paperImage = '')}
                    class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5"
                  >{$t('adminBattlesFrameArtClear')}</button>
                {/if}
              </div>
            </div>

            <!-- The reverse. Never wears the frame above, whatever picture it shows —
                 the carving is the front's own dress. -->
            <div class="pt-5 border-t border-[#34251c]/10">
              <p class="mb-3 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesBackArt')}</p>
              <p class="max-w-[62ch] mb-3 text-[11px] leading-relaxed italic text-[#8a6a55]">{$t('adminBattlesBackArtHint')}</p>
              <div class="flex flex-wrap items-end gap-3">
                <button
                  onclick={uploadBackArt}
                  disabled={uploading}
                  class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5 disabled:opacity-40"
                >{uploading ? '…' : $t('adminBattlesBackArtUpload')}</button>
                <label class="block flex-1 min-w-[16rem]">
                  <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">
                    {#if !frames[frameIndex].backImage.trim()}{$t('adminBattlesBackArtNone')}{:else}URL{/if}
                  </span>
                  <input bind:value={frames[frameIndex].backImage} placeholder="/static/frames/…" class="w-full px-2 py-1.5 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35" />
                </label>
                {#if frames[frameIndex].backImage.trim()}
                  <button
                    onclick={() => (frames[frameIndex].backImage = '')}
                    class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5"
                  >{$t('adminBattlesFrameArtClear')}</button>
                {/if}
              </div>
            </div>

            <!-- Where the opening in that frame actually is. -->
            <div class="pt-5 border-t border-[#34251c]/10">
              <p class="mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesFrameWindow')}</p>
              <p class="max-w-[62ch] mb-3 text-[11px] leading-relaxed italic text-[#8a6a55]">{$t('adminBattlesFrameWindowHint')}</p>
              <p class="max-w-[62ch] mb-3 text-[11px] leading-relaxed italic text-[#8a6a55]">{$t('adminBattlesBandsHint')}</p>
              <div class="flex flex-wrap gap-5">
                <label class="block w-40">
                  <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesInsetTop')} · {frames[frameIndex].insetTop.toFixed(0)}%</span>
                  <input
                    type="range" min="0" max="45" step="0.5"
                    value={frames[frameIndex].insetTop}
                    oninput={(e) => setInset('insetTop', Number(e.currentTarget.value))}
                    class="w-full"
                  />
                </label>
                <label class="block w-40">
                  <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesInsetRight')} · {frames[frameIndex].insetRight.toFixed(0)}%</span>
                  <input
                    type="range" min="0" max="45" step="0.5"
                    value={frames[frameIndex].insetRight}
                    oninput={(e) => setInset('insetRight', Number(e.currentTarget.value))}
                    class="w-full"
                  />
                </label>
                <label class="block w-40">
                  <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesInsetBottom')} · {frames[frameIndex].insetBottom.toFixed(0)}%</span>
                  <input
                    type="range" min="0" max="45" step="0.5"
                    value={frames[frameIndex].insetBottom}
                    oninput={(e) => setInset('insetBottom', Number(e.currentTarget.value))}
                    class="w-full"
                  />
                </label>
                <label class="block w-40">
                  <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesInsetLeft')} · {frames[frameIndex].insetLeft.toFixed(0)}%</span>
                  <input
                    type="range" min="0" max="45" step="0.5"
                    value={frames[frameIndex].insetLeft}
                    oninput={(e) => setInset('insetLeft', Number(e.currentTarget.value))}
                    class="w-full"
                  />
                </label>
                <label class="block w-40">
                  <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesAspect')} · {frames[frameIndex].aspect.toFixed(2)}</span>
                  <input type="range" min="0.45" max="1.4" step="0.01" bind:value={frames[frameIndex].aspect} class="w-full" />
                </label>
                <label class="block w-40">
                  <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesHeaderShare')} · {(frames[frameIndex].headerShare * 100).toFixed(0)}%</span>
                  <input type="range" min="0" max="0.3" step="0.005" bind:value={frames[frameIndex].headerShare} class="w-full" />
                </label>
                <label class="block w-40">
                  <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesArtShare')} · {(frames[frameIndex].artShare * 100).toFixed(0)}%</span>
                  <input type="range" min="0.12" max="0.85" step="0.01" bind:value={frames[frameIndex].artShare} class="w-full" />
                </label>
                <label class="block w-40">
                  <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesFootShare')} · {(frames[frameIndex].footShare * 100).toFixed(0)}%</span>
                  <input type="range" min="0" max="0.3" step="0.005" bind:value={frames[frameIndex].footShare} class="w-full" />
                </label>
              </div>
            </div>

            <!-- The name, and the colours the renderer paints when there is no
                 photograph — still the ground under one that fails to load. -->
            <div class="pt-5 border-t border-[#34251c]/10 flex flex-wrap items-end gap-4">
              <label class="block">
                <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesTitleFont')}</span>
                <select bind:value={frames[frameIndex].titleFont} class="px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none">
                  <option value="">{$t('adminBattlesTitleFontDefault')}</option>
                  {#each SITE_FONTS as font (font.id)}
                    <option value={font.id}>{font.name}</option>
                  {/each}
                </select>
              </label>
              <label class="block">
                <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesTitleInk')}</span>
                <input
                  type="color"
                  value={frames[frameIndex].titleInk || frames[frameIndex].ink}
                  oninput={(e) => (frames[frameIndex].titleInk = e.currentTarget.value)}
                  class="w-12 h-8 bg-transparent border border-[#34251c]/15"
                />
              </label>
              {#each [['paper', $t('adminBattlesFramePaper')], ['ink', $t('adminBattlesFrameInk')], ['border', $t('adminBattlesFrameBorder')]] as [key, label] (key)}
                <label class="block">
                  <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{label}</span>
                  <input
                    type="color"
                    value={frames[frameIndex][key as 'paper' | 'ink' | 'border']}
                    oninput={(e) => (frames[frameIndex][key as 'paper' | 'ink' | 'border'] = e.currentTarget.value)}
                    class="w-12 h-8 bg-transparent border border-[#34251c]/15"
                  />
                </label>
              {/each}
              <label class="block flex-1 min-w-[14rem]">
                <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">
                  {$t('adminBattlesFrameFoil')}
                  {#if !frames[frameIndex].foil.trim()}<span class="normal-case tracking-normal italic"> — {$t('adminBattlesFrameNoFoil')}</span>{/if}
                </span>
                <input bind:value={frames[frameIndex].foil} placeholder="rgba(198,95,60,0.28)" class="w-full px-2 py-1.5 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35" />
              </label>
            </div>
          </div>
        {/if}

        <button
          onclick={saveFrames}
          disabled={saving}
          class="mt-7 px-4 py-2 text-[10px] uppercase tracking-[0.16em] bg-[#34251c] text-[#f8f1e7] disabled:opacity-40"
        >{$t('adminBattlesFramesSave')}</button>
      </div>

      <!-- The frame is judged on a card, not on a swatch: the window can only be
           placed by watching a real photograph sit inside the carving. -->
      <aside class="w-80 flex-shrink-0 p-5 border-l border-[#34251c]/10 overflow-y-auto">
        <p class="mb-3 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesPreview')}</p>
        <BattleCard card={frameSample} {frames} owned={true} transition={false} interactive={false} frameEditable={true} />
      </aside>
    </div>
  {:else if view === 'races'}
    <!-- ── The race dictionary ──────────────────────────────────────────── -->
    <div class="flex-1 flex min-h-0">
      <div class="flex-1 overflow-y-auto p-6 min-w-0">
        <p class="max-w-[62ch] mb-5 text-xs leading-relaxed text-[#5f4636]">{$t('adminBattlesRacesHint')}</p>

        {#if !races.length}
          <p class="mb-5 text-xs italic text-[#5f4636]">{$t('adminBattlesRacesEmpty')}</p>
        {:else}
          <ul class="max-w-2xl mb-6 border-t border-[#34251c]/10">
            {#each races as race (race.id)}
              <li class="flex items-center gap-3 py-2 border-b border-[#34251c]/10">
                <button
                  onclick={() => openRace(race)}
                  class="flex-1 text-left hover:text-[#c65f3c] {raceDraftId === race.id ? 'text-[#c65f3c]' : ''}"
                >
                  <span class="text-sm" style="font-family: 'Cormorant Garamond', Georgia, serif;">{race.nameRu}</span>
                  <span class="ml-2 text-[11px] text-[#8a6a55]">{race.nameEn}</span>
                </button>
                <!-- What a rename or a removal would touch. -->
                <span class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">
                  {race.cardCount} {$t('adminBattlesRaceCards')}
                </span>
                <button
                  onclick={() => removeRace(race)}
                  class="px-2 py-1 text-xs border border-[#34251c]/20 text-[#6f3b24] hover:bg-[#c65f3c]/10"
                >×</button>
              </li>
            {/each}
          </ul>
        {/if}

        <div class="max-w-2xl p-4 border border-[#34251c]/12">
          <p class="mb-3 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">
            {raceDraftId ? $t('adminBattlesRaceEdit') : $t('adminBattlesRaceNew')}
          </p>
          <div class="flex flex-wrap gap-3">
            <label class="block w-52">
              <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesFrameName')} · RU</span>
              <input bind:value={raceNameRu} maxlength="60" class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35" />
            </label>
            <label class="block w-52">
              <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesFrameName')} · EN</span>
              <input bind:value={raceNameEn} maxlength="60" class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35" />
            </label>
          </div>
          <div class="flex flex-wrap gap-3 mt-3">
            <label class="block flex-1 min-w-[14rem]">
              <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesRaceNote')} · RU</span>
              <input bind:value={raceNoteRu} maxlength="200" class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35" />
            </label>
            <label class="block flex-1 min-w-[14rem]">
              <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesRaceNote')} · EN</span>
              <input bind:value={raceNoteEn} maxlength="200" class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35" />
            </label>
          </div>
          <div class="flex items-center gap-3 mt-4">
            <button
              onclick={saveRace}
              disabled={saving}
              class="px-4 py-2 text-[10px] uppercase tracking-[0.16em] bg-[#34251c] text-[#f8f1e7] disabled:opacity-40"
            >{$t('adminBattlesSave')}</button>
            {#if raceDraftId}
              <button
                onclick={() => openRace(null)}
                class="px-4 py-2 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20"
              >{$t('adminBattlesRaceNew')}</button>
            {/if}
          </div>
        </div>
      </div>

      <!-- Judged on a card, the same way a frame is: an icon the size of a
           swatch tells the keeper nothing about how it reads in the header. -->
      <aside class="w-80 flex-shrink-0 p-5 border-l border-[#34251c]/10 overflow-y-auto">
        <p class="mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesRaceIcon')}</p>
        <p class="max-w-[62ch] mb-3 text-[11px] leading-relaxed italic text-[#8a6a55]">{$t('adminBattlesRaceIconHint')}</p>
        <BattleCard
          card={raceSample}
          {frames}
          owned={true}
          transition={false}
          interactive={false}
          raceIconEditable={true}
          onIconUpload={(url) => (raceIconUrl = url)}
          onError={(e) => flash(e, 6000)}
        />
        {#if raceIconUrl.trim()}
          <button
            onclick={() => (raceIconUrl = '')}
            class="mt-3 px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5"
          >{$t('adminBattlesFrameArtClear')}</button>
        {/if}
      </aside>
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

      <!-- ── The card itself: the editor ───────────────────────────────── -->
      <div class="flex-1 flex flex-col min-h-0">
        <div class="flex items-center gap-4 px-5 py-2 border-b border-[#34251c]/10">
          <div class="flex border border-[#34251c]/15 text-[10px] uppercase tracking-[0.16em]">
            <button
              onclick={() => (editLang = 'ru')}
              class="px-2.5 py-1 {editLang === 'ru' ? 'bg-[#34251c] text-[#f8f1e7]' : ''}"
            >RU</button>
            <button
              onclick={() => (editLang = 'en')}
              class="px-2.5 py-1 {editLang === 'en' ? 'bg-[#34251c] text-[#f8f1e7]' : ''}"
            >EN</button>
          </div>
          <span class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesEditLang')}</span>
          <label class="flex items-center gap-2 ml-auto text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">
            <input type="checkbox" bind:checked={facedown} />
            {$t('adminBattlesPreviewDown')}
          </label>
        </div>

        <div class="flex-1 flex min-h-0">
          <div class="flex-1 overflow-y-auto p-8 flex items-start justify-center">
            <div class="w-full max-w-[460px]">
              <BattleCard
                bind:card={draft}
                {frames}
                editable={true}
                frameEditable={true}
                {editLang}
                owned={!facedown}
                level={previewLevel}
                interactive={false}
                transition={false}
                onEditRace={jumpToRace}
                onError={(e) => flash(e, 6000)}
              />
            </div>
          </div>

          <!-- ── Everything with no place on the card's own face ─────────── -->
          <aside class="w-96 flex-shrink-0 p-5 border-l border-[#34251c]/10 overflow-y-auto">
            <div class="space-y-5">
              <label class="block">
                <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesStatus')}</span>
                <select bind:value={draft.status} class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none">
                  <option value="draft">{$t('adminBattlesStatusDraft')}</option>
                  <option value="published">{$t('adminBattlesStatusPublished')}</option>
                  <option value="retired">{$t('adminBattlesStatusRetired')}</option>
                </select>
              </label>

              <label class="block">
                <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesSlug')}</span>
                <input bind:value={draft.slug} class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none" />
              </label>

              <!-- ── What the card says: typed here at a normal size, read live
                   on the card beside it — same `draft`, no second copy. ─────── -->
              <div class="pt-4 border-t border-[#34251c]/10 space-y-3">
                <label class="block">
                  <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesTitle')}</span>
                  <input
                    maxlength="80"
                    value={editLang === 'en' ? draft.titleEn : draft.titleRu}
                    oninput={(e) => {
                      if (editLang === 'en') draft.titleEn = e.currentTarget.value;
                      else draft.titleRu = e.currentTarget.value;
                    }}
                    class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                  />
                </label>

                <div class="flex gap-3">
                  <label class="block flex-1 min-w-0">
                    <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesRace')}</span>
                    <select
                      value={draft.raceId ?? ''}
                      onchange={(e) => selectRace(e.currentTarget.value)}
                      class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none"
                    >
                      <option value="">{$t('adminBattlesRaceNone')}</option>
                      {#each races as race (race.id)}
                        <option value={race.id}>{editLang === 'en' ? race.nameEn : race.nameRu}</option>
                      {/each}
                    </select>
                  </label>
                  <label class="block flex-1 min-w-0">
                    <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('battlesTypeLabel')}</span>
                    <input
                      maxlength="40"
                      value={editLang === 'en' ? (draft.typeEn ?? '') : (draft.typeRu ?? '')}
                      oninput={(e) => {
                        if (editLang === 'en') draft.typeEn = e.currentTarget.value;
                        else draft.typeRu = e.currentTarget.value;
                      }}
                      class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                    />
                  </label>
                </div>

                <label class="block">
                  <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesEffect')}</span>
                  <textarea
                    maxlength="400"
                    rows="3"
                    value={editLang === 'en' ? (draft.effectEn ?? '') : (draft.effectRu ?? '')}
                    oninput={(e) => {
                      if (editLang === 'en') draft.effectEn = e.currentTarget.value || null;
                      else draft.effectRu = e.currentTarget.value || null;
                    }}
                    class="w-full px-2 py-1.5 text-sm leading-relaxed bg-transparent border border-[#34251c]/15 outline-none resize-y focus:border-[#34251c]/35"
                  ></textarea>
                </label>

                <div>
                  <p class="mb-2 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesTraits')}</p>
                  {#if !(draft.traits ?? []).length}
                    <p class="mb-2 text-[11px] italic text-[#8a6a55]">{$t('adminBattlesTraitsEmpty')}</p>
                  {/if}
                  <div class="space-y-2">
                    {#each draft.traits ?? [] as trait, i (i)}
                      <div class="flex items-start gap-1.5 p-2 border border-[#34251c]/10">
                        <div class="flex-1 min-w-0 space-y-1.5">
                          <input
                            maxlength="60"
                            placeholder={$t('adminBattlesTraitName')}
                            value={editLang === 'en' ? trait.nameEn : trait.nameRu}
                            oninput={(e) => {
                              if (editLang === 'en') trait.nameEn = e.currentTarget.value;
                              else trait.nameRu = e.currentTarget.value;
                            }}
                            class="w-full px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                          />
                          <input
                            maxlength="200"
                            placeholder={$t('adminBattlesTraitText')}
                            value={editLang === 'en' ? trait.textEn : trait.textRu}
                            oninput={(e) => {
                              if (editLang === 'en') trait.textEn = e.currentTarget.value;
                              else trait.textRu = e.currentTarget.value;
                            }}
                            class="w-full px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                          />
                        </div>
                        <div class="flex flex-col gap-0.5 flex-shrink-0">
                          <button type="button" onclick={() => moveTrait(i, -1)} disabled={i === 0} class="px-1.5 text-xs border border-[#34251c]/20 disabled:opacity-30">↑</button>
                          <button type="button" onclick={() => moveTrait(i, 1)} disabled={i === (draft.traits?.length ?? 0) - 1} class="px-1.5 text-xs border border-[#34251c]/20 disabled:opacity-30">↓</button>
                          <button type="button" onclick={() => removeTrait(i)} class="px-1.5 text-xs border border-[#34251c]/20 hover:bg-[#c65f3c]/10">×</button>
                        </div>
                      </div>
                    {/each}
                  </div>
                  <button
                    type="button"
                    onclick={addTrait}
                    disabled={(draft.traits?.length ?? 0) >= TRAITS_MAX}
                    class="mt-2 px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5 disabled:opacity-40"
                  >+ {$t('adminBattlesTraitAdd')}</button>
                </div>

                <div class="flex gap-3">
                  <label class="block flex-1">
                    <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('battlesHealthLabel')}</span>
                    <input
                      type="number" min="0" max="99"
                      bind:value={draft.health}
                      onfocus={selectOnFocus}
                      onwheel={blurOnWheel}
                      class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                    />
                  </label>
                  <label class="block flex-1">
                    <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('battlesManaLabel')}</span>
                    <input
                      type="number" min="0" max="99"
                      bind:value={draft.mana}
                      onfocus={selectOnFocus}
                      onwheel={blurOnWheel}
                      class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                    />
                  </label>
                  <label class="block flex-1">
                    <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('battlesCostLabel')}</span>
                    <input
                      type="number" min="0" max="20"
                      bind:value={draft.cost}
                      onfocus={selectOnFocus}
                      onwheel={blurOnWheel}
                      class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                    />
                  </label>
                  <label class="block flex-1">
                    <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('battlesPowerLabel')}</span>
                    <input
                      type="number" min="0" max="99"
                      bind:value={draft.power}
                      onfocus={selectOnFocus}
                      onwheel={blurOnWheel}
                      class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                    />
                  </label>
                </div>
              </div>

              <label class="block">
                <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesWork')}</span>
                <input
                  bind:value={workQuery}
                  placeholder={$t('adminBattlesSearch')}
                  class="w-full mb-1.5 px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                />
                <select
                  value={draft.figurineId ?? ''}
                  onchange={(e) => {
                    const id = e.currentTarget.value || null;
                    draft.figurineId = id;
                    const fig = id ? figurines.find((f) => f.id === id) : null;
                    if (fig && !draft.titleEn.trim()) draft.titleEn = fig.name;
                  }}
                  class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none"
                >
                  <option value="">{$t('adminBattlesWorkNone')}</option>
                  {#each visibleFigurines as fig (fig.id)}
                    <option value={fig.id}>{fig.name}</option>
                  {/each}
                </select>
              </label>

              {#if workTaken}
                <p class="text-[11px] leading-relaxed text-[#c65f3c]">{$t('adminBattlesWorkTaken')}</p>
              {/if}

              {#if draft.artUrlOverride}
                <p class="text-[11px] leading-relaxed italic text-[#8a6a55]">
                  {$t('adminBattlesArtOwn')}
                  <button
                    onclick={() => (draft.artUrlOverride = null)}
                    class="ml-1 not-italic underline decoration-dotted hover:text-[#c65f3c]"
                  >{$t('adminBattlesArtClear')}</button>
                </p>
              {:else}
                <p class="text-[11px] leading-relaxed italic text-[#8a6a55]">{$t('adminBattlesArtFromWork')}</p>
              {/if}

              <div class="pt-4 border-t border-[#34251c]/10">
                <p class="mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesAim')}</p>
                <p class="mb-3 text-[11px] leading-relaxed italic text-[#8a6a55]">{$t('adminBattlesAimHint')}</p>
                <div class="space-y-3">
                  <label class="block">
                    <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesFocusX')} · {(focal.x * 100).toFixed(0)}%</span>
                    <input type="range" min="0" max="1" step="0.01" value={focal.x} oninput={(e) => setFocal({ x: Number(e.currentTarget.value) })} class="w-full" />
                  </label>
                  <label class="block">
                    <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesFocusY')} · {(focal.y * 100).toFixed(0)}%</span>
                    <input type="range" min="0" max="1" step="0.01" value={focal.y} oninput={(e) => setFocal({ y: Number(e.currentTarget.value) })} class="w-full" />
                  </label>
                  <label class="block">
                    <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesZoom')} · {focal.zoom.toFixed(2)}</span>
                    <input type="range" min="1" max="3" step="0.05" value={focal.zoom} oninput={(e) => setFocal({ zoom: Number(e.currentTarget.value) })} class="w-full" />
                  </label>
                  <button
                    onclick={() => setFocal({ x: 0.5, y: 0.5, zoom: 1 })}
                    class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5"
                  >{$t('adminBattlesAimReset')}</button>
                </div>
              </div>

              <div class="pt-4 border-t border-[#34251c]/10">
                <p class="mb-3 text-[11px] leading-relaxed italic text-[#8a6a55]">{$t('adminBattlesPriceHint')}</p>
                <div class="flex gap-3">
                  <label class="block flex-1">
                    <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesPriceDust')}</span>
                    <input
                      type="number" min="0"
                      value={draft.priceDust ?? ''}
                      oninput={(e) => priceInput('priceDust', e.currentTarget.value)}
                      onfocus={selectOnFocus}
                      onwheel={blurOnWheel}
                      class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                    />
                  </label>
                  <label class="block flex-1">
                    <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesPriceFeed')}</span>
                    <input
                      type="number" min="0"
                      value={draft.priceFeed ?? ''}
                      oninput={(e) => priceInput('priceFeed', e.currentTarget.value)}
                      onfocus={selectOnFocus}
                      onwheel={blurOnWheel}
                      class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                    />
                  </label>
                </div>
              </div>

              <label class="block">
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

              <div class="flex items-center gap-3 pt-2">
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
          </aside>
        </div>
      </div>
    </div>
  {/if}
</div>
