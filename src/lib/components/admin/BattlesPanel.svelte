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
  import { t, lang, type TranslationKey } from '$lib/i18n';
  import BattleScene from '$lib/components/BattleScene.svelte';
  import {
    DEFAULT_FRAMES,
    FRAME_MODES,
    LAYOUTS,
    TIERS,
    applyInsetDelta,
    frameName,
    parseFocal,
    parseLevelFrames,
    pickImageFile,
    type FrameOverride,
    type InsetKey,
  } from '$lib/battles';
  import { SITE_FONTS } from '$lib/fonts';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import type {
    BattleCard as BattleCardDto,
    BattleDustRates,
    BattleCardStatus,
    BattleFrame,
    BattleRace,
    BattleKeyword,
    BattleWeigh,
    BattleAction,
    BattleEvent,
    BattleMatch,
    Bench,
    ChallengeSetup,
    AbilityVerb,
    AbilityShape,
    AbilityTrigger,
    FigurineListItem,
    SaveBattleCardRequest,
  } from '$lib/types/api';

  const REORDER_MS = 600;

  let view = $state<'cards' | 'frames' | 'races' | 'keywords' | 'bench'>('cards');
  let cards = $state<BattleCardDto[]>([]);
  let frames = $state<BattleFrame[]>([...DEFAULT_FRAMES]);
  let figurines = $state<FigurineListItem[]>([]);
  let races = $state<BattleRace[]>([]);
  let keywords = $state<BattleKeyword[]>([]);
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
  let keywordDraftId = $state<string | null>(null);
  let keywordNameEn = $state('');
  let keywordNameRu = $state('');
  let keywordRulesEn = $state('');
  let keywordRulesRu = $state('');
  let keywordPoints = $state<number | null>(null);
  let raceNameEn = $state('');
  let raceNameRu = $state('');
  let raceNoteEn = $state('');
  let raceNoteRu = $state('');
  let raceIconUrl = $state('');
  /** This race's own dress per level of an owned copy — 5 slots, index 0 = level 1. */
  let raceLevelFrames = $state<(FrameOverride | null)[]>([null, null, null, null, null]);
  /** Which level's slot the uploader and the sample card currently show. */
  let raceLevelPreview = $state(1);

  /**
   * Шесть чисел тела. Списком, а не шестью копиями разметки: они отличаются
   * только пределами, и разъехаться им незачем.
   */
  const bodyStats = [
    { key: 'armor', label: 'adminBattlesArmor', min: 0, max: 20 },
    { key: 'ward', label: 'adminBattlesWard', min: 0, max: 20 },
    { key: 'reach', label: 'adminBattlesReach', min: 0, max: 5 },
    { key: 'step', label: 'adminBattlesStep', min: 0, max: 3 },
    { key: 'speed', label: 'adminBattlesSpeed', min: 1, max: 5 },
    { key: 'mend', label: 'adminBattlesMend', min: 0, max: 20 },
  ] as const;

  /** Тот же порог, что в `battles.rs`: выше 1.15 перегруз, ниже 0.85 мертва. */
  function verdictWord(index: number): string {
    if (index > 1.15) return $t('adminBattlesOverloaded');
    if (index < 0.85) return $t('adminBattlesUnderweight');
    return $t('adminBattlesOnCurve');
  }

  function verdictColour(index: number): string {
    if (index > 1.15) return '#8f2f22';
    if (index < 0.85) return '#4d6673';
    return '#4a6141';
  }

  const tierBudget = (tier: number) => 8 + 6 * (Math.max(1, Math.min(5, tier)) - 1);

  // ── Стол ───────────────────────────────────────────────────────────────
  //
  // Ничего не записывает: расстановка и журнал целиком едут с каждым запросом,
  // доска пересобирается свёрткой. Поэтому со стола нельзя ни начислить пыль,
  // ни оставить мусор в базе, а свойство «журнал переигрывается» проверяется
  // на каждом клике.

  const BENCH_WIDTH = 3;
  const BENCH_DEPTH = 6;

  /** Кто может встать на поле: опубликованные и с ненулевым здоровьем. */
  let benchable = $derived(cards.filter((c) => c.status === 'published' && c.health > 0));

  let benchBoard = $state<Record<string, string>>({});
  let benchHands = $state<{ player: string[]; keeper: string[] }>({ player: [], keeper: [] });
  let benchJournal = $state<BattleAction[]>([]);
  let bench = $state<Bench | null>(null);
  let benchBusy = $state(false);
  let benchBoth = $state(false);
  let benchComplaint = $state<string | null>(null);

  let benchSetup = $derived.by((): ChallengeSetup => {
    const board = (half: 'keeper' | 'player') =>
      Object.entries(benchBoard)
        .filter(([key]) => {
          const y = Number(key.split(',')[1]);
          return half === 'keeper' ? y < 3 : y >= 3;
        })
        .map(([key, card]) => {
          const [x, y] = key.split(',').map(Number);
          return { card, x, y };
        });
    return {
      playerBoard: board('player'),
      playerHand: benchHands.player,
      keeperBoard: board('keeper'),
      keeperHand: benchHands.keeper,
    };
  });

  let benchReady = $derived(
    benchSetup.playerBoard.length > 0 && benchSetup.keeperBoard.length > 0,
  );

  /** Стол притворяется партией, чтобы доску рисовал тот же компонент. */
  let benchMatch = $derived.by((): BattleMatch | null =>
    bench
      ? {
          id: 'bench',
          challengeId: null,
          seq: 0,
          state: bench.state,
          legalActions: bench.legalActions,
          events: bench.events,
          outcome: bench.outcome,
          rewardDust: 0,
        }
      : null,
  );

  async function benchCall(next: BattleAction | null, playOut = false) {
    benchBusy = true;
    benchComplaint = null;
    try {
      const answer = await api.adminBenchBattle({
        setup: benchSetup,
        actions: benchJournal,
        next,
        // За обе стороны — хранитель молчит и ждёт, пока за него сходят.
        autoKeeper: !benchBoth,
        playOut,
      });
      bench = answer;
      benchJournal = answer.actions;
    } catch (e) {
      benchComplaint = String(e);
    } finally {
      benchBusy = false;
    }
  }

  function benchStart() {
    benchJournal = [];
    bench = null;
    void benchCall(null);
  }

  function benchReset() {
    benchBoard = {};
    benchHands = { player: [], keeper: [] };
    benchJournal = [];
    bench = null;
    benchComplaint = null;
  }

  function benchAddToHand(side: 'player' | 'keeper', slug: string) {
    if (!slug) return;
    benchHands = { ...benchHands, [side]: [...benchHands[side], slug] };
  }

  function benchDropFromHand(side: 'player' | 'keeper', at: number) {
    benchHands = { ...benchHands, [side]: benchHands[side].filter((_, i) => i !== at) };
  }

  const benchTitle = (slug: string) => cards.find((c) => c.slug === slug)?.titleRu || slug;

  /** Событие одной строкой, с разбором урона там, где он есть. */
  function benchLine(e: BattleEvent): string {
    if (typeof e !== 'object') return String(e);
    if ('played' in e) return `выставлен ${benchUnitName(e.played.unit)} за ${e.played.cost}`;
    if ('moved' in e)
      return `${benchUnitName(e.moved.unit)}: ${e.moved.from.x},${e.moved.from.y} → ${e.moved.to.x},${e.moved.to.y}`;
    if ('damaged' in e) {
      const d = e.damaged;
      const why = d.trail.map((b) => `${b.step}: ${b.from}→${b.to}`).join('; ');
      const total = d.toHealth + d.toShield;
      return `${benchUnitName(d.target)} получает ${total}${why ? `  [${why}]` : ''}`;
    }
    if ('healed' in e) return `${benchUnitName(e.healed.target)} залечен на ${e.healed.amount}`;
    if ('immune' in e) return `${benchUnitName(e.immune.target)} не почувствовал`;
    if ('died' in e) return `пал ${benchUnitName(e.died.target)}`;
    if ('turnEnded' in e) return `— ход ${e.turnEnded.side === 'player' ? 'гостя' : 'хранителя'} окончен`;
    if ('finished' in e) return `итог: ${e.finished.outcome}`;
    return '';
  }

  const benchUnitName = (id: number) =>
    bench ? benchTitle(bench.state.units[id]?.card.name ?? String(id)) : String(id);

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
      raceLevelFrames: null,
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
      kind: 'unit',
      armor: 0,
      ward: 0,
      attackChannel: 'physical',
      reach: 1,
      step: 1,
      speed: 3,
      mend: 0,
      abilities: [],
      budgetPoints: null,
      balanceIndex: null,
      rulesVersion: 1,
      priceDust: null,
      priceFeed: null,
      levelPriceDust: null,
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
    raceLevelFrames: raceLevelFrames.some((f) => f) ? JSON.stringify(raceLevelFrames) : null,
  });

  /** A frame picture, kept transparent, stretched to the card's fixed ratio. */
  async function uploadFrameArt() {
    const file = await pickImageFile();
    if (!file) return;
    uploading = true;
    try {
      const art = await api.adminUploadBattleFrameArt(file);
      const frame = frames[frameIndex];
      frame.frameImage = art.url;
      // The card's ratio is fixed game-wide; the picture is stretched to fit
      // it, not the other way around, so different frame uploads can never
      // leave cards different shapes. The aspect slider still overrides it.
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

  function openKeyword(keyword: BattleKeyword | null) {
    keywordDraftId = keyword?.id ?? null;
    keywordNameEn = keyword?.nameEn ?? '';
    keywordNameRu = keyword?.nameRu ?? '';
    keywordRulesEn = keyword?.rulesEn ?? '';
    keywordRulesRu = keyword?.rulesRu ?? '';
    keywordPoints = keyword?.pointValue ?? null;
  }

  async function saveKeyword() {
    saving = true;
    try {
      await api.adminSaveBattleKeyword(
        {
          nameEn: keywordNameEn.trim(),
          nameRu: keywordNameRu.trim(),
          rulesEn: keywordRulesEn.trim() || null,
          rulesRu: keywordRulesRu.trim() || null,
          pointValue: keywordPoints,
        },
        keywordDraftId ?? undefined,
      );
      keywords = await api.getBattleKeywords();
      openKeyword(null);
      flash($t('adminBattlesSaved'));
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      saving = false;
    }
  }

  async function removeKeyword(keyword: BattleKeyword) {
    if (!confirm($t('adminBattlesKeywordDelete') + '?')) return;
    try {
      await api.adminDeleteBattleKeyword(keyword.id);
      keywords = await api.getBattleKeywords();
      if (keywordDraftId === keyword.id) openKeyword(null);
      flash($t('adminBattlesDeleted'));
    } catch (e) {
      flash(String(e), 6000);
    }
  }

  function openRace(race: BattleRace | null) {
    raceDraftId = race?.id ?? null;
    raceNameEn = race?.nameEn ?? '';
    raceNameRu = race?.nameRu ?? '';
    raceNoteEn = race?.noteEn ?? '';
    raceNoteRu = race?.noteRu ?? '';
    raceIconUrl = race?.iconUrl ?? '';
    raceLevelFrames = parseLevelFrames(race?.levelFrames);
    raceLevelPreview = 1;
  }

  async function saveRace() {
    saving = true;
    try {
      const saved = await api.adminSaveBattleRace(
        {
          nameEn: raceNameEn.trim(),
          nameRu: raceNameRu.trim(),
          noteEn: raceNoteEn.trim() || null,
          noteRu: raceNoteRu.trim() || null,
          iconUrl: raceIconUrl.trim() || null,
          levelFrames: raceLevelFrames.some((f) => f) ? JSON.stringify(raceLevelFrames) : null,
        },
        raceDraftId ?? undefined,
      );
      // Not re-fetched: the race dictionary is a cached public read (an hour
      // on the shelf, a minute here), so a re-fetch right after saving could
      // still hand back what was true before this save. The save's own
      // response is already the truth, no round-trip needed to confirm it.
      races = raceDraftId
        ? races.map((r) => (r.id === saved.id ? saved : r))
        : [...races, saved];
      openRace(null);
      flash($t('adminBattlesRaceSaved'));
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      saving = false;
    }
  }

  /** A picture for this one level of this race, kept transparent, stretched
   *  to the card's fixed ratio — the same choice a tier's own frame makes. */
  async function uploadRaceLevelFrame(index: number) {
    const file = await pickImageFile();
    if (!file) return;
    uploading = true;
    try {
      const art = await api.adminUploadBattleFrameArt(file);
      const patch: FrameOverride = {
        frameImage: art.url,
        frameMode: art.hasAlpha ? 'overlay' : 'behind',
      };
      if (!art.hasAlpha) flash($t('adminBattlesFrameNoAlpha'), 8000);
      raceLevelFrames[index] = patch;
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      uploading = false;
    }
  }

  function clearRaceLevelFrame(index: number) {
    raceLevelFrames[index] = null;
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
    draft.raceLevelFrames = picked?.levelFrames ?? null;
  }

  const TRAITS_MAX = 8;

  // ── Способности ────────────────────────────────────────────────────────
  //
  // Списки закрыты и повторяют `battles.rs`. Повторяются намеренно: сервер
  // отбрасывает неизвестный глагол молча, а форма не должна давать его выбрать.

  const ABILITIES_MAX = 6;

  // Ключ перевода рядом с самим значением, а не собранный из строки: тогда
  // забытый в словаре глагол — ошибка компиляции, а не «battlesVerbFoo» на
  // экране. Порядок записи здесь и есть порядок в списке.
  const VERB_LABELS = {
    damage: 'battlesVerbDamage',
    dot: 'battlesVerbDot',
    heal: 'battlesVerbHeal',
    hot: 'battlesVerbHot',
    shield: 'battlesVerbShield',
    zone: 'battlesVerbZone',
    bless: 'battlesVerbBless',
    curse: 'battlesVerbCurse',
    control: 'battlesVerbControl',
    silence: 'battlesVerbSilence',
    disarm: 'battlesVerbDisarm',
    charm: 'battlesVerbCharm',
    veil: 'battlesVerbVeil',
    guard: 'battlesVerbGuard',
    immune: 'battlesVerbImmune',
    thorns: 'battlesVerbThorns',
    move: 'battlesVerbMove',
    summon: 'battlesVerbSummon',
    sacrifice: 'battlesVerbSacrifice',
    cleanse: 'battlesVerbCleanse',
    dispel: 'battlesVerbDispel',
    mana: 'battlesVerbMana',
  } as const satisfies Record<AbilityVerb, TranslationKey>;

  const SHAPE_LABELS = {
    self: 'battlesShapeSelf',
    one: 'battlesShapeOne',
    adjacent: 'battlesShapeAdjacent',
    chain: 'battlesShapeChain',
    line: 'battlesShapeLine',
    radius: 'battlesShapeRadius',
    side: 'battlesShapeSide',
    cell: 'battlesShapeCell',
  } as const satisfies Record<AbilityShape, TranslationKey>;

  const TRIGGER_LABELS = {
    active: 'battlesTriggerActive',
    onPlay: 'battlesTriggerOnPlay',
    onHit: 'battlesTriggerOnHit',
    onDamaged: 'battlesTriggerOnDamaged',
    onDeath: 'battlesTriggerOnDeath',
    turnStart: 'battlesTriggerTurnStart',
    aura: 'battlesTriggerAura',
    once: 'battlesTriggerOnce',
  } as const satisfies Record<AbilityTrigger, TranslationKey>;

  const VERBS = Object.keys(VERB_LABELS) as AbilityVerb[];
  const SHAPES = Object.keys(SHAPE_LABELS) as AbilityShape[];
  const TRIGGERS = Object.keys(TRIGGER_LABELS) as AbilityTrigger[];

  /** Только `chain` и `radius` несут число; у остальных поле нечего заполнять. */
  const shapeCarriesNumber = (shape: string) => shape === 'chain' || shape === 'radius';

  function addAbility() {
    const list = draft.abilities ?? [];
    if (list.length >= ABILITIES_MAX) return;
    draft.abilities = [
      ...list,
      {
        // Собственный id внутри карты: по нему весы кладут число к нужной строке.
        id: `a${Date.now().toString(36)}`,
        nameEn: '',
        nameRu: '',
        verb: 'damage',
        channel: 'physical',
        amount: 1,
        shape: 'one',
        radius: 1,
        range: 1,
        duration: 0,
        trigger: 'active',
        manaCost: 0,
        cooldown: 0,
        keywords: [],
      },
    ];
  }

  function removeAbility(at: number) {
    draft.abilities = (draft.abilities ?? []).filter((_, i) => i !== at);
  }

  function moveAbility(at: number, by: number) {
    const list = draft.abilities ?? [];
    const to = at + by;
    if (to < 0 || to >= list.length) return;
    const next = [...list];
    [next[at], next[to]] = [next[to], next[at]];
    draft.abilities = next;
  }

  function keywordsInput(at: number, raw: string) {
    const list = [...(draft.abilities ?? [])];
    list[at] = {
      ...list[at],
      keywords: raw.split(',').map((k) => k.trim()).filter(Boolean).slice(0, 4),
    };
    draft.abilities = list;
  }

  const abilityPoints = (id: string) =>
    weigh?.abilities.find((a) => a.id === id)?.points ?? null;

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

  /**
   * Цена одной ступени уровня. Ступеней всегда четыре — 1→2, 2→3, 3→4, 4→5, —
   * поэтому лестница либо есть целиком, либо её нет: сервер отказывает списку
   * другой длины, а не дополняет его нулями (карта с молча подаренными
   * ступенями обнаружится тем, кто по ним поднялся).
   *
   * Опустошить любое поле — значит убрать лестницу целиком, и это честнее, чем
   * оставить карту с тремя ценами из четырёх.
   */
  function levelPriceInput(step: number, raw: string) {
    const text = raw.trim();
    const ladder = draft.levelPriceDust ?? [0, 0, 0, 0];
    if (text === '') {
      draft.levelPriceDust = null;
      return;
    }
    const next = [...ladder];
    next[step] = Math.max(0, Math.round(Number(text)));
    draft.levelPriceDust = next;
  }

  // ── Ставки начисления за внимание ──────────────────────────────────────
  let dustRates = $state<BattleDustRates>({ liked: 2, seen: 1, read: 3 });
  let ratesSaving = $state(false);

  async function loadDustRates() {
    try {
      dustRates = await api.adminGetBattleDustRates();
    } catch {
      // Настройка не прочиталась — стол показывает умолчания и не мешает.
    }
  }

  async function saveDustRates() {
    ratesSaving = true;
    try {
      dustRates = await api.adminSaveBattleDustRates(dustRates);
      flash($t('adminBattlesRatesSaved'), 2500);
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      ratesSaving = false;
    }
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

  /**
   * Тело запроса — одно и то же для сохранения и для весов.
   *
   * Не ради краткости: если бы весы собирали карту по-своему, они однажды
   * начали бы взвешивать не ту карту, которую записывает кнопка, и разошлись бы
   * молча.
   */
  function cardBody(): SaveBattleCardRequest {
    return {
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
      kind: draft.kind,
      armor: draft.armor,
      ward: draft.ward,
      attackChannel: draft.attackChannel,
      reach: draft.reach,
      step: draft.step,
      speed: draft.speed,
      mend: draft.mend,
      // Прозу движок не читает; исполняемая половина едет отдельно.
      abilities: draft.abilities ?? [],
      raceId: draft.raceId || null,
      typeEn: draft.typeEn?.trim() || null,
      typeRu: draft.typeRu?.trim() || null,
      priceDust: draft.priceDust,
      priceFeed: draft.priceFeed,
      levelPriceDust: draft.levelPriceDust,
      artUrl: draft.artUrlOverride?.trim() || null,
      artFocal: draft.artFocal,
      frameOverride: draft.frameOverride,
      figurineId: draft.figurineId || null,
    };
  }

  let weigh = $state<BattleWeigh | null>(null);
  let weighTimer: ReturnType<typeof setTimeout> | undefined;

  /** Всё, что меняет вес. Читается целиком, иначе эффект не переподпишется. */
  let weighKey = $derived(
    JSON.stringify({
      tier: draft.tier,
      cost: draft.cost,
      health: draft.health,
      power: draft.power,
      armor: draft.armor,
      ward: draft.ward,
      reach: draft.reach,
      speed: draft.speed,
      mend: draft.mend,
      abilities: draft.abilities,
    }),
  );

  // Считает сервер, по той же формуле, что при сохранении. Задержка — чтобы
  // набор числа руками не превращался в очередь запросов.
  $effect(() => {
    weighKey;
    clearTimeout(weighTimer);
    weighTimer = setTimeout(async () => {
      try {
        weigh = await api.adminWeighBattleCard(cardBody());
      } catch {
        // Весы — подсказка, а не условие сохранения. Молча гаснут.
        weigh = null;
      }
    }, 250);
    return () => clearTimeout(weighTimer);
  });

  async function save() {
    saving = true;
    try {
      const body = cardBody();      const saved = await api.adminSaveBattleCard(body, selectedId ?? undefined);
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
        loadDustRates(),
      ]);
      figurines = figs;
      races = savedRaces;
      keywords = await api.getBattleKeywords();
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
      <button
        onclick={() => (view = 'keywords')}
        class="px-3 py-1 {view === 'keywords' ? 'bg-[#34251c] text-[#f8f1e7]' : ''}"
      >{$t('adminBattlesKeywords')}</button>
      <button
        onclick={() => (view = 'bench')}
        class="px-3 py-1 {view === 'bench' ? 'bg-[#34251c] text-[#f8f1e7]' : ''}"
      >{$t('adminBattlesBench')}</button>
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

        <!-- Ставки начисления. Стоят рядом с рамками, потому что и то и другое —
             настройка комнаты целиком, а не свойство одной карты. -->
        <div class="mb-8 pb-6 border-b border-[#34251c]/10">
          <p class="mb-1 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesRates')}</p>
          <p class="max-w-[62ch] mb-3 text-[11px] leading-relaxed italic text-[#8a6a55]">{$t('adminBattlesRatesHint')}</p>
          <div class="flex flex-wrap items-end gap-3">
            {#each [
              { key: 'liked' as const, label: $t('adminBattlesRateLiked') },
              { key: 'seen' as const, label: $t('adminBattlesRateSeen') },
              { key: 'read' as const, label: $t('adminBattlesRateRead') },
            ] as row (row.key)}
              <label class="block w-40">
                <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{row.label}</span>
                <input
                  type="number" min="0"
                  value={dustRates[row.key]}
                  oninput={(e) => (dustRates[row.key] = Math.max(0, Math.round(Number(e.currentTarget.value) || 0)))}
                  onfocus={selectOnFocus}
                  onwheel={blurOnWheel}
                  class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                />
              </label>
            {/each}
            <button
              type="button"
              disabled={ratesSaving}
              onclick={saveDustRates}
              class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/25 hover:bg-[#34251c]/5 disabled:opacity-40"
            >{$t('adminBattlesRatesSave')}</button>
          </div>
        </div>

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
  {:else if view === 'bench'}
    <!--
      ── Стол хранителя ────────────────────────────────────────────────────
      Доску рисует тот же `BattleScene`, что и комната гостей: второй
      отрисовщик неизбежно разошёлся бы с первым, и стол начал бы врать ровно
      про то, что на нём проверяют.
    -->
    <div class="flex-1 flex min-h-0">
      <div class="flex-1 overflow-y-auto p-6 min-w-0">
        <p class="max-w-[62ch] mb-1 text-xs leading-relaxed text-[#5f4636]">{$t('adminBattlesBenchHint')}</p>
        <p class="max-w-[62ch] mb-5 text-[11px] leading-relaxed text-[#8a6a55]">{$t('adminBattlesBenchNoHealth')}</p>

        {#if benchComplaint}
          <p class="mb-4 text-xs text-[#8f2f22]">{benchComplaint}</p>
        {/if}

        <div class="flex flex-wrap gap-8 items-start">
          <!-- Расстановка. Клетка — это просто выпадающий список: перетаскивание
               здесь ничего не проверяет, а сломать может. -->
          <div class="w-[22rem]">
            <p class="mb-2 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesBenchPlace')}</p>
            {#each Array.from({ length: BENCH_DEPTH }, (_, y) => y) as y (y)}
              <div class="flex gap-1 mb-1 {y === 3 ? 'mt-2 pt-2 border-t border-dashed border-[#34251c]/20' : ''}">
                {#each Array.from({ length: BENCH_WIDTH }, (_, x) => x) as x (x)}
                  <select
                    value={benchBoard[`${x},${y}`] ?? ''}
                    onchange={(e) => {
                      const slug = e.currentTarget.value;
                      const next = { ...benchBoard };
                      if (slug) next[`${x},${y}`] = slug;
                      else delete next[`${x},${y}`];
                      benchBoard = next;
                    }}
                    class="flex-1 min-w-0 px-1 py-1 text-[11px] bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                  >
                    <option value="">·</option>
                    {#each benchable as c (c.id)}
                      <option value={c.slug}>{c.titleRu}</option>
                    {/each}
                  </select>
                {/each}
              </div>
            {/each}
            <p class="mt-1 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">
              {$t('adminBattlesBenchKeeperHalf')} ↑ · {$t('adminBattlesBenchGuestHalf')} ↓
            </p>
          </div>

          <!-- Руки обеих сторон. -->
          <div class="w-64 flex flex-col gap-4">
            {#each [{ side: 'keeper' as const, label: $t('adminBattlesBenchKeeperHalf') }, { side: 'player' as const, label: $t('adminBattlesBenchGuestHalf') }] as row (row.side)}
              <div>
                <p class="mb-1 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesBenchHand')} · {row.label}</p>
                <select
                  value=""
                  onchange={(e) => { benchAddToHand(row.side, e.currentTarget.value); e.currentTarget.value = ''; }}
                  class="w-full px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                >
                  <option value="">+</option>
                  {#each benchable as c (c.id)}
                    <option value={c.slug}>{c.titleRu}</option>
                  {/each}
                </select>
                <div class="mt-1 flex flex-wrap gap-1">
                  {#each benchHands[row.side] as slug, i (i)}
                    <button
                      type="button"
                      onclick={() => benchDropFromHand(row.side, i)}
                      class="px-1.5 py-0.5 text-[11px] border border-[#34251c]/15 hover:bg-[#c65f3c]/10"
                    >{benchTitle(slug)} ×</button>
                  {:else}
                    <span class="text-[11px] italic text-[#8a6a55]">{$t('adminBattlesBenchEmpty')}</span>
                  {/each}
                </div>
              </div>
            {/each}

            <label class="flex items-center gap-2 text-[11px] text-[#5f4636]">
              <input type="checkbox" bind:checked={benchBoth} />
              {$t('adminBattlesBenchBothSides')}
            </label>

            <div class="flex flex-wrap gap-2">
              <button
                type="button"
                disabled={!benchReady || benchBusy}
                onclick={benchStart}
                class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] bg-[#34251c] text-[#f8f1e7] disabled:opacity-40"
              >{$t('adminBattlesBenchStart')}</button>
              <button
                type="button"
                disabled={!benchReady || benchBusy}
                onclick={() => benchCall(null, true)}
                class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/25 disabled:opacity-40"
              >{$t('adminBattlesBenchPlayOut')}</button>
              <button
                type="button"
                onclick={benchReset}
                class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20"
              >{$t('adminBattlesBenchReset')}</button>
            </div>
            {#if !benchReady}
              <p class="text-[11px] italic text-[#8a6a55]">{$t('adminBattlesBenchNeedsBodies')}</p>
            {/if}
          </div>

          <!-- Сама партия. -->
          {#if benchMatch}
            <div class="w-[24rem]">
              <!-- Исход рисует сама сцена — сургучной печатью. Второй строки
                   под доской здесь больше нет: стол должен видеть ровно то,
                   что увидит гость. -->
              <BattleScene
                match={benchMatch}
                {cards}
                {frames}
                busy={benchBusy}
                control={benchBoth ? 'both' : 'player'}
                onact={(a) => benchCall(a)}
              />
            </div>
          {/if}
        </div>

        <!-- Журнал. Ради разбора урона он и нужен: видно, почему три, а не восемь. -->
        {#if bench && bench.events.length}
          <div class="mt-8 max-w-3xl">
            <p class="mb-2 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesBenchLog')}</p>
            <ul class="text-[11px] leading-relaxed text-[#5f4636] font-mono">
              {#each bench.events as e, i (i)}
                <li>{benchLine(e)}</li>
              {/each}
            </ul>
          </div>
        {/if}
      </div>
    </div>

  {:else if view === 'keywords'}
    <!--
      ── The keyword dictionary ────────────────────────────────────────────
      A rule worded once and priced once. `pointValue` is why this is a table
      and not a constant in the server: rebalancing the whole game is an edit
      here, not a deployment.
    -->
    <div class="flex-1 flex min-h-0">
      <div class="flex-1 overflow-y-auto p-6 min-w-0">
        <p class="max-w-[62ch] mb-5 text-xs leading-relaxed text-[#5f4636]">{$t('adminBattlesKeywordsHint')}</p>

        {#if !keywords.length}
          <p class="mb-5 text-xs italic text-[#5f4636]">{$t('adminBattlesKeywordsEmpty')}</p>
        {:else}
          <ul class="max-w-2xl mb-6 border-t border-[#34251c]/10">
            {#each keywords as keyword (keyword.id)}
              <li class="flex items-center gap-3 py-2 border-b border-[#34251c]/10">
                <button
                  onclick={() => openKeyword(keyword)}
                  class="flex-1 text-left hover:text-[#c65f3c] {keywordDraftId === keyword.id ? 'text-[#c65f3c]' : ''}"
                >
                  <span class="text-sm" style="font-family: 'Cormorant Garamond', Georgia, serif;">{keyword.nameRu}</span>
                  <span class="ml-2 text-[11px] text-[#8a6a55]">{keyword.nameEn}</span>
                  {#if keyword.rulesRu}
                    <span class="block text-[11px] leading-snug text-[#8a6a55]">{keyword.rulesRu}</span>
                  {/if}
                </button>
                <span class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] tabular-nums">
                  {keyword.pointValue == null ? '—' : keyword.pointValue.toFixed(2)}
                </span>
                <button
                  onclick={() => removeKeyword(keyword)}
                  class="px-2 py-1 text-xs border border-[#34251c]/20 text-[#6f3b24] hover:bg-[#c65f3c]/10"
                >×</button>
              </li>
            {/each}
          </ul>
        {/if}

        <div class="max-w-2xl p-4 border border-[#34251c]/12">
          <p class="mb-3 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">
            {keywordDraftId ? $t('adminBattlesKeywordName') : $t('adminBattlesKeywordAdd')}
          </p>
          <div class="flex flex-wrap gap-3">
            <label class="block w-52">
              <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesKeywordName')} · RU</span>
              <input bind:value={keywordNameRu} maxlength="60" class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35" />
            </label>
            <label class="block w-52">
              <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesKeywordName')} · EN</span>
              <input bind:value={keywordNameEn} maxlength="60" class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35" />
            </label>
            <label class="block w-40">
              <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesKeywordPoints')}</span>
              <input
                type="number" min="0" max="100" step="0.05"
                bind:value={keywordPoints}
                onfocus={selectOnFocus}
                onwheel={blurOnWheel}
                class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
              />
            </label>
          </div>
          <div class="flex flex-wrap gap-3 mt-3">
            <label class="block flex-1 min-w-[14rem]">
              <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesKeywordRules')} · RU</span>
              <input bind:value={keywordRulesRu} maxlength="300" class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35" />
            </label>
            <label class="block flex-1 min-w-[14rem]">
              <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesKeywordRules')} · EN</span>
              <input bind:value={keywordRulesEn} maxlength="300" class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35" />
            </label>
          </div>
          <div class="flex items-center gap-3 mt-4">
            <button
              onclick={saveKeyword}
              disabled={saving}
              class="px-4 py-2 text-[10px] uppercase tracking-[0.16em] bg-[#34251c] text-[#f8f1e7] disabled:opacity-40"
            >{$t('adminBattlesSave')}</button>
            {#if keywordDraftId}
              <button
                onclick={() => openKeyword(null)}
                class="px-4 py-2 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20"
              >{$t('adminBattlesKeywordAdd')}</button>
            {/if}
          </div>
        </div>
      </div>
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
          level={raceLevelPreview}
          transition={false}
          interactive={false}
          raceIconEditable={true}
          frameEditable={!!raceLevelFrames[raceLevelPreview - 1]}
          frameEditTarget={raceLevelFrames[raceLevelPreview - 1]}
          onIconUpload={(url) => (raceIconUrl = url)}
          onError={(e) => flash(e, 6000)}
        />
        {#if raceIconUrl.trim()}
          <button
            onclick={() => (raceIconUrl = '')}
            class="mt-3 px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5"
          >{$t('adminBattlesFrameArtClear')}</button>
        {/if}

        <p class="mt-6 mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesRaceLevelFrames')}</p>
        <p class="max-w-[62ch] mb-3 text-[11px] leading-relaxed italic text-[#8a6a55]">{$t('adminBattlesRaceLevelFramesHint')}</p>
        <div class="flex gap-2">
          {#each [1, 2, 3, 4, 5] as lvl (lvl)}
            <button
              onclick={() => (raceLevelPreview = lvl)}
              class="w-11 h-11 flex items-center justify-center text-xs border {raceLevelPreview === lvl ? 'border-[#c65f3c] text-[#c65f3c]' : 'border-[#34251c]/20 text-[#5f4636]'}"
              style={raceLevelFrames[lvl - 1]?.frameImage
                ? `background-image:url("${raceLevelFrames[lvl - 1]?.frameImage}");background-size:100% 100%;background-repeat:no-repeat;`
                : ''}
            >{#if !raceLevelFrames[lvl - 1]?.frameImage}{lvl}{/if}</button>
          {/each}
        </div>
        <div class="flex items-center gap-3 mt-3">
          <button
            onclick={() => uploadRaceLevelFrame(raceLevelPreview - 1)}
            disabled={uploading}
            class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5 disabled:opacity-40"
          >{$t('adminBattlesFrameArtUpload')}</button>
          {#if raceLevelFrames[raceLevelPreview - 1]}
            <button
              onclick={() => clearRaceLevelFrame(raceLevelPreview - 1)}
              class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5"
            >{$t('adminBattlesFrameArtClear')}</button>
          {/if}
        </div>
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

                <!--
                  Тело для движка. Отделено от прозы выше не рамкой ради красоты:
                  проза печатается на карте и читается человеком, эти числа читает
                  только движок, и путать их — та самая ошибка, из-за которой
                  правила пытаются разобрать естественный язык.
                -->
                <div class="mt-4 pt-3 border-t border-dashed border-[#34251c]/15">
                  <div class="flex items-baseline justify-between gap-3 mb-2">
                    <span class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesBody')}</span>
                    {#if draft.balanceIndex != null && draft.budgetPoints != null}
                      <span class="text-[11px] tabular-nums" style="color: {verdictColour(draft.balanceIndex)}">
                        {draft.budgetPoints.toFixed(1)} · {verdictWord(draft.balanceIndex)}
                        <span class="text-[#8a6a55]">({$t('adminBattlesBudget')} {tierBudget(draft.tier)})</span>
                      </span>
                    {:else}
                      <span class="text-[11px] text-[#8a6a55]">{$t('adminBattlesScalesPending')}</span>
                    {/if}
                  </div>
                  <p class="mb-2.5 text-[11px] leading-snug text-[#8a6a55]">{$t('adminBattlesBodyHint')}</p>

                  <div class="flex gap-3 mb-2">
                    <label class="block flex-1">
                      <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesKind')}</span>
                      <select bind:value={draft.kind} class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35">
                        <option value="unit">{$t('adminBattlesKindUnit')}</option>
                        <option value="spell">{$t('adminBattlesKindSpell')}</option>
                        <option value="relic">{$t('adminBattlesKindRelic')}</option>
                      </select>
                    </label>
                    <label class="block flex-1">
                      <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesChannel')}</span>
                      <select bind:value={draft.attackChannel} class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35">
                        <option value="physical">{$t('adminBattlesChannelPhysical')}</option>
                        <option value="magic">{$t('adminBattlesChannelMagic')}</option>
                        <option value="pure">{$t('adminBattlesChannelPure')}</option>
                        <option value="none">{$t('adminBattlesChannelNone')}</option>
                      </select>
                    </label>
                  </div>

                  <div class="flex gap-3">
                    {#each bodyStats as stat (stat.key)}
                      <label class="block flex-1">
                        <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t(stat.label)}</span>
                        <input
                          type="number" min={stat.min} max={stat.max}
                          bind:value={draft[stat.key]}
                          onfocus={selectOnFocus}
                          onwheel={blurOnWheel}
                          class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                        />
                      </label>
                    {/each}
                  </div>
                </div>

                <!--
                  ── Способности для движка ───────────────────────────────
                  Числа справа считает сервер той же формулой, что и при
                  сохранении: браузер не знает ни одного курса.
                -->
                <div class="mt-4 pt-3 border-t border-dashed border-[#34251c]/15">
                  <div class="flex items-baseline justify-between gap-3 mb-2">
                    <span class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesAbilities')}</span>
                    {#if weigh}
                      <span class="text-[11px] tabular-nums" style="color: {verdictColour(weigh.balanceIndex)}">
                        {$t('adminBattlesWeighTotal')} {weigh.totalPoints.toFixed(1)}
                        <span class="text-[#8a6a55]">({$t('adminBattlesWeighBody')} {weigh.bodyPoints.toFixed(1)})</span>
                        · {verdictWord(weigh.balanceIndex)}
                        {#if weigh.suggestedCost !== draft.cost}
                          <span class="text-[#8a6a55]">· {$t('adminBattlesWeighSuggested')} {weigh.suggestedCost}</span>
                        {/if}
                      </span>
                    {/if}
                  </div>
                  <p class="mb-2.5 text-[11px] leading-snug text-[#8a6a55]">{$t('adminBattlesAbilitiesHint')}</p>

                  <div class="flex flex-col gap-2">
                    {#each draft.abilities ?? [] as ability, i (ability.id)}
                      <div class="p-2.5 border border-[#34251c]/12 bg-[#34251c]/[0.02]">
                        <div class="flex items-start gap-2">
                          <div class="flex-1 min-w-0 flex flex-col gap-2">
                            <div class="flex flex-wrap gap-2">
                              <label class="block w-44">
                                <span class="block mb-0.5 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesAbilityVerb')}</span>
                                <select bind:value={draft.abilities[i].verb} class="w-full px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35">
                                  {#each VERBS as verb (verb)}
                                    <option value={verb}>{$t(VERB_LABELS[verb])}</option>
                                  {/each}
                                </select>
                              </label>
                              <label class="block w-20">
                                <span class="block mb-0.5 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesAbilityAmount')}</span>
                                <input type="number" min="0" max="99" bind:value={draft.abilities[i].amount} onfocus={selectOnFocus} onwheel={blurOnWheel} class="w-full px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35" />
                              </label>
                              <label class="block w-44">
                                <span class="block mb-0.5 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesAbilityShape')}</span>
                                <select bind:value={draft.abilities[i].shape} class="w-full px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35">
                                  {#each SHAPES as shape (shape)}
                                    <option value={shape}>{$t(SHAPE_LABELS[shape])}</option>
                                  {/each}
                                </select>
                              </label>
                              <!-- Число несут только цепь и радиус; у остальных заполнять нечего. -->
                              <label class="block w-24" class:opacity-30={!shapeCarriesNumber(ability.shape)}>
                                <span class="block mb-0.5 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesAbilityRadius')}</span>
                                <input type="number" min="0" max="3" disabled={!shapeCarriesNumber(ability.shape)} bind:value={draft.abilities[i].radius} onfocus={selectOnFocus} onwheel={blurOnWheel} class="w-full px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35" />
                              </label>
                              <label class="block w-20">
                                <span class="block mb-0.5 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesAbilityRange')}</span>
                                <input type="number" min="0" max="5" bind:value={draft.abilities[i].range} onfocus={selectOnFocus} onwheel={blurOnWheel} class="w-full px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35" />
                              </label>
                              <label class="block w-20">
                                <span class="block mb-0.5 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesAbilityDuration')}</span>
                                <input type="number" min="0" max="5" bind:value={draft.abilities[i].duration} onfocus={selectOnFocus} onwheel={blurOnWheel} class="w-full px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35" />
                              </label>
                            </div>

                            <div class="flex flex-wrap gap-2">
                              <label class="block w-52">
                                <span class="block mb-0.5 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesAbilityTrigger')}</span>
                                <select bind:value={draft.abilities[i].trigger} class="w-full px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35">
                                  {#each TRIGGERS as trigger (trigger)}
                                    <option value={trigger}>{$t(TRIGGER_LABELS[trigger])}</option>
                                  {/each}
                                </select>
                              </label>
                              <label class="block w-36">
                                <span class="block mb-0.5 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesChannel')}</span>
                                <select bind:value={draft.abilities[i].channel} class="w-full px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35">
                                  <option value="physical">{$t('adminBattlesChannelPhysical')}</option>
                                  <option value="magic">{$t('adminBattlesChannelMagic')}</option>
                                  <option value="pure">{$t('adminBattlesChannelPure')}</option>
                                  <option value="none">{$t('adminBattlesChannelNone')}</option>
                                </select>
                              </label>
                              <label class="block w-20">
                                <span class="block mb-0.5 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesAbilityMana')}</span>
                                <input type="number" min="0" max="20" bind:value={draft.abilities[i].manaCost} onfocus={selectOnFocus} onwheel={blurOnWheel} class="w-full px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35" />
                              </label>
                              <label class="block w-24">
                                <span class="block mb-0.5 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesAbilityCooldown')}</span>
                                <input type="number" min="0" max="5" bind:value={draft.abilities[i].cooldown} onfocus={selectOnFocus} onwheel={blurOnWheel} class="w-full px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35" />
                              </label>
                            </div>

                            <div class="flex flex-wrap gap-2">
                              <label class="block flex-1 min-w-[10rem]">
                                <span class="block mb-0.5 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesTraitName')} · RU</span>
                                <input bind:value={draft.abilities[i].nameRu} maxlength="60" class="w-full px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35" />
                              </label>
                              <label class="block flex-1 min-w-[10rem]">
                                <span class="block mb-0.5 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesTraitName')} · EN</span>
                                <input bind:value={draft.abilities[i].nameEn} maxlength="60" class="w-full px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35" />
                              </label>
                              <label class="block flex-1 min-w-[12rem]">
                                <span class="block mb-0.5 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminBattlesAbilityKeywords')}</span>
                                <input
                                  value={(ability.keywords ?? []).join(', ')}
                                  oninput={(e) => keywordsInput(i, e.currentTarget.value)}
                                  class="w-full px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                                />
                              </label>
                            </div>
                          </div>

                          <div class="flex flex-col items-end gap-1 flex-shrink-0">
                            <!-- Цена именно этой строки, посчитанная сервером. -->
                            <span class="text-[11px] tabular-nums text-[#6f3b24]">
                              {abilityPoints(ability.id)?.toFixed(1) ?? '—'}
                              <span class="text-[9px] text-[#8a6a55]">{$t('adminBattlesAbilityPoints')}</span>
                            </span>
                            <div class="flex flex-col gap-0.5">
                              <button type="button" onclick={() => moveAbility(i, -1)} disabled={i === 0} class="px-1.5 text-xs border border-[#34251c]/20 disabled:opacity-30">↑</button>
                              <button type="button" onclick={() => moveAbility(i, 1)} disabled={i === (draft.abilities?.length ?? 0) - 1} class="px-1.5 text-xs border border-[#34251c]/20 disabled:opacity-30">↓</button>
                              <button type="button" onclick={() => removeAbility(i)} class="px-1.5 text-xs border border-[#34251c]/20 hover:bg-[#c65f3c]/10">×</button>
                            </div>
                          </div>
                        </div>
                      </div>
                    {/each}
                  </div>

                  <button
                    type="button"
                    onclick={addAbility}
                    disabled={(draft.abilities?.length ?? 0) >= ABILITIES_MAX}
                    class="mt-2 px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5 disabled:opacity-40"
                  >+ {$t('adminBattlesAbilityAdd')}</button>
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

                <!-- Лестница уровней. Заводится сейчас, поднимаются по ней в 1c. -->
                <p class="mt-4 mb-2 text-[11px] leading-relaxed italic text-[#8a6a55]">{$t('adminBattlesLevelPriceHint')}</p>
                <div class="flex gap-2">
                  {#each [0, 1, 2, 3] as step (step)}
                    <label class="block flex-1">
                      <span class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">{step + 1}→{step + 2}</span>
                      <input
                        type="number" min="0"
                        value={draft.levelPriceDust?.[step] ?? ''}
                        oninput={(e) => levelPriceInput(step, e.currentTarget.value)}
                        onfocus={selectOnFocus}
                        onwheel={blurOnWheel}
                        class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                      />
                    </label>
                  {/each}
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
