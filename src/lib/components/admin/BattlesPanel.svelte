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
  import { onMount, tick } from "svelte";
  import { api } from "$lib/api";
  import { t, lang, type TranslationKey } from "$lib/i18n";
  import BattleScene from "$lib/components/BattleScene.svelte";
  import {
    DEFAULT_FRAMES,
    FRAME_MODES,
    LAYOUTS,
    KIND_SIDES,
    SLICE_FITS,
    SHEET_SHOWS,
    statMark,
    sheetSlotMark,
    SHEET_SLOT_BANDS,
    SLICE_GROW_MAX,
    SLICE_KIND,
    SLICE_KINDS,
    SLICE_LAYERS,
    SLICE_SLOTS,
    SLICE_TURNS,
    TIERS,
    HOUSE_RULES,
    rulesApart,
    applyInsetDelta,
    badgeReserve,
    clampTier,
    completeSlices,
    defaultSheet,
    moveSheetRow,
    SHEET_BANDS,
    defaultSlices,
    dressWindowMissing,
    newOrnament,
    dressOf,
    frameName,
    kindOf,
    livePiece,
    normalizeSheet,
    sliceSigns,
    parseFocal,
    parseFrameOverride,
    parseLevelFrames,
    pickImageFile,
    type FrameOverride,
    type InsetKey,
  } from "$lib/battles";
  import { SITE_FONTS } from "$lib/fonts";
  import BattleCard from "$lib/components/BattleCard.svelte";
  import BattleIcon from "$lib/components/BattleIcon.svelte";
  import SheetHead from "$lib/components/admin/sheet/SheetHead.svelte";
  import SheetPanel from "$lib/components/admin/sheet/SheetPanel.svelte";
  import SheetField from "$lib/components/admin/sheet/SheetField.svelte";
  import Medallion from "$lib/components/admin/sheet/Medallion.svelte";
  import StatPlate from "$lib/components/admin/sheet/StatPlate.svelte";
  import StatCell from "$lib/components/admin/sheet/StatCell.svelte";
  import BattleAssetsPanel from "$lib/components/admin/BattleAssetsPanel.svelte";
  import BattleAssetPicker from "$lib/components/admin/BattleAssetPicker.svelte";
  import BattleFramePicker from "$lib/components/admin/BattleFramePicker.svelte";
  import BattleErrandsPanel from "$lib/components/admin/BattleErrandsPanel.svelte";
  import BattleMotionsPanel from "$lib/components/admin/BattleMotionsPanel.svelte";
  import BattleMotionWear from "$lib/components/admin/BattleMotionWear.svelte";
  import type {
    BattleAssetRole,
    BattleCard as BattleCardDto,
    BattleDustRates,
    BattleWelcomeGift,
    BattleClock,
    AdminBattleErrand,
    BattleCardStatus,
    BattleChannel,
    CardAbility,
    BattleFrame,
    BattleFramePreset,
    BattleRules,
    Motion,
    SheetBand,
    SheetRow,
    SheetShow,
    SheetSlot,
    SliceFit,
    SliceKind,
    SliceOrnament,
    SlicePiece,
    SliceSide,
    SliceSlot,
    SliceTurn,
    BattleRace,
    BattleKeyword,
    BattleWeigh,
    BattleAction,
    BattleEvent,
    BattleMatch,
    BattleMatches,
    MatchReplay,
    BattleChallenge,
    BattlePlayerSide,
    Bench,
    ChallengeSetup,
    AbilityVerb,
    AbilityShape,
    AbilityTrigger,
    FigurineListItem,
    AdminUserListItem,
    BattleMe,
    SaveBattleCardRequest,
  } from "$lib/types/api";

  const REORDER_MS = 600;

  let view = $state<
    | "cards"
    | "frames"
    | "face"
    | "motions"
    | "assets"
    | "races"
    | "keywords"
    | "bench"
    | "hand"
    | "matches"
    | "errands"
  >("cards");
  let cards = $state<BattleCardDto[]>([]);
  let frames = $state<BattleFrame[]>(DEFAULT_FRAMES.map(completeSlices));
  /** Свод движений. Столу он нужен ради двух вещей сразу: надеть движение на
   *  карту и проиграть его на скамье — та же комната, тот же проигрыватель. */
  let motions = $state<Motion[]>([]);
  let figurines = $state<FigurineListItem[]>([]);
  let races = $state<BattleRace[]>([]);
  let keywords = $state<BattleKeyword[]>([]);
  let loading = $state(true);
  let saving = $state(false);
  let message = $state("");
  let listQuery = $state("");

  // ── The card being written — one live object, edited on the card itself ──
  let selectedId = $state<string | null>(null);
  let draft = $state<BattleCardDto>(emptyCard());
  /** Which language the on-card fields read and write. The card shows one
   *  language at a time, same as a reader would see it. */
  let editLang = $state<"en" | "ru">(($lang as "en" | "ru") ?? "ru");
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
  /** Поле имени наряда на табличке рамки — чтобы «Новая» сразу просила имя. */
  let frameNameBox = $state<HTMLInputElement | null>(null);

  // ── The drawer of saved dresses ────────────────────────────────────────
  //
  // A frame is slow to build: a picture cut into nine pieces, four insets set
  // by eye, bands balanced against a real photograph. Presets are the drawer
  // that work goes into, so the second card that wants that frame costs one
  // click instead of an evening. Nothing renders a preset — it is only ever
  // taken out ONTO something: a rank, a race's level, all five of them, or one
  // card.
  let presets = $state<BattleFramePreset[]>([]);
  let presetName = $state("");
  /** Which saved dress the two dictionaries are about to put on. Kept apart
   *  from the frames view's own list so choosing one in the race editor never
   *  moves what the rank editor is showing. */
  let presetChosen = $state<string | null>(null);
  /**
   * Какой наряд вынут из ящика НА ЭТОТ ЧИН — то есть с чего начата работа.
   *
   * Без этого стол помнил только «надето» и тут же забывал, откуда: хранитель
   * брал наряд, дописывал к нему завиток, жал «Сохранить» — и сохранялся ЧИН,
   * а наряд в ящике оставался вчерашним. Второй раз его доставали, и завитка
   * там не было. Единственный способ было положить его обратно — вспомнить имя
   * по буквам и напечатать заново, иначе в ящике заводился близнец.
   *
   * Теперь ящик знает, что открыт, и «обновить» стоит рядом с ним.
   */
  let presetOpen = $state<string | null>(null);

  let dragFrom = $state<number | null>(null);
  let dragOver = $state<number | null>(null);
  let reorderTimer: ReturnType<typeof setTimeout> | null = null;
  let flashTimer: ReturnType<typeof setTimeout> | null = null;

  // The race dictionary, edited in its own view.
  let raceDraftId = $state<string | null>(null);
  let keywordDraftId = $state<string | null>(null);
  let keywordNameEn = $state("");
  let keywordNameRu = $state("");
  let keywordRulesEn = $state("");
  let keywordRulesRu = $state("");
  let keywordPoints = $state<number | null>(null);
  let raceNameEn = $state("");
  let raceNameRu = $state("");
  let raceNoteEn = $state("");
  let raceNoteRu = $state("");
  let raceIconUrl = $state("");
  /** Движения расы. Стоят между картой и домом: все лучники расы стреляют её
   *  стрелой, пока отдельная карта не скажет иначе. */
  let raceMotionWear = $state<string | null>(null);
  /** This race's own dress per level of an owned copy — 5 slots, index 0 = level 1. */
  let raceLevelFrames = $state<(FrameOverride | null)[]>([
    null,
    null,
    null,
    null,
    null,
  ]);
  /** Which level's slot the uploader and the sample card currently show. */
  let raceLevelPreview = $state(1);

  /**
   * Шесть чисел тела. Списком, а не шестью копиями разметки: они отличаются
   * только пределами, и разъехаться им незачем.
   */
  /**
   * Пять чисел тела. Списком, а не пятью копиями разметки: они отличаются
   * только пределами, и разъехаться им незачем.
   *
   * Скорости здесь больше нет. Она бралась в весах по 2 очка за ступень — 8 за
   * пятую, весь бюджет первого чина, — а в движке её не существует: ход
   * чередуется, очерёдности по скорости в игре нет. Поле у карты сохранено и
   * значения не теряются (`cardBody` по-прежнему их отправляет), но задавать
   * число, которое ни на что не влияет, форма больше не предлагает.
   */
  // Знак берётся из `STAT_MARKS`, а не вписан сюда рукой: тот же знак стоит
  // рядом с тем же числом на лице карты, на листе взятия и в разборе сцены, и
  // плашка, называющая своё сердце отдельно, однажды покажет хранителю одну
  // картинку здесь и другую на карте, которую он этой плашкой правит.
  const bodyStats = [
    { key: "armor", label: "adminBattlesArmor", min: 0, max: 20 },
    { key: "ward", label: "adminBattlesWard", min: 0, max: 20 },
    { key: "reach", label: "adminBattlesReach", min: 0, max: 5 },
    { key: "step", label: "adminBattlesStep", min: 0, max: 3 },
    { key: "mend", label: "adminBattlesMend", min: 0, max: 20 },
  ] as const;

  /** Тот же порог, что в `battles.rs`: выше 1.15 перегруз, ниже 0.85 мертва. */
  function verdictWord(index: number): string {
    if (index > 1.15) return $t("adminBattlesOverloaded");
    if (index < 0.85) return $t("adminBattlesUnderweight");
    return $t("adminBattlesOnCurve");
  }

  function verdictColour(index: number): string {
    if (index > 1.15) return "#8f2f22";
    if (index < 0.85) return "#4d6673";
    return "#4a6141";
  }

  const tierBudget = (tier: number) =>
    8 + 6 * (Math.max(1, Math.min(5, tier)) - 1);

  // ── Стол ───────────────────────────────────────────────────────────────
  //
  // Ничего не записывает: расстановка и журнал целиком едут с каждым запросом,
  // доска пересобирается свёрткой. Поэтому со стола нельзя ни начислить пыль,
  // ни оставить мусор в базе, а свойство «журнал переигрывается» проверяется
  // на каждом клике.

  const BENCH_WIDTH = 3;
  const BENCH_DEPTH = 6;

  /** Сколько карт на столе гостя: три на доске и три в руке. То же число, что
   *  `DECK_BOARD + DECK_HAND` на сервере, — здесь оно нужно, чтобы понять,
   *  сколько заёмных карт дому надо иметь, чтобы стол не был шестью копиями
   *  одной. */
  const DECK_SIZE = 6;

  /** Кто может встать на поле: опубликованные и с ненулевым здоровьем. */
  let benchable = $derived(
    cards.filter((c) => c.status === "published" && c.health > 0),
  );

  let benchBoard = $state<Record<string, string>>({});
  let benchHands = $state<{ player: string[]; keeper: string[] }>({
    player: [],
    keeper: [],
  });
  let benchJournal = $state<BattleAction[]>([]);
  let bench = $state<Bench | null>(null);
  let benchBusy = $state(false);
  let benchBoth = $state(false);
  let benchComplaint = $state<string | null>(null);

  let benchSetup = $derived.by((): ChallengeSetup => {
    const board = (half: "keeper" | "player") =>
      Object.entries(benchBoard)
        .filter(([key]) => {
          const y = Number(key.split(",")[1]);
          return half === "keeper" ? y < 3 : y >= 3;
        })
        .map(([key, card]) => {
          const [x, y] = key.split(",").map(Number);
          return { card, x, y };
        });
    return {
      playerBoard: board("player"),
      playerHand: benchHands.player,
      keeperBoard: board("keeper"),
      keeperHand: benchHands.keeper,
      // Правила едут вместе с расстановкой — и на стол, и в запись. Стол,
      // играющий домашними правилами при своих у этюда, проверял бы не тот
      // этюд, а хранитель узнавал бы об этом от гостя.
      rules: etudeOwnRules ? etudeRules : null,
    };
  });

  let benchReady = $derived(
    benchSetup.playerBoard.length > 0 && benchSetup.keeperBoard.length > 0,
  );

  /** Стол притворяется партией, чтобы доску рисовал тот же компонент. */
  let benchMatch = $derived.by((): BattleMatch | null =>
    bench
      ? {
          id: "bench",
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

  async function benchCall(
    next: BattleAction | null,
    playOut = false,
    autoKeeper?: boolean,
  ) {
    benchBusy = true;
    benchComplaint = null;
    try {
      const answer = await api.adminBenchBattle({
        setup: benchSetup,
        actions: benchJournal,
        next,
        // За обе стороны — хранитель молчит и ждёт, пока за него сходят.
        autoKeeper: autoKeeper ?? !benchBoth,
        // Той же рукой, какой этюд пройдёт гость: стол, проверяющий одним
        // ботом то, что достанется другому, проверяет не то.
        botDepth: etudeDepth,
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

  /**
   * Отменить последнее действие.
   *
   * Журнал и есть истина, доска — только кэш, поэтому отмена это «выбросить
   * последнюю строку и пересобрать». Ответчик при этом молчит намеренно: иначе
   * отменённый ход хранителя он тут же сделал бы снова, и кнопка выглядела бы
   * сломанной. Если отмена вернула очередь хранителю — жмите ещё раз или
   * походите за него.
   */
  function benchUndo() {
    if (!benchJournal.length || benchBusy) return;
    benchJournal = benchJournal.slice(0, -1);
    void benchCall(null, false, false);
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

  function benchAddToHand(side: "player" | "keeper", slug: string) {
    if (!slug) return;
    benchHands = { ...benchHands, [side]: [...benchHands[side], slug] };
  }

  function benchDropFromHand(side: "player" | "keeper", at: number) {
    benchHands = {
      ...benchHands,
      [side]: benchHands[side].filter((_, i) => i !== at),
    };
  }

  const benchTitle = (slug: string) =>
    cards.find((c) => c.slug === slug)?.titleRu || slug;

  /** Событие одной строкой, с разбором урона там, где он есть. */
  function benchLine(e: BattleEvent): string {
    if (typeof e !== "object") return String(e);
    if ("played" in e)
      return `выставлен ${benchUnitName(e.played.unit)} за ${e.played.cost}`;
    if ("moved" in e)
      return `${benchUnitName(e.moved.unit)}: ${e.moved.from.x},${e.moved.from.y} → ${e.moved.to.x},${e.moved.to.y}`;
    if ("damaged" in e) {
      const d = e.damaged;
      const why = d.trail.map((b) => `${b.step}: ${b.from}→${b.to}`).join("; ");
      const total = d.toHealth + d.toShield;
      return `${benchUnitName(d.target)} получает ${total}${why ? `  [${why}]` : ""}`;
    }
    if ("healed" in e)
      return `${benchUnitName(e.healed.target)} залечен на ${e.healed.amount}`;
    if ("immune" in e)
      return `${benchUnitName(e.immune.target)} не почувствовал`;
    if ("died" in e) return `пал ${benchUnitName(e.died.target)}`;
    if ("turnEnded" in e)
      return `— ход ${e.turnEnded.side === "player" ? "гостя" : "хранителя"} окончен`;
    if ("finished" in e) return `итог: ${e.finished.outcome}`;
    return "";
  }

  const benchUnitName = (id: number) =>
    bench
      ? benchTitle(bench.state.units[id]?.card.name ?? String(id))
      : String(id);

  // ── Этюды ──────────────────────────────────────────────────────────────
  //
  // Этюд редактируется НА СТОЛЕ, а не в отдельной форме со своей второй
  // сеткой. Причина не в экономии разметки: этюд — это расстановка, у которой
  // есть решение, и единственный способ узнать, что решение есть, — разыграть
  // её. Форма, в которой расставляют, но не играют, позволяет выложить
  // непроходимый этюд и узнать об этом от гостя.
  //
  // Поэтому порядок работы такой: расставить → разыграть → оставить. Полка
  // слева — это то, что оставили; поля наверху — то, чем оставленное
  // подписано. Расстановку они не хранят: её всегда даёт `benchSetup`.

  let challenges = $state<BattleChallenge[]>([]);
  let etudeId = $state<string | null>(null);
  let etudeTitleRu = $state("");
  let etudeTitleEn = $state("");
  let etudeNoteRu = $state("");
  let etudeNoteEn = $state("");
  /** Рука хранителя. Сложность — это она и только она: бот, которому дали
   *  лишнюю ману, ломает и честность, и всякую возможность измерить силу
   *  карты.
   *
   *  Ступеней две, а не три. Третья была измерена и не оказалась сильнее
   *  второй, а попытка сделать её сильнее считала ход секундами вместо
   *  миллисекунд. Обоснование — в `battle_core::bot::DEPTH_MAX`. */
  const BOT_HANDS = [
    { depth: 1, label: "adminBattlesHandGreedy" },
    { depth: 2, label: "adminBattlesHandSearching" },
  ] as const;
  let etudeDepth = $state(1);
  let etudeReward = $state(0);
  /** За доведённое до конца — платится и проигравшему. */
  let etudeFinish = $state(0);
  let etudeStatus = $state<BattleCardStatus>("draft");
  /** Кем задана сторона гостя. `scripted` — рукой (этюд, у него есть решение),
   *  `deck` — столом гостя (встреча). У встречи половину гостя расставлять не
   *  надо и нельзя: её приносит гость. */
  let etudeSide = $state<BattlePlayerSide>("scripted");
  // ── Правила этюда ───────────────────────────────────────────────────────
  //
  // До этой формы все этюды игрались умолчаниями дома, и ни одна из измеренных
  // ручек полке доступна не была. Между тем каждая из них меняет не оттенок, а
  // игру: шаг, тратящий ход целиком, уводит долю партий, решённых очерёдностью
  // а не картой, с 43 % на 98 %; штраф стрелку в упор переворачивает крайний
  // случай §15.2 с 2 побед из 16 на 11 из 18.
  //
  // Поэтому здесь не «настройки сложности», а вторая половина замысла этюда:
  // расстановка говорит, кто стоит, правила — во что играют.

  /** Играет ли этюд своими правилами. Выключено — в записи `null`, то есть
   *  дом; и это не то же самое, что «свои правила, совпадающие с домашними»:
   *  дом со временем меняется замером, и этюд, сказавший «как дома», обязан
   *  меняться вместе с ним. */
  let etudeOwnRules = $state(false);
  let etudeRules = $state<BattleRules>({ ...HOUSE_RULES });

  /** Ручки в том порядке, в каком их выбирают: сперва то, что меняет игру
   *  целиком, потом то, что её подкручивает. */
  const RULE_DIALS = [
    { key: "walkSpendsTurn", label: "adminBattlesRuleWalk", kind: "flag" },
    { key: "retaliation", label: "adminBattlesRuleRetaliation", kind: "flag" },
    { key: "actsPerTurn", label: "adminBattlesRuleActs", kind: "number", min: 1, max: 255 },
    { key: "openingAttacks", label: "adminBattlesRuleOpening", kind: "number", min: 0, max: 255 },
    { key: "idleToll", label: "adminBattlesRuleIdle", kind: "number", min: 0, max: 5 },
    { key: "escalationFrom", label: "adminBattlesRuleEscalation", kind: "number", min: 0, max: 24 },
    { key: "maxRounds", label: "adminBattlesRuleRounds", kind: "number", min: 1, max: 24 },
    { key: "secondSideCoin", label: "adminBattlesRuleCoin", kind: "number", min: 0, max: 10 },
    { key: "pointBlankPower", label: "adminBattlesRulePointBlank", kind: "number", min: 0, max: 100 },
    { key: "longShotPower", label: "adminBattlesRuleLongShot", kind: "number", min: 0, max: 100 },
  ] as const;

  /** Чем эти правила отличаются от домашних — теми же словами, какими это
   *  прочтёт гость. Хранитель обязан видеть ту же строку, что и он. */
  let etudeApart = $derived(etudeOwnRules ? rulesApart(etudeRules) : []);

  /** Одна ручка. Пишется в копию, а не в объект: `$state` замечает подмену, а
   *  правку внутри — нет, и форма показывала бы вчерашнее число. */
  function tune<K extends keyof BattleRules>(key: K, value: BattleRules[K]) {
    const next = { ...etudeRules };
    next[key] = value;
    etudeRules = next;
  }

  /** Что нужно, чтобы это можно было оставить. У встречи — только половина
   *  хранителя: половину гостя приносит его стол, и требовать её здесь значило
   *  бы требовать чужого. То же правило, что и на сервере. */
  let etudeReady = $derived(
    etudeSide === "deck" ? benchSetup.keeperBoard.length > 0 : benchReady,
  );

  /** Карта, названная расстановкой, но снятая с полки: `benchable` её больше
   *  не предлагает, и без этой проверки она пропала бы из выпадающего списка
   *  молча, а отказ пришёл бы с сервера при сохранении. */
  const gone = (slug: string) =>
    !!slug && !benchable.some((c) => c.slug === slug);

  /** Все снятые карты, названные тем, что сейчас на столе. */
  let benchGone = $derived([
    ...new Set(
      [
        ...Object.values(benchBoard),
        ...benchHands.player,
        ...benchHands.keeper,
      ].filter(gone),
    ),
  ]);

  /** Разложить сохранённый этюд на столе. Журнал при этом сбрасывается:
   *  разыгранная партия принадлежала прежней расстановке. */
  function openEtude(challenge: BattleChallenge | null) {
    etudeId = challenge?.id ?? null;
    etudeTitleRu = challenge?.titleRu ?? "";
    etudeTitleEn = challenge?.titleEn ?? "";
    etudeNoteRu = challenge?.noteRu ?? "";
    etudeNoteEn = challenge?.noteEn ?? "";
    etudeDepth = challenge?.botDepth ?? 1;
    etudeReward = challenge?.rewardDust ?? 0;
    etudeFinish = challenge?.rewardFinishDust ?? 0;
    etudeStatus = challenge?.status ?? "draft";
    etudeSide = challenge?.playerSide ?? "scripted";
    // Своё или дом — читается по наличию, а не по совпадению чисел: этюд,
    // сказавший «как дома», и этюд, промолчавший, — разные вещи.
    const kept = challenge?.setup.rules ?? null;
    etudeOwnRules = !!kept;
    etudeRules = { ...HOUSE_RULES, ...(kept ?? {}) };

    benchJournal = [];
    bench = null;
    benchComplaint = null;
    if (!challenge) {
      benchBoard = {};
      benchHands = { player: [], keeper: [] };
      return;
    }
    const board: Record<string, string> = {};
    for (const p of [
      ...challenge.setup.keeperBoard,
      ...challenge.setup.playerBoard,
    ]) {
      board[`${p.x},${p.y}`] = p.card;
    }
    benchBoard = board;
    benchHands = {
      player: [...challenge.setup.playerHand],
      keeper: [...challenge.setup.keeperHand],
    };
  }

  async function saveEtude() {
    if (!etudeTitleRu.trim() && !etudeTitleEn.trim()) {
      flash($t("adminBattlesEtudeNeedsTitle"), 6000);
      return;
    }
    saving = true;
    try {
      // Расстановку даёт стол, а не отдельная копия: второй источник правды
      // разошёлся бы с тем, что хранитель только что разыграл.
      const saved = await api.adminSaveBattleChallenge(
        {
          titleEn: etudeTitleEn.trim(),
          titleRu: etudeTitleRu.trim(),
          noteEn: etudeNoteEn.trim() || null,
          noteRu: etudeNoteRu.trim() || null,
          setup: benchSetup,
          botDepth: etudeDepth,
          rewardDust: etudeReward,
          rewardFinishDust: etudeFinish,
          playerSide: etudeSide,
          status: etudeStatus,
        },
        etudeId ?? undefined,
      );
      challenges = await api.adminListBattleChallenges();
      etudeId = saved.id;
      flash($t("adminBattlesEtudeSaved"));
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      saving = false;
    }
  }

  async function removeEtude() {
    if (!etudeId || !confirm($t("adminBattlesEtudeDeleteConfirm"))) return;
    try {
      await api.adminDeleteBattleChallenge(etudeId);
      challenges = await api.adminListBattleChallenges();
      openEtude(null);
      flash($t("adminBattlesEtudeDeleted"));
    } catch (e) {
      flash(String(e), 6000);
    }
  }

  // ── Порядок полки этюдов ────────────────────────────────────────────────
  //
  // Своё состояние перетаскивания, а не общее с полкой карт: две полки видны
  // в разных вкладках, но одна пара переменных на обе — это перетаскивание,
  // которое помнит чужой список.
  let etudeDragFrom = $state<number | null>(null);
  let etudeDragOver = $state<number | null>(null);
  let etudeOrderTimer: ReturnType<typeof setTimeout> | null = null;

  function onEtudeDrop(to: number) {
    const from = etudeDragFrom;
    etudeDragFrom = null;
    etudeDragOver = null;
    if (from == null || from === to) return;
    const next = [...challenges];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    challenges = next;
    // Придержано на удар: три перетащенных подряд этюда — это одна полка, а не
    // три. Тот же приём и то же число, что у полки карт.
    if (etudeOrderTimer) clearTimeout(etudeOrderTimer);
    etudeOrderTimer = setTimeout(async () => {
      try {
        await api.adminReorderBattleChallenges(challenges.map((c) => c.id));
        flash($t("adminBattlesReordered"));
      } catch (e) {
        flash(String(e), 6000);
        challenges = await api.adminListBattleChallenges();
      }
    }, REORDER_MS);
  }

  // ── Из рук ──────────────────────────────────────────────────────────────
  //
  // Корм не оседает сам, как пыль: его даёт хранитель за настоящее —
  // состоявшийся показ, опубликованное впечатление, заказ работы. Поэтому
  // здесь нет ни ставки, ни правила: есть человек, число и записка.
  //
  // Записка обязательна и на сервере тоже. Корм без неё был бы просто выросшим
  // счётчиком — ровно тем, от чего эта комната отказывается.

  let guestQuery = $state("");
  let guests = $state<AdminUserListItem[]>([]);
  let guestChosen = $state<AdminUserListItem | null>(null);
  let grantCoin = $state<"feed" | "dust">("feed");
  let grantAmount = $state(1);
  let grantNote = $state("");
  let granting = $state(false);
  /**
   * Ключ ЭТОЙ выдачи — чеканится на открытие формы и заново после каждой
   * состоявшейся. Отсюда, а не с сервера: сервер, чеканящий ключ сам, сделал
   * бы двойной щелчок второй выдачей, а ключ из содержимого слил бы два
   * состоявшихся показа одному гостю в один.
   */
  let grantKey = $state(crypto.randomUUID());

  /** Что у выбранного гостя сейчас — ровно то, что видит он сам. Без этого
   *  проверять нечего: не видно ни что выдалось, ни что уже было. */
  let guestHas = $state<BattleMe | null>(null);
  let giveLevel = $state(1);
  let giving = $state(false);

  async function readGuest() {
    if (!guestChosen) {
      guestHas = null;
      return;
    }
    try {
      guestHas = await api.adminReadBattleGuest(guestChosen.id);
    } catch (e) {
      guestHas = null;
      flash(String(e), 6000);
    }
  }

  async function chooseGuest(guest: AdminUserListItem) {
    guestChosen = guest;
    await readGuest();
  }

  async function giveAllCards() {
    if (!guestChosen || giving) return;
    giving = true;
    try {
      const res = await api.adminGiveBattleCards({
        userId: guestChosen.id,
        all: true,
        level: giveLevel,
      });
      await readGuest();
      flash(`${$t("adminBattlesGiveDone")} ${res.touched}`);
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      giving = false;
    }
  }

  async function takeAllCards() {
    if (!guestChosen || giving) return;
    if (!confirm($t("adminBattlesTakeConfirm"))) return;
    giving = true;
    try {
      const res = await api.adminRevokeBattleCards({
        userId: guestChosen.id,
        all: true,
      });
      await readGuest();
      flash(`${$t("adminBattlesTakeDone")} ${res.touched}`);
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      giving = false;
    }
  }

  /** Обнулить монету. Строка книги со знаком минус, а не удаление строк:
   *  книга неизменяема, и ошибка — да и сброс — правятся обратной строкой. */
  async function zeroCoin(coin: "dust" | "feed") {
    if (!guestChosen || !guestHas) return;
    const has = coin === "dust" ? guestHas.dust : guestHas.feed;
    if (has === 0) return;
    try {
      await api.adminGrantBattleCoin({
        userId: guestChosen.id,
        currency: coin,
        amount: -has,
        note: $t("adminBattlesZeroNote"),
        idemKey: crypto.randomUUID(),
      });
      await readGuest();
      flash($t("adminBattlesZeroDone"));
    } catch (e) {
      flash(String(e), 6000);
    }
  }

  let grantReady = $derived(
    !!guestChosen && grantAmount !== 0 && grantNote.trim().length > 0,
  );

  async function findGuests() {
    try {
      const page = await api.adminListUsers({
        search: guestQuery.trim(),
        perPage: 20,
      });
      guests = page.items;
    } catch (e) {
      flash(String(e), 6000);
    }
  }

  async function giveByHand() {
    if (!guestChosen || !grantReady || granting) return;
    granting = true;
    try {
      const res = await api.adminGrantBattleCoin({
        userId: guestChosen.id,
        currency: grantCoin,
        amount: grantAmount,
        note: grantNote.trim(),
        idemKey: grantKey,
      });
      // Новый ключ — только после состоявшейся выдачи: пока она не прошла,
      // повтор тем же ключом безвреден и это ровно то, что нужно.
      grantKey = crypto.randomUUID();
      grantNote = "";
      await readGuest();
      flash(
        res.grantedNow
          ? `${$t("adminBattlesHandGiven")} ${res.balance}`
          : $t("adminBattlesHandAlready"),
      );
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      granting = false;
    }
  }

  const etudeTitleOf = (c: BattleChallenge) =>
    $lang === "ru" ? c.titleRu || c.titleEn : c.titleEn || c.titleRu;

  function emptyCard(): BattleCardDto {
    return {
      id: "",
      slug: "",
      status: "draft",
      tier: 1,
      raceId: null,
      raceNameEn: null,
      raceNameRu: null,
      raceIconUrl: null,
      raceLevelFrames: null,
      typeEn: null,
      typeRu: null,
      titleEn: "",
      titleRu: "",
      effectEn: null,
      effectRu: null,
      loreEn: null,
      loreRu: null,
      cost: 1,
      power: 1,
      health: 0,
      mana: 0,
      traits: [],
      kind: "unit",
      armor: 0,
      ward: 0,
      attackChannel: "physical",
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
      lendable: false,
      artUrl: null,
      artUrlOverride: null,
      artFocal: null,
      frameOverride: null,
      motionWear: null,
      shelfOrder: null,
      figurineId: null,
      figurineName: null,
      figurineSlug: null,
      createdAt: "",
      updatedAt: "",
    };
  }

  let visible = $derived(
    listQuery.trim()
      ? cards.filter((c) =>
          `${c.titleRu} ${c.titleEn} ${c.figurineName ?? ""}`
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
  let workQuery = $state("");
  let visibleFigurines = $derived(
    workQuery.trim()
      ? figurines.filter(
          (f) =>
            f.id === draft.figurineId ||
            f.name.toLowerCase().includes(workQuery.trim().toLowerCase()),
        )
      : figurines,
  );

  /** A figurine already wearing a card, spotted before the save round-trip
   *  reports it back as a `UNIQUE figurine_id` conflict — `battle_cards` allows
   *  exactly one card per work. */
  let workTaken = $derived(
    !!draft.figurineId &&
      cards.some(
        (c) => c.figurineId === draft.figurineId && c.id !== selectedId,
      ),
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

  /** The card the frames / face views dress: the one being written, else any
   *  real card of that rank, else a stand-in — so the sliders always have a
   *  subject. Race and card dresses are stripped: this desk edits the RANK,
   *  and a sample that wore another look would show a roster the sidebar is
   *  not editing. */
  let frameSample = $derived.by((): BattleCardDto => {
    const tier = frames[frameIndex]?.tier ?? 1;
    const raw =
      draft.titleRu || draft.titleEn
        ? { ...draft, tier }
        : (cards.find((c) => c.tier === tier) ?? {
            ...emptyCard(),
            tier,
            titleEn: "The Keeper of the Key",
            titleRu: "Хранительница Ключа",
            effectRu:
              "Вихрь Души: каждое третье заклинание создаёт копию эффекта.",
            effectEn:
              "Wind of Soul: every third spell makes a copy of its effect.",
            cost: 5,
            power: 10,
          });
    return { ...raw, frameOverride: null, raceLevelFrames: null };
  });

  /** The race dictionary's own sample, so the icon can be judged on a card
   *  rather than as a bare thumbnail — the same reasoning the frames view
   *  already follows for its own sample. */
  let raceSample = $derived<BattleCardDto>({
    ...emptyCard(),
    tier: 3,
    titleEn: "The Keeper of the Key",
    titleRu: "Хранительница Ключа",
    effectEn: "A sample card, to see the icon in place.",
    effectRu: "Пример карты — чтобы увидеть иконку на месте.",
    raceNameEn: raceNameEn || raceNameRu ? raceNameEn || raceNameRu : null,
    raceNameRu: raceNameRu || raceNameEn ? raceNameRu || raceNameEn : null,
    raceIconUrl: raceIconUrl.trim() || null,
    raceLevelFrames: raceLevelFrames.some((f) => f)
      ? JSON.stringify(raceLevelFrames)
      : null,
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
        frame.frameMode = "overlay";
      } else {
        // No hole in it: worn on top it would cover the card completely.
        frame.frameMode = "behind";
        flash($t("adminBattlesFrameNoAlpha"), 8000);
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
      const imported = await api.importMediaWithVariants(
        file,
        "images",
        "card-paper",
      );
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

  // ── Со склада ───────────────────────────────────────────────────────────
  //
  // Каждый из шести слотов `sliced` умеет две вещи: принять новый файл и взять
  // уже нарезанную деталь. Второе — обычный путь: детали приходят листами, и
  // перезаливать по одной то, что уже лежит на складе, значит плодить копии
  // одного и того же файла.
  let picker = $state<{
    role: BattleAssetRole;
    apply: (url: string) => void;
  } | null>(null);

  function fromStore(role: BattleAssetRole, apply: (url: string) => void) {
    picker = { role, apply };
  }

  // Шесть почти одинаковых загрузчиков стояли здесь, пока слоты были шестью
  // открытыми блоками. Блок теперь один — у детали, что в руке, — и загрузчик
  // тоже один: `uploadPiece` ниже.

  /**
   * Сменить, как надета рама.
   *
   * И дать собранной из частей полосы, если их ещё нет. Врезы у свежей рамы
   * нулевые, а полоса нулевой ширины — это деталь нулевого размера: хранитель
   * загружал четыре картинки и не видел ни одной, без единого слова о том,
   * почему. Десять процентов — не догадка о его замысле, а первое, что видно;
   * дальше он тянет окно сам.
   */
  function setFrameMode(mode: string) {
    const frame = frames[frameIndex];
    frame.frameMode = mode as typeof frame.frameMode;
    if (mode !== "sliced") return;
    if (
      frame.insetTop ||
      frame.insetRight ||
      frame.insetBottom ||
      frame.insetLeft
    )
      return;
    frame.insetTop = 10;
    frame.insetRight = 10;
    frame.insetBottom = 10;
    frame.insetLeft = 10;
  }

  /** С какой полки склада предлагать деталь для этого слота. Роль — не второй
   *  справочник, а слово, по которому хранитель отбирает: показать сразу углы,
   *  когда берут угол. */
  const STORE_ROLE: Record<SliceSlot, BattleAssetRole> = {
    corner: "corner",
    sideH: "sideH",
    sideV: "sideV",
    cornerExtra: "accent",
    sideMidH: "accent",
    sideMidV: "accent",
  };

  /** Загрузить картинку для той детали, что в руке. Шесть почти одинаковых
   *  загрузчиков стояли рядом, пока слоты были шестью открытыми блоками; теперь
   *  блок один, и загрузчик тоже. */
  async function uploadPiece(row: FramePiece) {
    const file = await pickImageFile();
    if (!file) return;
    uploading = true;
    try {
      const art = await api.adminUploadBattleFrameArt(file);
      mark();
      setPieceImage(row, art.url);
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      uploading = false;
    }
  }

  /**
   * Картинка жетона — тем же загрузчиком, что и детали рамки, и по той же
   * причине: обычный `/upload` пишет JPEG-копии, а жетон вырезан по альфе, и
   * залитый бумагой прямоугольник вместо бляхи ничего бы не сказал вслух.
   *
   * Куда её деть, инспектор говорит сам колбэком: он один знает, у какого из
   * трёх значков открыт стол, и повторять здесь его развилку значило бы
   * держать две таблицы полей вместо `BADGE_FIELDS`.
   */
  async function uploadBadgeArt(apply: (url: string) => void) {
    const file = await pickImageFile();
    if (!file) return;
    uploading = true;
    try {
      const art = await api.adminUploadBattleFrameArt(file);
      mark();
      apply(art.url);
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      uploading = false;
    }
  }

  /** Со склада — ролью `accent`: из шести слов это единственное, которым
   *  бляха, пломба или печать себя называют. Своей роли жетону не заведено —
   *  роль это CHECK из шести, а не ящик, куда доклада́ют. */
  function badgeArtFromStore(apply: (url: string) => void) {
    fromStore('accent', (url) => {
      mark();
      apply(url);
    });
  }

  function setPieceImage(row: FramePiece, url: string) {
    if (row.ornament) row.ornament.image = url;
    else
      (frames[frameIndex] as unknown as Record<string, string>)[
        SLOT_FIELD[row.id as SliceSlot]
      ] = url;
  }

  /**
   * Переложить деталь в стопке. Список ПЕРЕНУМЕРОВЫВАЕТСЯ целиком, сверху вниз,
   * а не меняет два числа местами: пока слои могли совпадать, порядок решала
   * разметка — то есть решал никто и невидимо. После перенумерации у каждой
   * детали свой слой, и список — единственное место, где порядок задают.
   */
  function restack(id: string, by: -1 | 1) {
    const order = stack.slice();
    const from = order.findIndex((row) => row.id === id);
    const to = from + by;
    if (from < 0 || to < 0 || to >= order.length) return;
    mark();
    const [moved] = order.splice(from, 1);
    order.splice(to, 0, moved);
    // Сверху — самый большой слой. Хватает на 24 детали; ниже пола просто
    // упирается, и там снова решает порядок списка, что честно: он и есть
    // порядок разметки.
    order.forEach((row, i) => {
      row.piece.layer = Math.max(1, SLICE_LAYERS - i);
    });
  }

  /* ── Опись ──────────────────────────────────────────────────────────────
     Что печатается на карте. Живёт в раме и правится здесь, а не отдельной
     вкладкой: рама и есть внешний вид карты, она уже ездит по цепочке чин →
     уровень расы → карта, уже кладётся в ящик нарядов и уже под отменой. */

  /** Строки описи открытой рамы. Живой объект, а не копия: галочки и выбор
   *  полосы пишут прямо в него, и «не сохранено» загорается само. */
  let sheet = $derived(frames[frameIndex]?.sheet ?? []);

  /** Подписи. `SheetSlot` закрыт, поэтому таблица полная и новая строка в
   *  доме не соберётся без своего слова. */
  const SHEET_SLOT_KEY: Record<SheetSlot, TranslationKey> = {
    raceIcon: "adminBattlesRosterRaceIcon",
    race: "adminBattlesRosterRace",
    kind: "adminBattlesRosterKind",
    channel: "adminBattlesRosterChannel",
    pips: "adminBattlesRosterPips",
    title: "adminBattlesRosterTitle",
    rank: "adminBattlesRosterRank",
    traits: "adminBattlesRosterTraits",
    effect: "adminBattlesRosterEffect",
    lore: "adminBattlesRosterLore",
    health: "battlesHealthLabel",
    mana: "battlesManaLabel",
    armor: "battleStatArmour",
    ward: "battleStatWard",
    reach: "battleStatReach",
    step: "battleStatStep",
    mend: "battleStatMend",
    stats: "adminBattlesRosterStats",
    cost: "adminBattlesRosterCost",
    power: "adminBattlesRosterPower",
    healthMark: "adminBattlesRosterHealthMark",
    new: "adminBattlesRosterNew",
    costWord: "adminBattlesRosterCostWord",
    powerWord: "adminBattlesRosterPowerWord",
  };

  const SHEET_SHOW_KEY: Record<SheetShow, TranslationKey> = {
    never: "adminBattlesRosterNever",
    large: "adminBattlesRosterLarge",
    always: "adminBattlesRosterAlways",
    cell: "adminBattlesRosterCell",
    cellOnly: "adminBattlesRosterCellOnly",
  };

  const SHEET_SHOW_SHORT: Record<SheetShow, TranslationKey> = {
    never: "adminBattlesRosterNeverShort",
    large: "adminBattlesRosterLargeShort",
    always: "adminBattlesRosterAlwaysShort",
    cell: "adminBattlesRosterCellShort",
    cellOnly: "adminBattlesRosterCellOnlyShort",
  };

  const SHEET_BAND_KEY: Record<SheetBand, TranslationKey> = {
    head: "adminBattlesRosterBandHead",
    props: "adminBattlesRosterBandProps",
    foot: "adminBattlesRosterBandFoot",
    over: "adminBattlesRosterBandOver",
  };

  /** Какая строка описи в руке. Одна на два входа — на карту и на список
   *  сбоку, — потому что берут её и там и там, а несут одну. */
  let rowHeld = $state<SheetSlot | null>(null);
  /**
   * Куда она сядет, если отпустить в списке.
   *
   * Названо соседом, а не номером: номер в полосе считается БЕЗ взятой строки,
   * и держать его в состоянии значило бы пересчитывать его на каждое движение
   * мыши — с той же ошибкой на единицу, которую он и заводит. `before: null` —
   * в конец полосы.
   */
  let rowOver = $state<{ band: SheetBand; before: SheetSlot | null } | null>(null);

  /** Строки одной полосы, в порядке описи. */
  function bandRows(band: SheetBand): SheetRow[] {
    return sheet.filter((row) => row.band === band);
  }

  /** Одна пересадка на оба входа. Стол не переставляет строки сам — он зовёт
   *  `moveSheetRow`, ту же, которой отвечает карта. */
  function moveRow(slot: SheetSlot, band: SheetBand, before: SheetSlot | null) {
    if (!frames[frameIndex]) return;
    mark();
    frames[frameIndex].sheet = moveSheetRow(frames[frameIndex].sheet, slot, band, before);
  }

  /** Переставить строку описи стрелками — то же, что мышью, но точно. */
  /** Куда метит рука в списке. Полоса, в которой строке стоять нельзя, не
   *  берётся вовсе — метка просто не появляется. */
  function aimRowList(band: SheetBand, before: SheetSlot | null) {
    if (!rowHeld || !SHEET_SLOT_BANDS[rowHeld].includes(band)) {
      rowOver = null;
      return;
    }
    rowOver = { band, before };
  }

  /** Строку отпустили в списке. */
  function dropRowList() {
    const held = rowHeld;
    const aim = rowOver;
    rowHeld = null;
    rowOver = null;
    if (!held || !aim) return;
    moveRow(held, aim.band, aim.before);
  }

  /** Сосед, перед которым сядет строка, если отпустить над этой. Верхняя
   *  половина — перед ней, нижняя — перед следующей, а взятая пропускается:
   *  «перед самой собой» не место. */
  function seatBy(band: SheetBand, over: SheetSlot, event: DragEvent): SheetSlot | null {
    const box = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const below = event.clientY > box.top + box.height / 2;
    const list = bandRows(band).filter((one) => one.slot !== rowHeld);
    const at = list.findIndex((one) => one.slot === over);
    if (at < 0) return below ? null : (list[0]?.slot ?? null);
    const seat = below ? at + 1 : at;
    return list[seat]?.slot ?? null;
  }

  function resheet(slot: SheetSlot, by: -1 | 1) {
    const row = sheet.find((one) => one.slot === slot);
    if (!row) return;
    const all = bandRows(row.band);
    const here = all.findIndex((one) => one.slot === slot);
    if (here < 0) return;
    if (by < 0 ? here === 0 : here === all.length - 1) return;
    // По списку БЕЗ взятой строки: она вынимается первой, и «на одну ниже» —
    // это перед той, что стоит через одну, а не перед соседкой.
    const rest = all.filter((one) => one.slot !== slot);
    moveRow(slot, row.band, rest[by < 0 ? here - 1 : here + 1]?.slot ?? null);
  }

  function resetSheet() {
    if (!frames[frameIndex]) return;
    mark();
    frames[frameIndex].sheet = defaultSheet();
  }

  /* ── Сторож переполнения ────────────────────────────────────────────────
     Написать на карту больше, чем держит окно, можно молча: полоса свойств не
     выливается, а прокручивается, и лишнее просто пропадает — полосы прокрутки
     на карте никто не видит и не ищет. Карт десятки, руками их не перебрать,
     поэтому мерит не глаз, а сама карта. */

  /** На сколько не влезло у карты, которая открыта на столе. */
  let draftFit = $state(0);

  /**
   * Стенд «Лица карты»: одна и та же карта в трёх величинах разом.
   *
   * Ширины не круглые для красоты — это те самые числа, по которым карта
   * делит себя сама (281 и 160), взятые по обе стороны от порогов, чтобы
   * каждый стенд стоял ЗАВЕДОМО в своей полосе, а не на её краю. Первая —
   * рабочая: за неё тянут строки, остальные две стоят свидетелями, и без них
   * ступень «только крупно» на столе не видна вообще.
   */
  const FACE_SIZES: { px: number; label: TranslationKey }[] = [
    { px: 400, label: "adminBattlesFitSheet" },
    { px: 261, label: "adminBattlesFitShelf" },
    { px: 140, label: "adminBattlesFitCell" },
  ];

  /**
   * Ширины, на которых меряют. Клетки боя здесь нет нарочно: уже́ 160 px полоса
   * свойств перестаёт быть коробкой с прокруткой (`overflow: visible`), там
   * переполниться нечему, и лишняя треть работы ничего бы не сказала.
   */
  const FIT_WIDTHS: { px: number; label: TranslationKey }[] = [
    { px: 261, label: "adminBattlesFitShelf" },
    { px: 400, label: "adminBattlesFitSheet" },
  ];

  /** Идёт ли замер. Пока идёт — на столе стоит невидимая стопка настоящих
   *  карт: считать переполнение иначе, чем это делает комната, значит завести
   *  вторую правду о том, что влезает. */
  let fitProbe = $state(false);
  let fitOver = $state<Record<string, Record<number, number>>>({});

  function noteFit(id: string, width: number, over: number) {
    const was = fitOver[id]?.[width] ?? 0;
    if (was === over) return;
    fitOver = { ...fitOver, [id]: { ...(fitOver[id] ?? {}), [width]: over } };
  }

  let fitTrouble = $derived(
    cards
      .map((card) => ({
        card,
        where: FIT_WIDTHS.filter(({ px }) => (fitOver[card.id]?.[px] ?? 0) > 0),
      }))
      .filter((row) => row.where.length > 0),
  );

  /**
   * Кто на самом деле носит открытый чин.
   *
   * Стол рамок правит чин, а гость видит КАРТУ — и между ними стоит цепочка
   * нарядов: карта может носить свою картинку, раса — свою на каждом уровне.
   * Без этой полки хранитель красит бумагу вслепую и не знает, что чин, над
   * которым он сидит, не виден ни на одной карте полки.
   */
  let rankWorn = $derived.by(() => {
    const tier = frames[frameIndex]?.tier ?? 1;
    const mine = cards.filter((one) => clampTier(one.tier) === tier);
    const own: BattleCardDto[] = [];
    const byRace: BattleCardDto[] = [];
    const plain: BattleCardDto[] = [];
    for (const one of mine) {
      // Свой наряд карты перекрывает чин ВСЕГДА; наряд расы — только на тех
      // уровнях, которые она одела, поэтому это разные списки, а не один.
      if (parseFrameOverride(one.frameOverride)) own.push(one);
      else if (parseLevelFrames(one.raceLevelFrames).some(Boolean)) byRace.push(one);
      else plain.push(one);
    }
    return { mine, own, byRace, plain };
  });

  /** Открыть карту с любой вкладки: полка нарядов — не список имён, а дверь. */
  function jumpToCard(card: BattleCardDto) {
    openCard(card);
    view = "cards";
  }

  /** Наряд этой карты принёс картинку рамы, но не принёс окна. */
  let draftDressBlind = $derived(
    dressWindowMissing(parseFrameOverride(draft.frameOverride)),
  );

  /** Показывать ли деталь целиком — все её копии разом. Отдельные копии
   *  гасятся своими галочками ниже. */
  function pieceShown(row: FramePiece): boolean {
    return KIND_SIDES[row.kind].some(
      (side) => row.piece.places[side]?.shown !== false,
    );
  }

  function showPiece(row: FramePiece, on: boolean) {
    mark();
    for (const side of KIND_SIDES[row.kind]) {
      const at = row.piece.places[side];
      if (at) at.shown = on;
    }
  }

  /** Какая КОПИЯ детали сейчас в руке — левый верхний угол, а не «угол»
   *  вообще: четыре угла кладут порознь. `id` — именованный слот или id
   *  добавленного украшения: анатомию рамы и завитки хранителя таскает один и
   *  тот же код. Одна на стол, и её же держит карта. */
  let sliceHeld = $state<{ id: string; side: SliceSide } | null>(null);

  // ── Что можно сделать с деталью, не уходя от неё ────────────────────────
  //
  // Раньше всё жило в колонке: чтобы убрать картинку или взять другую со
  // склада, надо было найти нужный блок среди шести. Теперь у взятой детали
  // прямо на карте появляется своя полоска — там, где на неё смотрят.

  /** Отложенная деталь: картинка, форма и вся укладка. Не системный буфер —
   *  это внутренняя мерка стола, а не текст, который куда-то вставляют. */
  let clip = $state<{
    image: string;
    kind: SliceKind;
    piece: SlicePiece;
  } | null>(null);

  function copyPiece(row: FramePiece) {
    clip = {
      image: row.image,
      kind: row.kind,
      piece: JSON.parse(JSON.stringify(row.piece)) as SlicePiece,
    };
  }

  /** Вырезать — это скопировать и убрать. Украшение уходит из списка целиком;
   *  именованный слот остаётся, потому что он анатомия рамы: у него отнимают
   *  картинку, а не место. */
  function cutPiece(row: FramePiece) {
    copyPiece(row);
    if (row.ornament) dropOrnament(row.id);
    else setPieceImage(row, "");
  }

  /** Вставить в ту деталь, что в руке: картинку, заполнение, разворот, связку
   *  и укладку каждой стороны, какая у обеих есть. У формы с другими копиями
   *  переносится то, что совпало, — угол в сторону не втиснуть. */
  function pastePiece(row: FramePiece) {
    if (!clip) return;
    setPieceImage(row, clip.image);
    row.piece.fit = clip.piece.fit;
    row.piece.turn = clip.piece.turn;
    row.piece.linked = clip.piece.linked;
    for (const side of KIND_SIDES[row.kind]) {
      const from = clip.piece.places[side];
      const to = row.piece.places[side];
      if (from && to) Object.assign(to, { ...from });
    }
  }

  /** Ещё одна такая же — новым украшением. Самый частый способ получить второй
   *  медальон: не искать ту же деталь на складе заново, а повторить ту, что уже
   *  встала как надо. */
  function twinPiece(row: FramePiece) {
    const twin = newOrnament(row.image, row.kind);
    twin.fit = row.piece.fit;
    twin.turn = row.piece.turn;
    twin.linked = row.piece.linked;
    twin.layer = row.piece.layer;
    for (const side of KIND_SIDES[row.kind]) {
      const from = row.piece.places[side];
      const to = twin.places[side];
      if (from && to) Object.assign(to, { ...from });
    }
    frames[frameIndex].ornaments.push(twin);
    sliceHeld = { id: twin.id, side: KIND_SIDES[row.kind][0] };
  }

  /** Видна ли ИМЕННО та копия, что в руке. У полоски на карте свой смысл:
   *  погасить эту, а не всю деталь — тем и отличается от глаза в списке. */
  function heldShown(): boolean {
    if (!sliceHeld || !heldRow) return true;
    return heldRow.piece.places[sliceHeld.side]?.shown !== false;
  }

  function toggleHeldCopy() {
    if (!sliceHeld || !heldRow) return;
    const at = heldRow.piece.places[sliceHeld.side];
    if (at) at.shown = at.shown === false;
  }

  const SLICE_NUMBERS = [
    { key: 'growX', label: 'adminBattlesSliceGrowX' },
    { key: 'growY', label: 'adminBattlesSliceGrowY' },
    { key: 'nudgeX', label: 'adminBattlesSliceNudgeX' },
    { key: 'nudgeY', label: 'adminBattlesSliceNudgeY' },
  ] as const satisfies readonly { key: 'growX' | 'growY' | 'nudgeX' | 'nudgeY'; label: TranslationKey }[];

  /** Как называется каждая копия. Полными словами, а не «ЛВ»: сокращение
   *  экономит три знака и стоит хранителю секунды на каждый выбор. */
  const SIDE_KEY: Record<SliceSide, TranslationKey> = {
    tl: 'adminBattlesSideTl',
    tr: 'adminBattlesSideTr',
    bl: 'adminBattlesSideBl',
    br: 'adminBattlesSideBr',
    top: 'adminBattlesSideTop',
    bottom: 'adminBattlesSideBottom',
    left: 'adminBattlesSideLeft',
    right: 'adminBattlesSideRight',
  };

  const FIT_KEY: Record<SliceFit, TranslationKey> = {
    stretch: 'adminBattlesSliceFitStretch',
    contain: 'adminBattlesSliceFitContain',
    cover: 'adminBattlesSliceFitCover',
    tile: 'adminBattlesSliceFitTile',
  };

  const TURN_KEY: Record<SliceTurn, TranslationKey> = {
    mirror: 'adminBattlesSliceTurnMirror',
    rotate: 'adminBattlesSliceTurnRotate',
    none: 'adminBattlesSliceTurnNone',
  };

  const STAGE_BASE = 320;
  const ZOOMS = [1, 1.5, 2, 3, 4];

  /** Во сколько раз увеличен предпросмотр. Не `transform`: карта меряет себя
   *  контейнерными единицами, поэтому увеличенная ширина увеличивает и резьбу,
   *  и шрифт по-настоящему, а `getBoundingClientRect` под перетаскиванием
   *  остаётся честным без единой поправки. Полтора, а не один: стол широкий, и
   *  карта в 320 px на нём теряется. */
  let stageZoom = $state(1.5);

  /**
   * Показывать карту так, как она стоит В КЛЕТКЕ БОЯ.
   *
   * Без этого кружок здоровья на столе недостижим: он выходит только в бою, а
   * стол — не бой, и хранитель, пришедший поправить здоровье, не находил на
   * карте ничего. Не второй облик: тот же `BattleCard` получает те же `alive`
   * и `hurt`, что даёт ему сцена, — стол показывает бой, а не рисунок боя, и
   * соврать поэтому не может.
   */
  let stageInMatch = $state(false);
  /** Сколько здоровья осталось у карты на стенде. Ползунок, а не поле: сургуч
   *  смотрят в движении — от целого к почти сломанному, — а не по числу. */
  let stageHurt = $state(0.55);

  /** Как называется каждый из шести именованных слотов. Украшение зовётся
   *  своей формой и номером: имени у него нет, а строка `/static/assets/…`
   *  именем не работает. */
  const SLOT_KEY: Record<SliceSlot, TranslationKey> = {
    corner: 'adminBattlesPieceCorner',
    sideH: 'adminBattlesPieceSideH',
    sideV: 'adminBattlesPieceSideV',
    cornerExtra: 'adminBattlesPieceCornerExtra',
    sideMidH: 'adminBattlesPieceSideMidH',
    sideMidV: 'adminBattlesPieceSideMidV',
  };

  /** Какое поле рамы держит картинку каждого слота. */
  const SLOT_FIELD = {
    corner: 'cornerImage',
    sideH: 'sideImageH',
    sideV: 'sideImageV',
    cornerExtra: 'cornerExtra',
    sideMidH: 'sideMidH',
    sideMidV: 'sideMidV',
  } as const satisfies Record<SliceSlot, keyof BattleFrame>;

  /** Одна строка списка деталей. Именованные слоты и свои украшения приходят
   *  сюда одинаково — иначе список врал бы про то, что лежит на карте. */
  interface FramePiece {
    id: string;
    kind: SliceKind;
    label: string;
    image: string;
    piece: SlicePiece;
    /** Украшение можно убрать и переназвать формой; слот — нельзя. */
    ornament: SliceOrnament | null;
  }

  let stack = $derived.by<FramePiece[]>(() => {
    const frame = frames[frameIndex];
    if (!frame) return [];
    const rows: (FramePiece & { at: number })[] = [];
    SLICE_SLOTS.forEach((slot, at) => {
      rows.push({
        id: slot,
        kind: SLICE_KIND[slot],
        label: $t(SLOT_KEY[slot]),
        image: String(frame[SLOT_FIELD[slot]] ?? '').trim(),
        piece: frame.slices[slot],
        ornament: null,
        at,
      });
    });
    frame.ornaments.forEach((one, i) => {
      rows.push({
        id: one.id,
        kind: one.kind,
        label: `${$t(KIND_KEY[one.kind])} · ${i + 1}`,
        image: one.image.trim(),
        piece: one,
        ornament: one,
        at: SLICE_SLOTS.length + i,
      });
    });
    // Сверху то, что рисуется поверх. При равных слоях выигрывает тот, кто
    // позже в разметке, — список обязан показывать ровно это, иначе он опишет
    // порядок, которого на карте нет.
    return rows.sort((a, b) => b.piece.layer - a.piece.layer || b.at - a.at);
  });

  // Полоска встаёт ЧУТЬ НИЖЕ КУРСОРА — у того места, куда хранитель только что
  // нажал.
  //
  // Сначала она вставала по коробке детали, и на высокой стороне это выносило
  // её к нижнему краю карты: коробка левой стороны идёт от притолоки до порога,
  // и «под коробкой» — это в самом низу, за полкарты от того места, куда
  // смотрели. У детали нет одной точки, которую можно назвать «где она»; у
  // нажатия есть.
  //
  // Меряется всё равно ОТ КАРТЫ, а не от стола: стол прокручивается,
  // центрирует карту и меняет ширину вместе с колонкой, и число, посчитанное
  // от него, уезжает ровно на половину этой разницы.
  let cardBox = $state<HTMLElement | null>(null);
  /** Стол. Полоска лежит В НЁМ, а не в карте: карта меняет ширину от
   *  увеличения и от колонки, и прибитая к ней полоска переезжала бы вместе с
   *  ней — а прибивают её как раз затем, чтобы она никуда не девалась. */
  let stageBox = $state<HTMLElement | null>(null);
  /** Насколько стол прокручен. Место полоски считается в его СОДЕРЖИМОМ, а
   *  прибитое держится за видимую часть, — значит, прокрутку надо знать, иначе
   *  увеличенную карту не увезти из-под неподвижной полоски. */
  let stageScroll = $state({ x: 0, y: 0 });
  let barTick = $state(0);
  let barWide = $state(0);
  let barTall = $state(0);
  /** Где нажали, в координатах карты. Пусто, когда деталь взяли из списка
   *  сбоку: тогда курсор был не на карте и указывать нечего. */
  let pokedAt = $state<{ x: number; y: number } | null>(null);
  let barAt = $state<{ x: number; y: number } | null>(null);

  /**
   * Куда полоску поставили рукой, в координатах ВИДИМОЙ части стола.
   *
   * Пусто — полоска ходит за курсором, как и ходила. Непусто — стоит там, где
   * её оставили, не пропадает, когда из руки всё выпустили, и всё равно
   * работает с тем, что в руке сейчас: место — не выбор детали.
   *
   * Оно и есть весь признак «прибита»: отдельный флажок рядом с координатами
   * рано или поздно разошёлся бы с ними — прибита, а места нет.
   */
  const BAR_PIN_KEY = "gotiga_battle_barpin";
  let barPin = $state<{ x: number; y: number } | null>(null);

  function rememberBar() {
    try {
      if (barPin) localStorage.setItem(BAR_PIN_KEY, JSON.stringify(barPin));
      else localStorage.removeItem(BAR_PIN_KEY);
    } catch {
      // Приватное окно или запрет на хранение. Полоска работает и без памяти.
    }
  }

  onMount(() => {
    try {
      const raw = localStorage.getItem(BAR_PIN_KEY);
      const put = raw ? JSON.parse(raw) : null;
      if (put && Number.isFinite(put.x) && Number.isFinite(put.y))
        barPin = { x: put.x, y: put.y };
    } catch {
      barPin = null;
    }
  });

  /** Прибить туда, где полоска стоит сейчас, — или отпустить обратно под
   *  курсор. Место снимается с самой полоски, а не считается заново: прибивают
   *  ровно то, на что смотрят. */
  function pinBar() {
    if (barPin) {
      barPin = null;
      rememberBar();
      return;
    }
    const stage = stageBox?.getBoundingClientRect();
    const bar = stageBox?.querySelector("[data-piece-bar]");
    if (!stage || !bar) return;
    const box = bar.getBoundingClientRect();
    barPin = {
      x: box.left + box.width / 2 - stage.left,
      y: box.top - stage.top,
    };
    rememberBar();
  }

  /** Смещение от курсора до полоски, пока её тащат. За рукоять берут где
   *  придётся, и без этого полоска прыгала бы к курсору серединой. */
  let barGrab: { x: number; y: number } | null = null;

  function grabBar(event: PointerEvent) {
    const hand = event.currentTarget as HTMLElement;
    const bar = hand.closest("[data-piece-bar]");
    if (!bar) return;
    const box = bar.getBoundingClientRect();
    barGrab = {
      x: box.left + box.width / 2 - event.clientX,
      y: box.top - event.clientY,
    };
    hand.setPointerCapture(event.pointerId);
    event.preventDefault();
  }

  function dragBar(event: PointerEvent) {
    const stage = stageBox?.getBoundingClientRect();
    if (!barGrab || !stage) return;
    // Тащат — значит, стоять ей здесь: иначе первое же нажатие по карте
    // забрало бы её обратно под курсор, и перенести полоску было бы нельзя.
    barPin = {
      x: event.clientX + barGrab.x - stage.left,
      y: event.clientY + barGrab.y - stage.top,
    };
  }

  function dropBar() {
    if (!barGrab) return;
    barGrab = null;
    rememberBar();
  }

  function poke(event: PointerEvent) {
    const card = cardBox?.getBoundingClientRect();
    if (!card) return;
    // Нажатие по самой полоске сюда не приходит: она лежит на столе рядом с
    // картой, а не в ней. Пока она была внутри, каждое нажатие по её же
    // кнопке уводило её на восемнадцать точек вниз — из-под пальца, ещё до
    // отпускания, — и нажать на неё было нельзя вовсе.
    pokedAt = { x: event.clientX - card.left, y: event.clientY - card.top };
  }

  $effect(() => {
    void barTick;
    // Пересчитывается на любую правку рамы, потому что деталь под полоской
    // могла как раз поехать или вырасти.
    void shot;
    const held = sliceHeld;
    const room = cardBox;
    if (!held || !room) {
      barAt = null;
      return;
    }
    const card = room.getBoundingClientRect();
    const desk = stageBox?.getBoundingClientRect();
    if (!card.width || !desk) return;
    // Из координат карты — в координаты содержимого стола, где полоска и
    // лежит. Прокрутка входит в обе половины и потому не меняет число.
    const dx = card.left - desk.left + (stageBox?.scrollLeft ?? 0);
    const dy = card.top - desk.top + (stageBox?.scrollTop ?? 0);
    if (pokedAt) {
      barAt = { x: pokedAt.x + dx, y: pokedAt.y + dy + 18 };
      return;
    }
    // Взяли из списка — курсора на карте не было. Тогда у НАЧАЛА детали: у
    // высокой это её верх, а не низ, и это ближе к тому, с чего работу с ней
    // начинают.
    const at = room.querySelector(
      `[data-piece="${CSS.escape(held.id)}"][data-side="${held.side}"]`,
    );
    // У пустого слота копий на карте нет — и полоска была бы недостижима ровно
    // тогда, когда через неё и берут картинку. Такая деталь получает полоску
    // посреди карты: место неточное, но зато оно есть.
    if (!at) {
      barAt = { x: card.width / 2 + dx, y: card.height / 2 + dy };
      return;
    }
    const box = at.getBoundingClientRect();
    barAt = {
      x: box.left + box.width / 2 - card.left + dx,
      y: box.top - card.top + Math.min(box.height / 2, 18) + dy,
    };
  });

  /**
   * Полоска не должна уезжать за край СТОЛА — но и подбираться к середине
   * карты раньше времени тоже не должна.
   *
   * Зажим по карте был бы проще и хуже: полоска шириной в три четверти карты
   * от любого нажатия у края отпрыгивала бы к центру, то есть переставала бы
   * стоять под курсором ровно там, где по краю и работают. Стол шире карты на
   * добрых полторы сотни точек с каждой стороны, и свисать на них полоске
   * ничто не мешает.
   */
  function barLeft(x: number): number {
    const stage = stageBox;
    const half = barWide / 2;
    if (!stage || !barWide) return x;
    // Видимая часть стола, в координатах его содержимого.
    const from = stageScroll.x + half + 4;
    const to = stageScroll.x + stage.clientWidth - half - 4;
    if (from > to) return x;
    return Math.min(Math.max(x, from), to);
  }

  /** Вниз полоска не зажималась никогда: под курсором ей место и у нижнего
   *  края. Прибитую зажимать приходится — окно с тех пор могли уменьшить, а
   *  полоска, оказавшаяся за краем, недостижима. */
  function barTop(y: number): number {
    const stage = stageBox;
    if (!stage || !barTall) return y;
    const from = stageScroll.y + 4;
    const to = stageScroll.y + stage.clientHeight - barTall - 4;
    if (from > to) return y;
    return Math.min(Math.max(y, from), to);
  }

  /** Где полоска стоит: прибитая — там, где её оставили (место держится за
   *  видимую часть стола, поэтому прокрутка её не уносит), прочая — под тем
   *  местом, куда нажали. */
  let barSpot = $derived.by<{ x: number; y: number } | null>(() => {
    if (barPin)
      return {
        x: barLeft(barPin.x + stageScroll.x),
        y: barTop(barPin.y + stageScroll.y),
      };
    return barAt && { x: barLeft(barAt.x), y: barAt.y };
  });

  /** Что сейчас в руке, строкой списка. */
  let heldRow = $derived(stack.find((row) => row.id === sliceHeld?.id) ?? null);

  // ── Отмена, и знание о несохранённом ────────────────────────────────────
  //
  // Перетаскивание пишет в раму сразу. Без отмены одно движение не по той
  // детали стоит хранителю всех чисел, и трогать становится страшно — а
  // прямое манипулирование живо ровно тем, что пробовать не страшно.

  let history = $state<string[]>([]);
  let ahead = $state<string[]>([]);
  /** Слепок, записанный на сервер последним. Всё, что от него отличается, —
   *  несохранённое, и об этом должно быть видно. */
  let stored = $state("");
  let shot = $derived(JSON.stringify(frames));
  let dirty = $derived(!!stored && shot !== stored);

  /** Снять слепок ПЕРЕД правкой. Зовётся один раз на жест: с карты — в начале
   *  перетаскивания, из колонки — перехватом нажатия и фокуса на всей колонке,
   *  так что ни один орган управления не приходится оборачивать вручную. */
  function mark() {
    const now = JSON.stringify(frames);
    if (history[history.length - 1] === now) return;
    history.push(now);
    if (history.length > 60) history.shift();
    ahead = [];
  }

  function stepBack() {
    const was = history.pop();
    if (!was) return;
    ahead.push(JSON.stringify(frames));
    frames = (JSON.parse(was) as BattleFrame[]).map(completeSlices);
    sliceHeld = null;
  }

  function stepOn() {
    const next = ahead.pop();
    if (!next) return;
    history.push(JSON.stringify(frames));
    frames = (JSON.parse(next) as BattleFrame[]).map(completeSlices);
    sliceHeld = null;
  }

  /** Стрелки двигают взятую копию. Мышь на карте в 320 px даёт 0.31 % на
   *  пиксель — точнее неё клавиатура и должна быть, а не грубее, как было при
   *  шаге в полпроцента. Alt — не двигает, а наращивает нахлёст. */
  function nudgeHeld(event: KeyboardEvent) {
    if (!sliceHeld) return;
    const way: Record<string, [number, number]> = {
      ArrowLeft: [-1, 0],
      ArrowRight: [1, 0],
      ArrowUp: [0, -1],
      ArrowDown: [0, 1],
    };
    const step = way[event.key];
    if (!step) return;
    const frame = frames[frameIndex];
    const piece = livePiece(frame, sliceHeld.id);
    const kind = kindOf(frame, sliceHeld.id);
    if (!piece || !kind) return;
    event.preventDefault();
    mark();
    const by = event.shiftKey ? 1 : 0.1;
    const dx = step[0] * by;
    const dy = step[1] * by;
    const sign = sliceSigns(sliceHeld.side);
    const sides = piece.linked !== false ? KIND_SIDES[kind] : [sliceHeld.side];
    const hold = (v: number) =>
      Math.min(SLICE_GROW_MAX, Math.max(-SLICE_GROW_MAX, v));
    for (const side of sides) {
      const at = piece.places[side];
      if (!at) continue;
      if (event.altKey) {
        at.growX = hold(at.growX + dx * sign.growX);
        at.growY = hold(at.growY + dy * sign.growY);
      } else {
        at.nudgeX = hold(at.nudgeX + dx * sign.nudgeX);
        at.nudgeY = hold(at.nudgeY + dy * sign.nudgeY);
      }
    }
  }

  function stageKeys(event: KeyboardEvent) {
    const meta = event.metaKey || event.ctrlKey;
    if (meta && event.key.toLowerCase() === "z") {
      event.preventDefault();
      if (event.shiftKey) stepOn();
      else stepBack();
      return;
    }
    if (event.key === "Escape") {
      sliceHeld = null;
      return;
    }
    nudgeHeld(event);
  }

  /** Back to the placement the piece has always had — the way out of an
   *  experiment, without hunting a dozen numbers back to zero by hand. Its
   *  picture and its shape are not part of the experiment and stay. */
  function resetSlice(id: string) {
    const frame = frames[frameIndex];
    if ((SLICE_SLOTS as string[]).includes(id)) {
      frame.slices[id as SliceSlot] = defaultSlices()[id as SliceSlot];
      return;
    }
    const one = frame.ornaments.find((each) => each.id === id);
    if (one)
      Object.assign(one, newOrnament(one.image, one.kind), { id: one.id });
  }

  /** What each shape is called, and where its copies land. */
  const KIND_KEY: Record<SliceKind, TranslationKey> = {
    corner: "adminBattlesKindCorner",
    edgeH: "adminBattlesKindEdgeH",
    edgeV: "adminBattlesKindEdgeV",
    midH: "adminBattlesKindMidH",
    midV: "adminBattlesKindMidV",
  };

  // ── Свои украшения ──────────────────────────────────────────────────────
  //
  // Шесть слотов — анатомия рамы: два угла картинки и две стороны, названные
  // потому, что платье, надетое на другой ранг, должно значить там то же самое.
  // Украшение — не анатомия, а завиток, которого захотела именно эта рама, и
  // честного постоянного числа таких завитков не бывает. Поэтому список.

  /** Свежий завиток из уже нарезанной детали — обычный путь: детали приходят
   *  листами, и перезаливать по одной то, что лежит на складе, значит плодить
   *  копии одного файла. */
  function addOrnamentFromStore() {
    fromStore("accent", (url) => {
      frames[frameIndex].ornaments.push(newOrnament(url));
    });
  }

  async function addOrnamentUpload() {
    const file = await pickImageFile();
    if (!file) return;
    uploading = true;
    try {
      const art = await api.adminUploadBattleFrameArt(file);
      frames[frameIndex].ornaments.push(newOrnament(art.url));
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      uploading = false;
    }
  }

  /** Убрать завиток. Взятое в руку отпускается заодно — иначе стол держал бы
   *  то, чего на карте больше нет. */
  function dropOrnament(id: string) {
    const frame = frames[frameIndex];
    frame.ornaments = frame.ornaments.filter((one) => one.id !== id);
    if (sliceHeld?.id === id) sliceHeld = null;
  }

  /** Сменить форму украшения. Копии у форм разные, поэтому места пересобираются
   *  под новую — иначе у «угла», ставшего «верхом», не оказалось бы ни одного
   *  места, в которое можно писать. */
  function reshapeOrnament(one: SliceOrnament, kind: SliceKind) {
    Object.assign(one, newOrnament(one.image, kind), {
      id: one.id,
      layer: one.layer,
    });
    if (sliceHeld?.id === one.id)
      sliceHeld = { id: one.id, side: KIND_SIDES[kind][0] };
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
    keywordNameEn = keyword?.nameEn ?? "";
    keywordNameRu = keyword?.nameRu ?? "";
    keywordRulesEn = keyword?.rulesEn ?? "";
    keywordRulesRu = keyword?.rulesRu ?? "";
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
      flash($t("adminBattlesSaved"));
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      saving = false;
    }
  }

  async function removeKeyword(keyword: BattleKeyword) {
    if (!confirm($t("adminBattlesKeywordDelete") + "?")) return;
    try {
      await api.adminDeleteBattleKeyword(keyword.id);
      keywords = await api.getBattleKeywords();
      if (keywordDraftId === keyword.id) openKeyword(null);
      flash($t("adminBattlesDeleted"));
    } catch (e) {
      flash(String(e), 6000);
    }
  }

  function openRace(race: BattleRace | null) {
    raceDraftId = race?.id ?? null;
    raceNameEn = race?.nameEn ?? "";
    raceNameRu = race?.nameRu ?? "";
    raceNoteEn = race?.noteEn ?? "";
    raceNoteRu = race?.noteRu ?? "";
    raceIconUrl = race?.iconUrl ?? "";
    raceMotionWear = race?.motionWear ?? null;
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
          levelFrames: raceLevelFrames.some((f) => f)
            ? JSON.stringify(raceLevelFrames)
            : null,
          motionWear: raceMotionWear,
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
      flash($t("adminBattlesRaceSaved"));
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
        frameMode: art.hasAlpha ? "overlay" : "behind",
      };
      if (!art.hasAlpha) flash($t("adminBattlesFrameNoAlpha"), 8000);
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
    if (!confirm($t("adminBattlesRaceDeleteConfirm"))) return;
    try {
      await api.adminDeleteBattleRace(race.id);
      races = await api.getBattleRaces();
      await loadCards();
      if (raceDraftId === race.id) openRace(null);
      flash($t("adminBattlesRaceDeleted"));
    } catch (e) {
      flash(String(e), 6000);
    }
  }

  /** The per-card editor's icon click, when the icon belongs to a race
   *  someone else's card also wears: sent here instead of editing it in place. */
  function jumpToRace() {
    const race = races.find((r) => r.id === draft.raceId) ?? null;
    openRace(race);
    view = "races";
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
    draft.raceMotionWear = picked?.motionWear ?? null;
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
    damage: "battlesVerbDamage",
    dot: "battlesVerbDot",
    heal: "battlesVerbHeal",
    hot: "battlesVerbHot",
    shield: "battlesVerbShield",
    zone: "battlesVerbZone",
    bless: "battlesVerbBless",
    curse: "battlesVerbCurse",
    control: "battlesVerbControl",
    silence: "battlesVerbSilence",
    disarm: "battlesVerbDisarm",
    charm: "battlesVerbCharm",
    veil: "battlesVerbVeil",
    guard: "battlesVerbGuard",
    immune: "battlesVerbImmune",
    thorns: "battlesVerbThorns",
    move: "battlesVerbMove",
    summon: "battlesVerbSummon",
    sacrifice: "battlesVerbSacrifice",
    cleanse: "battlesVerbCleanse",
    dispel: "battlesVerbDispel",
    mana: "battlesVerbMana",
  } as const satisfies Record<AbilityVerb, TranslationKey>;

  const SHAPE_LABELS = {
    self: "battlesShapeSelf",
    one: "battlesShapeOne",
    adjacent: "battlesShapeAdjacent",
    chain: "battlesShapeChain",
    line: "battlesShapeLine",
    radius: "battlesShapeRadius",
    side: "battlesShapeSide",
    cell: "battlesShapeCell",
  } as const satisfies Record<AbilityShape, TranslationKey>;

  const TRIGGER_LABELS = {
    active: "battlesTriggerActive",
    onPlay: "battlesTriggerOnPlay",
    onHit: "battlesTriggerOnHit",
    onDamaged: "battlesTriggerOnDamaged",
    onDeath: "battlesTriggerOnDeath",
    turnStart: "battlesTriggerTurnStart",
    aura: "battlesTriggerAura",
    once: "battlesTriggerOnce",
  } as const satisfies Record<AbilityTrigger, TranslationKey>;

  /** Значки каналов. Формам и поводам такой таблицы не нужно: их значки
   *  названы теми же словами, что и сами значения, и разойтись им негде. */
  const CHANNEL_ICON = {
    physical: "sword",
    magic: "spark",
    pure: "pure",
    none: "nil",
  } as const satisfies Record<BattleChannel, string>;

  const CHANNELS = Object.keys(CHANNEL_ICON) as BattleChannel[];

  const CHANNEL_LABELS = {
    physical: "adminBattlesChannelPhysical",
    magic: "adminBattlesChannelMagic",
    pure: "adminBattlesChannelPure",
    none: "adminBattlesChannelNone",
  } as const satisfies Record<BattleChannel, TranslationKey>;

  /**
   * Значки глаголов.
   *
   * Таблица, а не совпадение имён, как у форм и поводов: пять глаголов носят
   * значок, который уже есть у чего-то другого и означает то же самое, —
   * «урон» это меч, «мана» это капля, — и рисовать им вторые такие же было бы
   * два рисунка одного предмета, которые однажды разойдутся.
   */
  const VERB_ICON = {
    damage: "sword",
    dot: "flame",
    heal: "sprig",
    hot: "bloom",
    shield: "shield",
    zone: "zone",
    bless: "bless",
    curse: "curse",
    control: "control",
    silence: "silence",
    disarm: "disarm",
    charm: "charm",
    veil: "veil",
    guard: "guard",
    immune: "immune",
    thorns: "thorns",
    move: "move",
    summon: "summon",
    sacrifice: "sacrifice",
    cleanse: "cleanse",
    dispel: "dispel",
    mana: "drop",
  } as const satisfies Record<AbilityVerb, string>;

  const VERBS = Object.keys(VERB_LABELS) as AbilityVerb[];
  const SHAPES = Object.keys(SHAPE_LABELS) as AbilityShape[];
  const TRIGGERS = Object.keys(TRIGGER_LABELS) as AbilityTrigger[];

  /** Только `chain` и `radius` несут число; у остальных поле нечего заполнять. */
  const shapeCarriesNumber = (shape: string) =>
    shape === "chain" || shape === "radius";

  function addAbility() {
    const list = draft.abilities ?? [];
    if (list.length >= ABILITIES_MAX) return;
    draft.abilities = [
      ...list,
      {
        // Собственный id внутри карты: по нему весы кладут число к нужной строке.
        id: `a${Date.now().toString(36)}`,
        nameEn: "",
        nameRu: "",
        verb: "damage",
        channel: "physical",
        amount: 1,
        shape: "one",
        radius: 1,
        range: 1,
        duration: 0,
        trigger: "active",
        manaCost: 0,
        cooldown: 0,
        keywords: [],
      },
    ];
    // Заведённое берётся в руку сразу: его затем и заводили, а лента без этого
    // отрастила бы кружок, настройки которого лежат под другим кружком.
    abilityHeld = draft.abilities[draft.abilities.length - 1].id;
  }

  /**
   * Какое умение в руке.
   *
   * Держим ИМЯ строки, а не её номер — по той же причине, по которой деталь
   * рамы держат за `id`: номер меняется от всякого перемещения по списку, и
   * поднятое вверх умение выпадало бы из рук на каждый щелчок. Стёртое
   * уступает первому: пустых рук у панели с умениями не бывает.
   */
  let abilityHeld = $state<string | null>(null);
  let abilityAt = $derived.by(() => {
    const list = draft.abilities ?? [];
    if (!list.length) return -1;
    const at = list.findIndex((a) => a.id === abilityHeld);
    return at >= 0 ? at : 0;
  });
  let abilityInHand = $derived(
    abilityAt >= 0 ? (draft.abilities ?? [])[abilityAt] : null,
  );

  /** Чем умение подписано в ленте: своим именем, а если его нет — глаголом.
   *  Безымянных умений на карте много, и «без названия» пять раз подряд не
   *  говорит ничего, а глагол говорит всё. */
  function abilityName(ability: CardAbility): string {
    const own = (editLang === "en" ? ability.nameEn : ability.nameRu)?.trim();
    return own || $t(VERB_LABELS[ability.verb]);
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
      keywords: raw
        .split(",")
        .map((k) => k.trim())
        .filter(Boolean)
        .slice(0, 4),
    };
    draft.abilities = list;
  }

  const abilityPoints = (id: string) =>
    weigh?.abilities.find((a) => a.id === id)?.points ?? null;

  function addTrait() {
    const list = draft.traits ?? [];
    if (list.length >= TRAITS_MAX) return;
    draft.traits = [
      ...list,
      { nameEn: "", nameRu: "", textEn: "", textRu: "" },
    ];
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

  function priceInput(coin: "priceDust" | "priceFeed", raw: string) {
    const text = raw.trim();
    draft[coin] = text === "" ? null : Math.max(0, Math.round(Number(text)));
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
    if (text === "") {
      draft.levelPriceDust = null;
      return;
    }
    const next = [...ladder];
    next[step] = Math.max(0, Math.round(Number(text)));
    draft.levelPriceDust = next;
  }

  // ── Сыгранные партии ───────────────────────────────────────────────────
  //
  // Единственное окно в живую игру. До него партии писались в базу, а
  // посмотреть их было нечем: баланс правился симуляцией по правилам, которых
  // игроки не видели.
  //
  // Читается по щелчку на вкладку, а не при открытии стола: разбор пятисот
  // партий не нужен тому, кто пришёл поправить рамку.

  let matches = $state<BattleMatches | null>(null);
  let matchesBusy = $state(false);

  async function loadMatches() {
    matchesBusy = true;
    try {
      matches = await api.adminReadBattleMatches();
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      matchesBusy = false;
    }
  }

  function openMatches() {
    view = "matches";
    if (!matches) void loadMatches();
  }

  // ── Пересмотр одной партии ─────────────────────────────────────────────
  //
  // Доску рисует тот же `BattleScene`, что и комната гостей: увидеть надо
  // ровно то, что видел человек. Правила берутся из записи, а не нынешние, —
  // это делает сервер, здесь об этом знать нечего.
  //
  // Управление у сцены остаётся «за гостя», но список законных действий пуст,
  // поэтому нажать нельзя ничего: пересмотр — это чтение.

  let replayId = $state<string | null>(null);
  let replay = $state<MatchReplay | null>(null);
  let replayBusy = $state(false);

  async function stepTo(id: string, upto: number) {
    replayBusy = true;
    try {
      replay = await api.adminReplayBattleMatch(id, Math.max(0, upto));
      replayId = id;
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      replayBusy = false;
    }
  }

  function closeReplay() {
    replayId = null;
    replay = null;
  }

  /** Партия притворяется живой, чтобы доску рисовал тот же компонент. */
  let replayMatch = $derived.by((): BattleMatch | null =>
    replay
      ? {
          id: "replay",
          challengeId: null,
          seq: 0,
          state: replay.state,
          legalActions: [],
          events: replay.events,
          outcome: replay.outcome,
          rewardDust: 0,
        }
      : null,
  );

  /** Доля побед одной стороны. Пусто, пока считать не из чего: «0 %» при нуле
   *  партий — это не ноль, это отсутствие ответа. */
  function share(part: number, whole: number): string {
    return whole > 0 ? `${Math.round((100 * part) / whole)}%` : "—";
  }

  const OUTCOME_WORD: Record<string, TranslationKey> = {
    player: "adminBattlesOutcomeGuest",
    keeper: "adminBattlesOutcomeKeeper",
    draw: "adminBattlesOutcomeDraw",
  };

  const tallyTitle = (t: {
    titleRu: string | null;
    titleEn: string | null;
    slug?: string;
  }) =>
    ($lang === "ru" ? t.titleRu || t.titleEn : t.titleEn || t.titleRu) ||
    t.slug ||
    "—";

  const shortDate = (iso: string) =>
    new Date(iso).toLocaleString($lang === "ru" ? "ru-RU" : "en-GB", {
      day: "2-digit",
      month: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    });

  // ── Ставки начисления за внимание ──────────────────────────────────────
  let dustRates = $state<BattleDustRates>({ liked: 2, seen: 1, read: 3 });
  let ratesSaving = $state(false);

  // ── Дар первого входа ──────────────────────────────────────────────────
  //
  // Стоит рядом со ставками, потому что это то же самое: числа, которые
  // подбираются на живом доме, а не закладываются в миграцию. Умолчания здесь
  // повторяют серверные — стол, который до ответа показывает нули, соврал бы
  // хранителю ровно про то, что тот пришёл править.
  let gift = $state<BattleWelcomeGift>({ dust: 10, feed: 1 });
  let giftSaving = $state(false);

  // ── Часы дома ──────────────────────────────────────────────────────────
  //
  // По ним поворачивается «сегодня» у повторяющихся поручений. Одно число —
  // и оно стоит здесь, а не в столе поручений, потому что это настройка
  // комнаты целиком, как ставки и дар: у каждого поручения свои часы были бы
  // не настройкой, а способом однажды запутаться.
  let clock = $state<BattleClock>({ offsetMin: 180 });
  let clockSaving = $state(false);

  // ── Дела, выдаваемые рукой ─────────────────────────────────────────────
  //
  // Те самые поручения, что названы гостю на полке и которые дом не платит.
  // Здесь они — заготовки выдачи: щелчок заполняет монету, число и записку,
  // чтобы «состоявшийся показ» стоил везде одинаково и назывался одинаково.
  // Перебивать заполненное можно: заготовка — это память, а не правило.
  let deeds = $state<AdminBattleErrand[]>([]);

  async function loadDustRates() {
    try {
      dustRates = await api.adminGetBattleDustRates();
    } catch {
      // Настройка не прочиталась — стол показывает умолчания и не мешает.
    }
    try {
      gift = await api.adminGetBattleGift();
    } catch {
      // То же: дар читается отдельно, и одно не должно ронять другое.
    }
    try {
      clock = await api.adminGetBattleClock();
    } catch {
      // И часы тоже: три настройки, три отдельных чтения.
    }
    try {
      deeds = (await api.adminListBattleErrands()).filter(
        (e) => e.byHand && e.status === "published",
      );
    } catch {
      // Заготовки — удобство. Без них выдача работает как раньше, руками.
    }
  }

  function takeDeed(deed: AdminBattleErrand) {
    grantCoin = deed.currency;
    grantAmount = deed.amount;
    grantNote = $lang === "ru" ? deed.titleRu : deed.titleEn;
  }

  async function saveClock() {
    clockSaving = true;
    try {
      clock = await api.adminSaveBattleClock(clock);
      flash($t("adminBattlesClockSaved"), 2500);
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      clockSaving = false;
    }
  }

  async function saveGift() {
    giftSaving = true;
    try {
      gift = await api.adminSaveBattleGift(gift);
      flash($t("adminBattlesGiftSaved"), 2500);
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      giftSaving = false;
    }
  }

  async function saveDustRates() {
    ratesSaving = true;
    try {
      dustRates = await api.adminSaveBattleDustRates(dustRates);
      flash($t("adminBattlesRatesSaved"), 2500);
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
    draft: "bg-[#b9a68f]",
    published: "bg-emerald-600",
    retired: "bg-[#b0a08e]",
  };

  function flash(text: string, ms = 3000) {
    message = text;
    if (flashTimer) clearTimeout(flashTimer);
    flashTimer = setTimeout(() => (message = ""), ms);
  }

  function titleOf(card: BattleCardDto): string {
    return (
      ($lang === "ru"
        ? card.titleRu || card.titleEn
        : card.titleEn || card.titleRu) || "—"
    );
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
      traits: (draft.traits ?? []).filter(
        (t) => t.nameEn.trim() || t.nameRu.trim(),
      ),
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
      lendable: draft.lendable,
      artUrl: draft.artUrlOverride?.trim() || null,
      artFocal: draft.artFocal,
      frameOverride: draft.frameOverride,
      motionWear: draft.motionWear ?? null,
      figurineId: draft.figurineId || null,
    };
  }

  /** Наряд движений расы, за которой карта записана СЕЙЧАС — не тот, что
   *  приехал с карты: расу в форме меняют, и подпись «как у расы» обязана
   *  меняться вместе с ней, иначе она показывает вчерашнюю. */
  let raceWear = $derived(
    (draft.raceId && races.find((r) => r.id === draft.raceId)?.motionWear) || null,
  );

  let weigh = $state<BattleWeigh | null>(null);
  let weighTimer: ReturnType<typeof setTimeout> | undefined;

  /**
   * Когда весы спросить заново — то есть ВСЁ, что уезжает на сервер.
   *
   * Здесь стоял список полей, набранный руками: чин, цена, здоровье, сила и
   * ещё несколько. Он был верен ровно как «всё, что меняет ВЕС», — и перестал
   * быть верным в тот день, когда тем же ответом поехала годность. Годность
   * считается по имени, по строке эффекта и по свойствам, а их в списке не
   * было, потому что на вес они не влияют. Получалось так: опубликованная
   * карта требует имени на обоих языках, хранитель его набирает — и ничего не
   * происходит. Запрос не уходит, ответ прежний, надпись стоит, кнопка серая.
   * Заполнить было нельзя ничем.
   *
   * Поэтому мерка снимается с самого тела запроса. Оно уже собрано в одном
   * месте и ровно для того, чтобы весы и кнопка не расходились; список полей
   * рядом с ним — это второе место, где сказано то же самое, и второе место
   * однажды отстаёт. Лишние запросы (наводка, наряд) стоят одного маленького
   * POST раз в четверть секунды и не стоят ни одного часа на такую поломку.
   */
  let weighKey = $derived(JSON.stringify(cardBody()));

  /** Как карта зовётся НА ПОЛКЕ, а не в поле ввода. Связь этюдов идёт по
   *  сохранённому слугу: пока правку не записали, они смотрят на старое имя. */
  let savedSlug = $derived(cards.find((c) => c.id === selectedId)?.slug ?? "");

  /** Этюды, которые называют эту карту. Считается здесь, а не спрашивается у
   *  сервера: полка этюдов уже загружена целиком, и второй источник той же
   *  правды разошёлся бы с первым. */
  let etudesUsing = $derived(
    savedSlug
      ? challenges.filter(
          (c) =>
            [...c.setup.playerBoard, ...c.setup.keeperBoard].some(
              (p) => p.card === savedSlug,
            ) ||
            [...c.setup.playerHand, ...c.setup.keeperHand].some(
              (slug) => slug === savedSlug,
            ),
        )
      : [],
  );

  /** Снять со стола: карта уходит с доски у всех, кто её называет. В отличие от
   *  переименования это НЕ переезжает — этюд просто останется без тела. */
  let willEmptyEtudes = $derived(
    etudesUsing.length > 0 && draft.status !== "published",
  );

  /** Вылезла ли карта за бюджет своего чина. Считает сервер той же формулой,
   *  которой откажет при сохранении; здесь — чтобы полоска покраснела раньше,
   *  чем придёт отказ. */
  let overBudget = $derived(
    !!weigh && weigh.totalPoints > tierBudget(draft.tier),
  );
  let budgetFill = $derived(
    weigh
      ? Math.min(100, (weigh.totalPoints / tierBudget(draft.tier)) * 100)
      : 0,
  );

  /** Пока непусто — сохранить нельзя, и сервер откажет тем же словом. */
  let blocking = $derived(weigh?.readiness.blocking ?? []);

  /**
   * Куда на листе показывает каждый отказ.
   *
   * Разделение здесь такое же, как между картой и рамой, и держать его надо
   * так же твёрдо: сервер решает, ЧТО не так, — стол знает только, ГДЕ это
   * лежит. Второй раз проверять «пусто ли имя», чтобы решить, отказывать ли,
   * значит завести второе правило, которое однажды разойдётся с первым. А
   * знать, что отказ по имени показывает на поле имени, — не правило, а карта
   * комнаты, и без неё жалоба внизу листа не помогает никому.
   */
  const FAULT_AT: Record<string, string> = {
    noTitle: "fault-title",
    noEffect: "fault-effect",
    traitsWithoutAbilities: "fault-abilities",
    noHealth: "fault-health",
    costBeyondMana: "fault-cost",
    overTierBudget: "fault-body",
  };
  let faults = $derived(new Set(blocking));

  /** На каком языке пусто. Спрашивается ТОЛЬКО чтобы навести на нужную
   *  сторону переключателя: отказал уже сервер, здесь выбирают, куда вести. */
  function blankSide(field: "title" | "effect"): "en" | "ru" | null {
    const en = ((field === "title" ? draft.titleEn : draft.effectEn) ?? "").trim();
    const ru = ((field === "title" ? draft.titleRu : draft.effectRu) ?? "").trim();
    // Сперва то, что перед глазами. Пусто здесь — вести некуда, поле уже под
    // курсором; иначе на пустых обеих сторонах отказ говорил бы «заполнено, но
    // пусто на другом языке», стоя над пустым полем.
    if (!(editLang === "en" ? en : ru)) return null;
    if (!en) return "en";
    if (!ru) return "ru";
    return null;
  }

  /** Отвести к полю, на которое показывает отказ. Языковую сторону меняем до
   *  прокрутки: иначе хранитель приезжает к полю, которое уже заполнил. */
  function goToFault(fault: string) {
    if (fault === "noTitle" || fault === "noEffect") {
      const side = blankSide(fault === "noTitle" ? "title" : "effect");
      if (side) editLang = side;
    }
    const at = FAULT_AT[fault];
    if (!at) return;
    // Плавность — не украшение: `prefers-reduced-motion` в доме обязательство.
    const still = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    requestAnimationFrame(() => {
      const box = document.getElementById(at);
      if (!box) return;
      box.scrollIntoView({
        block: "center",
        behavior: still ? "auto" : "smooth",
      });
      box.querySelector<HTMLElement>("input, textarea, select")?.focus();
    });
  }

  /**
   * «1 карта», «2 карты», «5 карт».
   *
   * Слова лежат в словаре, а ветка выбора — здесь: склонение по числу это
   * свойство языка, а не текста, и в строку его не записать. Пока форма была
   * одна, полка расы говорила «1 карт» — мелочь, но она стоит в заголовке
   * листа крупными буквами.
   */
  function cardsOf(count: number): string {
    const tail = count % 100;
    const last = count % 10;
    const form =
      $lang === "ru"
        ? tail >= 11 && tail <= 14
          ? "Many"
          : last === 1
            ? "One"
            : last >= 2 && last <= 4
              ? "Few"
              : "Many"
        : count === 1
          ? "One"
          : "Many";
    return `${count} ${$t(`adminBattlesRaceCards${form}` as TranslationKey)}`;
  }

  /** Слово отказа — то же, которым откажет сервер. */
  const faultWord = (fault: string) =>
    $t(
      `adminBattlesFault${fault[0].toUpperCase()}${fault.slice(1)}` as TranslationKey,
    );
  let notes = $derived(weigh?.readiness.notes ?? []);

  /**
   * Чего не хватает комнате, чтобы игра работала целиком.
   *
   * Считается по уже загруженным спискам. Карты без здоровья сюда попадают
   * только старые: завести новую такую сервер больше не даёт, а те, что были
   * заведены раньше, сами не починятся.
   */
  let roomTrouble = $derived.by(() => {
    if (loading) return [] as string[];
    const out: string[] = [];
    const shelf = cards.filter((c) => c.status === "published");
    if (!shelf.length) out.push("noCards");
    const lifeless = shelf.filter((c) => c.health <= 0).length;
    if (lifeless) out.push(`lifeless:${lifeless}`);
    // Дом раздаёт заём, перебирая пул по кругу, поэтому одной отмеченной
    // карты хватает, чтобы стол «работал», — и новый гость садится за шесть
    // копий одного тела. Считаем до полного стола, а не до единицы.
    const lendable = shelf.filter(
      (c) => c.lendable && c.tier === 1 && c.health > 0,
    ).length;
    if (!lendable) out.push("noLendable");
    else if (lendable < DECK_SIZE) out.push(`fewLendable:${lendable}`);
    if (!challenges.some((c) => c.status === "published"))
      out.push("noBattles");
    return out;
  });

  /** «lifeless:2» → «lifeless» + число. */
  const troubleWord = (t: string) => t.split(":")[0];
  const troubleCount = (t: string) => t.split(":")[1] ?? "";

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
      const body = cardBody();
      const saved = await api.adminSaveBattleCard(
        body,
        selectedId ?? undefined,
      );
      await loadCards();
      openCard(saved);
      flash($t("adminBattlesSaved"));
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      saving = false;
    }
  }

  async function remove() {
    if (!selectedId) return;
    // Спрашивается ровно то, что будет потеряно. Удаление карты, которую
    // называют этюды, не переезжает никуда: они останутся без тела, и узнать
    // об этом хранитель должен ДО, а не от гостя.
    const warning = etudesUsing.length
      ? `${$t("adminBattlesDeleteConfirm")}\n\n${$t("adminBattlesEtudesWillEmpty")} ${etudesUsing
          .map(etudeTitleOf)
          .join(", ")}`
      : $t("adminBattlesDeleteConfirm");
    if (!confirm(warning)) return;
    try {
      await api.adminDeleteBattleCard(selectedId);
      await loadCards();
      blank();
      flash($t("adminBattlesDeleted"));
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
        flash($t("adminBattlesReordered"));
      } catch (e) {
        flash(String(e), 6000);
        await loadCards();
      }
    }, REORDER_MS);
  }

  /**
   * Сохранить — и СЛИЧИТЬ. Сервер отвечает нормализованным набором, стол его
   * принимает, и до сих пор этого было довольно, чтобы правки молча исчезали:
   * поле, которого сервер не знает, serde выбрасывает, а колонка показывает
   * обрезанный ответ как ни в чём не бывало. Теперь отправленное сравнивается
   * с вернувшимся, и если сервер что-то не принял, об этом говорят вслух.
   */
  async function saveFrames() {
    saving = true;
    const sent = JSON.stringify(frames);
    try {
      const saved = await api.adminSaveBattleFrames({ frames });
      frames = saved.frames.map(completeSlices);
      const back = JSON.stringify(frames);
      stored = back;
      // Решает РАЗБОР, а не сравнение строк: строки расходятся и тогда, когда
      // сервер положил число в свою ячейку одинарной точности и вернул его же,
      // на седьмом знаке. Пусто в разборе — значит, ничего не потеряно.
      const lost =
        back === sent ? "" : whatChanged(JSON.parse(sent), JSON.parse(back));
      if (!lost) {
        flash($t("adminBattlesFramesSaved"));
      } else {
        flash(`${$t("adminBattlesFramesSavedPartly")} ${lost}`, 12000);
      }
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      saving = false;
    }
  }

  /**
   * Одно ли это число — по мерке СЕРВЕРА, а не по мерке браузера.
   *
   * Числа рамы на сервере одинарной точности, в браузере — двойной. Всякое
   * перетаскивание складывает доли пикселя в двойной, отсылает `2.9166668666…`
   * и получает обратно `2.916667`: то же самое число, положенное в ту ячейку,
   * какая для него и заведена. Строгое сравнение объявляло это расхождением —
   * и стол честно кричал «сохранено не всё», перечисляя ровно те числа, что
   * хранитель только что и тянул. Разница там в одну десятимиллионную процента
   * ширины карты: её нет ни на экране, ни в базе, ни в чьих-либо глазах.
   *
   * Поэтому сравнивается округлённое до одинарной точности. Настоящую потерю —
   * поле, которое сервер выбросил, потому что не знает его, — это по-прежнему
   * ловит: там не соседнее число, там `undefined`.
   */
  function sameNumber(sent: number, back: unknown): boolean {
    return typeof back === "number" && Math.fround(sent) === Math.fround(back);
  }

  /** Первые несколько мест, где ответ разошёлся с отправленным. Путём, а не
   *  «что-то изменилось»: хранителю нужно знать, какую именно настройку сервер
   *  не взял, чтобы понять, что он смотрит на старый сервер. */
  function whatChanged(
    sent: unknown,
    back: unknown,
    path = "",
    found: string[] = [],
  ): string {
    if (found.length >= 6) return found.join(", ");
    if (sent === back) return found.join(", ");
    if (typeof sent === "number") {
      if (!sameNumber(sent, back)) found.push(path || "?");
      return found.join(", ");
    }
    const bothObjects =
      sent && back && typeof sent === "object" && typeof back === "object";
    if (!bothObjects) {
      if (JSON.stringify(sent) !== JSON.stringify(back))
        found.push(path || "?");
      return found.join(", ");
    }
    const keys = new Set([
      ...Object.keys(sent as object),
      ...Object.keys(back as object),
    ]);
    for (const key of keys) {
      whatChanged(
        (sent as Record<string, unknown>)[key],
        (back as Record<string, unknown>)[key],
        path ? `${path}.${key}` : key,
        found,
      );
      if (found.length >= 6) break;
    }
    return found.join(", ");
  }

  // ── Presets: putting a dress away, and taking it out ────────────────────

  /** Every write to the drawer goes through the server and comes back
   *  normalised, so what the desk shows is what is actually kept — a preset
   *  the server tidied away must not linger on screen as if it existed. */
  async function savePresets(next: BattleFramePreset[], note: TranslationKey) {
    saving = true;
    try {
      const saved = await api.adminSaveBattleFramePresets({ presets: next });
      presets = saved.presets;
      if (presetChosen && !presets.some((p) => p.id === presetChosen))
        presetChosen = null;
      if (presetOpen && !presets.some((p) => p.id === presetOpen))
        presetOpen = null;
      flash($t(note));
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      saving = false;
    }
  }

  /** Fold the rank being edited into the drawer under a name of the keeper's
   *  own. A name already in the drawer is overwritten rather than doubled:
   *  two dresses called the same thing could not be told apart when the time
   *  came to wear one. */
  function keepFrameAsPreset() {
    const name = presetName.trim();
    const frame = frames[frameIndex];
    if (!name || !frame) return;
    const already = presets.find(
      (p) => p.name.toLowerCase() === name.toLowerCase(),
    );
    const kept: BattleFramePreset = {
      id: already?.id ?? crypto.randomUUID(),
      name,
      frame: $state.snapshot(frame),
    };
    const next = already
      ? presets.map((p) => (p.id === already.id ? kept : p))
      : [...presets, kept];
    // Имя остаётся на табличке: отложенный наряд — это тот, с которым теперь
    // и работают, и стереть имя значило бы сразу же скрыть, какой он.
    presetName = kept.name;
    // Отложенный наряд — это тот, с которым теперь и работают: следующая правка
    // должна ложиться обратно в него, а не заводить в ящике второго с почти
    // тем же именем.
    presetOpen = kept.id;
    savePresets(
      next,
      already ? "adminBattlesPresetReplaced" : "adminBattlesPresetKept",
    );
  }

  /** Имя, которого ещё нет в ящике. «simple2» при занятом даёт «simple2 2»,
   *  а не затирает исходный — иначе «отложить как новый» молча убивал бы
   *  рамку, с которой копировали. */
  function uniquePresetName(seed: string): string {
    const taken = new Set(presets.map((p) => p.name.toLowerCase()));
    const base = seed.trim() || $t("adminBattlesFrameNew");
    if (!taken.has(base.toLowerCase())) return base;
    let n = 2;
    while (taken.has(`${base} ${n}`.toLowerCase())) n += 1;
    return `${base} ${n}`;
  }

  /** Новая рамка в ящике с того, что на столе. Исходная не трогается. */
  function keepFrameAsNew() {
    const frame = frames[frameIndex];
    if (!frame) return;
    const seed = presetName.trim() || presetWorn?.name || "";
    if (!seed) {
      tick().then(() => frameNameBox?.focus());
      return;
    }
    const kept: BattleFramePreset = {
      id: crypto.randomUUID(),
      name: uniquePresetName(seed),
      frame: $state.snapshot(frame),
    };
    presetName = kept.name;
    presetOpen = kept.id;
    savePresets([...presets, kept], "adminBattlesPresetKept");
  }

  /** Наряд, вынутый на этот чин, — если он ещё в ящике. */
  let presetWorn = $derived(presets.find((p) => p.id === presetOpen) ?? null);

  /**
   * Сравнить два наряда, не полагаясь на порядок полей.
   *
   * Обычный `JSON.stringify` тут врёт дважды: с сервера рама приходит в одном
   * порядке ключей, а `completeSlices` перекладывает `slices` и `ornaments` в
   * конец; и числа на сервере одинарной точности, а на столе двойной. Ключи
   * сортируются, числа округляются до серверных — тогда строка зависит только
   * от того, что на раме действительно нарисовано.
   */
  function canon(value: unknown): string {
    // Число — по мерке сервера: наряд, только что отложенный, возвращается
    // округлённым до одинарной точности, а на столе лежит двойная. Без этого
    // «изменён» загоралось бы сразу после сохранения и не гасло никогда.
    if (typeof value === "number") return String(Math.fround(value));
    if (!value || typeof value !== "object") return JSON.stringify(value) ?? "";
    if (Array.isArray(value)) return `[${value.map(canon).join(",")}]`;
    const at = value as Record<string, unknown>;
    return `{${Object.keys(at)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${canon(at[key])}`)
      .join(",")}}`;
  }

  /** Разошёлся ли чин с нарядом, который на него надели. Ровно то место, где
   *  хранителю нужно решить: положить правку обратно в ящик или оставить её
   *  этому чину. Пока об этом молчали, решение принималось само — и не в его
   *  пользу. */
  let presetChanged = $derived.by(() => {
    const frame = frames[frameIndex];
    if (!presetWorn || !frame) return false;
    // Обе стороны через `completeSlices`: наряд, отложенный до того, как у
    // деталей появились места, донёс бы отсутствие мест как различие — а это
    // не различие, а старая запись того же самого.
    return (
      canon(dressOf(completeSlices($state.snapshot(frame)))) !==
      canon(dressOf(completeSlices(presetWorn.frame)))
    );
  });

  /** Положить правку обратно в тот наряд, из которого её начали. Имя остаётся
   *  прежним: обновляют наряд, а не заводят похожий. */
  function updateOpenPreset() {
    const frame = frames[frameIndex];
    if (!presetWorn || !frame) return;
    const kept: BattleFramePreset = {
      id: presetWorn.id,
      name: presetWorn.name,
      frame: $state.snapshot(frame),
    };
    savePresets(
      presets.map((p) => (p.id === kept.id ? kept : p)),
      "adminBattlesPresetUpdated",
    );
  }

  function forgetPreset(preset: BattleFramePreset) {
    if (
      !confirm(
        $t("adminBattlesPresetForgetSure").replace("{name}", preset.name),
      )
    )
      return;
    if (preset.id === presetOpen) presetName = "";
    savePresets(
      presets.filter((p) => p.id !== preset.id),
      "adminBattlesPresetForgotten",
    );
  }

  /** Новая рамка на этом чине: дом, имя чина на месте, ящик закрыт.
   *  Наряд ещё не в ящике — его откладывают, когда будет имя. */
  function beginNewFrame() {
    const current = frames[frameIndex];
    if (!current) return;
    mark();
    const house = DEFAULT_FRAMES[current.tier - 1];
    frames[frameIndex] = completeSlices({
      ...house,
      tier: current.tier,
      nameEn: current.nameEn,
      nameRu: current.nameRu,
    });
    presetOpen = null;
    presetName = "";
    sliceHeld = null;
    rowHeld = null;
    tick().then(() => frameNameBox?.focus());
  }

  let presetTaken = $derived(
    presets.find((p) => p.id === presetChosen) ?? null,
  );

  /** Onto a rank: the whole design, but never the rank's number or its name —
   *  rank four is still rank four, and still called what the dictionary calls
   *  it, however it ends up dressed. Not saved here: the keeper looks at the
   *  card first and presses the frames view's own Save, the same as after any
   *  other change made in this view. */
  function wearPresetOnRank(preset: BattleFramePreset) {
    const frame = frames[frameIndex];
    if (!frame) return;
    mark();
    // Опись и кегль — дело вкладки «Лицо карты» на чине; `dressOf` их не
    // несёт (иначе раса заморозила бы вчерашний список). На чин надевают
    // целиком, и опись пресета сюда кладётся явно.
    frames[frameIndex] = completeSlices({
      ...frame,
      ...dressOf(preset.frame),
      sheet: normalizeSheet(preset.frame.sheet),
      typeScale: preset.frame.typeScale || 1,
      inkFade: preset.frame.inkFade || 1,
    });
    // В руке могло остаться украшение, которого на новом наряде нет: стол
    // держал бы деталь, которой на карте больше не рисуется.
    if (sliceHeld && !kindOf(frames[frameIndex], sliceHeld.id)) sliceHeld = null;
    presetOpen = preset.id;
    presetName = "";
    flash($t("adminBattlesPresetWorn"));
  }

  /** Onto one level of a race's copies. Saved with the race, like any other
   *  change in that editor. */
  function wearPresetOnLevel(index: number) {
    if (!presetTaken) return;
    // Снимок, а не сам наряд: `dressOf` копирует поля вширь, и `slices` с
    // `ornaments` остались бы ТЕМИ ЖЕ объектами, что лежат в ящике, — тогда
    // потянутый на уровне вырез молча правил бы отложенный наряд.
    raceLevelFrames[index] = dressOf($state.snapshot(presetTaken.frame));
  }

  function wearPresetOnAllLevels() {
    if (!presetTaken) return;
    // A copy each: five levels sharing one object would move together under
    // the inset handles, which is precisely what per-level dressing is for.
    raceLevelFrames = raceLevelFrames.map(() =>
      dressOf($state.snapshot(presetTaken!.frame)),
    );
  }

  /** Onto this one card, over whatever its rank and race already say. */
  function wearPresetOnCard() {
    if (!presetTaken) return;
    draft.frameOverride = JSON.stringify(dressOf(presetTaken.frame));
    flash($t("adminBattlesPresetWorn"));
  }

  // Уйти с несохранённым молча больше нельзя: ровно так и теряют вечер работы.
  $effect(() => {
    if (!dirty) return;
    const warn = (e: BeforeUnloadEvent) => e.preventDefault();
    window.addEventListener("beforeunload", warn);
    return () => window.removeEventListener("beforeunload", warn);
  });

  onMount(async () => {
    // С чем стол открылся — запасная мерка на случай, если рамы не придут.
    // Снимается ДО ожиданий: по столу уже можно кликать, пока он грузится, и
    // мерка, снятая после, записала бы эти правки в «сохранённое».
    const opening = JSON.stringify(frames);
    try {
      const [, figs, savedFrames, savedRaces, savedMotions] = await Promise.all([
        loadCards(),
        api.getAllFigurines(),
        api.getBattleFrames(),
        api.getBattleRaces(),
        api.getBattleMotions(),
        loadDustRates(),
      ]);
      figurines = figs;
      races = savedRaces;
      motions = savedMotions.motions;
      keywords = await api.getBattleKeywords();
      if (savedFrames.frames.length) {
        frames = savedFrames.frames.map(completeSlices);
        // Мерка снимается ЗДЕСЬ, а не в `finally`: пока стол грузится, по нему
        // уже можно кликать, и мерка, снятая после, записала бы эти правки в
        // «сохранённое» — то есть промолчала бы ровно о них.
        stored = JSON.stringify(frames);
      }
      presets = (await api.adminGetBattleFramePresets()).presets;
      // Последним и намеренно: полка этюдов — самый молодой запрос на столе, и
      // если он однажды откажет, стол не должен открыться в рамках по умолчанию
      // из-за неприменённой строки выше.
      challenges = await api.adminListBattleChallenges();
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      loading = false;
      blank();
      // Если рамы не пришли вовсе, меркой становится то, с чем стол открылся:
      // иначе он молчал бы о несохранённом ровно в тот раз, когда молчать
      // всего опаснее.
      if (!stored) stored = opening;
    }
  });
</script>

<div class="h-full flex flex-col bg-[#f8f1e7] text-[#34251c]">
  <div
    class="flex items-center gap-3 px-4 py-2 border-b border-[#34251c]/10 text-[10px] uppercase tracking-[0.16em]"
  >
    <div class="flex border border-[#34251c]/15">
      <button
        onclick={() => (view = "cards")}
        class="px-3 py-1 {view === 'cards'
          ? 'bg-[#34251c] text-[#f8f1e7]'
          : ''}">{$t("adminBattlesCardsView")}</button
      >
      <button
        onclick={() => (view = "frames")}
        class="px-3 py-1 {view === 'frames'
          ? 'bg-[#34251c] text-[#f8f1e7]'
          : ''}">{$t("adminBattlesFramesView")}</button
      >
      <button
        onclick={() => (view = "face")}
        class="px-3 py-1 {view === 'face'
          ? 'bg-[#34251c] text-[#f8f1e7]'
          : ''}">{$t("adminBattlesFaceView")}</button
      >
      <button
        onclick={() => (view = "motions")}
        class="px-3 py-1 {view === 'motions'
          ? 'bg-[#34251c] text-[#f8f1e7]'
          : ''}">{$t("adminBattlesMotionsView")}</button
      >
      <button
        onclick={() => (view = "assets")}
        class="px-3 py-1 {view === 'assets'
          ? 'bg-[#34251c] text-[#f8f1e7]'
          : ''}">{$t("adminBattlesAssetsView")}</button
      >
      <button
        onclick={() => (view = "races")}
        class="px-3 py-1 {view === 'races'
          ? 'bg-[#34251c] text-[#f8f1e7]'
          : ''}">{$t("adminBattlesRacesView")}</button
      >
      <button
        onclick={() => (view = "keywords")}
        class="px-3 py-1 {view === 'keywords'
          ? 'bg-[#34251c] text-[#f8f1e7]'
          : ''}">{$t("adminBattlesKeywords")}</button
      >
      <button
        onclick={() => (view = "bench")}
        class="px-3 py-1 {view === 'bench'
          ? 'bg-[#34251c] text-[#f8f1e7]'
          : ''}">{$t("adminBattlesBench")}</button
      >
      <button
        onclick={() => (view = "hand")}
        class="px-3 py-1 {view === 'hand' ? 'bg-[#34251c] text-[#f8f1e7]' : ''}"
        >{$t("adminBattlesHand")}</button
      >
      <button
        onclick={openMatches}
        class="px-3 py-1 {view === 'matches'
          ? 'bg-[#34251c] text-[#f8f1e7]'
          : ''}">{$t("adminBattlesMatches")}</button
      >
      <button
        onclick={() => (view = "errands")}
        class="px-3 py-1 {view === 'errands'
          ? 'bg-[#34251c] text-[#f8f1e7]'
          : ''}">{$t("adminBattlesErrands")}</button
      >
    </div>
    {#if message}
      <span
        class="ml-auto normal-case tracking-normal text-[11px] text-[#6f3b24]"
        >{message}</span
      >
    {/if}
  </div>

  <!-- Чего не хватает комнате. Стоит под вкладками, а не внутри одной из них:
       нехватка заёмных карт видна в карточном редакторе, а мешает она на столе
       гостя, и искать её там, где она мешает, поздно. -->
  {#if roomTrouble.length}
    <div class="px-4 py-2 border-b border-[#c65f3c]/25 bg-[#c65f3c]/[0.06]">
      <p class="text-[10px] uppercase tracking-[0.16em] text-[#8f2f22]">
        {$t("adminBattlesRoomTitle")}
      </p>
      <ul class="mt-1 space-y-0.5">
        {#each roomTrouble as trouble (trouble)}
          <li class="text-[11px] leading-relaxed text-[#6f3b24]">
            {$t(
              `adminBattlesRoom${troubleWord(trouble)[0].toUpperCase()}${troubleWord(trouble).slice(1)}` as TranslationKey,
            )}
            {troubleCount(trouble)}
          </li>
        {/each}
      </ul>
    </div>
  {/if}

  <!-- Как лежит одна деталь. Один и тот же набор для всех шести слотов, потому
       что нестыковка была у всех шести одна и та же: полоса знала, где деталь
       начинается, и на этом всё кончалось. -->

  {#snippet placement(id: string, kind: SliceKind, piece: SlicePiece)}
    {@const sides = KIND_SIDES[kind]}
    {@const side = sliceHeld?.id === id ? sliceHeld.side : sides[0]}
    {@const at = piece.places[side]}
    <div
      class="mb-5 pl-3 border-l {sliceHeld?.id === id
        ? 'border-[#c65f3c]'
        : 'border-[#34251c]/10'}"
    >
      <p
        class="mb-2 text-[9px] uppercase tracking-[0.16em] {sliceHeld?.id === id
          ? 'text-[#c65f3c]'
          : 'text-[#8a6a55]'}"
      >
        {$t("adminBattlesSlicePlacement")}
      </p>

      <!-- Стороны. Выбор здесь и есть то, что взято на карте: два входа, одно
           значение, поэтому они не могут разойтись во мнении о том, что двигают.
           Галочка у каждой — рисуется ли эта копия вообще: медальон над
           притолокой и ничего на пороге снимается здесь, а не второй заливкой
           картинки, у которой половина стёрта. -->
      <div class="flex flex-wrap items-center gap-1.5 mb-2">
        {#each sides as one (one)}
          {@const there = piece.places[one]}
          <span
            class="inline-flex items-center border {sliceHeld?.id === id &&
            sliceHeld.side === one
              ? 'border-[#34251c]'
              : 'border-[#34251c]/20'}"
          >
            <input
              type="checkbox"
              title={$t("adminBattlesSliceShown")}
              checked={there ? there.shown !== false : true}
              onchange={(e) => {
                if (there) there.shown = e.currentTarget.checked;
              }}
              class="ml-1.5 accent-[#34251c]"
            />
            <button
              type="button"
              onclick={() => ((pokedAt = null), (sliceHeld = { id, side: one }))}
              class="px-2 py-1 text-[9px] uppercase tracking-[0.14em] {sliceHeld?.id ===
                id && sliceHeld.side === one
                ? 'bg-[#34251c] text-[#f8f1e7]'
                : 'hover:bg-[#34251c]/5'} {there && there.shown === false
                ? 'line-through opacity-50'
                : ''}">{$t(SIDE_KEY[one])}</button
            >
          </span>
        {/each}
        <label
          class="flex items-center gap-1.5 ml-2 text-[9px] uppercase tracking-[0.14em] text-[#8a6a55] cursor-pointer"
        >
          <input
            type="checkbox"
            bind:checked={piece.linked}
            class="accent-[#34251c]"
          />
          {$t("adminBattlesSliceLinked")}
        </label>
      </div>

      <!-- Картинка одна на все копии, поэтому слой, заполнение и разворот —
           детали, а не стороны. -->
      <div class="flex flex-wrap items-end gap-2 mb-2">
        <label class="block w-52">
          <span
            class="block mb-1 text-[9px] uppercase tracking-[0.14em] text-[#8a6a55]"
            >{$t("adminBattlesSliceFit")}</span
          >
          <select
            bind:value={piece.fit}
            class="w-full px-2 py-1.5 text-xs bg-transparent border border-[#34251c]/15 outline-none"
          >
            {#each SLICE_FITS as fit (fit)}
              <option value={fit}>{$t(FIT_KEY[fit])}</option>
            {/each}
          </select>
        </label>
        <label class="block w-44">
          <span
            class="block mb-1 text-[9px] uppercase tracking-[0.14em] text-[#8a6a55]"
            >{$t("adminBattlesSliceTurn")}</span
          >
          <select
            bind:value={piece.turn}
            class="w-full px-2 py-1.5 text-xs bg-transparent border border-[#34251c]/15 outline-none"
          >
            {#each SLICE_TURNS as turn (turn)}
              <option value={turn}>{$t(TURN_KEY[turn])}</option>
            {/each}
          </select>
        </label>
      </div>

      <!-- И четыре числа выбранной стороны — то же, что мышь пишет на карте,
           сказанное точно. -->
      {#if at}
        <div class="flex flex-wrap items-end gap-2">
          {#each SLICE_NUMBERS as row (row.key)}
            <label class="block w-[4.5rem]">
              <span
                class="block mb-1 text-[9px] uppercase tracking-[0.14em] text-[#8a6a55]"
                >{$t(row.label)}</span
              >
              <input
                type="number"
                step="0.1"
                min={-SLICE_GROW_MAX}
                max={SLICE_GROW_MAX}
                value={at[row.key]}
                oninput={(e) => {
                  const given = Number(e.currentTarget.value);
                  at[row.key] = Number.isFinite(given)
                    ? Math.min(SLICE_GROW_MAX, Math.max(-SLICE_GROW_MAX, given))
                    : 0;
                }}
                onfocus={selectOnFocus}
                onwheel={blurOnWheel}
                class="w-full px-2 py-1.5 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
              />
            </label>
          {/each}
          <button
            type="button"
            onclick={() => resetSlice(id)}
            class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5"
            >{$t("adminBattlesSliceReset")}</button
          >
        </div>
      {/if}
    </div>
  {/snippet}

  {#if view === "frames"}
    <!-- ── Пять рам, по одной на ранг ──────────────────────────────────────
         Стол резчика: карта занимает место, сбоку — список деталей и настройки
         ТОЛЬКО той, что в руке. До этого было наоборот, и полтораста органов
         управления стояли столбиком возле миниатюры в 320 px.

         Список рамок — на всю ширину, над сценой и колонкой: это и есть то,
         с чем работают. Подпись «эта рамка» имени не выбирает, а ящик в
         колонке прятал тот же список за прокруткой. -->
    <div class="flex-1 flex flex-col min-h-0">
      {#if frames[frameIndex]}
        <div
          class="flex flex-wrap items-center gap-x-3 gap-y-2 px-4 py-2.5 border-b border-[#34251c]/10 bg-[#f8f1e7]"
        >
          <div class="w-[22rem] max-w-full">
            <BattleFramePicker
              {presets}
              bind:chosen={presetOpen}
              onchoose={wearPresetOnRank}
              onforget={forgetPreset}
              disabled={saving}
              size="desk"
              label={$t("adminBattlesPresetChoose")}
            />
          </div>
          <div class="flex flex-wrap items-center gap-2">
            <input
              bind:this={frameNameBox}
              bind:value={presetName}
              maxlength="60"
              placeholder={presetWorn
                ? $t("adminBattlesPresetCopyName")
                : $t("adminBattlesPresetName")}
              onkeydown={(e) =>
                e.key === "Enter" &&
                (presetWorn ? keepFrameAsNew() : keepFrameAsPreset())}
              class="w-44 px-2 py-1.5 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
            />
            {#if presetWorn}
              <button
                onclick={keepFrameAsNew}
                disabled={saving}
                title={$t("adminBattlesPresetKeepNewHint")}
                class="flex items-center gap-1.5 px-2.5 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/25 hover:bg-[#34251c]/5 disabled:opacity-40"
                ><BattleIcon name="twin" />{$t(
                  "adminBattlesPresetKeepNew",
                )}</button
              >
              <button
                onclick={updateOpenPreset}
                disabled={saving || !presetChanged}
                title={$t("adminBattlesPresetUpdateHint")}
                class="flex items-center gap-1.5 px-2.5 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/25 hover:bg-[#34251c]/5 disabled:opacity-40"
                ><BattleIcon name="keep" />{$t(
                  "adminBattlesPresetUpdate",
                )}</button
              >
              {#if presetChanged}
                <span
                  class="flex items-center gap-1.5 text-[10px] uppercase tracking-[0.16em] text-[#8f2f22]"
                >
                  <span class="w-1.5 h-1.5 rounded-full bg-[#c65f3c]"></span>
                  {$t("adminBattlesPresetDrifted")}
                </span>
              {/if}
              <button
                onclick={() => forgetPreset(presetWorn)}
                disabled={saving}
                title={$t("adminBattlesPresetForgetSure").replace(
                  "{name}",
                  presetWorn.name,
                )}
                class="flex items-center gap-1.5 px-2.5 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#c65f3c]/40 text-[#8f2f22] hover:bg-[#c65f3c]/10 disabled:opacity-40"
                ><BattleIcon name="trash" />{$t(
                  "adminBattlesFrameDrop",
                )}</button
              >
            {:else}
              <button
                onclick={keepFrameAsPreset}
                disabled={saving || !presetName.trim()}
                title={$t("adminBattlesPresetKeep")}
                class="flex items-center gap-1.5 px-2.5 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/25 hover:bg-[#34251c]/5 disabled:opacity-40"
                ><BattleIcon name="keep" />{$t("adminBattlesPresetKeep")}</button
              >
            {/if}
            <button
              onclick={beginNewFrame}
              title={$t("adminBattlesFrameNewHint")}
              class="flex items-center gap-1.5 px-2.5 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/25 hover:bg-[#34251c]/5"
              ><BattleIcon name="plus" />{$t("adminBattlesFrameNew")}</button
            >
          </div>
        </div>
      {/if}

      <div class="flex-1 flex min-h-0">
      <section class="flex-1 min-w-0 flex flex-col bg-[#f1e8db]">
        <div
          class="flex flex-wrap items-center gap-3 px-4 py-2 border-b border-[#34251c]/10"
        >
          <div class="flex border border-[#34251c]/15">
            {#each frames as frame, i (frame.tier)}
              <button
                onclick={() => {
                  frameIndex = i;
                  sliceHeld = null;
                  // Открытый наряд — про ТОТ чин, с которого его сняли. На
                  // соседнем он ничего не значит, и «обновить» у чужой рамы
                  // положило бы в ящик не то, что доставали.
                  presetOpen = null;
                  presetName = "";
                }}
                class="px-3 py-1 text-[11px] {frameIndex === i
                  ? 'bg-[#34251c] text-[#f8f1e7]'
                  : 'hover:bg-[#34251c]/5'}"
                >{frame.tier} · {frameName(frame, $lang)}</button
              >
            {/each}
          </div>

          <!-- Увеличение. Не `transform`: карта меряет себя контейнерными
               единицами, поэтому большая ширина увеличивает и резьбу, и шрифт
               по-настоящему, а перетаскивание остаётся точным без поправок. -->
          <div class="flex border border-[#34251c]/15">
            {#each ZOOMS as z (z)}
              <button
                onclick={() => (stageZoom = z)}
                class="px-2 py-1 text-[10px] {stageZoom === z
                  ? 'bg-[#34251c] text-[#f8f1e7]'
                  : 'hover:bg-[#34251c]/5'}">{z}×</button
              >
            {/each}
          </div>

          <!-- Клетка боя. Стоит рядом с увеличением, а не в колонке справа:
               это способ СМОТРЕТЬ на карту, как и увеличение, а не её
               свойство. Без него кружок здоровья на столе недостижим — он
               выходит только в бою, а стол не бой. -->
          <div class="flex items-center gap-2 border border-[#34251c]/15 px-2 py-1">
            <label
              class="flex items-center gap-1.5 text-[10px] uppercase tracking-[0.14em] cursor-pointer"
            >
              <input type="checkbox" bind:checked={stageInMatch} class="accent-[#34251c]" />
              {$t("adminBattlesStageInMatch")}
            </label>
            {#if stageInMatch}
              <input
                type="range"
                min="0.05"
                max="1"
                step="0.05"
                bind:value={stageHurt}
                title={$t("adminBattlesStageHurt")}
                class="w-24 accent-[#c65f3c]"
              />
              <span class="text-[10px] tabular-nums text-[#8a6a55]"
                >{Math.round(stageHurt * 100)}%</span
              >
            {/if}
          </div>

          <div class="flex border border-[#34251c]/15">
            <button
              onclick={stepBack}
              disabled={!history.length}
              title="{$t('adminBattlesUndo')} · ⌘Z"
              class="px-2.5 py-1 text-[11px] hover:bg-[#34251c]/5 disabled:opacity-30"
              >↺</button
            >
            <button
              onclick={stepOn}
              disabled={!ahead.length}
              title="{$t('adminBattlesRedo')} · ⇧⌘Z"
              class="px-2.5 py-1 text-[11px] hover:bg-[#34251c]/5 disabled:opacity-30"
              >↻</button
            >
          </div>

          <div class="ml-auto flex items-center gap-3">
            {#if dirty}
              <span
                class="flex items-center gap-1.5 text-[10px] uppercase tracking-[0.16em] text-[#8f2f22]"
              >
                <span class="w-1.5 h-1.5 rounded-full bg-[#c65f3c]"></span>
                {$t("adminBattlesUnsaved")}
              </span>
            {/if}
            <button
              onclick={saveFrames}
              disabled={saving}
              class="px-4 py-1.5 text-[10px] uppercase tracking-[0.16em] {dirty
                ? 'bg-[#34251c] text-[#f8f1e7]'
                : 'border border-[#34251c]/25'} disabled:opacity-40"
              >{$t("adminBattlesFramesSave")}</button
            >
          </div>
        </div>

        <!-- Сцена. Своя прокрутка, поэтому увеличенная карта возится по столу
             вместо того, чтобы гнать колонку настроек за собой. Слушает
             клавиши: стрелки двигают взятую копию на 0.1 % (с Shift — на 1 %,
             с Alt — наращивают нахлёст), ⌘Z отменяет. -->
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <div
          role="application"
          aria-label={$t("adminBattlesPreview")}
          tabindex="0"
          onkeydown={stageKeys}
          bind:this={stageBox}
          onscroll={() =>
            (stageScroll = {
              x: stageBox?.scrollLeft ?? 0,
              y: stageBox?.scrollTop ?? 0,
            })}
          class="relative flex-1 overflow-auto p-8 outline-none"
        >
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="relative mx-auto"
            bind:this={cardBox}
            onpointerdowncapture={poke}
            style="width:{Math.round(STAGE_BASE * stageZoom)}px"
          >
            <BattleCard
              card={frameSample}
              {frames}
              owned={true}
              transition={false}
              interactive={false}
              frameEditable={true}
              rowsEditable={true}
              hurt={stageInMatch ? stageHurt : 1}
              alive={stageInMatch
                ? Math.max(0, Math.round((frameSample.health || 10) * stageHurt))
                : null}
              wearSeed={7}
              onEditStart={mark}
              onBadgeArtUpload={uploadBadgeArt}
              onBadgeArtStore={badgeArtFromStore}
              onEditEnd={() => barTick++}
              onRowMove={moveRow}
              bind:sliceHeld
              bind:rowHeld
            />
          </div>

          <!-- Полоска взятой детали. Стоит у неё, а не в колонке: за картинкой
               со склада и за «убрать» ходили через шесть блоков подряд.
               Перехват нажатия снимает слепок для отмены до правки, а
               отпускание двигает саму полоску вслед за тем, что она только
               что изменила.

               Лежит НА СТОЛЕ, а не в карте: прибитую полоску карта возила бы
               за собой на каждом увеличении, а прибивают её ровно затем,
               чтобы она стояла. Прибитая не пропадает и с пустой рукой —
               место остаётся местом, — но делает всегда то, что в руке
               сейчас. -->
          {#if barSpot && (heldRow || barPin)}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              data-piece-bar
              bind:clientWidth={barWide}
              bind:clientHeight={barTall}
              onpointerdowncapture={mark}
              onpointerupcapture={() => barTick++}
              style="left:{barSpot.x}px; top:{barSpot.y}px"
              class="absolute z-20 flex -translate-x-1/2 items-center gap-0.5 p-1 bg-[#f8f1e7] border border-[#34251c]/25 shadow-[0_2px_10px_rgba(52,37,28,0.18)]"
            >
              <!-- Рукоять и гвоздь. Полоску таскают за рукоять и прибивают
                   гвоздём: прибитая стоит на своём месте, а не выскакивает
                   каждый раз там, где нажали. -->
              <button
                onpointerdown={grabBar}
                onpointermove={dragBar}
                onpointerup={dropBar}
                onpointercancel={dropBar}
                title={$t("adminBattlesBarMove")}
                class="p-1.5 cursor-move touch-none text-[#8a6a55] hover:bg-[#34251c]/8"
                ><BattleIcon name="move" /></button
              >
              <button
                onclick={pinBar}
                title={barPin
                  ? $t("adminBattlesBarUnpin")
                  : $t("adminBattlesBarPin")}
                class="p-1.5 hover:bg-[#34251c]/8 {barPin
                  ? 'text-[#c65f3c]'
                  : 'text-[#8a6a55]'}"><BattleIcon name="pin" /></button
              >

              <span class="w-px h-5 mx-0.5 bg-[#34251c]/15"></span>

              {#if heldRow}
                <button
                  onclick={() => uploadPiece(heldRow!)}
                  disabled={uploading}
                  title={$t("adminBattlesFrameArtUpload")}
                  class="p-1.5 hover:bg-[#34251c]/8 disabled:opacity-30"
                  ><BattleIcon name="upload" /></button
                >
                <button
                  onclick={() =>
                    fromStore(
                      heldRow!.ornament
                        ? "accent"
                        : STORE_ROLE[heldRow!.id as SliceSlot],
                      (url) => setPieceImage(heldRow!, url),
                    )}
                  title={$t("adminAssetsPick")}
                  class="p-1.5 hover:bg-[#34251c]/8"
                  ><BattleIcon name="store" /></button
                >

                <span class="w-px h-5 mx-0.5 bg-[#34251c]/15"></span>

                <button
                  onclick={() => copyPiece(heldRow!)}
                  disabled={!heldRow.image}
                  title={$t("adminBattlesPieceCopy")}
                  class="p-1.5 hover:bg-[#34251c]/8 disabled:opacity-30"
                  ><BattleIcon name="copy" /></button
                >
                <button
                  onclick={() => cutPiece(heldRow!)}
                  disabled={!heldRow.image}
                  title={$t("adminBattlesPieceCut")}
                  class="p-1.5 hover:bg-[#34251c]/8 disabled:opacity-30"
                  ><BattleIcon name="cut" /></button
                >
                <button
                  onclick={() => pastePiece(heldRow!)}
                  disabled={!clip}
                  title={clip
                    ? $t("adminBattlesPiecePaste")
                    : $t("adminBattlesPieceNothingCopied")}
                  class="p-1.5 hover:bg-[#34251c]/8 disabled:opacity-30"
                  ><BattleIcon name="paste" /></button
                >
                <button
                  onclick={() => twinPiece(heldRow!)}
                  disabled={!heldRow.image}
                  title={$t("adminBattlesPieceTwin")}
                  class="p-1.5 hover:bg-[#34251c]/8 disabled:opacity-30"
                  ><BattleIcon name="twin" /></button
                >

                <span class="w-px h-5 mx-0.5 bg-[#34251c]/15"></span>

                <button
                  onclick={() => restack(heldRow!.id, -1)}
                  title={$t("adminBattlesStackUp")}
                  class="p-1.5 hover:bg-[#34251c]/8"
                  ><BattleIcon name="up" /></button
                >
                <button
                  onclick={() => restack(heldRow!.id, 1)}
                  title={$t("adminBattlesStackDown")}
                  class="p-1.5 hover:bg-[#34251c]/8"
                  ><BattleIcon name="down" /></button
                >
                <button
                  onclick={toggleHeldCopy}
                  title={$t("adminBattlesSliceShown")}
                  class="p-1.5 hover:bg-[#34251c]/8 {heldShown()
                    ? ''
                    : 'text-[#c65f3c]'}"
                  ><BattleIcon name={heldShown() ? "eye" : "eye-off"} /></button
                >
                <button
                  onclick={() =>
                    (heldRow!.piece.linked = heldRow!.piece.linked === false)}
                  title={$t("adminBattlesSliceLinked")}
                  class="p-1.5 hover:bg-[#34251c]/8 {heldRow.piece.linked ===
                  false
                    ? 'text-[#c65f3c]'
                    : ''}"
                  ><BattleIcon name={heldRow.piece.linked === false ? "unlink" : "link"} /></button
                >

                <span class="w-px h-5 mx-0.5 bg-[#34251c]/15"></span>

                <button
                  onclick={() => resetSlice(heldRow!.id)}
                  title={$t("adminBattlesSliceReset")}
                  class="p-1.5 hover:bg-[#34251c]/8"
                  ><BattleIcon name="reset" /></button
                >
                <button
                  onclick={() =>
                    heldRow!.ornament
                      ? dropOrnament(heldRow!.id)
                      : setPieceImage(heldRow!, "")}
                  disabled={!heldRow.image}
                  title={heldRow.ornament
                    ? $t("adminBattlesOrnamentDrop")
                    : $t("adminBattlesFrameArtClear")}
                  class="p-1.5 text-[#8f2f22] hover:bg-[#c65f3c]/12 disabled:opacity-30"
                  ><BattleIcon name="trash" /></button
                >
              {:else}
                <!-- Прибитая полоска с пустой рукой. Кнопкам нечего делать,
                     но место — это и есть то, за чем её прибивали: пусть
                     стоит и ждёт, а не пропадает, чтобы появиться в другом
                     углу. -->
                <span
                  class="px-2 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]"
                  >{$t("adminBattlesBarIdle")}</span
                >
              {/if}
            </div>
          {/if}
        </div>
      </section>

      <!-- Колонка. Перехват нажатия и фокуса на ней целиком снимает слепок для
           отмены перед любой правкой — иначе каждый из полутораста органов
           управления пришлось бы оборачивать руками. -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <aside
        class="w-[27rem] flex-shrink-0 border-l border-[#34251c]/10 overflow-y-auto"
        onpointerdowncapture={mark}
        onfocusincapture={mark}
      >
        {#if frames[frameIndex]}
          <!-- ── Кто это носит ────────────────────────────────────────────
               Стоит ПЕРВЫМ и над ящиком нарядов, потому что это не настройка,
               а обстановка: стол правит ЧИН, а гость видит КАРТУ, и между ними
               стоит цепочка нарядов. Без этой полки чин красят вслепую — и не
               узнают, что он не виден ни на одной карте. -->
          <div class="p-4 border-b border-[#34251c]/10">
            <p class="mb-2 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">
              {$t("adminBattlesWornBy")}
            </p>
            {#if !rankWorn.mine.length}
              <p class="text-[11px] italic text-[#8a6a55]">
                {$t("adminBattlesWornNone")}
              </p>
            {:else}
              <p class="text-[11px] leading-relaxed text-[#6f3b24]">
                {$t("adminBattlesWornPlain")}
                <b class="tabular-nums">{rankWorn.plain.length}</b>
                {$t("adminBattlesWornOf")}
                <b class="tabular-nums">{rankWorn.mine.length}</b>
              </p>
              {#if !rankWorn.plain.length}
                <p
                  class="mt-1 flex items-start gap-1.5 text-[11px] leading-relaxed text-[#8f2f22]"
                >
                  <span class="mt-1.5 w-1.5 h-1.5 flex-shrink-0 rounded-full bg-[#c65f3c]"
                  ></span>
                  {$t("adminBattlesWornBlind")}
                </p>
              {/if}
              {#each [{ list: rankWorn.own, word: "adminBattlesWornOwn" as TranslationKey }, { list: rankWorn.byRace, word: "adminBattlesWornRace" as TranslationKey }] as group (group.word)}
                {#if group.list.length}
                  <p
                    class="mt-2 mb-0.5 text-[9px] uppercase tracking-[0.14em] text-[#8a6a55]"
                  >
                    {$t(group.word)} · {group.list.length}
                  </p>
                  <ul class="space-y-0.5">
                    {#each group.list as one (one.id)}
                      <li>
                        <button
                          onclick={() => jumpToCard(one)}
                          class="text-left text-[11px] leading-snug text-[#6f3b24] hover:underline"
                          >{titleOf(one)}</button
                        >
                      </li>
                    {/each}
                  </ul>
                {/if}
              {/each}
            {/if}
          </div>

          <!-- Как надета. Первое решение о раме, а не настройка в середине
               списка. Список рамок стоит на табличке над столом. -->
          <div class="p-4 border-b border-[#34251c]/10">
            <p
              class="mb-2 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]"
            >
              {$t("adminBattlesFrameMode")}
            </p>
            <div class="flex border border-[#34251c]/20">
              {#each FRAME_MODES as mode (mode)}
                <button
                  onclick={() => setFrameMode(mode)}
                  class="flex-1 px-2 py-1.5 text-[10px] leading-tight {frames[
                    frameIndex
                  ].frameMode === mode
                    ? 'bg-[#34251c] text-[#f8f1e7]'
                    : 'hover:bg-[#34251c]/5'}"
                  >{mode === "overlay"
                    ? $t("adminBattlesFrameOverlay")
                    : mode === "behind"
                      ? $t("adminBattlesFrameBehind")
                      : $t("adminBattlesFrameSliced")}</button
                >
              {/each}
            </div>
            {#if frames[frameIndex].frameMode === "sliced"}
              <details class="mt-2">
                <summary
                  class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
                  >{$t("adminBattlesHintOpen")}</summary
                >
                <p
                  class="mt-2 text-[11px] leading-relaxed italic text-[#8a6a55]"
                >
                  {$t("adminBattlesFrameSlicedHint")}
                </p>
              </details>
            {/if}
          </div>

          {#if frames[frameIndex].frameMode === "sliced"}
            <!-- Список деталей. Сверху то, что рисуется поверх; порядок задают
                 здесь, и только здесь, поэтому невидимых ничьих между равными
                 слоями больше нет. -->
            <div class="p-4 border-b border-[#34251c]/10">
              <p
                class="mb-2 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]"
              >
                {$t("adminBattlesStack")}
              </p>
              <details class="mb-3">
                <summary
                  class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
                  >{$t("adminBattlesHintOpen")}</summary
                >
                <p
                  class="mt-2 text-[11px] leading-relaxed italic text-[#8a6a55]"
                >
                  {$t("adminBattlesStackHint")}
                </p>
              </details>
              <div class="border border-[#34251c]/12">
                {#each stack as row (row.id)}
                  <div
                    class="flex items-center gap-2 px-1.5 py-1 border-b last:border-b-0 border-[#34251c]/8 {sliceHeld?.id ===
                    row.id
                      ? 'bg-[#c65f3c]/[0.09]'
                      : ''}"
                  >
                    <span class="flex flex-col leading-none">
                      <button
                        onclick={() => restack(row.id, -1)}
                        title={$t("adminBattlesStackUp")}
                        class="px-1 text-[8px] text-[#8a6a55] hover:text-[#34251c]"
                        >▲</button
                      >
                      <button
                        onclick={() => restack(row.id, 1)}
                        title={$t("adminBattlesStackDown")}
                        class="px-1 text-[8px] text-[#8a6a55] hover:text-[#34251c]"
                        >▼</button
                      >
                    </span>
                    <button
                      onclick={() => showPiece(row, !pieceShown(row))}
                      title={$t("adminBattlesSliceShown")}
                      class="w-4 text-[11px] {pieceShown(row)
                        ? 'text-[#34251c]'
                        : 'text-[#34251c]/25'}"
                      >{pieceShown(row) ? "◉" : "○"}</button
                    >
                    <button
                      onclick={() =>
                        ((pokedAt = null),
                        (sliceHeld = {
                          id: row.id,
                          side: KIND_SIDES[row.kind][0],
                        }))}
                      class="flex-1 flex items-center gap-2 py-0.5 text-left min-w-0"
                    >
                      <!-- Миниатюра. До неё в слоте стояла строка вида
                           `/static/assets/0158db49-….webp`, по которой нельзя
                           узнать ни одну деталь. -->
                      <span
                        class="w-8 h-8 flex-shrink-0 border border-[#34251c]/15 bg-[#34251c]/[0.04] bg-center bg-contain bg-no-repeat"
                        style:background-image={row.image
                          ? `url("${row.image}")`
                          : "none"}
                      ></span>
                      <span class="min-w-0">
                        <span
                          class="block text-[11px] truncate {row.image
                            ? ''
                            : 'text-[#8a6a55] italic'}">{row.label}</span
                        >
                        <span
                          class="block text-[9px] uppercase tracking-[0.14em] text-[#8a6a55] truncate"
                        >
                          {row.image
                            ? $t(KIND_KEY[row.kind])
                            : $t("adminBattlesPieceEmpty")}
                        </span>
                      </span>
                    </button>
                  </div>
                {/each}
              </div>
              <div class="flex flex-wrap items-center gap-2 mt-3">
                <button
                  onclick={addOrnamentUpload}
                  disabled={uploading}
                  class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5 disabled:opacity-40"
                  >{uploading ? "…" : $t("adminBattlesOrnamentAdd")}</button
                >
                <button
                  onclick={addOrnamentFromStore}
                  class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5"
                  >{$t("adminBattlesOrnamentAddStore")}</button
                >
              </div>
            </div>

            <!-- Настройки ТОЛЬКО взятой детали. Шесть блоков разом были прежде
                 всегда открыты, и колонка не помещалась на экран. -->
            {#if heldRow}
              <div class="p-4 border-b border-[#34251c]/10">
                <p
                  class="mb-3 text-[10px] uppercase tracking-[0.16em] text-[#c65f3c]"
                >
                  {heldRow.label}
                </p>
                <div class="flex flex-wrap items-end gap-2 mb-3">
                  <button
                    onclick={() => uploadPiece(heldRow!)}
                    disabled={uploading}
                    class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5 disabled:opacity-40"
                    >{uploading
                      ? "…"
                      : $t("adminBattlesFrameArtUpload")}</button
                  >
                  <button
                    onclick={() =>
                      fromStore(
                        heldRow!.ornament
                          ? "accent"
                          : STORE_ROLE[heldRow!.id as SliceSlot],
                        (url) => setPieceImage(heldRow!, url),
                      )}
                    class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5"
                    >{$t("adminAssetsPick")}</button
                  >
                  {#if heldRow.image}
                    <button
                      onclick={() => setPieceImage(heldRow!, "")}
                      class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5"
                      >{$t("adminBattlesFrameArtClear")}</button
                    >
                  {/if}
                  {#if heldRow.ornament}
                    <button
                      onclick={() => dropOrnament(heldRow!.id)}
                      class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#c65f3c]/40 text-[#8f2f22] hover:bg-[#c65f3c]/10"
                      >{$t("adminBattlesOrnamentDrop")}</button
                    >
                  {/if}
                </div>
                {#if heldRow.ornament}
                  <label class="block w-full mb-3">
                    <span
                      class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                      >{$t("adminBattlesOrnamentKind")}</span
                    >
                    <select
                      value={heldRow.ornament.kind}
                      onchange={(e) =>
                        reshapeOrnament(
                          heldRow!.ornament!,
                          e.currentTarget.value as SliceKind,
                        )}
                      class="w-full px-2 py-1.5 text-xs bg-transparent border border-[#34251c]/15 outline-none"
                    >
                      {#each SLICE_KINDS as kind (kind)}
                        <option value={kind}>{$t(KIND_KEY[kind])}</option>
                      {/each}
                    </select>
                  </label>
                {/if}
                <input
                  value={heldRow.image}
                  oninput={(e) =>
                    setPieceImage(heldRow!, e.currentTarget.value)}
                  placeholder="/static/assets/…"
                  class="w-full mb-3 px-2 py-1.5 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                />
                {@render placement(heldRow.id, heldRow.kind, heldRow.piece)}
                <details class="mt-2">
                  <summary
                    class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
                    >{$t("adminBattlesHintOpen")}</summary
                  >
                  <p
                    class="mt-2 text-[11px] leading-relaxed italic text-[#8a6a55]"
                  >
                    {$t("adminBattlesSliceHint")}
                  </p>
                </details>
              </div>
            {:else}
              <p
                class="p-4 border-b border-[#34251c]/10 text-[11px] leading-relaxed italic text-[#8a6a55]"
              >
                {$t("adminBattlesStackNothingHeld")}
              </p>
            {/if}
          {/if}

          <details class="border-b border-[#34251c]/10">
            <summary
              class="px-4 py-2.5 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
              >{$t("adminBattlesFrameArt")}</summary
            >
            <div class="px-4 pb-4 space-y-4">
              <div class="flex flex-wrap items-end gap-4">
                <label class="block">
                  <span
                    class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                    >{$t("adminBattlesFrameName")} · EN</span
                  >
                  <input
                    bind:value={frames[frameIndex].nameEn}
                    class="px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                  />
                </label>
                <label class="block">
                  <span
                    class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                    >{$t("adminBattlesFrameName")} · RU</span
                  >
                  <input
                    bind:value={frames[frameIndex].nameRu}
                    class="px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                  />
                </label>
                <label class="block">
                  <span
                    class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                    >{$t("adminBattlesFrameLayout")}</span
                  >
                  <select
                    bind:value={frames[frameIndex].layout}
                    class="px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none"
                  >
                    {#each LAYOUTS as option (option)}
                      <option value={option}>
                        {option === "corners"
                          ? $t("adminBattlesLayoutCorners")
                          : $t("adminBattlesLayoutPlaque")}
                      </option>
                    {/each}
                  </select>
                </label>
              </div>
              <!-- Одна целая фотография рамы — для `overlay` и `behind`.
                 Собранной из частей она не нужна: та строит себя из деталей. -->
              {#if frames[frameIndex].frameMode !== "sliced"}
                <div class="flex flex-wrap items-end gap-3">
                  <button
                    onclick={uploadFrameArt}
                    disabled={uploading}
                    class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5 disabled:opacity-40"
                    >{uploading
                      ? "…"
                      : $t("adminBattlesFrameArtUpload")}</button
                  >
                  <label class="block flex-1 min-w-[12rem]">
                    <span
                      class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                    >
                      {#if !frames[frameIndex].frameImage.trim()}{$t(
                          "adminBattlesFrameArtNone",
                        )}{:else}URL{/if}
                    </span>
                    <input
                      bind:value={frames[frameIndex].frameImage}
                      placeholder="/static/frames/…"
                      class="w-full px-2 py-1.5 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                    />
                  </label>
                  {#if frames[frameIndex].frameImage.trim()}
                    <button
                      onclick={() => (frames[frameIndex].frameImage = "")}
                      class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5"
                      >{$t("adminBattlesFrameArtClear")}</button
                    >
                  {/if}
                </div>
              {/if}

              <!-- What shows through the hole in a cut-out frame. -->
              <div class="flex flex-wrap items-end gap-3 mt-4">
                <button
                  onclick={uploadPaperArt}
                  disabled={uploading}
                  class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5 disabled:opacity-40"
                  >{uploading ? "…" : $t("adminBattlesPaperUpload")}</button
                >
                <label class="block flex-1 min-w-[16rem]">
                  <span
                    class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                  >
                    {#if !frames[frameIndex].paperImage.trim()}{$t(
                        "adminBattlesPaperNone",
                      )}{:else}URL{/if}
                  </span>
                  <input
                    bind:value={frames[frameIndex].paperImage}
                    placeholder="/static/images/preview/…"
                    class="w-full px-2 py-1.5 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                  />
                </label>
                {#if frames[frameIndex].paperImage.trim()}
                  <button
                    onclick={() => (frames[frameIndex].paperImage = "")}
                    class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5"
                    >{$t("adminBattlesFrameArtClear")}</button
                  >
                {/if}
              </div>

              <!-- The reverse. Never wears the frame above, whatever picture it shows —
               the carving is the front's own dress. -->
              <div class="pt-5 border-t border-[#34251c]/10">
                <p
                  class="mb-3 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                >
                  {$t("adminBattlesBackArt")}
                </p>
                <details class="mb-3">
                  <summary
                    class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
                    >{$t("adminBattlesHintOpen")}</summary
                  >
                  <p
                    class="max-w-[62ch] mt-2 text-[11px] leading-relaxed italic text-[#8a6a55]"
                  >
                    {$t("adminBattlesBackArtHint")}
                  </p>
                </details>
                <div class="flex flex-wrap items-end gap-3">
                  <button
                    onclick={uploadBackArt}
                    disabled={uploading}
                    class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5 disabled:opacity-40"
                    >{uploading ? "…" : $t("adminBattlesBackArtUpload")}</button
                  >
                  <label class="block flex-1 min-w-[16rem]">
                    <span
                      class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                    >
                      {#if !frames[frameIndex].backImage.trim()}{$t(
                          "adminBattlesBackArtNone",
                        )}{:else}URL{/if}
                    </span>
                    <input
                      bind:value={frames[frameIndex].backImage}
                      placeholder="/static/frames/…"
                      class="w-full px-2 py-1.5 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                    />
                  </label>
                  {#if frames[frameIndex].backImage.trim()}
                    <button
                      onclick={() => (frames[frameIndex].backImage = "")}
                      class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5"
                      >{$t("adminBattlesFrameArtClear")}</button
                    >
                  {/if}
                </div>
              </div>
            </div>
          </details>

          <details class="border-b border-[#34251c]/10">
            <summary
              class="px-4 py-2.5 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
              >{$t("adminBattlesFrameWindow")}</summary
            >
            <div class="px-4 pb-4">
              <!-- Where the opening in that frame actually is. -->
              <div class="pt-5 border-t border-[#34251c]/10">
                <p
                  class="mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                >
                  {$t("adminBattlesFrameWindow")}
                </p>
                <details class="mb-3">
                  <summary
                    class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
                    >{$t("adminBattlesHintOpen")}</summary
                  >
                  <p
                    class="max-w-[62ch] mt-2 text-[11px] leading-relaxed italic text-[#8a6a55]"
                  >
                    {$t("adminBattlesFrameWindowHint")}
                  </p>
                  <p
                    class="max-w-[62ch] mt-2 text-[11px] leading-relaxed italic text-[#8a6a55]"
                  >
                    {$t("adminBattlesBandsHint")}
                  </p>
                </details>
                <div class="flex flex-wrap gap-5">
                  <label class="block w-40">
                    <span
                      class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                      >{$t("adminBattlesInsetTop")} · {frames[
                        frameIndex
                      ].insetTop.toFixed(0)}%</span
                    >
                    <input
                      type="range"
                      min="0"
                      max="45"
                      step="0.5"
                      value={frames[frameIndex].insetTop}
                      oninput={(e) =>
                        setInset("insetTop", Number(e.currentTarget.value))}
                      class="w-full"
                    />
                  </label>
                  <label class="block w-40">
                    <span
                      class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                      >{$t("adminBattlesInsetRight")} · {frames[
                        frameIndex
                      ].insetRight.toFixed(0)}%</span
                    >
                    <input
                      type="range"
                      min="0"
                      max="45"
                      step="0.5"
                      value={frames[frameIndex].insetRight}
                      oninput={(e) =>
                        setInset("insetRight", Number(e.currentTarget.value))}
                      class="w-full"
                    />
                  </label>
                  <label class="block w-40">
                    <span
                      class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                      >{$t("adminBattlesInsetBottom")} · {frames[
                        frameIndex
                      ].insetBottom.toFixed(0)}%</span
                    >
                    <input
                      type="range"
                      min="0"
                      max="45"
                      step="0.5"
                      value={frames[frameIndex].insetBottom}
                      oninput={(e) =>
                        setInset("insetBottom", Number(e.currentTarget.value))}
                      class="w-full"
                    />
                  </label>
                  <label class="block w-40">
                    <span
                      class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                      >{$t("adminBattlesInsetLeft")} · {frames[
                        frameIndex
                      ].insetLeft.toFixed(0)}%</span
                    >
                    <input
                      type="range"
                      min="0"
                      max="45"
                      step="0.5"
                      value={frames[frameIndex].insetLeft}
                      oninput={(e) =>
                        setInset("insetLeft", Number(e.currentTarget.value))}
                      class="w-full"
                    />
                  </label>
                  <label class="block w-40">
                    <span
                      class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                      >{$t("adminBattlesAspect")} · {frames[
                        frameIndex
                      ].aspect.toFixed(2)}</span
                    >
                    <input
                      type="range"
                      min="0.45"
                      max="1.4"
                      step="0.01"
                      bind:value={frames[frameIndex].aspect}
                      class="w-full"
                    />
                  </label>
                  <label class="block w-40">
                    <span
                      class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                      >{$t("adminBattlesHeaderShare")} · {(
                        frames[frameIndex].headerShare * 100
                      ).toFixed(0)}%</span
                    >
                    <input
                      type="range"
                      min="0"
                      max="0.3"
                      step="0.005"
                      bind:value={frames[frameIndex].headerShare}
                      class="w-full"
                    />
                  </label>
                  <label class="block w-40">
                    <span
                      class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                      >{$t("adminBattlesArtShare")} · {(
                        frames[frameIndex].artShare * 100
                      ).toFixed(0)}%</span
                    >
                    <input
                      type="range"
                      min="0.12"
                      max="0.85"
                      step="0.01"
                      bind:value={frames[frameIndex].artShare}
                      class="w-full"
                    />
                  </label>
                  <label class="block w-40">
                    <span
                      class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                      >{$t("adminBattlesFootShare")} · {(
                        frames[frameIndex].footShare * 100
                      ).toFixed(0)}%</span
                    >
                    <input
                      type="range"
                      min="0"
                      max="0.3"
                      step="0.005"
                      bind:value={frames[frameIndex].footShare}
                      class="w-full"
                    />
                  </label>
                </div>
              </div>
            </div>
          </details>

          <details class="border-b border-[#34251c]/10">
            <summary
              class="px-4 py-2.5 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
              >{$t("adminBattlesFramePaper")}</summary
            >
            <div class="px-4 pb-4">
              <!-- The name, and the colours the renderer paints when there is no
                 photograph — still the ground under one that fails to load. -->
              <div
                class="pt-5 border-t border-[#34251c]/10 flex flex-wrap items-end gap-4"
              >
                <label class="block">
                  <span
                    class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                    >{$t("adminBattlesTitleFont")}</span
                  >
                  <select
                    bind:value={frames[frameIndex].titleFont}
                    class="px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none"
                  >
                    <option value=""
                      >{$t("adminBattlesTitleFontDefault")}</option
                    >
                    {#each SITE_FONTS as font (font.id)}
                      <option value={font.id}>{font.name}</option>
                    {/each}
                  </select>
                </label>
                <label class="block">
                  <span
                    class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                    >{$t("adminBattlesTitleInk")}</span
                  >
                  <input
                    type="color"
                    value={frames[frameIndex].titleInk ||
                      frames[frameIndex].ink}
                    oninput={(e) =>
                      (frames[frameIndex].titleInk = e.currentTarget.value)}
                    class="w-12 h-8 bg-transparent border border-[#34251c]/15"
                  />
                </label>
                {#each [["paper", $t("adminBattlesFramePaper")], ["ink", $t("adminBattlesFrameInk")], ["border", $t("adminBattlesFrameBorder")]] as [key, label] (key)}
                  <label class="block">
                    <span
                      class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                      >{label}</span
                    >
                    <input
                      type="color"
                      value={frames[frameIndex][
                        key as "paper" | "ink" | "border"
                      ]}
                      oninput={(e) =>
                        (frames[frameIndex][key as "paper" | "ink" | "border"] =
                          e.currentTarget.value)}
                      class="w-12 h-8 bg-transparent border border-[#34251c]/15"
                    />
                  </label>
                {/each}
                <label class="block flex-1 min-w-[14rem]">
                  <span
                    class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                  >
                    {$t("adminBattlesFrameFoil")}
                    {#if !frames[frameIndex].foil.trim()}<span
                        class="normal-case tracking-normal italic"
                      >
                        — {$t("adminBattlesFrameNoFoil")}</span
                      >{/if}
                  </span>
                  <input
                    bind:value={frames[frameIndex].foil}
                    placeholder="rgba(198,95,60,0.28)"
                    class="w-full px-2 py-1.5 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                  />
                </label>
              </div>
            </div>
          </details>
        {/if}

      </aside>
      </div>
    </div>
  {:else if view === "face"}
    <!-- ── Лицо карты ───────────────────────────────────
         Что карта ГОВОРИТ: какие строки печатаются, в какой полосе, в каком
         порядке и с какой величины видно. Своя вкладка, а не раздел рамок:
         рама — про резьбу, бумагу и окно, опись — про слова, и хранитель,
         пришедший поправить одно, листал мимо другого. Опись живёт в ЧИНЕ
         (пять рамок словаря) и читается с полки через `frameForCard`: раса
         надевает резьбу, но не подменяет опись — иначе правка здесь не
         доходила бы до карт расы. Сохраняет эта вкладка той же кнопкой и тем
         же запросом, что стол резчика.

         Стенд показывает карту в трёх величинах разом, и это главное, что
         вкладка вообще умеет: «только крупно» — единственная ступень, которую
         на одной карте увидеть нельзя. Крупная — рабочая, за неё тянут;
         полка и клетка боя стоят свидетелями. Метка «новая» на стенде
         зажжена нарочно (`isNew`): иначе ступень правили вслепую. -->
    <div class="flex-1 flex flex-col min-h-0">
      <!-- Ящик нарядов — буква в букву тот же, что на вкладке рамок: лицо
           рамки слева, имя справа. Иначе непонятно, чьё лицо правят. -->
      {#if frames[frameIndex]}
        <div
          class="flex flex-wrap items-center gap-x-3 gap-y-2 px-4 py-2.5 border-b border-[#34251c]/10 bg-[#f8f1e7]"
        >
          <div class="w-[22rem] max-w-full">
            <BattleFramePicker
              {presets}
              bind:chosen={presetOpen}
              onchoose={wearPresetOnRank}
              onforget={forgetPreset}
              disabled={saving}
              size="desk"
              label={$t("adminBattlesPresetChoose")}
            />
          </div>
          <div class="flex flex-wrap items-center gap-2">
            <input
              bind:this={frameNameBox}
              bind:value={presetName}
              maxlength="60"
              placeholder={presetWorn
                ? $t("adminBattlesPresetCopyName")
                : $t("adminBattlesPresetName")}
              onkeydown={(e) =>
                e.key === "Enter" &&
                (presetWorn ? keepFrameAsNew() : keepFrameAsPreset())}
              class="w-44 px-2 py-1.5 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
            />
            {#if presetWorn}
              <button
                onclick={keepFrameAsNew}
                disabled={saving}
                title={$t("adminBattlesPresetKeepNewHint")}
                class="flex items-center gap-1.5 px-2.5 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/25 hover:bg-[#34251c]/5 disabled:opacity-40"
                ><BattleIcon name="twin" />{$t(
                  "adminBattlesPresetKeepNew",
                )}</button
              >
              <button
                onclick={updateOpenPreset}
                disabled={saving || !presetChanged}
                title={$t("adminBattlesPresetUpdateHint")}
                class="flex items-center gap-1.5 px-2.5 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/25 hover:bg-[#34251c]/5 disabled:opacity-40"
                ><BattleIcon name="keep" />{$t(
                  "adminBattlesPresetUpdate",
                )}</button
              >
              {#if presetChanged}
                <span
                  class="flex items-center gap-1.5 text-[10px] uppercase tracking-[0.16em] text-[#8f2f22]"
                >
                  <span class="w-1.5 h-1.5 rounded-full bg-[#c65f3c]"></span>
                  {$t("adminBattlesPresetDrifted")}
                </span>
              {/if}
              <button
                onclick={() => forgetPreset(presetWorn)}
                disabled={saving}
                title={$t("adminBattlesPresetForgetSure").replace(
                  "{name}",
                  presetWorn.name,
                )}
                class="flex items-center gap-1.5 px-2.5 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#c65f3c]/40 text-[#8f2f22] hover:bg-[#c65f3c]/10 disabled:opacity-40"
                ><BattleIcon name="trash" />{$t(
                  "adminBattlesFrameDrop",
                )}</button
              >
            {:else}
              <button
                onclick={keepFrameAsPreset}
                disabled={saving || !presetName.trim()}
                title={$t("adminBattlesPresetKeep")}
                class="flex items-center gap-1.5 px-2.5 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/25 hover:bg-[#34251c]/5 disabled:opacity-40"
                ><BattleIcon name="keep" />{$t("adminBattlesPresetKeep")}</button
              >
            {/if}
            <button
              onclick={beginNewFrame}
              title={$t("adminBattlesFrameNewHint")}
              class="flex items-center gap-1.5 px-2.5 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/25 hover:bg-[#34251c]/5"
              ><BattleIcon name="plus" />{$t("adminBattlesFrameNew")}</button
            >
          </div>
        </div>
      {/if}

      <div class="flex-1 flex min-h-0">
      <section class="flex-1 min-w-0 flex flex-col bg-[#f1e8db]">
        <div
          class="flex flex-wrap items-center gap-3 px-4 py-2 border-b border-[#34251c]/10"
        >
          <div class="flex border border-[#34251c]/15">
            {#each frames as frame, i (frame.tier)}
              <button
                onclick={() => {
                  frameIndex = i;
                  // Открытый наряд — про ТОТ чин. На соседнем он ничего не
                  // значит, как и на вкладке рамок.
                  presetOpen = null;
                  presetName = "";
                }}
                class="px-3 py-1 text-[11px] {frameIndex === i
                  ? 'bg-[#34251c] text-[#f8f1e7]'
                  : 'hover:bg-[#34251c]/5'}"
                >{frame.tier} · {frameName(frame, $lang)}</button
              >
            {/each}
          </div>

          <div class="flex border border-[#34251c]/15">
            <button
              onclick={stepBack}
              disabled={!history.length}
              title="{$t('adminBattlesUndo')} · ⌘Z"
              class="px-2.5 py-1 text-[11px] hover:bg-[#34251c]/5 disabled:opacity-30"
              >↺</button
            >
            <button
              onclick={stepOn}
              disabled={!ahead.length}
              title="{$t('adminBattlesRedo')} · ⇧⌘Z"
              class="px-2.5 py-1 text-[11px] hover:bg-[#34251c]/5 disabled:opacity-30"
              >↻</button
            >
          </div>

          <div class="ml-auto flex items-center gap-3">
            {#if dirty}
              <span
                class="flex items-center gap-1.5 text-[10px] uppercase tracking-[0.16em] text-[#8f2f22]"
              >
                <span class="w-1.5 h-1.5 rounded-full bg-[#c65f3c]"></span>
                {$t("adminBattlesUnsaved")}
              </span>
            {/if}
            <button
              onclick={saveFrames}
              disabled={saving}
              class="px-4 py-1.5 text-[10px] uppercase tracking-[0.16em] {dirty
                ? 'bg-[#34251c] text-[#f8f1e7]'
                : 'border border-[#34251c]/25'} disabled:opacity-40"
              >{$t("adminBattlesFramesSave")}</button
            >
          </div>
        </div>

        <!-- Стенд трёх величин. Ширины не круглые для красоты: это те самые
             числа, по которым карта делит себя сама (281 и 160), взятые по
             обе стороны от порогов. -->
        <div class="flex-1 overflow-auto p-8">
          <div class="flex flex-wrap items-start gap-8">
            {#each FACE_SIZES as size (size.px)}
              <div style="width:{size.px}px" class="flex-shrink-0">
                <p
                  class="mb-2 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                >
                  {$t(size.label)} · {size.px}px
                </p>
                <BattleCard
                  card={frameSample}
                  {frames}
                  owned={true}
                  level={3}
                  isNew={true}
                  transition={false}
                  interactive={false}
                  rowsEditable={size.px === FACE_SIZES[0].px}
                  onEditStart={mark}
                  onRowMove={moveRow}
                  bind:rowHeld
                  onfit={(over) => noteFit("face", size.px, over)}
                />
                {#if (fitOver["face"]?.[size.px] ?? 0) > 0}
                  <p
                    class="mt-2 flex items-center gap-1.5 text-[10px] uppercase tracking-[0.16em] text-[#8f2f22]"
                  >
                    <span class="w-1.5 h-1.5 rounded-full bg-[#c65f3c]"></span>
                    {$t("adminBattlesFitOver")}
                    {Math.round(fitOver["face"][size.px])}px
                  </p>
                {/if}
              </div>
            {/each}
          </div>
        </div>
      </section>

      <aside
        class="w-[27rem] flex-shrink-0 border-l border-[#34251c]/10 overflow-y-auto"
        onpointerdowncapture={mark}
        onfocusincapture={mark}
      >
        {#if frames[frameIndex]}
          <div class="p-4">
          <details class="mb-3">
            <summary
              class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
              >{$t("adminBattlesHintOpen")}</summary
            >
            <p
              class="max-w-[62ch] mt-2 text-[11px] leading-relaxed italic text-[#8a6a55]"
            >
              {$t("adminBattlesRosterHint")}
            </p>
          </details>
          <!-- Ящик на полосу. Строку перетаскивают внутри ящика — это
               порядок, — и из ящика в ящик — это полоса. Полоса, в которой
               строке стоять нельзя, метки не показывает вовсе: список
               предлагает только то, что имеет смысл. То же делается прямо
               на карте; здесь второй вход, для точности и для того, что на
               карте закрыто резьбой. -->
          {#each SHEET_BANDS as band (band)}
            {@const rows = bandRows(band)}
            <div
              role="list"
              ondragover={(e) => {
                e.preventDefault();
                aimRowList(band, null);
              }}
              ondrop={(e) => {
                e.preventDefault();
                dropRowList();
              }}
              class="mb-2 border {rowOver?.band === band
                ? 'border-[#c65f3c]'
                : 'border-[#34251c]/12'}"
            >
              <p
                class="px-2 py-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55] bg-[#34251c]/[0.04]"
              >
                {$t(SHEET_BAND_KEY[band])}
              </p>
              {#if !rows.length}
                <p class="px-2 py-2 text-[10px] italic text-[#8a6a55]">
                  {$t("adminBattlesRosterBandEmpty")}
                </p>
              {/if}
              {#each rows as row (row.slot)}
                <div
                  role="listitem"
                  draggable="true"
                  ondragstart={(e) => {
                    rowHeld = row.slot;
                    e.dataTransfer?.setData("text/plain", row.slot);
                  }}
                  ondragover={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    aimRowList(band, seatBy(band, row.slot, e));
                  }}
                  ondragend={() => {
                    rowHeld = null;
                    rowOver = null;
                  }}
                  ondrop={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    dropRowList();
                  }}
                  class="px-1.5 py-1.5 border-t border-[#34251c]/8 cursor-grab {rowHeld ===
                  row.slot
                    ? 'opacity-40'
                    : ''} {rowOver?.band === band &&
                  rowOver.before === row.slot
                    ? 'shadow-[inset_0_2px_0_0_#c65f3c]'
                    : ''}"
                >
                  <!-- Имя строкой, ступени под ним. В один ряд они не встают:
                       пять слов съедали всю ширину, и от названия оставалась
                       буква с многоточием — список, по которому нельзя узнать
                       ни одной строки. -->
                  <div class="flex items-center gap-2">
                    <span
                      class="text-[10px] text-[#8a6a55] select-none"
                      title={$t("adminBattlesRosterDrag")}>⠿</span
                    >
                    <span class="flex flex-col leading-none">
                      <button
                        onclick={() => resheet(row.slot, -1)}
                        title={$t("adminBattlesStackUp")}
                        class="px-1 text-[8px] text-[#8a6a55] hover:text-[#34251c]"
                        >▲</button
                      >
                      <button
                        onclick={() => resheet(row.slot, 1)}
                        title={$t("adminBattlesStackDown")}
                        class="px-1 text-[8px] text-[#8a6a55] hover:text-[#34251c]"
                        >▼</button
                      >
                    </span>
                    <span
                      class="flex-1 min-w-0 flex items-center gap-1.5 text-[11px] {row.show ===
                      'never'
                        ? 'text-[#8a6a55] line-through'
                        : ''}"
                    >
                      <!-- Знак у строки-числа. Список описи — это пятнадцать
                           строк подряд, и «Броня» от «Оберега» в нём отличаются
                           одной буквой; на карте они отличаются знаком, и стол,
                           который его не показывает, заставляет хранителя
                           узнавать свою же строку по памяти. У строк-слов знака
                           нет, и пустого места под него тоже: `flex` сдвигает
                           имя к самому ⠿. -->
                      {#if sheetSlotMark(row.slot)}
                        <span class="shrink-0 text-[#8a6a55]"
                          ><BattleIcon
                            name={sheetSlotMark(row.slot) ?? ''}
                            size={11}
                            weight={1.4}
                          /></span
                        >
                      {/if}
                      <span class="min-w-0 truncate">{$t(SHEET_SLOT_KEY[row.slot])}</span>
                    </span>
                  </div>
                  <!-- Пять ступеней, а не тумблер: разница между полкой и
                       клеткой боя — не «бой», а ширина. Четыре говорят, с
                       какой величины строку видно, пятая — до какой. -->
                  <div class="flex mt-1 ml-[1.55rem] border border-[#34251c]/15">
                    {#each SHEET_SHOWS as show (show)}
                      <button
                        onclick={() => (row.show = show)}
                        title={$t(SHEET_SHOW_KEY[show])}
                        class="flex-1 px-1 py-0.5 text-[9px] uppercase tracking-[0.06em] {row.show ===
                        show
                          ? 'bg-[#34251c] text-[#f8f1e7]'
                          : 'hover:bg-[#34251c]/5'}"
                        >{$t(SHEET_SHOW_SHORT[show])}</button
                      >
                    {/each}
                  </div>
                </div>
              {/each}
              {#if rowOver?.band === band && rowOver.before === null}
                <div class="h-0.5 bg-[#c65f3c]"></div>
              {/if}
            </div>
          {/each}
          <button
            onclick={resetSheet}
            class="mt-3 px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5"
            >{$t("adminBattlesRosterReset")}</button
          >

          <!-- Кегль и чернила. Множители, а не размеры: размеры карта
               считает из своей ширины, и рамка, назначающая пиксели,
               отняла бы у неё ровно то, ради чего она их считает. -->
          <div class="mt-5 pt-4 border-t border-[#34251c]/10 flex flex-wrap gap-5">
            <label class="block w-44">
              <span
                class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                >{$t("adminBattlesTypeScale")} · {(
                  frames[frameIndex].typeScale || 1
                ).toFixed(2)}</span
              >
              <input
                type="range"
                min="0.75"
                max="1.5"
                step="0.01"
                value={frames[frameIndex].typeScale || 1}
                oninput={(e) =>
                  (frames[frameIndex].typeScale = Number(
                    e.currentTarget.value,
                  ))}
                class="w-full"
              />
            </label>
            <label class="block w-44">
              <span
                class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                >{$t("adminBattlesInkFade")} · {(
                  frames[frameIndex].inkFade || 1
                ).toFixed(2)}</span
              >
              <input
                type="range"
                min="0.5"
                max="1.6"
                step="0.01"
                value={frames[frameIndex].inkFade || 1}
                oninput={(e) =>
                  (frames[frameIndex].inkFade = Number(
                    e.currentTarget.value,
                  ))}
                class="w-full"
              />
            </label>
          </div>
          </div>
        {/if}
      </aside>
      </div>
    </div>
  {:else if view === "bench"}
    <!--
      ── Стол хранителя ────────────────────────────────────────────────────
      Доску рисует тот же `BattleScene`, что и комната гостей: второй
      отрисовщик неизбежно разошёлся бы с первым, и стол начал бы врать ровно
      про то, что на нём проверяют.
    -->
    <div class="flex-1 flex min-h-0">
      <!-- ── Полка этюдов ─────────────────────────────────────────────────
           То, что оставили. Щелчок раскладывает этюд на столе — вместе с
           расстановкой, а не рядом с ней. -->
      <aside
        class="w-56 flex-shrink-0 flex flex-col border-r border-[#34251c]/10"
      >
        <div class="p-3 border-b border-[#34251c]/10">
          <button
            onclick={() => openEtude(null)}
            class="w-full px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5"
            >{$t("adminBattlesEtudeNew")}</button
          >
        </div>
        <div class="flex-1 overflow-y-auto">
          {#if !challenges.length}
            <p class="p-3 text-xs italic text-[#5f4636]">
              {$t("adminBattlesEtudeEmpty")}
            </p>
          {:else}
            <p
              class="px-3 py-2 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
            >
              {$t("adminBattlesDragHint")}
            </p>
            <ul class="pb-4">
              {#each challenges as challenge, i (challenge.id)}
                <li
                  draggable="true"
                  ondragstart={() => (etudeDragFrom = i)}
                  ondragover={(e) => {
                    e.preventDefault();
                    etudeDragOver = i;
                  }}
                  ondragleave={() => {
                    if (etudeDragOver === i) etudeDragOver = null;
                  }}
                  ondrop={(e) => {
                    e.preventDefault();
                    onEtudeDrop(i);
                  }}
                  ondragend={() => {
                    etudeDragFrom = null;
                    etudeDragOver = null;
                  }}
                  class="border-b border-[#34251c]/5 {etudeDragOver === i
                    ? 'bg-[#c65f3c]/10'
                    : ''} {etudeDragFrom === i ? 'opacity-40' : ''}"
                >
                  <button
                    onclick={() => openEtude(challenge)}
                    class="w-full text-left px-3 py-2.5 flex gap-2 items-start hover:bg-[#34251c]/[0.04] {etudeId ===
                    challenge.id
                      ? 'bg-[#34251c]/[0.06]'
                      : ''}"
                  >
                    <span
                      class="mt-1.5 w-1.5 h-1.5 rounded-full flex-shrink-0 {STATUS_TONE[
                        challenge.status
                      ]}"
                    ></span>
                    <span class="min-w-0 flex-1">
                      <span
                        class="block text-[13px] leading-snug truncate"
                        style="font-family: 'Cormorant Garamond', Georgia, serif;"
                      >
                        {etudeTitleOf(challenge)}
                      </span>
                      <span class="block text-[10px] text-[#8a6a55]">
                        {challenge.playerSide === "deck"
                          ? $t("adminBattlesEtudeSideDeck")
                          : $t("adminBattlesEtudeSideScripted")}
                        · {challenge.botDepth >= 2
                          ? $t("adminBattlesHandSearching")
                          : $t("adminBattlesHandGreedy")}
                        {#if challenge.rewardDust > 0}
                          · {challenge.rewardDust}
                        {/if}
                      </span>
                    </span>
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      </aside>

      <div class="flex-1 overflow-y-auto p-6 min-w-0">
        <details class="mb-1">
          <summary
            class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
            >{$t("adminBattlesHintOpen")}</summary
          >
          <p class="max-w-[62ch] mt-2 text-xs leading-relaxed text-[#5f4636]">
            {$t("adminBattlesBenchHint")}
          </p>
        </details>
        <p class="max-w-[62ch] mb-5 text-[11px] leading-relaxed text-[#8a6a55]">
          {$t("adminBattlesBenchNoHealth")}
        </p>

        <details class="border-b border-[#34251c]/10">
          <summary
            class="px-4 py-2.5 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
            >{$t("adminBattlesRates")}</summary
          >
          <div class="px-4 pb-4">
            <!-- Ставки начисления, подарок новичку и часы. Это настройки
                 КОМНАТЫ, а не чина: рама решает, как карта выглядит, а здесь
                 решают, по чему она достаётся и когда. На вкладке рамок они
                 стояли исторически, и хранитель, пришедший красить бумагу,
                 листал мимо них курсы пыли. -->
            <div class="mb-8 pb-6 border-b border-[#34251c]/10">
              <p
                class="mb-1 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]"
              >
                {$t("adminBattlesRates")}
              </p>
              <details class="mb-3">
                <summary
                  class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
                  >{$t("adminBattlesHintOpen")}</summary
                >
                <p
                  class="max-w-[62ch] mt-2 text-[11px] leading-relaxed italic text-[#8a6a55]"
                >
                  {$t("adminBattlesRatesHint")}
                </p>
              </details>
              <div class="flex flex-wrap items-end gap-3">
                {#each [{ key: "liked" as const, label: $t("adminBattlesRateLiked") }, { key: "seen" as const, label: $t("adminBattlesRateSeen") }, { key: "read" as const, label: $t("adminBattlesRateRead") }] as row (row.key)}
                  <label class="block w-40">
                    <span
                      class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                      >{row.label}</span
                    >
                    <input
                      type="number"
                      min="0"
                      value={dustRates[row.key]}
                      oninput={(e) =>
                        (dustRates[row.key] = Math.max(
                          0,
                          Math.round(Number(e.currentTarget.value) || 0),
                        ))}
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
                  >{$t("adminBattlesRatesSave")}</button
                >
              </div>
            </div>

            <!-- Дар первого входа. Ставки говорят, что дом платит за внимание;
                 дар говорит, с чего человек начинает, когда внимания ещё нет. -->
            <div class="mb-8 pb-6 border-b border-[#34251c]/10">
              <p
                class="mb-1 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]"
              >
                {$t("adminBattlesGift")}
              </p>
              <details class="mb-3">
                <summary
                  class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
                  >{$t("adminBattlesHintOpen")}</summary
                >
                <p
                  class="max-w-[62ch] mt-2 text-[11px] leading-relaxed italic text-[#8a6a55]"
                >
                  {$t("adminBattlesGiftHint")}
                </p>
              </details>
              <div class="flex flex-wrap items-end gap-3">
                {#each [{ key: "dust" as const, label: $t("adminBattlesGiftDust") }, { key: "feed" as const, label: $t("adminBattlesGiftFeed") }] as row (row.key)}
                  <label class="block w-40">
                    <span
                      class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                      >{row.label}</span
                    >
                    <input
                      type="number"
                      min="0"
                      value={gift[row.key]}
                      oninput={(e) =>
                        (gift[row.key] = Math.max(
                          0,
                          Math.round(Number(e.currentTarget.value) || 0),
                        ))}
                      onfocus={selectOnFocus}
                      onwheel={blurOnWheel}
                      class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                    />
                  </label>
                {/each}
                <button
                  type="button"
                  disabled={giftSaving}
                  onclick={saveGift}
                  class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/25 hover:bg-[#34251c]/5 disabled:opacity-40"
                  >{$t("adminBattlesGiftSave")}</button
                >
              </div>
            </div>

            <!-- Часы дома. Одно число, от которого зависит, когда у человека
                 наступает «завтра» в повторяющихся поручениях. -->
            <div class="mb-8 pb-6 border-b border-[#34251c]/10">
              <p
                class="mb-1 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]"
              >
                {$t("adminBattlesClock")}
              </p>
              <details class="mb-3">
                <summary
                  class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
                  >{$t("adminBattlesHintOpen")}</summary
                >
                <p
                  class="max-w-[62ch] mt-2 text-[11px] leading-relaxed italic text-[#8a6a55]"
                >
                  {$t("adminBattlesClockHint")}
                </p>
              </details>
              <div class="flex flex-wrap items-end gap-3">
                <label class="block w-40">
                  <span
                    class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                    >{$t("adminBattlesClockOffset")}</span
                  >
                  <input
                    type="number"
                    step="30"
                    value={clock.offsetMin}
                    oninput={(e) =>
                      (clock.offsetMin = Math.round(
                        Number(e.currentTarget.value) || 0,
                      ))}
                    onfocus={selectOnFocus}
                    onwheel={blurOnWheel}
                    class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                  />
                </label>
                <p class="pb-1.5 text-[11px] text-[#8a6a55]">
                  {$t("adminBattlesClockNow")}
                  {new Date(Date.now() + (clock.offsetMin + new Date().getTimezoneOffset()) * 60000).toLocaleString($lang === "ru" ? "ru-RU" : "en-GB")}
                </p>
                <button
                  type="button"
                  disabled={clockSaving}
                  onclick={saveClock}
                  class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/25 hover:bg-[#34251c]/5 disabled:opacity-40"
                  >{$t("adminBattlesClockSave")}</button
                >
              </div>
            </div>
          </div>
        </details>

        <!-- ── Чем расстановка подписана, если её оставить ────────────────
             Поля стоят над столом, а расстановки не хранят: её всегда даёт
             `benchSetup`. Сохранить можно только то, что стоит на столе
             сейчас, — и только когда на обеих половинах кто-то есть, ровно
             как проверяет сервер. -->
        <div class="mb-6 pb-5 border-b border-[#34251c]/10">
          <p
            class="mb-1 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]"
          >
            {etudeId ? $t("adminBattlesEtude") : $t("adminBattlesEtudeNew")}
          </p>
          <details class="mb-3">
            <summary
              class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
              >{$t("adminBattlesHintOpen")}</summary
            >
            <p
              class="max-w-[62ch] mt-2 text-[11px] leading-relaxed italic text-[#8a6a55]"
            >
              {$t("adminBattlesEtudeHint")}
            </p>
          </details>

          <div class="flex flex-wrap gap-x-4 gap-y-3 items-end">
            {#each [{ label: $t("adminBattlesEtudeTitle") + " · RU", get: () => etudeTitleRu, set: (v: string) => (etudeTitleRu = v), wide: true }, { label: $t("adminBattlesEtudeTitle") + " · EN", get: () => etudeTitleEn, set: (v: string) => (etudeTitleEn = v), wide: true }, { label: $t("adminBattlesEtudeNote") + " · RU", get: () => etudeNoteRu, set: (v: string) => (etudeNoteRu = v), wide: false }, { label: $t("adminBattlesEtudeNote") + " · EN", get: () => etudeNoteEn, set: (v: string) => (etudeNoteEn = v), wide: false }] as field (field.label)}
              <label class="block {field.wide ? 'w-52' : 'w-64'}">
                <span
                  class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                  >{field.label}</span
                >
                <input
                  value={field.get()}
                  oninput={(e) => field.set(e.currentTarget.value)}
                  class="w-full px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                />
              </label>
            {/each}

            <label class="block w-44">
              <span
                class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                >{$t("adminBattlesEtudeDepth")}</span
              >
              <select
                bind:value={etudeDepth}
                class="w-full px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
              >
                {#each BOT_HANDS as hand (hand.depth)}
                  <option value={hand.depth}>{$t(hand.label)}</option>
                {/each}
              </select>
            </label>

            <label class="block w-32">
              <span
                class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                >{$t("adminBattlesEtudeReward")}</span
              >
              <input
                type="number"
                min="0"
                max="1000"
                bind:value={etudeReward}
                class="w-full px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
              />
            </label>

            <!-- За доведённое до конца. Платится и проигравшему: до сих пор
                 новичок, проигравший первую партию, не получал ровно ничего —
                 худший из возможных ответов на первую же попытку сыграть.
                 Ключ у неё свой, так что фермы это не открывает. -->
            <label class="block w-32">
              <span
                class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                >{$t("adminBattlesEtudeFinish")}</span
              >
              <input
                type="number"
                min="0"
                max="1000"
                bind:value={etudeFinish}
                class="w-full px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
              />
            </label>

            <label class="block w-44">
              <span
                class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                >{$t("adminBattlesEtudeSide")}</span
              >
              <select
                bind:value={etudeSide}
                class="w-full px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
              >
                <option value="scripted"
                  >{$t("adminBattlesEtudeSideScripted")}</option
                >
                <option value="deck">{$t("adminBattlesEtudeSideDeck")}</option>
              </select>
            </label>

            <label class="block w-32">
              <span
                class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                >{$t("adminBattlesStatus")}</span
              >
              <select
                bind:value={etudeStatus}
                class="w-full px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
              >
                <option value="draft">{$t("adminBattlesStatusDraft")}</option>
                <option value="published"
                  >{$t("adminBattlesStatusPublished")}</option
                >
                <option value="retired"
                  >{$t("adminBattlesStatusRetired")}</option
                >
              </select>
            </label>

            <div class="flex gap-2">
              <button
                type="button"
                disabled={!etudeReady || saving}
                onclick={saveEtude}
                class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] bg-[#34251c] text-[#f8f1e7] disabled:opacity-40"
                >{$t("adminBattlesEtudeSave")}</button
              >
              {#if etudeId}
                <button
                  type="button"
                  onclick={removeEtude}
                  class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20"
                  >{$t("adminBattlesEtudeDelete")}</button
                >
              {/if}
            </div>
          </div>

          <!-- Пыль за этюд, а не за победу: переигрывать можно сколько угодно,
               заплатят однажды. Сказано здесь, чтобы число ставили осознанно. -->
          <p
            class="mt-3 max-w-[62ch] text-[11px] leading-relaxed text-[#8a6a55]"
          >
            {$t("adminBattlesEtudeRewardNote")}
          </p>

          <!-- ── Правила этюда ──────────────────────────────────────────────
               Вторая половина замысла: расстановка говорит, кто стоит, правила
               — во что играют. Стол ниже играет ими же, поэтому проверяется
               именно тот этюд, который потом достанется гостю. -->
          <div class="mt-4 pt-4 border-t border-[#34251c]/10">
            <label class="inline-flex items-center gap-2 cursor-pointer">
              <input type="checkbox" bind:checked={etudeOwnRules} />
              <span class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]"
                >{$t("adminBattlesRulesOwn")}</span
              >
            </label>
            <details class="mt-1">
              <summary
                class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
                >{$t("adminBattlesHintOpen")}</summary
              >
              <p
                class="mt-2 max-w-[62ch] text-[11px] leading-relaxed italic text-[#8a6a55]"
              >
                {$t("adminBattlesRulesHint")}
              </p>
            </details>

            {#if etudeOwnRules}
              <div class="mt-3 flex flex-wrap gap-x-5 gap-y-3 items-end">
                {#each RULE_DIALS as dial (dial.key)}
                  {#if dial.kind === "flag"}
                    <label
                      class="inline-flex items-center gap-2 w-56 cursor-pointer"
                    >
                      <input
                        type="checkbox"
                        checked={etudeRules[dial.key]}
                        onchange={(e) =>
                          tune(dial.key, e.currentTarget.checked)}
                      />
                      <span class="text-[11px] text-[#5f4636]"
                        >{$t(dial.label)}</span
                      >
                    </label>
                  {:else}
                    <label class="block w-40">
                      <span
                        class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                        >{$t(dial.label)}</span
                      >
                      <input
                        type="number"
                        min={dial.min}
                        max={dial.max}
                        value={etudeRules[dial.key]}
                        oninput={(e) =>
                          tune(dial.key, Number(e.currentTarget.value))}
                        class="w-full px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                      />
                    </label>
                  {/if}
                {/each}
              </div>

              <!-- То же самое, что прочтёт гость на полке. Хранитель, видящий
                   другую строку, правит вслепую. -->
              <p
                class="mt-3 max-w-[62ch] text-[11px] leading-relaxed text-[#5f4636]"
              >
                {#if etudeApart.length}
                  {$t("adminBattlesRulesApart")}:
                  {#each etudeApart as line, i (line.key)}{i > 0
                      ? " · "
                      : " "}{$t(line.key)}{line.amount === null
                      ? ""
                      : ` — ${line.amount}`}{/each}
                {:else}
                  {$t("adminBattlesRulesSame")}
                {/if}
              </p>
              <button
                type="button"
                onclick={() => (etudeRules = { ...HOUSE_RULES })}
                class="mt-2 px-2 py-1 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20"
                >{$t("adminBattlesRulesReset")}</button
              >
            {/if}
          </div>
          {#if etudeSide === "deck"}
            <p
              class="mt-2 max-w-[62ch] text-[11px] leading-relaxed text-[#8a6a55]"
            >
              {$t("adminBattlesEtudeSideDeckNote")}
            </p>
          {/if}

          {#if benchGone.length}
            <p
              class="mt-2 max-w-[62ch] text-[11px] leading-relaxed text-[#8f2f22]"
            >
              {$t("adminBattlesEtudeGone")}: {benchGone
                .map(benchTitle)
                .join(", ")}
            </p>
          {/if}
        </div>

        {#if benchComplaint}
          <p class="mb-4 text-xs text-[#8f2f22]">{benchComplaint}</p>
        {/if}

        <div class="flex flex-wrap gap-8 items-start">
          <!-- Расстановка. Клетка — это просто выпадающий список: перетаскивание
               здесь ничего не проверяет, а сломать может. -->
          <div class="w-[22rem]">
            <p
              class="mb-2 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]"
            >
              {$t("adminBattlesBenchPlace")}
            </p>
            {#each Array.from({ length: BENCH_DEPTH }, (_, y) => y) as y (y)}
              <div
                class="flex gap-1 mb-1 {y === 3
                  ? 'mt-2 pt-2 border-t border-dashed border-[#34251c]/20'
                  : ''}"
              >
                {#each Array.from({ length: BENCH_WIDTH }, (_, x) => x) as x (x)}
                  <select
                    value={benchBoard[`${x},${y}`] ?? ""}
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
                    <!-- Карта, снятая с полки после того, как этюд был
                         сохранён, осталась бы без своей строки и пропала из
                         списка молча. Ей даётся строка с пометкой: убрать её
                         должен хранитель, а не выпадающий список. -->
                    {#if gone(benchBoard[`${x},${y}`] ?? "")}
                      {@const slug = benchBoard[`${x},${y}`]}
                      <option value={slug}
                        >{benchTitle(slug)} — {$t(
                          "adminBattlesEtudeGoneShort",
                        )}</option
                      >
                    {/if}
                    {#each benchable as c (c.id)}
                      <option value={c.slug}>{c.titleRu}</option>
                    {/each}
                  </select>
                {/each}
              </div>
            {/each}
            <p
              class="mt-1 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]"
            >
              {$t("adminBattlesBenchKeeperHalf")} ↑ · {$t(
                "adminBattlesBenchGuestHalf",
              )} ↓
            </p>
          </div>

          <!-- Руки обеих сторон. -->
          <div class="w-64 flex flex-col gap-4">
            {#each [{ side: "keeper" as const, label: $t("adminBattlesBenchKeeperHalf") }, { side: "player" as const, label: $t("adminBattlesBenchGuestHalf") }] as row (row.side)}
              <div>
                <p
                  class="mb-1 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]"
                >
                  {$t("adminBattlesBenchHand")} · {row.label}
                </p>
                <select
                  value=""
                  onchange={(e) => {
                    benchAddToHand(row.side, e.currentTarget.value);
                    e.currentTarget.value = "";
                  }}
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
                      class="px-1.5 py-0.5 text-[11px] border hover:bg-[#c65f3c]/10 {gone(
                        slug,
                      )
                        ? 'border-[#8f2f22]/40 text-[#8f2f22]'
                        : 'border-[#34251c]/15'}">{benchTitle(slug)} ×</button
                    >
                  {:else}
                    <span class="text-[11px] italic text-[#8a6a55]"
                      >{$t("adminBattlesBenchEmpty")}</span
                    >
                  {/each}
                </div>
              </div>
            {/each}

            <label class="flex items-center gap-2 text-[11px] text-[#5f4636]">
              <input type="checkbox" bind:checked={benchBoth} />
              {$t("adminBattlesBenchBothSides")}
            </label>

            <div class="flex flex-wrap gap-2">
              <button
                type="button"
                disabled={!benchReady || benchBusy}
                onclick={benchStart}
                class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] bg-[#34251c] text-[#f8f1e7] disabled:opacity-40"
                >{$t("adminBattlesBenchStart")}</button
              >
              <button
                type="button"
                disabled={!benchReady || benchBusy}
                onclick={() => benchCall(null, true)}
                class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/25 disabled:opacity-40"
                >{$t("adminBattlesBenchPlayOut")}</button
              >
              <button
                type="button"
                disabled={!benchJournal.length || benchBusy}
                onclick={benchUndo}
                class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 disabled:opacity-40"
                >{$t("adminBattlesBenchUndo")}</button
              >
              <button
                type="button"
                onclick={benchReset}
                class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20"
                >{$t("adminBattlesBenchReset")}</button
              >
            </div>
            {#if !benchReady}
              <p class="text-[11px] italic text-[#8a6a55]">
                {$t("adminBattlesBenchNeedsBodies")}
              </p>
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
                {motions}
                busy={benchBusy}
                control={benchBoth ? "both" : "player"}
                onact={(a) => benchCall(a)}
              />
            </div>
          {/if}
        </div>

        <!-- Журнал. Ради разбора урона он и нужен: видно, почему три, а не восемь. -->
        {#if bench && bench.events.length}
          <div class="mt-8 max-w-3xl">
            <p
              class="mb-2 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]"
            >
              {$t("adminBattlesBenchLog")}
            </p>
            <ul class="text-[11px] leading-relaxed text-[#5f4636] font-mono">
              {#each bench.events as e, i (i)}
                <li>{benchLine(e)}</li>
              {/each}
            </ul>
          </div>
        {/if}
      </div>
    </div>
  {:else if view === "hand"}
    <!--
      ── Из рук ────────────────────────────────────────────────────────────
      Единственный способ, каким в доме появляется корм. Не настройка комнаты
      (те живут при рамках), а поступок, обращённый к одному человеку, —
      поэтому своя комната, а не строка среди ставок.
    -->
    <div class="flex-1 overflow-y-auto p-6">
      <details class="mb-6">
        <summary
          class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
          >{$t("adminBattlesHintOpen")}</summary
        >
        <p class="max-w-[62ch] mt-2 text-xs leading-relaxed text-[#5f4636]">
          {$t("adminBattlesHandHint")}
        </p>
        <p
          class="max-w-[62ch] mt-2 text-[11px] leading-relaxed italic text-[#8a6a55]"
        >
          {$t("adminBattlesHandNoteHint")}
        </p>
      </details>

      <div class="max-w-xl">
        <!-- Кому. Поиск, а не длинный список: гостей больше, чем помещается. -->
        <label
          for="hand-who"
          class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
          >{$t("adminBattlesHandWho")}</label
        >
        <div class="flex gap-2">
          <input
            id="hand-who"
            bind:value={guestQuery}
            onkeydown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                void findGuests();
              }
            }}
            placeholder={$t("adminBattlesHandSearch")}
            class="flex-1 px-2 py-1.5 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
          />
          <button
            type="button"
            onclick={findGuests}
            class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5"
            >{$t("adminBattlesHandFind")}</button
          >
        </div>

        {#if guests.length}
          <ul class="mt-2 max-h-52 overflow-y-auto border border-[#34251c]/10">
            {#each guests as guest (guest.id)}
              <li class="border-b border-[#34251c]/5 last:border-b-0">
                <button
                  type="button"
                  onclick={() => chooseGuest(guest)}
                  class="w-full text-left px-3 py-2 text-xs hover:bg-[#34251c]/[0.04] {guestChosen?.id ===
                  guest.id
                    ? 'bg-[#34251c]/[0.06]'
                    : ''}"
                >
                  <span
                    style="font-family: 'Cormorant Garamond', Georgia, serif;"
                    >{guest.displayName}</span
                  >
                  <span class="block text-[10px] text-[#8a6a55]"
                    >{guest.email}</span
                  >
                </button>
              </li>
            {/each}
          </ul>
        {/if}

        {#if guestChosen}
          <p class="mt-4 text-xs text-[#5f4636]">
            {$t("adminBattlesHandTo")}
            <b style="font-family: 'Cormorant Garamond', Georgia, serif;"
              >{guestChosen.displayName}</b
            >
          </p>

          <!-- Что у гостя сейчас. Без этого выдача — действие вслепую: не видно
               ни что уже было, ни что изменилось. Это ровно то, что видит сам
               гость, а не второй отчёт, который может с ним разойтись. -->
          {#if guestHas}
            <div
              class="mt-2 flex flex-wrap items-center gap-x-5 gap-y-1 text-xs text-[#5f4636]"
            >
              <span>
                {$t("battlesCoinDust")}:
                <b class="tabular-nums">{guestHas.dust}</b>
                {#if guestHas.dust !== 0}
                  <button
                    type="button"
                    onclick={() => zeroCoin("dust")}
                    class="ml-1 text-[10px] uppercase tracking-[0.14em] text-[#8a6a55] hover:text-[#c65f3c] underline"
                    >{$t("adminBattlesZero")}</button
                  >
                {/if}
              </span>
              <span>
                {$t("battlesCoinFeed")}:
                <b class="tabular-nums">{guestHas.feed}</b>
                {#if guestHas.feed !== 0}
                  <button
                    type="button"
                    onclick={() => zeroCoin("feed")}
                    class="ml-1 text-[10px] uppercase tracking-[0.14em] text-[#8a6a55] hover:text-[#c65f3c] underline"
                    >{$t("adminBattlesZero")}</button
                  >
                {/if}
              </span>
              <span
                >{$t("adminBattlesGuestCards")}:
                <b class="tabular-nums">{guestHas.owned.length}</b></span
              >
            </div>
          {/if}

          <!-- Прямая выдача карт. Отдельно от монет, потому что это не покупка:
               кошелёк не трогается, цена не проверяется. Нужна, чтобы привести
               собрание в состояние, в котором игру можно проверить. -->
          <div class="mt-5 pt-4 border-t border-[#34251c]/10">
            <p
              class="mb-1 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]"
            >
              {$t("adminBattlesGive")}
            </p>
            <details class="mb-3">
              <summary
                class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
                >{$t("adminBattlesHintOpen")}</summary
              >
              <p
                class="max-w-[62ch] mt-2 text-[11px] leading-relaxed italic text-[#8a6a55]"
              >
                {$t("adminBattlesGiveHint")}
              </p>
            </details>
            <div class="flex flex-wrap items-end gap-3">
              <label class="block w-32">
                <span
                  class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                  >{$t("adminBattlesGiveLevel")}</span
                >
                <select
                  bind:value={giveLevel}
                  class="w-full px-2 py-1.5 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                >
                  {#each TIERS as step (step)}
                    <option value={step}>{step}</option>
                  {/each}
                </select>
              </label>
              <button
                type="button"
                disabled={giving}
                onclick={giveAllCards}
                class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] bg-[#34251c] text-[#f8f1e7] disabled:opacity-40"
                >{$t("adminBattlesGiveAll")}</button
              >
              <button
                type="button"
                disabled={giving || !guestHas?.owned.length}
                onclick={takeAllCards}
                class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/25 disabled:opacity-40"
                >{$t("adminBattlesTakeAll")}</button
              >
            </div>
          </div>
        {/if}

        <!-- Заготовки дел. Названы гостю на полке — значит, и выдаваться должны
             тем же числом и теми же словами, иначе полка обещает одно, а книга
             показывает другое. -->
        {#if deeds.length}
          <div class="mt-5">
            <p class="mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">
              {$t("adminBattlesDeeds")}
            </p>
            <div class="flex flex-wrap gap-2">
              {#each deeds as deed (deed.id)}
                <button
                  type="button"
                  onclick={() => takeDeed(deed)}
                  class="px-2.5 py-1 text-[11px] border border-[#34251c]/20 hover:bg-[#34251c]/5"
                >
                  {$lang === "ru" ? deed.titleRu : deed.titleEn}
                  <span class="ml-1 text-[#6f3b24]">{deed.amount}</span>
                </button>
              {/each}
            </div>
          </div>
        {/if}

        <div class="mt-4 flex flex-wrap items-end gap-3">
          <label class="block w-40">
            <span
              class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
              >{$t("adminBattlesHandCoin")}</span
            >
            <select
              bind:value={grantCoin}
              class="w-full px-2 py-1.5 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
            >
              <option value="feed">{$t("battlesCoinFeed")}</option>
              <option value="dust">{$t("battlesCoinDust")}</option>
            </select>
          </label>
          <label class="block w-32">
            <span
              class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
              >{$t("adminBattlesHandAmount")}</span
            >
            <input
              type="number"
              bind:value={grantAmount}
              onfocus={selectOnFocus}
              onwheel={blurOnWheel}
              class="w-full px-2 py-1.5 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
            />
          </label>
        </div>

        <label class="block mt-3">
          <span
            class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
            >{$t("adminBattlesHandNote")}</span
          >
          <textarea
            bind:value={grantNote}
            rows="2"
            placeholder={$t("adminBattlesHandNotePlaceholder")}
            class="w-full px-2 py-1.5 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
          ></textarea>
        </label>

        <button
          type="button"
          disabled={!grantReady || granting}
          onclick={giveByHand}
          class="mt-3 px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] bg-[#34251c] text-[#f8f1e7] disabled:opacity-40"
          >{granting
            ? $t("adminBattlesHandGiving")
            : $t("adminBattlesHandGive")}</button
        >

        <!-- Минус — не штраф, а поправка: книга неизменяема, и ошибка правится
             обратной строкой, а не правкой строки, которая была неверна. -->
        <p class="mt-4 max-w-[62ch] text-[11px] leading-relaxed text-[#8a6a55]">
          {$t("adminBattlesHandMinusNote")}
        </p>
      </div>
    </div>
  {:else if view === "matches"}
    <!--
      ── Сыгранные партии ──────────────────────────────────────────────────
      Три взгляда на одно и то же, от общего к частному: чем кончаются этюды,
      какие карты выходят на поле и что с ними случается, и наконец сами
      партии списком. Разбор считается по ЗАМОРОЖЕННОЙ расстановке каждой
      партии: карта могла быть с тех пор переписана, а выиграла та, что стояла
      на доске.
    -->
    <div class="flex-1 overflow-y-auto p-6">
      <div class="flex items-baseline gap-3 mb-1">
        <p class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">
          {$t("adminBattlesMatches")}
        </p>
        <button
          type="button"
          onclick={loadMatches}
          disabled={matchesBusy}
          class="text-[10px] uppercase tracking-[0.16em] text-[#6f3b24] underline disabled:opacity-40"
          >{$t("adminBattlesMatchesRefresh")}</button
        >
      </div>
      <details class="mb-6">
        <summary
          class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
          >{$t("adminBattlesHintOpen")}</summary
        >
        <p class="max-w-[70ch] mt-2 text-xs leading-relaxed text-[#5f4636]">
          {$t("adminBattlesMatchesHint")}
        </p>
      </details>

      {#if matchesBusy && !matches}
        <p class="text-xs text-[#5f4636]">…</p>
      {:else if !matches || !matches.rows.length}
        <p class="text-xs italic text-[#5f4636]">
          {$t("adminBattlesMatchesEmpty")}
        </p>
      {:else}
        <!-- ── Чем кончаются этюды ──────────────────────────────────────── -->
        <p class="mb-2 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">
          {$t("adminBattlesByChallenge")}
        </p>
        <table class="w-full mb-8 text-xs border-collapse">
          <thead class="text-[10px] uppercase tracking-[0.14em] text-[#8a6a55]">
            <tr class="border-b border-[#34251c]/15">
              <th class="py-1.5 text-left font-normal"
                >{$t("adminBattlesEtude")}</th
              >
              <th class="py-1.5 text-right font-normal"
                >{$t("adminBattlesPlayed")}</th
              >
              <th class="py-1.5 text-right font-normal"
                >{$t("adminBattlesGuestWon")}</th
              >
              <th class="py-1.5 text-right font-normal"
                >{$t("adminBattlesKeeperWon")}</th
              >
              <th class="py-1.5 text-right font-normal"
                >{$t("adminBattlesDraws")}</th
              >
              <th class="py-1.5 text-right font-normal"
                >{$t("adminBattlesUnfinished")}</th
              >
              <th class="py-1.5 text-right font-normal"
                >{$t("adminBattlesGuestShare")}</th
              >
            </tr>
          </thead>
          <tbody>
            {#each matches.byChallenge as row (row.challengeId ?? "none")}
              <tr class="border-b border-[#34251c]/5">
                <td
                  class="py-1.5"
                  style="font-family: 'Cormorant Garamond', Georgia, serif;"
                >
                  {row.challengeId
                    ? tallyTitle(row)
                    : $t("adminBattlesNoEtude")}
                </td>
                <td class="py-1.5 text-right tabular-nums">{row.played}</td>
                <td class="py-1.5 text-right tabular-nums">{row.guestWon}</td>
                <td class="py-1.5 text-right tabular-nums">{row.keeperWon}</td>
                <td class="py-1.5 text-right tabular-nums">{row.draws}</td>
                <td class="py-1.5 text-right tabular-nums text-[#8a6a55]"
                  >{row.unfinished}</td
                >
                <td class="py-1.5 text-right tabular-nums">
                  {share(
                    row.guestWon,
                    row.guestWon + row.keeperWon + row.draws,
                  )}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>

        <!-- ── Что случается с картами ──────────────────────────────────── -->
        <p class="mb-1 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">
          {$t("adminBattlesByCard")}
        </p>
        <details class="mb-2">
          <summary
            class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
            >{$t("adminBattlesHintOpen")}</summary
          >
          <p
            class="max-w-[70ch] mt-2 text-[11px] leading-relaxed italic text-[#8a6a55]"
          >
            {$t("adminBattlesByCardHint")}
          </p>
        </details>
        <table class="w-full mb-8 text-xs border-collapse">
          <thead class="text-[10px] uppercase tracking-[0.14em] text-[#8a6a55]">
            <tr class="border-b border-[#34251c]/15">
              <th class="py-1.5 text-left font-normal"
                >{$t("adminBattlesCardsView")}</th
              >
              <th class="py-1.5 text-right font-normal"
                >{$t("adminBattlesPlayed")}</th
              >
              <th class="py-1.5 text-right font-normal"
                >{$t("adminBattlesWon")}</th
              >
              <th class="py-1.5 text-right font-normal"
                >{$t("adminBattlesLost")}</th
              >
              <th class="py-1.5 text-right font-normal"
                >{$t("adminBattlesDraws")}</th
              >
              <th class="py-1.5 text-right font-normal"
                >{$t("adminBattlesWinShare")}</th
              >
            </tr>
          </thead>
          <tbody>
            {#each matches.byCard as row (row.slug)}
              <tr class="border-b border-[#34251c]/5">
                <td
                  class="py-1.5"
                  style="font-family: 'Cormorant Garamond', Georgia, serif;"
                  >{tallyTitle(row)}</td
                >
                <td class="py-1.5 text-right tabular-nums">{row.played}</td>
                <td class="py-1.5 text-right tabular-nums">{row.won}</td>
                <td class="py-1.5 text-right tabular-nums">{row.lost}</td>
                <td class="py-1.5 text-right tabular-nums">{row.draws}</td>
                <td class="py-1.5 text-right tabular-nums"
                  >{share(row.won, row.played)}</td
                >
              </tr>
            {/each}
          </tbody>
        </table>

        <!-- ── Пересмотр ────────────────────────────────────────────────
             Стоит НАД списком, а не под ним: список длинный, и доска,
             открывшаяся где-то внизу, осталась бы незамеченной. -->
        {#if replay && replayId}
          <div class="mb-8 p-4 border border-[#34251c]/15 bg-[#34251c]/[0.02]">
            <div class="flex flex-wrap items-center gap-2 mb-3">
              <span
                class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]"
              >
                {$t("adminBattlesReviewStep")}
                {replay.upto} / {replay.total}
              </span>
              <div class="flex gap-1">
                {#each [{ label: "⏮", to: 0, off: replay.upto === 0 }, { label: "‹", to: replay.upto - 1, off: replay.upto === 0 }, { label: "›", to: replay.upto + 1, off: replay.upto >= replay.total }, { label: "⏭", to: replay.total, off: replay.upto >= replay.total }] as step (step.label)}
                  <button
                    type="button"
                    disabled={replayBusy || step.off}
                    onclick={() => stepTo(replayId!, step.to)}
                    class="px-2.5 py-1 text-xs border border-[#34251c]/20 hover:bg-[#34251c]/5 disabled:opacity-30"
                    >{step.label}</button
                  >
                {/each}
              </div>
              <button
                type="button"
                onclick={closeReplay}
                class="ml-auto text-[10px] uppercase tracking-[0.16em] text-[#6f3b24] underline"
                >{$t("adminBattlesReviewClose")}</button
              >
            </div>

            {#if replay.diverged}
              <p class="mb-3 text-[11px] leading-relaxed text-[#8f2f22]">
                {$t("adminBattlesReviewDiverged")}
              </p>
            {/if}

            <div class="max-w-3xl">
              {#if replayMatch}
                <BattleScene
                  match={replayMatch}
                  {cards}
                  {frames}
                  {motions}
                  busy={replayBusy}
                  control="player"
                  onact={() => {}}
                />
              {/if}
            </div>

            <!-- Что случилось именно на этой ступени. Разбор урона тем же
                 словарём, что и на столе: видно, почему три, а не восемь. -->
            {#if replay.events.length}
              <ul
                class="mt-3 space-y-0.5 text-[11px] leading-relaxed text-[#5f4636]"
              >
                {#each replay.events as e, i (i)}
                  <li>{benchLine(e)}</li>
                {/each}
              </ul>
            {:else}
              <p class="mt-3 text-[11px] italic text-[#8a6a55]">
                {$t("adminBattlesReviewOpening")}
              </p>
            {/if}
          </div>
        {/if}

        <!-- ── Сами партии ──────────────────────────────────────────────── -->
        <p class="mb-2 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">
          {$t("adminBattlesMatchesList")} · {matches.read}
        </p>
        <table class="w-full text-xs border-collapse">
          <thead class="text-[10px] uppercase tracking-[0.14em] text-[#8a6a55]">
            <tr class="border-b border-[#34251c]/15">
              <th class="py-1.5 text-left font-normal"
                >{$t("adminBattlesWhen")}</th
              >
              <th class="py-1.5 text-left font-normal"
                >{$t("adminBattlesGuest")}</th
              >
              <th class="py-1.5 text-left font-normal"
                >{$t("adminBattlesEtude")}</th
              >
              <th class="py-1.5 text-left font-normal"
                >{$t("adminBattlesOutcome")}</th
              >
              <th class="py-1.5 text-right font-normal"
                >{$t("adminBattlesRoundsShort")}</th
              >
              <th class="py-1.5 text-right font-normal"
                >{$t("adminBattlesMoves")}</th
              >
              <th class="py-1.5"></th>
            </tr>
          </thead>
          <tbody>
            {#each matches.rows as row (row.id)}
              <tr class="border-b border-[#34251c]/5">
                <td class="py-1.5 tabular-nums text-[#8a6a55]"
                  >{shortDate(row.startedAt)}</td
                >
                <td
                  class="py-1.5"
                  style="font-family: 'Cormorant Garamond', Georgia, serif;"
                  >{row.guest}</td
                >
                <td class="py-1.5"
                  >{row.challengeId
                    ? tallyTitle(row)
                    : $t("adminBattlesNoEtude")}</td
                >
                <td class="py-1.5">
                  {#if row.outcome}
                    {$t(OUTCOME_WORD[row.outcome])}
                  {:else}
                    <span class="italic text-[#8a6a55]"
                      >{$t("adminBattlesOutcomeUnfinished")}</span
                    >
                  {/if}
                </td>
                <td class="py-1.5 text-right tabular-nums"
                  >{row.rounds ?? "—"}</td
                >
                <td class="py-1.5 text-right tabular-nums">{row.moves}</td>
                <td class="py-1.5 text-right">
                  <button
                    type="button"
                    disabled={replayBusy || !row.moves}
                    onclick={() => stepTo(row.id, 0)}
                    class="text-[10px] uppercase tracking-[0.16em] text-[#6f3b24] underline disabled:opacity-30 disabled:no-underline"
                    >{$t("adminBattlesReview")}</button
                  >
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>
  {:else if view === "keywords"}
    <!--
      ── The keyword dictionary ────────────────────────────────────────────
      A rule worded once and priced once. `pointValue` is why this is a table
      and not a constant in the server: rebalancing the whole game is an edit
      here, not a deployment.
    -->
    <div class="flex-1 flex min-h-0">
      <div class="flex-1 overflow-y-auto p-6 min-w-0">
        <details class="mb-5">
          <summary
            class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
            >{$t("adminBattlesHintOpen")}</summary
          >
          <p class="max-w-[62ch] mt-2 text-xs leading-relaxed text-[#5f4636]">
            {$t("adminBattlesKeywordsHint")}
          </p>
        </details>

        {#if !keywords.length}
          <p class="mb-5 text-xs italic text-[#5f4636]">
            {$t("adminBattlesKeywordsEmpty")}
          </p>
        {:else}
          <ul class="max-w-2xl mb-6 border-t border-[#34251c]/10">
            {#each keywords as keyword (keyword.id)}
              <li
                class="flex items-center gap-3 py-2 border-b border-[#34251c]/10"
              >
                <button
                  onclick={() => openKeyword(keyword)}
                  class="flex-1 text-left hover:text-[#c65f3c] {keywordDraftId ===
                  keyword.id
                    ? 'text-[#c65f3c]'
                    : ''}"
                >
                  <span
                    class="text-sm"
                    style="font-family: 'Cormorant Garamond', Georgia, serif;"
                    >{keyword.nameRu}</span
                  >
                  <span class="ml-2 text-[11px] text-[#8a6a55]"
                    >{keyword.nameEn}</span
                  >
                  {#if keyword.rulesRu}
                    <span class="block text-[11px] leading-snug text-[#8a6a55]"
                      >{keyword.rulesRu}</span
                    >
                  {/if}
                </button>
                <span
                  class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] tabular-nums"
                >
                  {keyword.pointValue == null
                    ? "—"
                    : keyword.pointValue.toFixed(2)}
                </span>
                <button
                  onclick={() => removeKeyword(keyword)}
                  class="px-2 py-1 text-xs border border-[#34251c]/20 text-[#6f3b24] hover:bg-[#c65f3c]/10"
                  >×</button
                >
              </li>
            {/each}
          </ul>
        {/if}

        <div class="max-w-2xl p-4 border border-[#34251c]/12">
          <p class="mb-3 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">
            {keywordDraftId
              ? $t("adminBattlesKeywordName")
              : $t("adminBattlesKeywordAdd")}
          </p>
          <div class="flex flex-wrap gap-3">
            <label class="block w-52">
              <span
                class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                >{$t("adminBattlesKeywordName")} · RU</span
              >
              <input
                bind:value={keywordNameRu}
                maxlength="60"
                class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
              />
            </label>
            <label class="block w-52">
              <span
                class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                >{$t("adminBattlesKeywordName")} · EN</span
              >
              <input
                bind:value={keywordNameEn}
                maxlength="60"
                class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
              />
            </label>
            <label class="block w-40">
              <span
                class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                >{$t("adminBattlesKeywordPoints")}</span
              >
              <input
                type="number"
                min="0"
                max="100"
                step="0.05"
                bind:value={keywordPoints}
                onfocus={selectOnFocus}
                onwheel={blurOnWheel}
                class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
              />
            </label>
          </div>
          <div class="flex flex-wrap gap-3 mt-3">
            <label class="block flex-1 min-w-[14rem]">
              <span
                class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                >{$t("adminBattlesKeywordRules")} · RU</span
              >
              <input
                bind:value={keywordRulesRu}
                maxlength="300"
                class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
              />
            </label>
            <label class="block flex-1 min-w-[14rem]">
              <span
                class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                >{$t("adminBattlesKeywordRules")} · EN</span
              >
              <input
                bind:value={keywordRulesEn}
                maxlength="300"
                class="w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
              />
            </label>
          </div>
          <div class="flex items-center gap-3 mt-4">
            <button
              onclick={saveKeyword}
              disabled={saving}
              class="px-4 py-2 text-[10px] uppercase tracking-[0.16em] bg-[#34251c] text-[#f8f1e7] disabled:opacity-40"
              >{$t("adminBattlesSave")}</button
            >
            {#if keywordDraftId}
              <button
                onclick={() => openKeyword(null)}
                class="px-4 py-2 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20"
                >{$t("adminBattlesKeywordAdd")}</button
              >
            {/if}
          </div>
        </div>
      </div>
    </div>
  {:else if view === "errands"}
    <!-- Стол поручений живёт отдельным файлом: эта панель и без него на пять
         тысяч строк, а справочник поручений ничего из неё не переиспользует. -->
    <BattleErrandsPanel />
  {:else if view === "assets"}
    <!-- ── Склад деталей рамки ──────────────────────────────────────────── -->
    <BattleAssetsPanel {flash} />
  {:else if view === "motions"}
    <BattleMotionsPanel {flash} onSaved={(list) => (motions = list)} />
  {:else if view === "races"}
    <!-- ── Лист расы ─────────────────────────────────────────────────────
         Тот же лист, что у карты, и это не подражание ради складности. Раса и
         карта правятся одним и тем же жестом — имя, наряд, движения, — и пока
         одно лежало полкой с панелями, а другое списком с формой под ним,
         хранителю приходилось помнить, в какой из двух комнат он стоит.

         Полка слева, лист посередине, образец справа: раса судится на карте,
         а не в отрыве, — значок величиной с ноготь не говорит ничего о том,
         как он читается в шапке. -->
    <div class="flex-1 flex min-h-0">
      <aside class="w-64 flex flex-col border-r border-[#34251c]/10">
        <div class="p-3 border-b border-[#34251c]/10">
          <button
            onclick={() => openRace(null)}
            class="w-full px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5"
            >{$t("adminBattlesRaceNew")}</button
          >
        </div>
        <div class="flex-1 overflow-y-auto">
          {#if !races.length}
            <p class="p-3 text-xs italic text-[#5f4636]">
              {$t("adminBattlesRacesEmpty")}
            </p>
          {:else}
            <ul class="pb-4">
              {#each races as race (race.id)}
                <li
                  class="relative group border-b border-[#34251c]/5 {raceDraftId ===
                  race.id
                    ? 'bg-[#34251c]/[0.06]'
                    : ''}"
                >
                  <button
                    onclick={() => openRace(race)}
                    class="w-full text-left px-3 py-2.5 pr-8 hover:bg-[#34251c]/[0.04]"
                  >
                    <span
                      class="block text-[13px] leading-snug truncate {raceDraftId ===
                      race.id
                        ? 'text-[#c65f3c]'
                        : ''}"
                      style="font-family: 'Cormorant Garamond', Georgia, serif;"
                      >{race.nameRu}</span
                    >
                    <span class="block text-[10px] text-[#8a6a55] truncate">
                      {race.nameEn} · {cardsOf(race.cardCount)}
                    </span>
                  </button>
                  <!-- Убрать расу — у самой расы, а не общей кнопкой внизу:
                       общая кнопка убирает ТУ, что открыта, и промахнуться ею
                       можно ровно один раз. -->
                  <button
                    onclick={() => removeRace(race)}
                    title={$t("adminBattlesDelete")}
                    class="absolute top-1/2 right-1 -translate-y-1/2 w-6 h-6 flex items-center justify-center text-xs leading-none text-[#8f2f22] opacity-0 group-hover:opacity-100 focus:opacity-100 hover:bg-[#c65f3c]/12"
                    >×</button
                  >
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      </aside>

      <div class="flex-1 flex flex-col min-h-0 overflow-y-auto @container">
        <div class="flex-1 w-full max-w-[1320px] mx-auto px-8 pt-7 pb-6">
          <SheetHead
            title={raceDraftId
              ? raceNameRu || raceNameEn || $t("adminBattlesRaceNew")
              : $t("adminBattlesRaceNew")}
            tag={raceDraftId
              ? cardsOf(races.find((r) => r.id === raceDraftId)?.cardCount ?? 0)
              : undefined}
            lead={$t("adminBattlesSheetRaceLead")}
          />

          <div class="flex flex-col-reverse gap-7 @4xl:flex-row">
            <div
              class="flex-1 min-w-0 grid grid-cols-1 items-start gap-x-6 gap-y-7 @6xl:grid-cols-2"
            >
              <SheetPanel
                wide
                title={$t("adminBattlesSheetRaceNames")}
                lead={$t("adminBattlesSheetRaceNamesLead")}
                note={$t("adminBattlesRacesHint")}
              >
                <!-- Оба языка разом, а не через переключатель, как на карте:
                     имён у расы всего два, и держать их рядом дешевле, чем
                     заводить ради них переключатель, которого здесь нет. -->
                <div class="grid grid-cols-2 gap-3">
                  <SheetField label={`${$t("adminBattlesRaceName")} · RU`}>
                    <input bind:value={raceNameRu} maxlength="60" />
                  </SheetField>
                  <SheetField label={`${$t("adminBattlesRaceName")} · EN`}>
                    <input bind:value={raceNameEn} maxlength="60" />
                  </SheetField>
                  <SheetField label={`${$t("adminBattlesRaceNote")} · RU`}>
                    <input bind:value={raceNoteRu} maxlength="200" />
                  </SheetField>
                  <SheetField label={`${$t("adminBattlesRaceNote")} · EN`}>
                    <input bind:value={raceNoteEn} maxlength="200" />
                  </SheetField>
                </div>
              </SheetPanel>

              <!-- Наряд по уровням. Пять плиток — тот же гардероб, что у карты:
                   лицо рамы величиной с раму, а не квадратик в сорок пикселей,
                   по которому уголок от притолоки не отличить. -->
              <SheetPanel
                wide
                title={$t("adminBattlesRaceLevelFrames")}
                note={$t("adminBattlesRaceLevelFramesHint")}
              >
                <div class="flex flex-wrap gap-2.5">
                  {#each [1, 2, 3, 4, 5] as lvl (lvl)}
                    <button
                      onclick={() => (raceLevelPreview = lvl)}
                      class="w-16 flex flex-col gap-1 text-left"
                    >
                      <!-- Пустая ступень — пунктиром, как «не выбрано» в
                           гардеробе: сплошная рамка вокруг пустоты читается
                           рамой без картинки, а это разные вещи. -->
                      <span
                        class="w-full border bg-center bg-contain bg-no-repeat {raceLevelFrames[
                          lvl - 1
                        ]?.frameImage
                          ? 'border-solid'
                          : 'border-dashed'} {raceLevelPreview === lvl
                          ? 'border-[#c65f3c] shadow-[0_0_0_2px_#c65f3c]'
                          : 'border-[#34251c]/25'}"
                        style="aspect-ratio: 5 / 7; {raceLevelFrames[lvl - 1]
                          ?.frameImage
                          ? `background-image:url('${raceLevelFrames[lvl - 1]?.frameImage}')`
                          : ''}"
                      ></span>
                      <!-- Только число: слово «уровень» под каждой из пяти
                           плиток переносилось на вторую строку и повторяло
                           название панели пять раз подряд. -->
                      <span
                        class="text-[11px] tabular-nums {raceLevelPreview === lvl
                          ? 'text-[#c65f3c]'
                          : 'text-[#8a6a55]'}">{lvl}</span
                      >
                    </button>
                  {/each}
                </div>

                <div class="flex flex-wrap items-center gap-2 mt-3">
                  <button
                    onclick={() => uploadRaceLevelFrame(raceLevelPreview - 1)}
                    disabled={uploading}
                    class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5 disabled:opacity-40"
                    >{$t("adminBattlesFrameArtUpload")}</button
                  >
                  {#if raceLevelFrames[raceLevelPreview - 1]}
                    <button
                      onclick={() => clearRaceLevelFrame(raceLevelPreview - 1)}
                      class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5"
                      >{$t("adminBattlesFrameArtClear")}</button
                    >
                  {/if}
                </div>

                <!-- Наряд без окна. Картинка приехала, врезки остались от
                     ранга — и содержимое ложится поверх чужой резьбы. Где у
                     ЭТОЙ картинки дыра, знает только тот, кто её рисовал,
                     поэтому стол говорит словами, а не догадывается. -->
                {#if dressWindowMissing(raceLevelFrames[raceLevelPreview - 1])}
                  <p
                    class="mt-3 max-w-[62ch] text-[11px] leading-relaxed italic text-[#8f2f22]"
                  >
                    {$t("adminBattlesDressNoWindow")}
                  </p>
                {/if}

                <!-- Или взять готовое из ящика — целиком, а не одной картинкой:
                     рамка, собранная из частей, иначе на расу бы не переехала. -->
                {#if presets.length}
                  <div class="mt-5 pt-4 border-t border-dashed border-[#34251c]/12">
                    <p
                      class="mb-2 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                    >
                      {$t("adminBattlesPresetChoose")}
                    </p>
                    <BattleFramePicker
                      {presets}
                      bind:chosen={presetChosen}
                      allowNone
                      layout="rack"
                      label={$t("adminBattlesPresetChoose")}
                    />
                    <div class="flex flex-wrap items-center gap-2 mt-3">
                      <button
                        onclick={() => wearPresetOnLevel(raceLevelPreview - 1)}
                        disabled={!presetTaken}
                        class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5 disabled:opacity-40"
                        >{$t("adminBattlesPresetWearLevel")}</button
                      >
                      <button
                        onclick={wearPresetOnAllLevels}
                        disabled={!presetTaken}
                        class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5 disabled:opacity-40"
                        >{$t("adminBattlesPresetWearAll")}</button
                      >
                    </div>
                  </div>
                {/if}
              </SheetPanel>

              <!-- Движения расы. Стоят рядом с нарядом, а не в отдельном месте:
                   и то и другое — то, что раса раздаёт всем своим картам
                   сразу, и надеваются они одним жестом. -->
              <SheetPanel wide title={$t("adminBattlesSheetMotion")}>
                <BattleMotionWear
                  wear={raceMotionWear}
                  {motions}
                  onChange={(raw) => (raceMotionWear = raw)}
                  onOpenBox={() => (view = "motions")}
                />
              </SheetPanel>
            </div>

            <!-- Сложенным столбиком карта не растягивается во всю ширину:
                 она не становится понятнее от того, что стала с ладонь. -->
            <aside
              class="w-full max-w-[360px] flex-shrink-0 @4xl:w-[360px] @4xl:max-w-none"
            >
              <div class="pt-1 @4xl:sticky @4xl:top-0">
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
                <p
                  class="mt-3 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                >
                  {$t("adminBattlesRaceIcon")}
                </p>
                <details class="mt-1">
                  <summary
                    class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
                    >{$t("adminBattlesHintOpen")}</summary
                  >
                  <p
                    class="mt-2 text-[11px] leading-relaxed italic text-[#8a6a55]"
                  >
                    {$t("adminBattlesRaceIconHint")}
                  </p>
                </details>
                {#if raceIconUrl.trim()}
                  <button
                    onclick={() => (raceIconUrl = "")}
                    class="mt-2 px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5"
                    >{$t("adminBattlesFrameArtClear")}</button
                  >
                {/if}
              </div>
            </aside>
          </div>
        </div>

        <div
          class="sticky bottom-0 z-10 border-t border-[#34251c]/12 bg-[#f8f1e7]"
        >
          <div
            class="w-full max-w-[1320px] mx-auto px-8 py-3 flex items-center gap-4"
          >
            <button
              onclick={saveRace}
              disabled={saving}
              class="px-4 py-2 text-[10px] uppercase tracking-[0.16em] bg-[#34251c] text-[#f8f1e7] disabled:opacity-40"
              >{$t("adminBattlesSaveRace")}</button
            >
            {#if raceDraftId}
              <button
                onclick={() => openRace(null)}
                class="px-4 py-2 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20"
                >{$t("adminBattlesRaceNew")}</button
              >
            {:else}
              <span class="text-[11px] italic text-[#8a6a55]"
                >{$t("adminBattlesSheetRaceNone")}</span
              >
            {/if}
          </div>
        </div>
      </div>
    </div>
  {:else}
    <div class="flex-1 flex min-h-0">
      <!-- ── The shelf ──────────────────────────────────────────────────── -->
      <aside class="w-64 flex flex-col border-r border-[#34251c]/10">
        <div class="p-3 space-y-2 border-b border-[#34251c]/10">
          <button
            onclick={blank}
            class="w-full px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5"
            >{$t("adminBattlesNew")}</button
          >
          <input
            bind:value={listQuery}
            placeholder={$t("adminBattlesSearch")}
            class="w-full px-2 py-1.5 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
          />
          <!-- Проверка полки. Карт десятки, и главная беда описи в том, что
               переполнение НЕВИДИМО: не влезшее уезжает в прокрутку, которой
               на карте никто не ищет. Кнопка ставит на стол невидимую стопку
               настоящих карт на двух ширинах и слушает, что они скажут. -->
          <button
            onclick={() => (fitProbe = !fitProbe)}
            disabled={loading || !cards.length}
            class="w-full px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border {fitProbe
              ? 'bg-[#34251c] text-[#f8f1e7] border-[#34251c]'
              : 'border-[#34251c]/20 hover:bg-[#34251c]/5'} disabled:opacity-40"
            >{$t("adminBattlesFitCheck")}</button
          >
          {#if fitProbe}
            {#if fitTrouble.length}
              <p class="text-[10px] uppercase tracking-[0.16em] text-[#8f2f22]">
                {$t("adminBattlesFitTitle")}
              </p>
              <ul class="space-y-0.5">
                {#each fitTrouble as row (row.card.id)}
                  <li>
                    <button
                      onclick={() => openCard(row.card)}
                      class="w-full text-left text-[11px] leading-snug text-[#6f3b24] hover:underline"
                      >{titleOf(row.card)} —
                      {row.where.map((one) => $t(one.label)).join(", ")}</button
                    >
                  </li>
                {/each}
              </ul>
            {:else}
              <p class="text-[11px] italic text-[#8a6a55]">
                {$t("adminBattlesFitNone")}
              </p>
            {/if}
            <details class="mt-2">
              <summary
                class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
                >{$t("adminBattlesHintOpen")}</summary
              >
              <p
                class="mt-2 text-[10px] leading-relaxed italic text-[#8a6a55]"
              >
                {$t("adminBattlesFitHint")}
              </p>
            </details>
          {/if}
        </div>
        <div class="flex-1 overflow-y-auto">
          {#if loading}
            <p class="p-3 text-xs text-[#5f4636]">…</p>
          {:else if visible.length === 0}
            <p class="p-3 text-xs italic text-[#5f4636]">
              {$t("adminBattlesEmpty")}
            </p>
          {:else}
            <p
              class="px-3 py-2 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
            >
              {$t("adminBattlesDragHint")}
            </p>
            <ul class="pb-4">
              {#each visible as card, i (card.id)}
                <li
                  draggable={!listQuery}
                  ondragstart={() => (dragFrom = i)}
                  ondragover={(e) => {
                    e.preventDefault();
                    dragOver = i;
                  }}
                  ondragleave={() => {
                    if (dragOver === i) dragOver = null;
                  }}
                  ondrop={(e) => {
                    e.preventDefault();
                    onDrop(i);
                  }}
                  ondragend={() => {
                    dragFrom = null;
                    dragOver = null;
                  }}
                  class="border-b border-[#34251c]/5 {dragOver === i
                    ? 'bg-[#c65f3c]/10'
                    : ''} {dragFrom === i ? 'opacity-40' : ''}"
                >
                  <button
                    onclick={() => openCard(card)}
                    class="w-full text-left px-3 py-2.5 flex gap-2 items-start hover:bg-[#34251c]/[0.04] {selectedId ===
                    card.id
                      ? 'bg-[#34251c]/[0.06]'
                      : ''}"
                  >
                    <span
                      class="mt-1.5 w-1.5 h-1.5 rounded-full flex-shrink-0 {STATUS_TONE[
                        card.status
                      ]}"
                    ></span>
                    <span class="min-w-0">
                      <span
                        class="block text-[13px] leading-snug truncate"
                        style="font-family: 'Cormorant Garamond', Georgia, serif;"
                      >
                        {titleOf(card)}
                      </span>
                      {#if card.figurineName}
                        <span class="block text-[10px] text-[#8a6a55] truncate"
                          >{card.figurineName}</span
                        >
                      {/if}
                    </span>
                    <span class="ml-auto text-[10px] text-[#8a6a55]"
                      >{card.tier}</span
                    >
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      </aside>

      <!-- Невидимая стопка. Настоящие `BattleCard` на настоящих ширинах:
           замер, считающий переполнение по-своему, — это вторая правда о том,
           что влезает, и однажды она разойдётся с комнатой. Стоит за краем
           окна, а не под `display: none`: у скрытого нет ни ширины, ни
           укладки, и мерить в нём нечего. -->
      {#if fitProbe}
        <div
          aria-hidden="true"
          class="fixed top-0 left-[-99999px] pointer-events-none"
        >
          {#each cards as one (one.id)}
            {#each FIT_WIDTHS as size (size.px)}
              <div style="width:{size.px}px">
                <BattleCard
                  card={one}
                  {frames}
                  owned={true}
                  interactive={false}
                  transition={false}
                  onfit={(over) => noteFit(one.id, size.px, over)}
                />
              </div>
            {/each}
          {/each}
        </div>
      {/if}

      <!-- ── Лист карты ──────────────────────────────────────────────────
           Была карта посередине и колонка в двадцать пять блоков справа: всё,
           что не поместилось на лицо, лежало одной лентой, разделённой
           пунктиром, и найти в ней «дальность» можно было только чтением
           подряд. Лист называет области и даёт им край, поэтому глаз находит
           нужную до того, как разберёт слова.

           Карта переехала вправо и прибита к верху: правят её саму, и она
           должна быть видна с любого места листа, а не только с начала. -->
      <!-- Лист меряет СЕБЯ, а не окно. Перелом сперва стоял на `lg:` — и это
           было неверно с первой минуты: до листа окно доходит уже без боковины
           администратора и без полки карт, четырьмя сотнями пикселей меньше. На
           окне в тысячу «широкая» раскладка включалась, а листу доставалось
           шестьсот, и колонка панелей схлопывалась в щепку рядом с картой в
           треть тысячи. Контейнерный запрос спрашивает ровно то, что нужно
           знать, — сколько места у листа, — и так же меряет себя сама карта. -->
      <div class="flex-1 flex flex-col min-h-0 overflow-y-auto @container">
        <div class="flex-1 w-full max-w-[1320px] mx-auto px-8 pt-7 pb-6">
          <SheetHead
            title={titleOf(draft) || $t("adminBattlesSheetUntitled")}
            tag={`${$t("adminBattlesTier")} ${draft.tier}`}
            lead={$t("adminBattlesSheetLead")}
          >
            <div
              class="flex border border-[#34251c]/15 text-[10px] uppercase tracking-[0.16em]"
            >
              <button
                onclick={() => (editLang = "ru")}
                class="px-2.5 py-1 {editLang === 'ru'
                  ? 'bg-[#34251c] text-[#f8f1e7]'
                  : ''}"
                title={$t("adminBattlesEditLang")}>RU</button
              >
              <button
                onclick={() => (editLang = "en")}
                class="px-2.5 py-1 {editLang === 'en'
                  ? 'bg-[#34251c] text-[#f8f1e7]'
                  : ''}"
                title={$t("adminBattlesEditLang")}>EN</button
              >
            </div>
            <label
              class="flex items-center gap-2 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]"
            >
              <input type="checkbox" bind:checked={facedown} />
              {$t("adminBattlesPreviewDown")}
            </label>
          </SheetHead>

          <!-- ── Панели по важности ──────────────────────────────────────
               Порядок не по устройству карты, а по тому, в каком порядке её
               заводят. Сперва чьё лицо она носит — карта заводится ОТ работы,
               и «Работа» стояла седьмой сверху только потому, что в прежней
               ленте блоки лежали в том порядке, в каком их дописывали. Дальше
               то, что на карте написано, чем она одета, и лишь затем числа,
               которыми она играет. Учёт — последним, у самой кнопки: статус
               решают, когда решено всё остальное. -->
          <!-- Узкое окно ломает лист двумя способами сразу, и оба надо
               закрыть: две колонки панелей на восьмистах пикселях — это две
               узкие колонки ни для чего, а прибитая карта, встав НАД панелями
               вместо того, чтобы стоять сбоку, снова начнёт их накрывать.
               Поэтому до xl панели идут одной колонкой, а до lg карта
               поднимается наверх и перестаёт быть прибитой. -->
          <div class="flex flex-col-reverse gap-7 @4xl:flex-row">
            <div
              class="flex-1 min-w-0 grid grid-cols-1 items-start gap-x-6 gap-y-7 @6xl:grid-cols-2"
            >
              <SheetPanel
                wide
                title={$t("adminBattlesSheetWork")}
                lead={$t("adminBattlesSheetWorkLead")}
              >
                <!-- Без подписи: панель уже названа «Работой», и второе то же
                     слово строчкой ниже сообщает только о том, что его забыли
                     убрать. -->
                <SheetField>
                  <input
                    bind:value={workQuery}
                    placeholder={$t("adminBattlesSearch")}
                    class="mb-1.5"
                  />
                  <select
                    value={draft.figurineId ?? ""}
                    onchange={(e) => {
                      const id = e.currentTarget.value || null;
                      draft.figurineId = id;
                      // The work's catalogue name is the link under the card,
                      // never the card's own title. Filling titleEn from it is
                      // how "Small doll Baba Yaga" sat where the card's name
                      // should have been.
                    }}
                  >
                    <option value="">{$t("adminBattlesWorkNone")}</option>
                    {#each visibleFigurines as fig (fig.id)}
                      <option value={fig.id}>{fig.name}</option>
                    {/each}
                  </select>
                </SheetField>

                {#if workTaken}
                  <p class="mt-2 text-[11px] leading-relaxed text-[#c65f3c]">
                    {$t("adminBattlesWorkTaken")}
                  </p>
                {/if}

                {#if draft.artUrlOverride}
                  <p class="mt-2 text-[11px] leading-relaxed italic text-[#8a6a55]">
                    {$t("adminBattlesArtOwn")}
                    <button
                      onclick={() => (draft.artUrlOverride = null)}
                      class="ml-1 not-italic underline decoration-dotted hover:text-[#c65f3c]"
                      >{$t("adminBattlesArtClear")}</button
                    >
                  </p>
                {:else}
                  <p class="mt-2 text-[11px] leading-relaxed italic text-[#8a6a55]">
                    {$t("adminBattlesArtFromWork")}
                  </p>
                {/if}

                <div class="mt-4 pt-3 border-t border-dashed border-[#34251c]/12">
                  <p class="mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">
                    {$t("adminBattlesAim")}
                  </p>
                  <details class="mb-3">
                    <summary
                      class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
                      >{$t("adminBattlesHintOpen")}</summary
                    >
                    <p
                      class="mt-2 text-[11px] leading-relaxed italic text-[#8a6a55]"
                    >
                      {$t("adminBattlesAimHint")}
                    </p>
                  </details>
                  <div class="grid grid-cols-3 gap-3">
                    <label class="block">
                      <span
                        class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                        >{$t("adminBattlesFocusX")} · {(focal.x * 100).toFixed(
                          0,
                        )}%</span
                      >
                      <input
                        type="range"
                        min="0"
                        max="1"
                        step="0.01"
                        value={focal.x}
                        oninput={(e) =>
                          setFocal({ x: Number(e.currentTarget.value) })}
                        class="w-full accent-[#c65f3c]"
                      />
                    </label>
                    <label class="block">
                      <span
                        class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                        >{$t("adminBattlesFocusY")} · {(focal.y * 100).toFixed(
                          0,
                        )}%</span
                      >
                      <input
                        type="range"
                        min="0"
                        max="1"
                        step="0.01"
                        value={focal.y}
                        oninput={(e) =>
                          setFocal({ y: Number(e.currentTarget.value) })}
                        class="w-full accent-[#c65f3c]"
                      />
                    </label>
                    <label class="block">
                      <span
                        class="block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                        >{$t("adminBattlesZoom")} · {focal.zoom.toFixed(2)}</span
                      >
                      <input
                        type="range"
                        min="1"
                        max="3"
                        step="0.05"
                        value={focal.zoom}
                        oninput={(e) =>
                          setFocal({ zoom: Number(e.currentTarget.value) })}
                        class="w-full accent-[#c65f3c]"
                      />
                    </label>
                  </div>
                  <button
                    onclick={() => setFocal({ x: 0.5, y: 0.5, zoom: 1 })}
                    class="mt-3 px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5"
                    >{$t("adminBattlesAimReset")}</button
                  >
                </div>
              </SheetPanel>

              <!-- ── What the card says: typed here at a normal size, read live
                   on the card beside it — same `draft`, no second copy. ─────── -->
              <SheetPanel
                wide
                title={$t("adminBattlesSheetPrint")}
                lead={$t("adminBattlesSheetPrintLead")}
                fault={faults.has("noTitle") || faults.has("noEffect")}
              >
                <div class="grid grid-cols-2 gap-3">
                  <SheetField
                    label={$t("adminBattlesTitle")}
                    wide
                    anchor="fault-title"
                    fault={faults.has("noTitle")}
                    faultNote={blankSide("title")
                      ? $t("adminBattlesFaultOtherSide")
                      : $t("adminBattlesFaultHere")}
                  >
                    <input
                      maxlength="80"
                      value={editLang === "en" ? draft.titleEn : draft.titleRu}
                      oninput={(e) => {
                        if (editLang === "en")
                          draft.titleEn = e.currentTarget.value;
                        else draft.titleRu = e.currentTarget.value;
                      }}
                    />
                  </SheetField>
                  <SheetField label={$t("adminBattlesRace")}>
                    <select
                      value={draft.raceId ?? ""}
                      onchange={(e) => selectRace(e.currentTarget.value)}
                    >
                      <option value="">{$t("adminBattlesRaceNone")}</option>
                      {#each races as race (race.id)}
                        <option value={race.id}
                          >{editLang === "en"
                            ? race.nameEn
                            : race.nameRu}</option
                        >
                      {/each}
                    </select>
                  </SheetField>
                  <SheetField label={$t("battlesTypeLabel")}>
                    <input
                      maxlength="40"
                      value={editLang === "en"
                        ? (draft.typeEn ?? "")
                        : (draft.typeRu ?? "")}
                      oninput={(e) => {
                        if (editLang === "en")
                          draft.typeEn = e.currentTarget.value;
                        else draft.typeRu = e.currentTarget.value;
                      }}
                    />
                  </SheetField>
                  <SheetField
                    label={$t("adminBattlesEffect")}
                    wide
                    anchor="fault-effect"
                    fault={faults.has("noEffect")}
                    faultNote={blankSide("effect")
                      ? $t("adminBattlesFaultOtherSide")
                      : $t("adminBattlesFaultHere")}
                  >
                    <textarea
                      maxlength="400"
                      rows="3"
                      value={editLang === "en"
                        ? (draft.effectEn ?? "")
                        : (draft.effectRu ?? "")}
                      oninput={(e) => {
                        if (editLang === "en")
                          draft.effectEn = e.currentTarget.value || null;
                        else draft.effectRu = e.currentTarget.value || null;
                      }}
                    ></textarea>
                  </SheetField>
                  <!-- Приписка. Карта её печатает, сервер принимает — а поля
                       ввода не было нигде, и задать её можно было только
                       запросом к API. -->
                  <SheetField label={$t("adminBattlesLore")} wide>
                    <textarea
                      maxlength="400"
                      rows="2"
                      value={editLang === "en"
                        ? (draft.loreEn ?? "")
                        : (draft.loreRu ?? "")}
                      oninput={(e) => {
                        if (editLang === "en")
                          draft.loreEn = e.currentTarget.value || null;
                        else draft.loreRu = e.currentTarget.value || null;
                      }}
                    ></textarea>
                  </SheetField>
                </div>
              </SheetPanel>

              <SheetPanel
                title={$t("adminBattlesTraits")}
                fault={faults.has("traitsWithoutAbilities")}
              >
                {#if !(draft.traits ?? []).length}
                  <p class="mb-2 text-[11px] italic text-[#8a6a55]">
                    {$t("adminBattlesTraitsEmpty")}
                  </p>
                {/if}
                <div class="space-y-2">
                  {#each draft.traits ?? [] as trait, i (i)}
                    <div
                      class="flex items-start gap-1.5 p-2 border border-[#34251c]/10"
                    >
                      <div class="flex-1 min-w-0 space-y-1.5">
                        <input
                          maxlength="60"
                          placeholder={$t("adminBattlesTraitName")}
                          value={editLang === "en" ? trait.nameEn : trait.nameRu}
                          oninput={(e) => {
                            if (editLang === "en")
                              trait.nameEn = e.currentTarget.value;
                            else trait.nameRu = e.currentTarget.value;
                          }}
                          class="w-full px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                        />
                        <input
                          maxlength="200"
                          placeholder={$t("adminBattlesTraitText")}
                          value={editLang === "en" ? trait.textEn : trait.textRu}
                          oninput={(e) => {
                            if (editLang === "en")
                              trait.textEn = e.currentTarget.value;
                            else trait.textRu = e.currentTarget.value;
                          }}
                          class="w-full px-2 py-1 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                        />
                      </div>
                      <div class="flex flex-col gap-0.5 flex-shrink-0">
                        <button
                          type="button"
                          onclick={() => moveTrait(i, -1)}
                          disabled={i === 0}
                          class="px-1.5 text-xs border border-[#34251c]/20 disabled:opacity-30"
                          >↑</button
                        >
                        <button
                          type="button"
                          onclick={() => moveTrait(i, 1)}
                          disabled={i === (draft.traits?.length ?? 0) - 1}
                          class="px-1.5 text-xs border border-[#34251c]/20 disabled:opacity-30"
                          >↓</button
                        >
                        <button
                          type="button"
                          onclick={() => removeTrait(i)}
                          class="px-1.5 text-xs border border-[#34251c]/20 hover:bg-[#c65f3c]/10"
                          >×</button
                        >
                      </div>
                    </div>
                  {/each}
                </div>
                <button
                  type="button"
                  onclick={addTrait}
                  disabled={(draft.traits?.length ?? 0) >= TRAITS_MAX}
                  class="mt-2 px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5 disabled:opacity-40"
                  >+ {$t("adminBattlesTraitAdd")}</button
                >
              </SheetPanel>

              <!-- Рамка только для этой карты. На лице карты есть «своя
                   картинка» — она грузит одну фотографию; здесь берут готовый
                   наряд из ящика целиком, вместе с отступами и полосами, иначе
                   собранная из частей рамка на одну карту бы не переехала. -->
              {#if presets.length}
                <SheetPanel
                  title={$t("adminBattlesSheetDress")}
                  lead={$t("adminBattlesSheetDressLead")}
                  note={$t("adminBattlesPresetCardHint")}
                >
                  <BattleFramePicker
                    {presets}
                    bind:chosen={presetChosen}
                    allowNone
                    layout="rack"
                    label={$t("adminBattlesPresetChoose")}
                  />
                  <div class="flex flex-wrap items-center gap-2 mt-2">
                    <button
                      onclick={wearPresetOnCard}
                      disabled={!presetTaken}
                      class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5 disabled:opacity-40"
                      >{$t("adminBattlesPresetWearCard")}</button
                    >
                    {#if draft.frameOverride}
                      <button
                        onclick={() => (draft.frameOverride = null)}
                        class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5"
                        >{$t("adminBattlesFrameResetCard")}</button
                      >
                    {/if}
                  </div>
                </SheetPanel>
              {/if}

              <!-- Медальоны, а не два выпадающих списка. Видов три, каналов
                   четыре, и оба перечня закрыты: список прячет весь выбор под
                   одно слово ровно там, где выбор можно показать целиком. -->
              <SheetPanel
                wide
                title={$t("adminBattlesSheetBlow")}
                lead={$t("adminBattlesSheetBlowLead")}
              >
                <p class="mb-2 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">
                  {$t("adminBattlesKind")}
                </p>
                <div class="flex flex-wrap gap-x-6 gap-y-3 mb-5">
                  <Medallion
                    icon="unit"
                    caption={$t("adminBattlesKindUnit")}
                    selected={draft.kind === "unit"}
                    onclick={() => (draft.kind = "unit")}
                  />
                  <!-- Помечены, а не убраны: вид карты хранится и ждёт движка,
                       но обещать, что он играет, форма не должна. Опубликовать
                       их и сейчас нельзя — обе требуют здоровья больше нуля, а
                       со здоровьем становятся обычным телом на клетке. -->
                  <Medallion
                    icon="spark"
                    caption={$t("adminBattlesKindSpell")}
                    note={$t("adminBattlesKindDead")}
                    selected={draft.kind === "spell"}
                    onclick={() => (draft.kind = "spell")}
                  />
                  <Medallion
                    icon="relic"
                    caption={$t("adminBattlesKindRelic")}
                    note={$t("adminBattlesKindDead")}
                    selected={draft.kind === "relic"}
                    onclick={() => (draft.kind = "relic")}
                  />
                </div>

                <p class="mb-2 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">
                  {$t("adminBattlesChannel")}
                </p>
                <div class="flex flex-wrap gap-x-6 gap-y-3">
                  {#each CHANNELS as channel (channel)}
                    <Medallion
                      icon={CHANNEL_ICON[channel]}
                      caption={$t(CHANNEL_LABELS[channel])}
                      selected={draft.attackChannel === channel}
                      onclick={() => (draft.attackChannel = channel)}
                    />
                  {/each}
                </div>
              </SheetPanel>

              <!--
                Тело для движка. Отделено от прозы выше не рамкой ради красоты:
                проза печатается на карте и читается человеком, эти числа читает
                только движок, и путать их — та самая ошибка, из-за которой
                правила пытаются разобрать естественный язык.

                Плашка стоит наверху панели, а не внизу листа, как на образце:
                внизу она была бы оторвана от бюджета, который эти же числа и
                судит, — а смотрят на них всегда вместе.
              -->
              <SheetPanel
                wide
                anchor="fault-body"
                title={$t("adminBattlesBody")}
                lead={$t("adminBattlesSheetBodyLead")}
                note={$t("adminBattlesBodyHint")}
                fault={faults.has("noHealth") ||
                  faults.has("costBeyondMana") ||
                  faults.has("overTierBudget")}
              >
                {#snippet aside()}
                  <!-- Живое число, а не сохранённое. Прежде здесь стояло то,
                       что вернул сервер при ПОСЛЕДНЕМ сохранении, а строчкой
                       ниже — живое: два вердикта одним словом из разных
                       моментов времени, и у новой карты первое молчало
                       навсегда. -->
                  {#if weigh}
                    <span
                      class="text-[10px] tabular-nums"
                      style="color: {verdictColour(weigh.balanceIndex)}"
                    >
                      {weigh.totalPoints.toFixed(1)} · {verdictWord(
                        weigh.balanceIndex,
                      )}
                    </span>
                  {:else}
                    <span class="text-[10px] text-[#8a6a55]"
                      >{$t("adminBattlesScalesPending")}</span
                    >
                  {/if}
                {/snippet}

                <!-- 7.4rem — не круглое число ради красоты: с ним девять
                     ячеек ложатся пятью и четырьмя, а не семью и двумя, где
                     две последние растягивались каждая на треть плашки. -->
                <StatPlate min="7.4rem">
                  <StatCell
                    icon={statMark("health")}
                    label={$t("battlesHealthLabel")}
                    anchor="fault-health"
                    fault={faults.has("noHealth")}
                    min={0}
                    max={99}
                    bind:value={draft.health}
                  />
                  <StatCell
                    icon={statMark("power")}
                    label={$t("battlesPowerLabel")}
                    min={0}
                    max={99}
                    bind:value={draft.power}
                  />
                  <StatCell
                    icon={statMark("cost")}
                    label={$t("battlesCostLabel")}
                    anchor="fault-cost"
                    fault={faults.has("costBeyondMana")}
                    min={0}
                    max={20}
                    bind:value={draft.cost}
                  />
                  <!-- Мана карты печатается на лицевой стороне и не играет.
                       Приглушена на плашке, а не убрана: число, которое ни на
                       что не влияет, должно признаваться в этом там, где его
                       набирают. -->
                  <StatCell
                    icon={statMark("mana")}
                    label={$t("battlesManaLabel")}
                    tone="quiet"
                    min={0}
                    max={99}
                    bind:value={draft.mana}
                  />
                  {#each bodyStats as stat (stat.key)}
                    <StatCell
                      icon={statMark(stat.key)}
                      label={$t(stat.label)}
                      min={stat.min}
                      max={stat.max}
                      bind:value={draft[stat.key]}
                    />
                  {/each}
                </StatPlate>
                <details class="mt-1.5">
                  <summary
                    class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
                    >{$t("adminBattlesHintOpen")}</summary
                  >
                  <p
                    class="mt-2 text-[10px] leading-snug italic text-[#8a6a55]"
                  >
                    {$t("adminBattlesManaHint")}
                  </p>
                </details>

                <!-- Бюджет чина. Забор, а не весы: сумма очков крупно права
                     («не больше двадцати на третий чин»), а тонко — нет, и
                     вопрос «сыграет ли это» она не решает. Поэтому полоска
                     показывает только, не вылезла ли карта за свой чин. -->
                <div class="mt-4">
                  <div
                    class="flex items-baseline justify-between text-[10px] uppercase tracking-[0.14em]"
                  >
                    <span class="text-[#8a6a55]">{$t("adminBattlesBudget")}</span
                    >
                    <span
                      class="tabular-nums"
                      style="color: {overBudget ? '#8f2f22' : '#5f4636'}"
                    >
                      {weigh ? weigh.totalPoints.toFixed(1) : "—"} / {tierBudget(
                        draft.tier,
                      )}
                    </span>
                  </div>
                  <div class="mt-1 h-1.5 bg-[#34251c]/10">
                    <div
                      class="h-full"
                      style="width: {budgetFill}%; background: {overBudget
                        ? '#8f2f22'
                        : '#4a6141'}"
                    ></div>
                  </div>
                </div>
              </SheetPanel>

              <!--
                ── Способности для движка ───────────────────────────────
                Числа справа считает сервер той же формулой, что и при
                сохранении: браузер не знает ни одного курса.
              -->
              <SheetPanel
                wide
                anchor="fault-abilities"
                title={$t("adminBattlesAbilities")}
                note={$t("adminBattlesAbilitiesHint")}
                fault={faults.has("traitsWithoutAbilities")}
              >
                {#snippet aside()}
                  {#if weigh}
                    <span
                      class="text-[10px] tabular-nums"
                      style="color: {verdictColour(weigh.balanceIndex)}"
                    >
                      {$t("adminBattlesWeighTotal")}
                      {weigh.totalPoints.toFixed(1)}
                      <span class="text-[#8a6a55]"
                        >({$t("adminBattlesWeighBody")}
                        {weigh.bodyPoints.toFixed(1)})</span
                      >
                      · {verdictWord(weigh.balanceIndex)}
                      {#if weigh.suggestedCost !== draft.cost}
                        <span class="text-[#8a6a55]"
                          >· {$t("adminBattlesWeighSuggested")}
                          {weigh.suggestedCost}</span
                        >
                      {/if}
                    </span>
                  {/if}
                {/snippet}

                <!-- Лента умений: по медальону на каждое, лицом глагола и своим
                     именем. Настройки — ТОЛЬКО того, что в руке.

                     Панель разворачивала все поля всех умений разом: четыре
                     умения это стена в пол-экрана из одинаковых выпадающих
                     списков, и найти в ней нужное можно было только чтением
                     подряд. Дом этот приём уже знает — стол рам показывает
                     список деталей и настройки одной, взятой; здесь тот же
                     жест и та же мебель. -->
                <div class="flex flex-wrap items-start gap-x-5 gap-y-4">
                  {#each draft.abilities ?? [] as ability (ability.id)}
                    <Medallion
                      icon={VERB_ICON[ability.verb]}
                      caption={abilityName(ability)}
                      note={abilityPoints(ability.id) != null
                        ? `${abilityPoints(ability.id)?.toFixed(1)} ${$t("adminBattlesAbilityPoints")}`
                        : undefined}
                      selected={ability.id === abilityInHand?.id}
                      onclick={() => (abilityHeld = ability.id)}
                    />
                  {/each}
                  {#if (draft.abilities?.length ?? 0) < ABILITIES_MAX}
                    <Medallion
                      icon="plus"
                      hollow
                      caption={$t("adminBattlesAbilityAdd")}
                      onclick={addAbility}
                    />
                  {/if}
                </div>

                {#if !(draft.abilities ?? []).length}
                  <p class="mt-4 text-[11px] leading-relaxed italic text-[#8a6a55]">
                    {$t("adminBattlesAbilitiesEmpty")}
                  </p>
                {:else if abilityInHand}
                  {@const i = abilityAt}
                  {@const ability = abilityInHand}
                  <div
                    class="mt-5 pt-4 border-t border-dashed border-[#34251c]/15"
                  >
                    <!-- Чьи это настройки — сказано вслух. Без подписи полоса
                         полей под лентой читается настройками ленты целиком. -->
                    <div class="flex items-baseline gap-2 mb-3">
                      <span
                        class="text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                        >{$t("adminBattlesAbilityChosen")}</span
                      >
                      <span class="text-xs text-[#6f3b24]"
                        >{abilityName(ability)}</span
                      >
                      <span class="ml-auto flex items-center gap-1">
                        <button
                          type="button"
                          onclick={() => moveAbility(i, -1)}
                          disabled={i === 0}
                          title={$t("adminBattlesAbilityRaise")}
                          class="w-6 h-6 flex items-center justify-center border border-[#34251c]/20 text-[#5f4636] disabled:opacity-30 hover:bg-[#34251c]/5"
                          ><BattleIcon name="up" /></button
                        >
                        <button
                          type="button"
                          onclick={() => moveAbility(i, 1)}
                          disabled={i === (draft.abilities?.length ?? 0) - 1}
                          title={$t("adminBattlesAbilityLower")}
                          class="w-6 h-6 flex items-center justify-center border border-[#34251c]/20 text-[#5f4636] disabled:opacity-30 hover:bg-[#34251c]/5"
                          ><BattleIcon name="down" /></button
                        >
                        <button
                          type="button"
                          onclick={() => removeAbility(i)}
                          title={$t("adminBattlesDelete")}
                          class="w-6 h-6 flex items-center justify-center border border-[#34251c]/20 text-[#8f2f22] hover:bg-[#c65f3c]/10"
                          ><BattleIcon name="trash" /></button
                        >
                      </span>
                    </div>

                    <div class="grid grid-cols-2 gap-x-4 gap-y-3">
                      <SheetField label={$t("adminBattlesAbilityVerb")}>
                        <span class="flex items-center gap-1.5">
                          <span class="flex-shrink-0 text-[#6f3b24]"
                            ><BattleIcon
                              name={VERB_ICON[ability.verb]}
                              size={15}
                            /></span
                          >
                          <select bind:value={draft.abilities[i].verb}>
                            {#each VERBS as verb (verb)}
                              <option value={verb}
                                >{$t(VERB_LABELS[verb])}</option
                              >
                            {/each}
                          </select>
                        </span>
                      </SheetField>

                      <!-- Значок стоит У списка, а не вместо него: форм восемь
                           и поводов восемь, медальонами это два ряда кружков на
                           каждое умение. Слово в списке уже написано. -->
                      <SheetField label={$t("adminBattlesAbilityShape")}>
                        <span class="flex items-center gap-1.5">
                          <span class="flex-shrink-0 text-[#6f3b24]"
                            ><BattleIcon name={ability.shape} size={15} /></span
                          >
                          <select bind:value={draft.abilities[i].shape}>
                            {#each SHAPES as shape (shape)}
                              <option value={shape}
                                >{$t(SHAPE_LABELS[shape])}</option
                              >
                            {/each}
                          </select>
                        </span>
                      </SheetField>

                      <SheetField label={$t("adminBattlesAbilityTrigger")}>
                        <span class="flex items-center gap-1.5">
                          <span class="flex-shrink-0 text-[#6f3b24]"
                            ><BattleIcon
                              name={ability.trigger}
                              size={15}
                            /></span
                          >
                          <select bind:value={draft.abilities[i].trigger}>
                            {#each TRIGGERS as trigger (trigger)}
                              <option value={trigger}
                                >{$t(TRIGGER_LABELS[trigger])}</option
                              >
                            {/each}
                          </select>
                        </span>
                      </SheetField>

                      <SheetField label={$t("adminBattlesAbilityKeywords")}>
                        <input
                          value={(ability.keywords ?? []).join(", ")}
                          oninput={(e) =>
                            keywordsInput(i, e.currentTarget.value)}
                        />
                      </SheetField>

                      <!-- Числа умения. Той же плашкой, что тело карты: это
                           тоже числа, которые читает только движок, и второй
                           вид для них означал бы, что они другого рода. -->
                      <div class="col-span-2">
                        <StatPlate min="6rem">
                          <StatCell
                            icon="sword"
                            label={$t("adminBattlesAbilityAmount")}
                            min={0}
                            max={99}
                            bind:value={draft.abilities[i].amount}
                          />
                          <StatCell
                            icon={ability.shape}
                            label={$t("adminBattlesAbilityRadius")}
                            tone={shapeCarriesNumber(ability.shape)
                              ? "plain"
                              : "quiet"}
                            readonly={!shapeCarriesNumber(ability.shape)}
                            min={0}
                            max={3}
                            bind:value={draft.abilities[i].radius}
                          />
                          <StatCell
                            icon="reach"
                            label={$t("adminBattlesAbilityRange")}
                            min={0}
                            max={5}
                            bind:value={draft.abilities[i].range}
                          />
                          <StatCell
                            icon="turnStart"
                            label={$t("adminBattlesAbilityDuration")}
                            min={0}
                            max={5}
                            bind:value={draft.abilities[i].duration}
                          />
                          <StatCell
                            icon="drop"
                            label={$t("adminBattlesAbilityMana")}
                            min={0}
                            max={20}
                            bind:value={draft.abilities[i].manaCost}
                          />
                          <StatCell
                            icon="once"
                            label={$t("adminBattlesAbilityCooldown")}
                            min={0}
                            max={5}
                            bind:value={draft.abilities[i].cooldown}
                          />
                        </StatPlate>
                        <!-- Оговорки о радиусе здесь нет и не нужно: ячейка
                             приглушена и не правится ровно у тех форм, которые
                             числа не несут, и это видно без слов. -->
                      </div>

                      <!-- Канал медальонами: их четыре, и это тот же выбор
                           теми же кружками, что у карты целиком в «Ударе». -->
                      <div class="col-span-2">
                        <span
                          class="block mb-1.5 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                          >{$t("adminBattlesChannel")}</span
                        >
                        <div class="flex items-start gap-4">
                          {#each CHANNELS as channel (channel)}
                            <Medallion
                              icon={CHANNEL_ICON[channel]}
                              caption={$t(CHANNEL_LABELS[channel])}
                              size={30}
                              selected={ability.channel === channel}
                              onclick={() =>
                                (draft.abilities[i].channel = channel)}
                            />
                          {/each}
                        </div>
                      </div>

                      <SheetField
                        label={`${$t("adminBattlesTraitName")} · RU`}
                      >
                        <input
                          bind:value={draft.abilities[i].nameRu}
                          maxlength="60"
                        />
                      </SheetField>
                      <SheetField
                        label={`${$t("adminBattlesTraitName")} · EN`}
                      >
                        <input
                          bind:value={draft.abilities[i].nameEn}
                          maxlength="60"
                        />
                      </SheetField>
                    </div>
                  </div>
                {/if}
              </SheetPanel>

              <SheetPanel
                title={$t("adminBattlesSheetPrice")}
                lead={$t("adminBattlesSheetPriceLead")}
                note={$t("adminBattlesPriceHint")}
              >
                <div class="grid grid-cols-2 gap-3">
                  <SheetField label={$t("adminBattlesPriceDust")}>
                    <input
                      type="number"
                      min="0"
                      value={draft.priceDust ?? ""}
                      oninput={(e) =>
                        priceInput("priceDust", e.currentTarget.value)}
                      onfocus={selectOnFocus}
                      onwheel={blurOnWheel}
                    />
                  </SheetField>
                  <SheetField label={$t("adminBattlesPriceFeed")}>
                    <input
                      type="number"
                      min="0"
                      value={draft.priceFeed ?? ""}
                      oninput={(e) =>
                        priceInput("priceFeed", e.currentTarget.value)}
                      onfocus={selectOnFocus}
                      onwheel={blurOnWheel}
                    />
                  </SheetField>
                </div>

                <!-- Лестница уровней. Заводится сейчас, поднимаются по ней в 1c. -->
                <details class="mt-4 mb-2">
                  <summary
                    class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
                    >{$t("adminBattlesHintOpen")}</summary
                  >
                  <p
                    class="mt-2 text-[11px] leading-relaxed italic text-[#8a6a55]"
                  >
                    {$t("adminBattlesLevelPriceHint")}
                  </p>
                </details>
                <div class="grid grid-cols-4 gap-2">
                  {#each [0, 1, 2, 3] as step (step)}
                    <SheetField label={`${step + 1}→${step + 2}`}>
                      <input
                        type="number"
                        min="0"
                        value={draft.levelPriceDust?.[step] ?? ""}
                        oninput={(e) =>
                          levelPriceInput(step, e.currentTarget.value)}
                        onfocus={selectOnFocus}
                        onwheel={blurOnWheel}
                      />
                    </SheetField>
                  {/each}
                </div>

                <!-- Заём. Стоит рядом с ценами, потому что это тоже про то, как
                     карта попадает к человеку, — только даром и на время. Без
                     заёма стол это запертая дверь ровно для того, кто пришёл
                     впервые: партия просит шести карт, а у него ноль. -->
                <label
                  class="mt-4 flex items-start gap-2 text-[11px] leading-relaxed text-[#5f4636]"
                >
                  <input
                    type="checkbox"
                    class="mt-0.5"
                    bind:checked={draft.lendable}
                  />
                  <span>
                    {$t("adminBattlesLendable")}
                    <details class="mt-1">
                      <summary
                        class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
                        >{$t("adminBattlesHintOpen")}</summary
                      >
                      <p
                        class="mt-2 text-[11px] leading-relaxed italic text-[#8a6a55]"
                      >
                        {$t("adminBattlesLendableHint")}
                      </p>
                    </details>
                  </span>
                </label>
              </SheetPanel>

              <!-- Чем эта карта показывает удар, чару, лечение. Стоит рядом с
                   нарядом, а не в отдельной вкладке: и то и другое — как карта
                   ВЫГЛЯДИТ, и надеваются они одним и тем же жестом. -->
              <SheetPanel title={$t("adminBattlesSheetMotion")}>
                <BattleMotionWear
                  wear={draft.motionWear}
                  {motions}
                  inherited={raceWear}
                  onChange={(raw) => (draft.motionWear = raw)}
                  onOpenBox={() => (view = "motions")}
                />
              </SheetPanel>

              <SheetPanel
                wide
                title={$t("adminBattlesSheetRecord")}
                lead={$t("adminBattlesSheetRecordLead")}
              >
                <div class="grid grid-cols-2 gap-3">
                  <SheetField label={$t("adminBattlesStatus")}>
                    <select bind:value={draft.status}>
                      <option value="draft"
                        >{$t("adminBattlesStatusDraft")}</option
                      >
                      <option value="published"
                        >{$t("adminBattlesStatusPublished")}</option
                      >
                      <option value="retired"
                        >{$t("adminBattlesStatusRetired")}</option
                      >
                    </select>
                  </SheetField>
                  <SheetField label={$t("adminBattlesSlug")}>
                    <input bind:value={draft.slug} />
                  </SheetField>
                </div>

                <!-- А вот это НЕ переезжает: снятая карта просто исчезает с
                     доски у всех, кто её называет, и этюд остаётся без тела. -->
                {#if willEmptyEtudes}
                  <p class="mt-2 text-[11px] leading-snug text-[#8f2f22]">
                    {$t("adminBattlesEtudesWillEmpty")}
                    {etudesUsing.map(etudeTitleOf).join(", ")}
                  </p>
                {/if}
                <!-- Слуг — это то, чем карту называют этюды. Раньше правка
                     этого поля осиротила бы их молча; теперь переименование
                     переезжает в них само, и сказать об этом надо здесь, где
                     печатают, а не в примечании к выпуску. -->
                {#if etudesUsing.length}
                  <p class="mt-2 text-[11px] leading-snug text-[#5f4636]">
                    {$t("adminBattlesInEtudes")}
                    {etudesUsing.length}: {etudesUsing
                      .map(etudeTitleOf)
                      .join(", ")}.
                    <span class="italic text-[#8a6a55]"
                      >{$t("adminBattlesSlugCarried")}</span
                    >
                  </p>
                {/if}
              </SheetPanel>
            </div>

            <!-- ── Сама карта ──────────────────────────────────────────────
                 Прибита к верху и стоит в колонке ОДНА. Под ней были наряд и
                 движения, и это не работало ни дня: прибитое не двигает
                 соседей, оно их накрывает, — а у карты посреди рамки дыра, и
                 сквозь неё было видно, как под карту въезжают чужие панели.
                 Колонка тянется во всю высоту листа (потому ряд и не
                 `items-start`), и прибивать внутри неё больше нечего. -->
            <!-- Сложенным столбиком карта не растягивается во всю ширину:
                 она не становится понятнее от того, что стала с ладонь. -->
            <aside
              class="w-full max-w-[360px] flex-shrink-0 @4xl:w-[360px] @4xl:max-w-none"
            >
              <div class="pt-1 @4xl:sticky @4xl:top-0">
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
                  onfit={(over) => (draftFit = over)}
                />

                <!-- Лампа «не влезло». Полоса свойств не выливается, а
                     прокручивается, и не влезшее просто пропадает: полосы
                     прокрутки на карте никто не видит и не ищет. Мерит сама
                     карта — после укладки и на той ширине, на которой стоит,
                     потому что без ширины у вопроса «влезло ли» ответа нет. -->
                {#if draftFit > 0}
                  <p
                    class="mt-2 flex items-center gap-1.5 text-[10px] uppercase tracking-[0.16em] text-[#8f2f22]"
                  >
                    <span class="w-1.5 h-1.5 rounded-full bg-[#c65f3c]"></span>
                    {$t("adminBattlesFitOver")}
                    {Math.round(draftFit)}px
                  </p>
                {/if}

                <!-- Наряд без окна. Молча это не чинится: где у ЭТОЙ картинки
                     дыра, знает только тот, кто её рисовал. -->
                {#if draftDressBlind}
                  <p
                    class="mt-2 max-w-[42ch] text-[11px] leading-relaxed italic text-[#8f2f22]"
                  >
                    {$t("adminBattlesDressNoWindow")}
                  </p>
                {/if}
                <div class="mt-3">
                  <SheetField label={$t("adminBattlesPreviewLevel")}>
                    <select
                      value={previewLevel ?? ""}
                      onchange={(e) =>
                        (previewLevel =
                          e.currentTarget.value === ""
                            ? null
                            : Number(e.currentTarget.value))}
                    >
                      <option value="">{$t("adminBattlesPreviewNone")}</option>
                      {#each TIERS as step (step)}
                        <option value={step}>{step}</option>
                      {/each}
                    </select>
                  </SheetField>
                </div>
              </div>
            </aside>
          </div>
        </div>

        <!-- ── Годность карты и сама кнопка ────────────────────────────────
             Прибита к низу листа. Слова о годности приходят с сервера тем же
             разбором, которым он откажет, — подсказка и отказ не разойдутся, —
             и читать их надо ровно перед сохранением. Пока полоса ехала вместе
             с колонкой, до неё надо было доскроллить, а причина отказа стояла
             ещё ниже кнопки, которая от неё погасла. -->
        <div
          class="sticky bottom-0 z-10 border-t border-[#34251c]/12 bg-[#f8f1e7]"
        >
          <div
            class="w-full max-w-[1320px] mx-auto px-8 py-3 flex items-center gap-4"
          >
            <button
              onclick={save}
              disabled={saving || blocking.length > 0 || workTaken}
              class="px-4 py-2 text-[10px] uppercase tracking-[0.16em] bg-[#34251c] text-[#f8f1e7] disabled:opacity-40"
              >{$t("adminBattlesSave")}</button
            >
            {#if selectedId}
              <button
                onclick={remove}
                class="px-4 py-2 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 text-[#6f3b24] hover:bg-[#c65f3c]/10"
                >{$t("adminBattlesDelete")}</button
              >
            {/if}
            <!-- Жалоба — кнопка, а не надпись. Пока она была надписью, отказ
                 сообщал, ЧТО не так, и ни слова о том, где это лежит: на листе
                 в восемь панелей «имя на обоих языках» отсылало искать поле
                 имени руками, и на другом языке, о котором в этот миг никто не
                 думает. Теперь она ведёт к полю, по дороге переключая язык на
                 ту сторону, где пусто. -->
            <div class="min-w-0 space-y-0.5">
              {#each blocking as fault (fault)}
                <button
                  type="button"
                  onclick={() => goToFault(fault)}
                  class="block w-full text-left text-[11px] leading-snug text-[#8f2f22] hover:text-[#c65f3c]"
                >
                  <span class="underline decoration-dotted underline-offset-2"
                    >{faultWord(fault)}</span
                  >
                  <span class="whitespace-nowrap text-[10px] uppercase tracking-[0.14em]"
                    >→ {$t("adminBattlesFaultGo")}</span
                  >
                </button>
              {/each}
              {#each notes as note (note)}
                <p class="text-[11px] leading-snug italic text-[#8a6a55]">
                  {$t(
                    `adminBattlesNote${note[0].toUpperCase()}${note.slice(1)}` as TranslationKey,
                  )}
                </p>
              {/each}
            </div>
          </div>
        </div>
      </div>
    </div>
  {/if}

  {#if picker}
    <BattleAssetPicker
      role={picker.role}
      onPick={(asset) => {
        picker?.apply(asset.url);
        picker = null;
      }}
      onClose={() => (picker = null)}
    />
  {/if}
</div>
