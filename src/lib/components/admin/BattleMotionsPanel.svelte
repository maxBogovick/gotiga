<script lang="ts">
  // Стол такта. Не копия стола рамок: движение живёт во времени.
  //
  // Лица сверху — дом, заготовки, ящик. Сцена — настоящие карты на поле 3×6,
  // тот же `stage()`, что в комнате, и крутится сама. Партитура — верстак под
  // сценой: время в ширину, жест в руке справа. Колодец слов ставит замах или
  // свет этому бару, а не селект из семнадцати.
  // Повод на записи — подсказка, не замок: карта вешает вид на любой повод.
  import { onMount, untrack } from 'svelte';
  import { api, resolveMediaUrl } from '$lib/api';
  import { t, lang } from '$lib/i18n';
  import BattleCard from '$lib/components/BattleCard.svelte';
  import BattleMotionStage from '$lib/components/BattleMotionStage.svelte';
  import BattleAssetPicker from '$lib/components/admin/BattleAssetPicker.svelte';
  import {
    DEFAULT_ASPECT,
    DEFAULT_MOTIONS,
    GESTURE_FADES,
    GESTURE_LAYERS,
    GESTURE_NUDGE_MAX,
    GESTURE_SIZE_MAX,
    GESTURE_TURNS,
    GESTURES_MAX,
    MOTION_FRAMES_MAX,
    MOTION_MS_MAX,
    MOTION_OCCASIONS,
    MOTIONS_MAX,
    STOCK_MOTIONS,
    STRIP_FRAMES,
    STRIP_POSE_MAX,
    STRIP_SCALE_MAX,
    STRIP_TURN_MAX,
    blankStripCell,
    completeSlices,
    frameForCard,
    isLight,
    isMove,
    isSlot,
    motionSpan,
    motionBars,
    motionTitle,
    motionWound,
    newGesture,
    newMotion,
    newSlot,
    oneStirPerBody,
    parseMotionWear,
    punchStripGround,
    splitMotionStrip,
    stage,
    stitchMotionStrip,
    struckOf,
    takeHouse,
    takeStock,
    type HitWear,
    type Staged,
    type StripCell,
  } from '$lib/battles';
  import type {
    BattleCard as BattleCardDto,
    BattleFrame,
    BattleRace,
    GestureBody,
    GestureFade,
    GestureTurn,
    GestureWhom,
    Motion,
    MotionGesture,
    MotionOccasion,
    BattleSplitRect,
  } from '$lib/types/api';
  import type { TranslationKey } from '$lib/i18n';

  let {
    flash,
    onSaved,
  }: {
    flash?: (text: string) => void;
    onSaved?: (motions: Motion[]) => void;
  } = $props();

  let motions = $state<Motion[]>([]);
  let cards = $state<BattleCardDto[]>([]);
  let races = $state<BattleRace[]>([]);
  let frames = $state<BattleFrame[]>([]);
  let loading = $state(true);
  let saving = $state(false);
  let complaint = $state('');

  let stored = $state('');
  let dirty = $derived(stored !== '' && JSON.stringify(motions) !== stored);

  type FaceKind = 'house' | 'stock' | 'mine';
  let faceKind = $state<FaceKind>('house');
  let houseAt = $state(0);
  let stockAt = $state(0);
  let heldId = $state<string | null>(null);

  let held = $derived.by((): Motion | null => {
    if (faceKind === 'house') return DEFAULT_MOTIONS[houseAt] ?? null;
    if (faceKind === 'stock') {
      const ready = STOCK_MOTIONS[stockAt];
      if (!ready) return null;
      return {
        id: `stock-${stockAt}`,
        nameEn: ready.nameEn,
        nameRu: ready.nameRu,
        occasion: ready.occasion,
        gestures: ready.gestures.map((g) => ({ ...g })),
      };
    }
    return motions.find((m) => m.id === heldId) ?? null;
  });

  let gestureAt = $state(0);
  let gesture = $derived(held?.gestures[gestureAt] ?? null);
  let track = $state<GestureWhom>('striker');

  /** Полоса, полёт, поле и пустой слот под рисунок на теле — то, на что
   *  кладут картинку. Замах без картинки сюда не входит. */
  const carriesArt = (g: MotionGesture) =>
    Boolean(g.image) || g.whom === 'flight' || g.whom === 'field' || g.body === 'none';

  const picturedIndex = (gestures: MotionGesture[]) => {
    const i = gestures.findIndex(carriesArt);
    return i < 0 ? 0 : i;
  };

  const faceImage = (gestures: MotionGesture[]) =>
    gestures.find((g) => g.image)?.image ?? '';

  let pictured = $derived(
    (held?.gestures ?? [])
      .map((g, i) => ({ g, i }))
      .filter((x) => carriesArt(x.g)),
  );

  let artAt = $derived.by(() => {
    if (!held) return -1;
    const current = held.gestures[gestureAt];
    if (current && carriesArt(current)) return gestureAt;
    return held.gestures.findIndex(carriesArt);
  });

  let artGesture = $derived(held && artAt >= 0 ? (held.gestures[artAt] ?? null) : null);

  function aimArt(at: number) {
    const g = held?.gestures[at];
    if (!g) return;
    gestureAt = at;
    track = g.whom;
    hand = 'gesture';
  }

  function showHouse(i: number) {
    faceKind = 'house';
    houseAt = i;
    const motion = DEFAULT_MOTIONS[i];
    gestureAt = picturedIndex(motion?.gestures ?? []);
    const g = motion?.gestures[gestureAt];
    if (g) track = g.whom;
    bump();
  }

  function showStock(i: number) {
    faceKind = 'stock';
    stockAt = i;
    const ready = STOCK_MOTIONS[i];
    gestureAt = picturedIndex(ready?.gestures ?? []);
    const g = ready?.gestures[gestureAt];
    if (g) track = g.whom;
    bump();
  }

  function showMine(id: string) {
    faceKind = 'mine';
    heldId = id;
    const motion = motions.find((m) => m.id === id);
    gestureAt = picturedIndex(motion?.gestures ?? []);
    const g = motion?.gestures[gestureAt];
    if (g) track = g.whom;
    bump();
  }

  const WIDTH = 3;
  const DEPTH = 6;
  let reach = $state(3);
  let along = $state(false);
  let strikerCard = $state('');
  let targetCard = $state('');

  let from = $derived({ x: 1, y: DEPTH - 1 });
  let to = $derived({
    x: 1,
    y: Math.max(0, DEPTH - 1 - Math.min(reach, DEPTH - 1)),
  });

  let spanX = $derived(along ? DEPTH : WIDTH);
  let spanY = $derived(along ? WIDTH : DEPTH);

  let spots = $derived(
    along
      ? Array.from({ length: WIDTH }, (_, x) =>
          Array.from({ length: DEPTH }, (_, y) => ({ x, y })),
        ).flat()
      : Array.from({ length: DEPTH }, (_, y) =>
          Array.from({ length: WIDTH }, (_, x) => ({ x, y })),
        ).flat(),
  );

  let play = $state<Staged | null>(null);
  let playKey = $state(0);
  let playGen = 0;
  let playRev = $state(0);
  let looping = $state(false);
  let pinned = $state(false);
  let playing = $state(false);

  /** Сцене — комната. Ширится КОНТЕЙНЕР, а не `transform`, ровно как на столе
   *  рамок: карта тогда крупнее по-настоящему, а не размыта, и прямоугольники,
   *  измеренные под перетаскиванием, остаются честными. */
  let zoom = $state(1);

  /** Остановленное время, мс от начала такта, или `null` — идёт.
   *  Движение правят не проигрыванием, а остановкой: посмотреть 270-ю
   *  миллисекунду можно только замерев на ней. */
  let hold = $state<number | null>(null);

  /** Одна рука на весь стол. Жест партитуры и кадр полосы — не два предмета,
   *  а два способа взять один; поэтому инспектор один, как на столе рамок. */
  let hand = $state<'gesture' | 'frame'>('gesture');

  /** Слово из колодца, показанное на сцене, но НЕ применённое. Семнадцать слов
   *  молчали: «нависает» узнавалось только после того, как его надели. */
  let tasting = $state<GestureBody | null>(null);

  /** Ящики под партитурой: источники, а не рабочая поверхность. */
  let stripOpen = $state(false);
  let namesOpen = $state(false);
  let taking = $state(false);

  /** Проба слова. Не запись в движение: пока держат мышь на слове, сцена
   *  играет одно это слово тем телом, на чьей дорожке стоит рука. */
  let tasteMotion = $derived.by((): Motion | null => {
    if (!tasting) return null;
    const whom: GestureWhom = track === 'flight' || track === 'field' ? 'target' : track;
    return {
      id: 'taste',
      nameEn: '',
      nameRu: '',
      occasion: held?.occasion ?? 'blow',
      gestures: [{ ...newGesture(whom), body: tasting, at: 0, dur: 420, fade: 'hold' }],
    };
  });

  const dtoOf = (id: string) => cards.find((c) => c.id === id) ?? null;
  const aspectOf = (dto: BattleCardDto) =>
    frameForCard(dto, frames).aspect || DEFAULT_ASPECT;
  let strikerDto = $derived(dtoOf(strikerCard));
  let targetDto = $derived(dtoOf(targetCard));

  /** Стол показывает след удара на цели, как комната: синяк и обломок на
   *  ударе, чернила на чаре. Числа учебные, не из партии. */
  function deskHit(motion: Motion): HitWear | null {
    const at = motionWound(motion);
    if (motion.occasion === 'blow') {
      return { remain: 0.42, blow: 0.28, seed: 3, channel: 'physical', at };
    }
    if (motion.occasion === 'spell') {
      return { remain: 0.7, blow: 0.2, seed: 3, channel: 'magic', at };
    }
    return null;
  }

  let previewHit = $derived(held ? deskHit(held) : null);
  let previewStruck = $derived(struckOf(previewHit));
  let woundOn = $state(false);

  function bump() {
    playRev += 1;
  }

  /** Величину правят на живой сцене: `bump` перезапускает карты, а слайдер
   *  этого не просил — только коробку рисунка. */
  function liveStage() {
    const motion = held;
    if (!motion) return;
    play = stage(motion, from, to, {
      spanX: along ? DEPTH : WIDTH,
      spanY: along ? WIDTH : DEPTH,
      along,
      calm: false,
    });
  }

  // Писать play/playKey/playing здесь нельзя без untrack: `playKey += 1`
  // и читает, и пишет одно поле — эффект гоняет себя до
  // `effect_update_depth_exceeded` и вешает вкладку.
  $effect(() => {
    faceKind;
    houseAt;
    stockAt;
    heldId;
    playRev;
    along;
    looping;
    pinned;
    hold;
    tasteMotion;
    const alongNow = along;
    const loop = looping;
    const pin = pinned;
    const at = hold;
    const taste = tasteMotion;

    const put = () => {
      const motion = taste ?? held;
      if (!motion) {
        play = null;
        playing = false;
        woundOn = false;
        return 0;
      }
      const still = taste ? null : at;
      play = stage(motion, from, to, {
        spanX: alongNow ? DEPTH : WIDTH,
        spanY: alongNow ? WIDTH : DEPTH,
        along: alongNow,
        calm: false,
        hold: still,
      });
      // Замершая сцена карт НЕ переставляет: скраббинг идёт по кадру на каждое
      // движение мыши, и пересборка `BattleCard` на каждом — это не работа.
      // Стили жестов при этом меняются, а анимация стоит на паузе, поэтому
      // замершая поза остаётся точной.
      if (still === null) {
        playGen += 1;
        playKey = playGen;
      }
      playing = true;
      const delay = motionWound(motion);
      if (still !== null) {
        woundOn = still >= delay;
        return 0;
      }
      woundOn = false;
      const gen = playGen;
      if (delay <= 0) woundOn = true;
      else {
        setTimeout(() => {
          if (playKey === gen) woundOn = true;
        }, delay);
      }
      return motionSpan(motion);
    };

    const spanMs = untrack(put);
    // Проба слова крутится сама, не спрашивая «по кругу»: её смотрят ровно
    // столько, сколько держат мышь на слове.
    if ((!loop && !taste) || pin || !spanMs) return;
    const wait = Math.max(480, spanMs + 380);
    const id = setInterval(() => untrack(put), wait);
    return () => clearInterval(id);
  });

  function ensureMine(): Motion | null {
    if (faceKind === 'mine') return motions.find((m) => m.id === heldId) ?? null;
    if (motions.length >= MOTIONS_MAX) return null;
    const src = held;
    if (!src) return null;
    const born: Motion = {
      ...structuredClone($state.snapshot(src)),
      id: newMotion().id,
    };
    motions = [...motions, born];
    faceKind = 'mine';
    heldId = born.id;
    return born;
  }

  function takeFace() {
    if (faceKind === 'house') {
      if (motions.length >= MOTIONS_MAX) return;
      const born = takeHouse(houseAt);
      if (!born) return;
      motions = [...motions, born];
      faceKind = 'mine';
      heldId = born.id;
      gestureAt = picturedIndex(born.gestures);
      const g = born.gestures[gestureAt];
      if (g) track = g.whom;
      bump();
      return;
    }
    if (faceKind === 'stock') {
      if (motions.length >= MOTIONS_MAX) return;
      const born = takeStock(stockAt);
      if (!born) return;
      motions = [...motions, born];
      faceKind = 'mine';
      heldId = born.id;
      gestureAt = picturedIndex(born.gestures);
      const g = born.gestures[gestureAt];
      if (g) track = g.whom;
      bump();
    }
  }

  function addBlank() {
    if (motions.length >= MOTIONS_MAX) return;
    const born = newMotion('blow');
    born.nameRu = $t('adminMotionsUntitled');
    born.nameEn = 'New motion';
    motions = [...motions, born];
    faceKind = 'mine';
    heldId = born.id;
    gestureAt = 0;
    bump();
  }

  function duplicate() {
    const src = ensureMine();
    if (!src || motions.length >= MOTIONS_MAX) return;
    const twin: Motion = {
      ...structuredClone($state.snapshot(src)),
      id: newMotion().id,
      nameRu: `${src.nameRu} ·`,
      nameEn: `${src.nameEn} ·`,
    };
    motions = [...motions, twin];
    heldId = twin.id;
    gestureAt = picturedIndex(twin.gestures);
    bump();
  }

  function dropHeld() {
    if (faceKind !== 'mine' || !heldId) return;
    motions = motions.filter((m) => m.id !== heldId);
    heldId = motions[0]?.id ?? null;
    if (!heldId) {
      faceKind = 'house';
      houseAt = 0;
    }
    gestureAt = 0;
    bump();
  }

  type ScoreKind = 'move' | 'light' | 'art';

  function scoreKind(g: MotionGesture): ScoreKind {
    if (isMove(g.body)) return 'move';
    if (isLight(g.body)) return 'light';
    return 'art';
  }

  function putBody(body: GestureBody) {
    const whom: GestureWhom =
      gesture?.whom === 'striker' || gesture?.whom === 'target' ? gesture.whom : track;
    if (whom === 'flight' || whom === 'field') return;
    const mine = ensureMine();
    if (!mine) return;
    const pred = isLight(body) ? isLight : isMove;
    const at = mine.gestures.findIndex((g) => g.whom === whom && pred(g.body));
    if (at >= 0) {
      mine.gestures[at].body = body;
      gestureAt = at;
      track = whom;
    } else {
      if (mine.gestures.length >= GESTURES_MAX) return;
      const g = { ...newGesture(whom), body, fade: 'hold' as const };
      mine.gestures = oneStirPerBody([...mine.gestures, g]);
      gestureAt = mine.gestures.findIndex((x) => x.whom === whom && x.body === body);
      track = whom;
    }
    mine.gestures = [...mine.gestures];
    hand = 'gesture';
    bump();
  }

  function newArt(whom: GestureWhom, at = 80, dur = 320): MotionGesture {
    if (whom === 'flight' || whom === 'field') return newSlot(whom, at, dur);
    return {
      ...newGesture(whom),
      body: 'none',
      image: '',
      at,
      dur,
      fade: 'inOut',
      layer: 8,
      size: 118,
    };
  }

  function findLaneGesture(gestures: MotionGesture[], whom: GestureWhom, kind: ScoreKind) {
    return gestures.findIndex((g) => g.whom === whom && scoreKind(g) === kind);
  }

  function putArtOn(whom: GestureWhom, at = 80) {
    const mine = ensureMine();
    if (!mine) return;
    const found = findLaneGesture(mine.gestures, whom, 'art');
    if (found >= 0) {
      gestureAt = found;
      track = whom;
      return;
    }
    if (mine.gestures.length >= GESTURES_MAX) return;
    mine.gestures = [...mine.gestures, newArt(whom, at)];
    gestureAt = mine.gestures.length - 1;
    track = whom;
    bump();
  }

  function dropGesture(i: number) {
    const mine = ensureMine();
    if (!mine) return;
    mine.gestures = mine.gestures.filter((_, k) => k !== i);
    gestureAt = Math.max(0, Math.min(i, mine.gestures.length - 1));
    bump();
  }

  let picking = $state<null | 'all' | number>(null);
  let uploading = $state(false);
  let stripCells = $state<StripCell[]>(
    Array.from({ length: STRIP_FRAMES }, blankStripCell),
  );
  let stripDirty = $state(false);
  let stripBusy = $state(false);
  let stripWork = $state<'cut' | 'join' | ''>('');
  let stripOver = $state<number | null>(null);
  let stripAt = $state(0);
  let stripHeld = $derived(stripCells[stripAt] ?? null);
  let composeWait = 0;

  function artIndex() {
    return artAt >= 0 ? artAt : gestureAt;
  }

  async function upload(file: File) {
    const at = artIndex();
    const mine = ensureMine();
    const g = mine?.gestures[at];
    if (!g) return;
    uploading = true;
    try {
      const got = await api.adminUploadBattleFrameArt(file);
      setArt(got.url);
    } catch (e) {
      complaint = String(e);
    } finally {
      uploading = false;
    }
  }

  function setArt(url: string) {
    setImage(artIndex(), url);
  }

  function setImage(i: number, url: string) {
    const mine = ensureMine();
    const g = mine?.gestures[i];
    if (!g) return;
    g.image = url;
    g.strip = [];
    // Дом рисует полосу на 118% клетки. Одиночная картина при той же величине —
    // второй портрет. Свести к удару.
    if (url && g.size >= 100) g.size = 56;
    mine.gestures = [...mine.gestures];
    gestureAt = i;
    track = g.whom;
    hand = 'gesture';
    bump();
  }

  function setBeat(n: number) {
    const mine = ensureMine();
    if (!mine) return;
    const bars = motionBars(mine);
    const ms = Math.round((Number(n) || 0) / 10) * 10;
    mine.span = Math.max(bars, Math.min(MOTION_MS_MAX, Math.max(80, ms)));
  }

  function setSize(n: number) {
    const at = artIndex();
    const mine = ensureMine();
    const g = mine?.gestures[at];
    if (!g) return;
    g.size = Math.max(0, Math.min(GESTURE_SIZE_MAX, Math.round(n) || 0));
    mine.gestures = [...mine.gestures];
    gestureAt = at;
    liveStage();
  }

  function setFrames(n: number) {
    const at = artIndex();
    const mine = ensureMine();
    const g = mine?.gestures[at];
    if (!g) return;
    g.frames = Math.max(1, Math.min(MOTION_FRAMES_MAX, Math.round(n) || 1));
    mine.gestures = [...mine.gestures];
    gestureAt = at;
    bump();
  }

  function openStore() {
    aimArt(artIndex());
    picking = 'all';
  }

  function emptyStrip(): StripCell[] {
    return Array.from({ length: STRIP_FRAMES }, blankStripCell);
  }

  function revokeStrip(url: string | null) {
    if (url?.startsWith('blob:')) URL.revokeObjectURL(url);
  }

  function poseStyle(cell: StripCell) {
    return `transform:translate(${cell.x}%,${cell.y}%) rotate(${cell.turn}deg) scale(${(cell.size || 100) / 100})`;
  }

  function holdPose(n: number, most: number) {
    return Math.max(-most, Math.min(most, n));
  }

  $effect(() => {
    const image = artGesture?.image ?? '';
    const frames = artGesture?.frames ?? 1;
    const saved = artGesture?.strip ?? [];
    stripDirty = false;
    let cancelled = false;
    if (saved.length === STRIP_FRAMES && saved.some((c) => c.image)) {
      stripCells = saved.map((c) => ({
        src: c.image,
        turn: c.turn || 0,
        size: c.size || 100,
        x: c.x || 0,
        y: c.y || 0,
      }));
    } else if (image && frames === STRIP_FRAMES) {
      const src = resolveMediaUrl(image) ?? image;
      void splitMotionStrip(src, STRIP_FRAMES)
        .then((parts) => {
          if (!cancelled && !stripDirty)
            stripCells = parts.map((src) => ({ ...blankStripCell(), src }));
        })
        .catch(() => {
          if (!cancelled && !stripDirty) stripCells = emptyStrip();
        });
    } else {
      stripCells = emptyStrip();
    }
    return () => {
      cancelled = true;
    };
  });

  function putCell(i: number, url: string) {
    revokeStrip(stripCells[i]?.src ?? null);
    stripDirty = true;
    stripAt = i;
    const had = stripCells[i] ?? blankStripCell();
    stripCells[i] = { ...had, src: url };
    stripCells = [...stripCells];
    if (stripCells.every((c) => c.src)) void composeStrip();
  }

  /** Взять кадр в руку. Настройки кадра живут в инспекторе, а не под полосой:
   *  инспектор один, и в нём ровно то, что в руке. */
  function holdCell(i: number) {
    stripAt = i;
    hand = 'frame';
  }

  function clearCell(i: number) {
    revokeStrip(stripCells[i]?.src ?? null);
    stripDirty = true;
    stripCells[i] = blankStripCell();
    stripCells = [...stripCells];
  }

  function poseCell(i: number, patch: Partial<StripCell>) {
    const had = stripCells[i];
    if (!had?.src) return;
    stripDirty = true;
    stripAt = i;
    stripCells[i] = {
      ...had,
      turn: holdPose(patch.turn ?? had.turn, STRIP_TURN_MAX),
      size: Math.max(8, Math.min(STRIP_SCALE_MAX, patch.size ?? had.size)),
      x: holdPose(patch.x ?? had.x, STRIP_POSE_MAX),
      y: holdPose(patch.y ?? had.y, STRIP_POSE_MAX),
    };
    stripCells = [...stripCells];
  }

  function spreadSrc(i: number) {
    const src = stripCells[i]?.src;
    if (!src) return;
    stripDirty = true;
    stripCells = stripCells.map((c, k) =>
      k === i ? c : { ...c, src, turn: c.src ? c.turn : 0, size: c.src ? c.size : 100, x: c.src ? c.x : 0, y: c.src ? c.y : 0 },
    );
    if (stripCells.every((c) => c.src)) void composeStrip();
  }

  function scheduleCompose() {
    window.clearTimeout(composeWait);
    composeWait = window.setTimeout(() => void composeStrip(), 280);
  }

  async function putCellFile(i: number, file: File) {
    stripAt = i;
    stripBusy = true;
    stripWork = 'join';
    try {
      const got = await api.adminUploadBattleFrameArt(file);
      putCell(i, got.url);
    } catch (e) {
      complaint = String(e);
    } finally {
      stripBusy = false;
      stripWork = '';
    }
  }

  async function putCellFiles(files: FileList | File[], start = 0) {
    const list = [...files].slice(0, STRIP_FRAMES - start);
    if (!list.length) return;
    stripBusy = true;
    stripWork = 'join';
    stripDirty = true;
    try {
      for (let k = 0; k < list.length; k++) {
        const file = list[k];
        if (!file) continue;
        const got = await api.adminUploadBattleFrameArt(file);
        const at = start + k;
        revokeStrip(stripCells[at]?.src ?? null);
        const had = stripCells[at] ?? blankStripCell();
        stripCells[at] = { ...had, src: got.url };
      }
      stripCells = [...stripCells];
      if (stripCells.every((c) => c.src)) void composeStrip();
    } catch (e) {
      complaint = String(e);
    } finally {
      stripBusy = false;
      stripWork = '';
    }
  }

  /** Готовая полоса из шести кадров в ряд: тот же разрез, что на вкладке
   *  ассетов — шесть равных колонок, каждая подтянута к рисунку внутри. */
  async function cutPreparedStrip(file: File) {
    stripBusy = true;
    stripWork = 'cut';
    stripDirty = true;
    complaint = '';
    let filled = false;
    try {
      const punched = await punchStripGround(file);
      const stem = file.name.replace(/\.[^.]+$/, '') || 'strip';
      const parent = await api.adminAddBattleAsset(punched, stem, 'motion');
      const rects: BattleSplitRect[] = Array.from({ length: STRIP_FRAMES }, (_, i) => ({
        x: i / STRIP_FRAMES,
        y: 0,
        w: 1 / STRIP_FRAMES,
        h: 1,
        name: `${stem} ${i + 1}`,
        role: 'motion',
      }));
      const parts = await api.adminSplitBattleAsset(parent.id, rects);
      const ordered = [...parts].sort((a, b) => {
        const ao = a.sortOrder ?? 0;
        const bo = b.sortOrder ?? 0;
        if (ao !== bo) return ao - bo;
        return a.name.localeCompare(b.name, undefined, { numeric: true });
      });
      if (ordered.length !== STRIP_FRAMES) throw new Error('cut');
      const next = emptyStrip();
      for (let i = 0; i < STRIP_FRAMES; i++) {
        const url = ordered[i]?.url;
        if (!url) throw new Error('cut');
        next[i] = { ...blankStripCell(), src: url };
      }
      for (const cell of stripCells) revokeStrip(cell.src);
      stripCells = next;
      filled = true;
    } catch (e) {
      complaint =
        e instanceof Error && e.message === 'cut'
          ? $t('adminMotionsStripCutFail')
          : String(e);
    } finally {
      stripBusy = false;
      stripWork = '';
    }
    if (filled) void composeStrip();
  }

  async function composeStrip() {
    const ready = stripCells.filter((c) => c.src);
    if (ready.length !== STRIP_FRAMES) return;
    if (stripBusy) {
      composeWait = window.setTimeout(() => void composeStrip(), 200);
      return;
    }
    stripBusy = true;
    stripWork = 'join';
    complaint = '';
    try {
      const blob = await stitchMotionStrip(stripCells, STRIP_FRAMES);
      const file = new File([blob], 'strip.png', { type: 'image/png' });
      const got = await api.adminUploadBattleFrameArt(file);
      applyStrip(got.url);
    } catch (e) {
      complaint = String(e);
    } finally {
      stripBusy = false;
      stripWork = '';
    }
  }

  function applyStrip(url: string) {
    const mine = ensureMine();
    if (!mine) return;
    let at = artAt >= 0 ? artAt : mine.gestures.findIndex(carriesArt);
    if (at < 0) {
      mine.gestures = [
        ...mine.gestures,
        {
          ...newGesture('target'),
          body: 'none',
          fade: 'inOut',
          layer: 8,
          at: 180,
          dur: 480,
          size: 56,
        },
      ];
      at = mine.gestures.length - 1;
    }
    const g = mine.gestures[at];
    if (!g) return;
    g.image = url;
    g.frames = STRIP_FRAMES;
    g.strip = stripCells.map((c) => ({
      image: c.src ?? '',
      turn: c.turn,
      size: c.size,
      x: c.x,
      y: c.y,
    }));
    mine.gestures = [...mine.gestures];
    gestureAt = at;
    track = g.whom;
    bump();
  }

  let stripDrag = $state<{
    i: number;
    x0: number;
    y0: number;
    ox: number;
    oy: number;
    w: number;
  } | null>(null);

  function poseDown(e: PointerEvent, i: number) {
    const cell = stripCells[i];
    if (!cell?.src) return;
    stripAt = i;
    hand = 'frame';
    const box = (e.currentTarget as HTMLElement).getBoundingClientRect();
    stripDrag = {
      i,
      x0: e.clientX,
      y0: e.clientY,
      ox: cell.x,
      oy: cell.y,
      w: Math.max(1, box.width),
    };
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }

  function poseMove(e: PointerEvent) {
    if (!stripDrag) return;
    const dx = ((e.clientX - stripDrag.x0) / stripDrag.w) * 100;
    const dy = ((e.clientY - stripDrag.y0) / stripDrag.w) * 100;
    poseCell(stripDrag.i, { x: stripDrag.ox + dx, y: stripDrag.oy + dy });
  }

  function poseUp() {
    if (stripDrag) scheduleCompose();
    stripDrag = null;
  }

  // ── Партитура: бар тянут ─────────────────────────────────────────────────

  let drag = $state<{
    i: number;
    mode: 'at' | 'dur';
    origin: number;
    startAt: number;
    startDur: number;
    width: number;
    total: number;
  } | null>(null);

  function msFromPx(dx: number, width: number, total: number) {
    if (width <= 0 || total <= 0) return 0;
    return Math.round(((dx / width) * total) / 10) * 10;
  }

  function barDown(e: PointerEvent, i: number, mode: 'at' | 'dur') {
    const mine = ensureMine();
    const g = mine?.gestures[i];
    if (!g) return;
    const lane = (e.currentTarget as HTMLElement).closest('.score-lane');
    const width = lane?.getBoundingClientRect().width ?? 1;
    pinned = true;
    gestureAt = i;
    track = g.whom;
    hand = 'gesture';
    drag = {
      i,
      mode,
      origin: e.clientX,
      startAt: g.at,
      startDur: g.dur,
      width,
      total: scoreMs,
    };
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }

  function barMove(e: PointerEvent) {
    if (!drag) return;
    const mine = motions.find((m) => m.id === heldId);
    const g = mine?.gestures[drag.i];
    if (!g) return;
    const delta = msFromPx(e.clientX - drag.origin, drag.width, drag.total);
    if (drag.mode === 'at') {
      g.at = Math.max(0, Math.min(MOTION_MS_MAX - g.dur, drag.startAt + delta));
    } else {
      g.dur = Math.max(40, Math.min(MOTION_MS_MAX - g.at, drag.startDur + delta));
    }
    mine.gestures = [...mine.gestures];
  }

  function barUp() {
    drag = null;
    pinned = false;
    bump();
  }

  function playTurn() {
    hold = null;
    bump();
  }

  // ── Остановленное время ──────────────────────────────────────────────────
  //
  // Партитура была только редактором: бегунок рисовался, но взять его было
  // нельзя. Между тем движение правят остановкой — «покажи мне 270-ю и держи»,
  // — а не тем, что смотрят такт целиком по третьему разу.

  let scrubbing = $state(false);

  function holdAt(lane: Element, clientX: number) {
    const box = lane.getBoundingClientRect();
    if (box.width <= 0) return 0;
    const t = ((clientX - box.left) / box.width) * scoreMs;
    return Math.max(0, Math.min(scoreMs, Math.round(t / 10) * 10));
  }

  function scrubDown(e: PointerEvent) {
    const lane = e.currentTarget as HTMLElement;
    scrubbing = true;
    hold = holdAt(lane, e.clientX);
    lane.setPointerCapture(e.pointerId);
  }

  function scrubMove(e: PointerEvent) {
    if (!scrubbing) return;
    hold = holdAt(e.currentTarget as HTMLElement, e.clientX);
  }

  function scrubUp() {
    scrubbing = false;
  }

  /** Линейка — настоящий ползунок, не картинка: десять миллисекунд мышью не
   *  берутся, а решают именно они. */
  function holdKey(e: KeyboardEvent) {
    const way = e.key === 'ArrowLeft' ? -1 : e.key === 'ArrowRight' ? 1 : 0;
    if (way) {
      e.preventDefault();
      const step = (e.shiftKey ? 100 : 10) * way;
      hold = Math.max(0, Math.min(scoreMs, (hold ?? 0) + step));
      return;
    }
    if (e.key === 'Home') {
      e.preventDefault();
      hold = 0;
    } else if (e.key === 'End') {
      e.preventDefault();
      hold = scoreMs;
    } else if (e.key === 'Escape') {
      e.preventDefault();
      hold = null;
    }
  }

  /** Стрелки двигают взятый жест: партитуру тянут мышью, но десять
   *  миллисекунд мышью не берутся, а именно они и решают. */
  function nudgeBar(e: KeyboardEvent, i: number) {
    const way = e.key === 'ArrowLeft' ? -1 : e.key === 'ArrowRight' ? 1 : 0;
    if (!way) return;
    const mine = ensureMine();
    const g = mine?.gestures[i];
    if (!g) return;
    e.preventDefault();
    mark();
    const step = (e.shiftKey ? 100 : 10) * way;
    if (e.altKey) g.dur = Math.max(40, Math.min(MOTION_MS_MAX - g.at, g.dur + step));
    else g.at = Math.max(0, Math.min(MOTION_MS_MAX - g.dur, g.at + step));
    mine.gestures = [...mine.gestures];
    bump();
  }

  // ── Отмена ───────────────────────────────────────────────────────────────
  //
  // Ровно то же, что на столе рамок, и по той же причине: полосу тянут мышью,
  // правка пишется сразу, и без возврата трогать становится страшно — а прямое
  // манипулирование живо тем, что пробовать не страшно.

  let history = $state<string[]>([]);
  let ahead = $state<string[]>([]);

  /** Слепок ПЕРЕД правкой. Один раз на жест: с партитуры — в начале
   *  перетаскивания, из колонки — перехватом нажатия и фокуса на всей колонке,
   *  чтобы ни один орган управления не оборачивать вручную. */
  function mark() {
    const now = JSON.stringify(motions);
    if (history[history.length - 1] === now) return;
    history.push(now);
    if (history.length > 60) history.shift();
    ahead = [];
  }

  function wear(shot: string) {
    motions = JSON.parse(shot) as Motion[];
    if (heldId && !motions.some((m) => m.id === heldId)) {
      heldId = motions[0]?.id ?? null;
      if (!heldId) faceKind = 'house';
    }
    gestureAt = Math.min(gestureAt, Math.max(0, (held?.gestures.length ?? 1) - 1));
    bump();
  }

  function stepBack() {
    // Слепок снимается перехватом нажатия на всю колонку, и нажатие бывает не
    // правкой: потянули линейку — ничего не изменилось. Одинаковые слепки
    // проматываются, иначе «вернуть» один раз ничего не делает.
    const now = JSON.stringify(motions);
    let was = history.pop();
    while (was !== undefined && was === now) was = history.pop();
    if (was === undefined) return;
    ahead.push(now);
    wear(was);
  }

  function stepOn() {
    const next = ahead.pop();
    if (next === undefined) return;
    history.push(JSON.stringify(motions));
    wear(next);
  }

  // ── Стенд ────────────────────────────────────────────────────────────────
  //
  // Пара карт, расстояние и разворот — не данные движения, а настройки стенда,
  // и хранитель выбирал их заново каждый день. Помнить их — то же, что помнить
  // место панели куска рамы.

  const STAND = 'gotiga_battle_motionstand';

  function readStand() {
    try {
      const raw = localStorage.getItem(STAND);
      if (!raw) return;
      const was = JSON.parse(raw) as Record<string, unknown>;
      if (typeof was.striker === 'string') strikerCard = was.striker;
      if (typeof was.target === 'string') targetCard = was.target;
      if (typeof was.reach === 'number') reach = Math.max(1, Math.min(DEPTH - 1, was.reach));
      if (typeof was.along === 'boolean') along = was.along;
      if (typeof was.zoom === 'number') zoom = Math.max(1, Math.min(4, was.zoom));
    } catch {
      /* приватное окно — стенд просто не помнится */
    }
  }

  function keepStand() {
    try {
      localStorage.setItem(
        STAND,
        JSON.stringify({ striker: strikerCard, target: targetCard, reach, along, zoom }),
      );
    } catch {
      /* некуда писать — не беда */
    }
  }

  async function load() {
    loading = true;
    try {
      const [moving, deck, dressing, kin] = await Promise.all([
        api.getBattleMotions(),
        api.adminListBattleCards(),
        api.getBattleFrames(),
        api.getBattleRaces(),
      ]);
      motions = moving.motions;
      cards = deck;
      races = kin;
      frames = (dressing.frames ?? []).map(completeSlices);
      stored = JSON.stringify(motions);
      // Стенд помнится между заходами, но карту могли убрать из колоды.
      if (!cards.some((c) => c.id === strikerCard)) strikerCard = cards[0]?.id ?? '';
      if (!cards.some((c) => c.id === targetCard))
        targetCard = cards[1]?.id ?? cards[0]?.id ?? '';
      if (motions[0]) {
        faceKind = 'mine';
        heldId = motions[0].id;
        gestureAt = picturedIndex(motions[0].gestures);
        const g = motions[0].gestures[gestureAt];
        if (g) track = g.whom;
      }
    } catch (e) {
      complaint = String(e);
    } finally {
      loading = false;
    }
  }

  async function save() {
    saving = true;
    complaint = '';
    try {
      const sent = $state.snapshot(motions);
      const back = await api.adminSaveBattleMotions({ motions: sent });
      motions = back.motions;
      stored = JSON.stringify(motions);
      onSaved?.(motions);
      if (heldId && !motions.some((m) => m.id === heldId)) {
        heldId = motions[0]?.id ?? null;
        if (!heldId) faceKind = 'house';
      }
      const lost = sent.length - back.motions.length;
      flash?.(lost > 0 ? `${$t('adminMotionsTrimmed')} ${lost}` : $t('adminMotionsSaved'));
    } catch (e) {
      complaint = String(e);
    } finally {
      saving = false;
    }
  }

  onMount(() => {
    readStand();
    void load();
    const guard = (e: BeforeUnloadEvent) => {
      if (untrack(() => dirty)) e.preventDefault();
    };
    window.addEventListener('beforeunload', guard);
    return () => window.removeEventListener('beforeunload', guard);
  });

  $effect(() => {
    strikerCard;
    targetCard;
    reach;
    along;
    zoom;
    if (!loading) keepStand();
  });

  const OCCASION_KEY: Record<MotionOccasion, TranslationKey> = {
    blow: 'adminMotionsOccasionBlow',
    spell: 'adminMotionsOccasionSpell',
    mend: 'adminMotionsOccasionMend',
    arrive: 'adminMotionsOccasionArrive',
    fall: 'adminMotionsOccasionFall',
    unseen: 'adminMotionsOccasionUnseen',
  };
  const WHOM_KEY: Record<GestureWhom, TranslationKey> = {
    striker: 'adminMotionsWhomStriker',
    target: 'adminMotionsWhomTarget',
    flight: 'adminMotionsWhomFlight',
    field: 'adminMotionsWhomField',
  };
  const BODY_KEY: Record<GestureBody, TranslationKey> = {
    none: 'adminMotionsBodyNone',
    lunge: 'adminMotionsBodyLunge',
    flinch: 'adminMotionsBodyFlinch',
    shiver: 'adminMotionsBodyShiver',
    sink: 'adminMotionsBodySink',
    rise: 'adminMotionsBodyRise',
    swell: 'adminMotionsBodySwell',
    bow: 'adminMotionsBodyBow',
    draw: 'adminMotionsBodyDraw',
    recoil: 'adminMotionsBodyRecoil',
    heave: 'adminMotionsBodyHeave',
    shudder: 'adminMotionsBodyShudder',
    sway: 'adminMotionsBodySway',
    loom: 'adminMotionsBodyLoom',
    kindle: 'adminMotionsBodyKindle',
    blanch: 'adminMotionsBodyBlanch',
    wither: 'adminMotionsBodyWither',
  };
  const TURN_KEY: Record<GestureTurn, TranslationKey> = {
    none: 'adminMotionsTurnNone',
    toTarget: 'adminMotionsTurnToTarget',
    mirror: 'adminMotionsTurnMirror',
  };
  const FADE_KEY: Record<GestureFade, TranslationKey> = {
    hold: 'adminMotionsFadeHold',
    in: 'adminMotionsFadeIn',
    out: 'adminMotionsFadeOut',
    inOut: 'adminMotionsFadeInOut',
  };

  const WELL: { key: TranslationKey; bodies: GestureBody[] }[] = [
    { key: 'adminMotionsWellToward', bodies: ['lunge', 'heave', 'draw'] },
    { key: 'adminMotionsWellStruck', bodies: ['flinch', 'shiver', 'shudder', 'recoil'] },
    { key: 'adminMotionsWellPose', bodies: ['bow', 'sway', 'loom'] },
    { key: 'adminMotionsWellPresence', bodies: ['swell', 'sink', 'rise'] },
    { key: 'adminMotionsWellLight', bodies: ['kindle', 'blanch', 'wither'] },
  ];

  const SCORE_LANES: { whom: GestureWhom; kind: ScoreKind; first: boolean }[] = [
    { whom: 'striker', kind: 'move', first: true },
    { whom: 'striker', kind: 'light', first: false },
    { whom: 'striker', kind: 'art', first: false },
    { whom: 'target', kind: 'move', first: true },
    { whom: 'target', kind: 'light', first: false },
    { whom: 'target', kind: 'art', first: false },
    { whom: 'flight', kind: 'art', first: true },
    { whom: 'field', kind: 'art', first: true },
  ];

  const KIND_KEY: Record<ScoreKind, TranslationKey> = {
    move: 'adminMotionsScoreMove',
    light: 'adminMotionsScoreLight',
    art: 'adminMotionsScoreArt',
  };

  let span = $derived(motionSpan(held));
  let barsMs = $derived(motionBars(held));
  let woundAt = $derived(motionWound(held));
  let scoreMs = $derived(Math.max(span, 280));
  let scoreTicks = $derived.by(() => {
    const out: number[] = [];
    for (let t = 0; t <= scoreMs; t += 100) out.push(t);
    return out;
  });

  function scorePct(ms: number) {
    return scoreMs > 0 ? (ms / scoreMs) * 100 : 0;
  }

  function laneKin(whom: GestureWhom, kind: ScoreKind) {
    return (held?.gestures ?? [])
      .map((g, i) => ({ g, i }))
      .filter((x) => x.g.whom === whom && scoreKind(x.g) === kind);
  }

  function timeAtPointer(lane: Element, clientX: number) {
    const box = lane.getBoundingClientRect();
    if (box.width <= 0) return 0;
    const t = ((clientX - box.left) / box.width) * scoreMs;
    return Math.max(0, Math.min(MOTION_MS_MAX, Math.round(t / 10) * 10));
  }

  function laneDown(e: PointerEvent, whom: GestureWhom, kind: ScoreKind) {
    const target = e.target as HTMLElement | null;
    if (target?.closest('.bar')) return;
    const lane = e.currentTarget as HTMLElement;
    const at = timeAtPointer(lane, e.clientX);
    track = whom;
    hand = 'gesture';
    const mineList = held?.gestures ?? [];
    const found = findLaneGesture(mineList, whom, kind);
    if (found >= 0) {
      gestureAt = found;
      return;
    }
    if (kind === 'art') {
      putArtOn(whom, at);
      return;
    }
    if (whom === 'flight' || whom === 'field') return;
    const body: GestureBody = kind === 'light' ? 'kindle' : 'lunge';
    const mine = ensureMine();
    if (!mine || mine.gestures.length >= GESTURES_MAX) return;
    const g = { ...newGesture(whom), body, fade: 'hold' as const, at, dur: 280 };
    mine.gestures = oneStirPerBody([...mine.gestures, g]);
    gestureAt = mine.gestures.findIndex((x) => x.whom === whom && x.body === body);
    mine.gestures = [...mine.gestures];
    bump();
  }

  async function putLaneFile(whom: GestureWhom, file: File) {
    putArtOn(whom);
    const mine = motions.find((m) => m.id === heldId);
    const i = mine ? findLaneGesture(mine.gestures, whom, 'art') : -1;
    if (i < 0) return;
    stripBusy = true;
    try {
      const got = await api.adminUploadBattleFrameArt(file);
      setImage(i, got.url);
    } catch (e) {
      complaint = String(e);
    } finally {
      stripBusy = false;
    }
  }

  let scoreOver = $state<string | null>(null);

  const barWord = (g: MotionGesture) => {
    if (isSlot(g)) return $t('adminMotionsSlot');
    if (g.image) return $t('adminMotionsHasArt');
    return $t(BODY_KEY[g.body]);
  };

  let wearers = $derived.by(() => {
    if (faceKind !== 'mine' || !heldId) return [];
    const out: { name: string; occasion: MotionOccasion }[] = [];
    const nameOf = (ru: string, en: string) => ($lang === 'ru' ? ru : en);
    for (const c of cards) {
      const w = parseMotionWear(c.motionWear);
      for (const o of MOTION_OCCASIONS) {
        if (w[o] === heldId) out.push({ name: nameOf(c.titleRu, c.titleEn), occasion: o });
      }
    }
    for (const r of races) {
      const w = parseMotionWear(r.motionWear);
      for (const o of MOTION_OCCASIONS) {
        if (w[o] === heldId)
          out.push({ name: nameOf(r.nameRu, r.nameEn), occasion: o });
      }
    }
    return out;
  });

  const activeBody = $derived.by(() => {
    const g = held?.gestures.find((x) => x.whom === track && isMove(x.body));
    const light = held?.gestures.find((x) => x.whom === track && isLight(x.body));
    return { move: g?.body, light: light?.body };
  });
</script>

<!--
  Стол такта, перекроенный по закону соседних столов: холст главный,
  инспектор сбоку, инструменты — там, куда смотрят.

  Сцена ПРИБИТА и берёт всю оставшуюся высоту; партитура — под ней, тоже
  прибитая; источники (шесть кадров, имя и повод) убраны в ящики внизу, потому
  что смотрят на них раз в час, а места они занимали больше всех. До этого
  сцена лежала пятым блоком в общем скролле — то есть каждая правка проигрывала
  движение туда, где его не видно.
-->
<div class="desk">
  <!-- ── Ящик и стол ───────────────────────────────────────────────────── -->
  <header class="faces">
    <div class="face-col">
      <p class="kicker">{$t('adminMotionsFacesMine')}</p>
      <div class="chips">
        {#each motions as motion (motion.id)}
          {@const img = faceImage(motion.gestures)}
          <button
            type="button"
            class="chip chip--face"
            class:chip--on={faceKind === 'mine' && heldId === motion.id}
            onclick={() => showMine(motion.id)}
          >
            {#if img}
              <span class="chip-art" style="background-image:url('{img}')"></span>
            {/if}
            <span class="chip-name">{motionTitle(motion, $lang)}</span>
            <!-- Повод — то, чем движение цепляется за карту. Без него два
                 «Секира» в ящике не отличаются ничем. -->
            <span class="chip-when">{$t(OCCASION_KEY[motion.occasion])}</span>
          </button>
        {/each}
        <button
          type="button"
          class="chip chip--add"
          disabled={motions.length >= MOTIONS_MAX}
          title={$t('adminMotionsUntitled')}
          onclick={() => {
            mark();
            addBlank();
          }}>+</button
        >
      </div>
    </div>
    <div class="face-save">
      <button
        type="button"
        class="btn"
        class:btn--on={taking || faceKind !== 'mine'}
        onclick={() => (taking = !taking)}>{$t('adminMotionsTakeReady')}</button
      >
      <span class="face-gap"></span>
      <button
        type="button"
        class="btn"
        disabled={!history.length}
        title={$t('adminMotionsBack')}
        onclick={stepBack}>↶</button
      >
      <button
        type="button"
        class="btn"
        disabled={!ahead.length}
        title={$t('adminMotionsForward')}
        onclick={stepOn}>↷</button
      >
      <button type="button" class="btn btn--do" disabled={saving || !dirty} onclick={save}
        >{$t('adminMotionsSave')}</button
      >
      {#if dirty}
        <span class="warn">{$t('adminMotionsUnsaved')}</span>
      {/if}
    </div>
  </header>

  {#if taking || faceKind !== 'mine'}
    <div class="ready">
      <div class="face-col">
        <p class="kicker">{$t('adminMotionsFacesHouse')}</p>
        <div class="chips">
          {#each DEFAULT_MOTIONS as motion, i (motion.id)}
            <button
              type="button"
              class="chip"
              class:chip--on={faceKind === 'house' && houseAt === i}
              onclick={() => showHouse(i)}>{motionTitle(motion, $lang)}</button
            >
          {/each}
        </div>
      </div>
      <div class="face-col">
        <p class="kicker">{$t('adminMotionsFacesStock')}</p>
        <div class="chips">
          {#each STOCK_MOTIONS as ready, i (i)}
            {@const img = faceImage(ready.gestures)}
            <button
              type="button"
              class="chip"
              class:chip--on={faceKind === 'stock' && stockAt === i}
              onclick={() => showStock(i)}
            >
              {#if img}
                <span class="chip-art" style="background-image:url('{img}')"></span>
              {/if}
              {$lang === 'ru' ? ready.nameRu : ready.nameEn}
            </button>
          {/each}
        </div>
      </div>
      {#if faceKind !== 'mine'}
        <div class="ready-take">
          <button
            type="button"
            class="btn btn--do"
            onclick={() => {
              mark();
              takeFace();
              taking = false;
            }}>{$t('adminMotionsTake')}</button
          >
          <p class="hint">{$t('adminMotionsReadOnly')}</p>
        </div>
      {/if}
    </div>
  {/if}

  {#if loading}
    <p class="note px-4 py-3">{$t('adminMotionsLoading')}</p>
  {:else if !held}
    <p class="note px-4 py-3">{$t('adminMotionsNothingHeld')}</p>
  {:else}
    <div class="body" class:body--locked={faceKind !== 'mine'}>
      <section class="stage-wrap">
        <!-- ── Сцена: всё оставшееся место ────────────────────────────── -->
        <div class="stage-dock">
          <div
            class="stage"
            class:stage--along={along}
            style="--x:{spanX};--y:{spanY};--zoom:{zoom}"
          >
            <div class="grid">
              {#each spots as cell (`${cell.x},${cell.y}`)}
                {@const isFrom = cell.x === from.x && cell.y === from.y}
                {@const isTo = cell.x === to.x && cell.y === to.y}
                <div class="cell" class:cell--used={isFrom || isTo}>
                  {#key playKey}
                    {#if isFrom && strikerDto}
                      <span class="figure" style:--fit={aspectOf(strikerDto)} style={play?.striker}>
                        <BattleCard
                          card={strikerDto}
                          {frames}
                          owned={true}
                          transition={false}
                          interactive={false}
                        />
                      </span>
                    {:else if isTo && targetDto}
                      <span class="figure" style:--fit={aspectOf(targetDto)} style={play?.target}>
                        <BattleCard
                          card={targetDto}
                          {frames}
                          owned={true}
                          transition={false}
                          interactive={false}
                          hurt={woundOn && previewStruck === 'bruise' ? (previewHit?.remain ?? 1) : 1}
                          wearSeed={3}
                          struck={woundOn && playing ? previewStruck : null}
                          scrap={hold === null &&
                          woundOn &&
                          playing &&
                          previewStruck === 'bruise' &&
                          previewHit
                            ? {
                                blow: previewHit.blow,
                                remain: previewHit.remain,
                                seed: previewHit.seed,
                              }
                            : null}
                        />
                      </span>
                    {/if}
                  {/key}
                </div>
              {/each}
            </div>
            {#key playKey}
              <BattleMotionStage motes={play?.motes ?? []} />
            {/key}
          </div>
        </div>

        <!-- ── Стенд: то, на чём смотрят, а не то, что правят ─────────── -->
        <div class="stage-bar">
          <button type="button" class="btn btn--do" onclick={playTurn}
            >▶ {$t('adminMotionsTurnPlay')}</button
          >
          <label class="inline">
            <input type="checkbox" bind:checked={looping} />
            {$t('adminMotionsLoop')}
          </label>
          {#if hold !== null}
            <button type="button" class="btn btn--hold" onclick={() => (hold = null)}
              >{$t('adminMotionsHold')} {hold}{$t('adminMotionsMs')} ×</button
            >
          {/if}
          <span class="bar-gap"></span>
          <label class="inline">
            {$t('adminMotionsZoom')}
            <input type="range" min="1" max="4" step="0.25" bind:value={zoom} />
            <span class="tabular-nums">{zoom.toFixed(2).replace(/\.?0+$/, '')}×</span>
          </label>
          <label class="inline">
            {$t('adminMotionsReach')}
            <input type="range" min="1" max={DEPTH - 1} bind:value={reach} onpointerup={bump} />
            <span class="tabular-nums">{reach}</span>
          </label>
          <label class="inline">
            <input type="checkbox" bind:checked={along} />
            {$t('adminMotionsAlong')}
          </label>
          <label class="inline">
            {$t('adminMotionsStrikerCard')}
            <select bind:value={strikerCard}>
              {#each cards as c (c.id)}
                <option value={c.id}>{$lang === 'ru' ? c.titleRu : c.titleEn}</option>
              {/each}
            </select>
          </label>
          <label class="inline">
            {$t('adminMotionsTargetCard')}
            <select bind:value={targetCard}>
              {#each cards as c (c.id)}
                <option value={c.id}>{$lang === 'ru' ? c.titleRu : c.titleEn}</option>
              {/each}
            </select>
          </label>
        </div>

        <!-- ── Партитура ──────────────────────────────────────────────── -->
        <div class="score-desk" onpointerdowncapture={mark} onfocusincapture={mark}>
          <div class="score-head">
            <p class="kicker">{$t('adminMotionsScore')}</p>
            <label class="score-beat">
              {$t('adminMotionsBeat')}
              <input
                type="range"
                min={Math.max(80, barsMs)}
                max={MOTION_MS_MAX}
                step="20"
                value={span}
                onpointerdown={() => (pinned = true)}
                oninput={(e) => setBeat(Number(e.currentTarget.value))}
                onpointerup={() => {
                  pinned = false;
                  bump();
                }}
              />
              <input
                type="number"
                min={Math.max(80, barsMs)}
                max={MOTION_MS_MAX}
                step="10"
                value={span}
                onchange={(e) => {
                  setBeat(Number(e.currentTarget.value));
                  bump();
                }}
              />
              <span>{$t('adminMotionsMs')}</span>
            </label>
          </div>
          <div class="score">
            <div class="score-overlay">
              {#if woundAt > 0}
                <i
                  class="score-wound"
                  style="left:{scorePct(woundAt)}%"
                  title="{$t('adminMotionsScoreWound')} {woundAt}{$t('adminMotionsMs')}"
                ></i>
              {/if}
              {#if hold !== null}
                <i class="score-hold" style="left:{scorePct(hold)}%"></i>
              {:else if span > 0 && !pinned}
                {#key playKey}
                  <i class="score-play" style="--play-end:{scorePct(span)}%;--play-ms:{span}ms"></i>
                {/key}
              {/if}
            </div>
            <!-- Линейка — она же ручка времени: тянут по ней, сцена замирает. -->
            <div class="score-ruler">
              <span class="score-whom">{$t('adminMotionsHoldPull')}</span>
              <span class="score-shelf"></span>
              <span
                class="score-lane score-lane--ruler"
                class:score-lane--scrub={scrubbing}
                role="slider"
                tabindex="0"
                aria-label={$t('adminMotionsHoldNote')}
                aria-valuemin={0}
                aria-valuemax={scoreMs}
                aria-valuenow={hold ?? 0}
                title={$t('adminMotionsHoldNote')}
                onpointerdown={scrubDown}
                onpointermove={scrubMove}
                onpointerup={scrubUp}
                onpointercancel={scrubUp}
                onkeydown={holdKey}
              >
                {#each scoreTicks as t (t)}
                  <i
                    class="score-tick"
                    class:score-tick--label={t % 200 === 0}
                    style="left:{scorePct(t)}%">{t % 200 === 0 ? t : ''}</i
                  >
                {/each}
              </span>
            </div>
            {#each SCORE_LANES as lane (`${lane.whom}-${lane.kind}`)}
              {@const kin = laneKin(lane.whom, lane.kind)}
              {@const laneId = `${lane.whom}-${lane.kind}`}
              <div
                class="score-row"
                class:score-row--on={Boolean(
                  gesture && gesture.whom === lane.whom && scoreKind(gesture) === lane.kind,
                )}
                class:score-row--over={scoreOver === laneId}
              >
                <span class="score-whom">{lane.first ? $t(WHOM_KEY[lane.whom]) : ''}</span>
                <span class="score-shelf"
                  >{lane.whom === 'flight' || lane.whom === 'field'
                    ? ''
                    : $t(KIND_KEY[lane.kind])}</span
                >
                <span
                  class="score-lane"
                  class:score-lane--art={lane.kind === 'art'}
                  role="group"
                  aria-label="{$t(WHOM_KEY[lane.whom])} · {$t(KIND_KEY[lane.kind])}"
                  onpointerdown={(e) => laneDown(e, lane.whom, lane.kind)}
                  ondragover={(e) => {
                    if (lane.kind !== 'art') return;
                    e.preventDefault();
                    scoreOver = laneId;
                  }}
                  ondragleave={() => {
                    if (scoreOver === laneId) scoreOver = null;
                  }}
                  ondrop={(e) => {
                    if (lane.kind !== 'art') return;
                    e.preventDefault();
                    scoreOver = null;
                    const file = e.dataTransfer?.files?.[0];
                    if (file) void putLaneFile(lane.whom, file);
                  }}
                >
                  {#each kin as { g, i } (i)}
                    <button
                      type="button"
                      class="bar"
                      class:bar--on={gestureAt === i}
                      class:bar--slot={isSlot(g)}
                      class:bar--light={lane.kind === 'light'}
                      class:bar--art={lane.kind === 'art' && Boolean(g.image)}
                      style="left:{scorePct(g.at)}%;width:{scorePct(g.dur)}%;{g.image
                        ? `background-image:url('${resolveMediaUrl(g.image) ?? g.image}')`
                        : ''}"
                      onpointerdown={(e) => {
                        e.stopPropagation();
                        barDown(e, i, 'at');
                      }}
                      onpointermove={barMove}
                      onpointerup={barUp}
                      onpointercancel={barUp}
                      onkeydown={(e) => nudgeBar(e, i)}
                      title="{barWord(g)} {g.at}–{g.at + g.dur}"
                    >
                      <span class="bar-label">{barWord(g)}</span>
                      <i
                        class="bar-grip"
                        onpointerdown={(e) => {
                          e.stopPropagation();
                          barDown(e, i, 'dur');
                        }}
                      ></i>
                    </button>
                  {/each}
                  {#if !kin.length}
                    <span class="score-empty">{$t('adminMotionsScoreEmpty')}</span>
                  {/if}
                </span>
              </div>
            {/each}
          </div>
          <details class="hint-fold">
            <summary>{$t('adminBattlesHintOpen')}</summary>
            <p class="hint">{$t('adminMotionsBeatNote')}</p>
          </details>
        </div>

        <!-- ── Ящики: источники, а не рабочая поверхность ─────────────── -->
        <div class="drawers" onpointerdowncapture={mark} onfocusincapture={mark}>
          <details class="box" bind:open={stripOpen}>
            <summary>
              {$t('adminMotionsStrip')}
              <span class="muted">
                {stripCells.filter((c) => c.src).length}/{STRIP_FRAMES}
                {#if stripBusy}
                  · {stripWork === 'cut'
                    ? $t('adminMotionsStripCutting')
                    : $t('adminMotionsStripBusy')}
                {/if}
              </span>
            </summary>
            <details class="hint-fold">
              <summary>{$t('adminBattlesHintOpen')}</summary>
              <p class="hint">{$t('adminMotionsStripNote')}</p>
            </details>
            <div class="strip-cells">
              {#each stripCells as cell, i (i)}
                <div
                  class="strip-cell"
                  role="group"
                  aria-label="{$t('adminMotionsStripPose')} {i + 1}"
                  class:strip-cell--on={Boolean(cell.src)}
                  class:strip-cell--held={stripAt === i && hand === 'frame'}
                  class:strip-cell--over={stripOver === i}
                  ondragover={(e) => {
                    e.preventDefault();
                    stripOver = i;
                  }}
                  ondragleave={() => {
                    if (stripOver === i) stripOver = null;
                  }}
                  ondrop={(e) => {
                    e.preventDefault();
                    stripOver = null;
                    const files = e.dataTransfer?.files;
                    if (files?.length) putCellFiles(files, i);
                  }}
                >
                  {#if cell.src}
                    <div
                      class="strip-face"
                      onpointerdown={(e) => poseDown(e, i)}
                      onpointermove={poseMove}
                      onpointerup={poseUp}
                      onpointercancel={poseUp}
                      onwheel={(e) => {
                        e.preventDefault();
                        poseCell(i, { turn: cell.turn + (e.deltaY > 0 ? 5 : -5) });
                        scheduleCompose();
                      }}
                    >
                      <img src={cell.src} alt="" style={poseStyle(cell)} />
                    </div>
                  {:else}
                    <button type="button" class="strip-face" onclick={() => holdCell(i)}>
                      <span class="strip-n">{i + 1}</span>
                    </button>
                  {/if}
                </div>
              {/each}
            </div>
            <div class="row mt-strip">
              <label class="btn">
                {$t('adminMotionsStripCut')}
                <input
                  type="file"
                  accept="image/*"
                  class="hidden"
                  disabled={stripBusy}
                  onchange={(e) => {
                    const file = e.currentTarget.files?.[0];
                    if (file) void cutPreparedStrip(file);
                    e.currentTarget.value = '';
                  }}
                />
              </label>
              <label class="btn">
                {$t('adminMotionsStripSix')}
                <input
                  type="file"
                  accept="image/*"
                  multiple
                  class="hidden"
                  disabled={stripBusy}
                  onchange={(e) => {
                    if (e.currentTarget.files?.length) void putCellFiles(e.currentTarget.files, 0);
                    e.currentTarget.value = '';
                  }}
                />
              </label>
            </div>
          </details>

          {#if faceKind === 'mine'}
            <details class="box" bind:open={namesOpen}>
              <summary>
                {motionTitle(held, $lang)}
                <span class="muted">· {$t(OCCASION_KEY[held.occasion])}</span>
              </summary>
              <div class="names">
                <label>
                  <span class="kicker">{$t('adminMotionsNameRu')}</span>
                  <input bind:value={held.nameRu} />
                </label>
                <label>
                  <span class="kicker">{$t('adminMotionsNameEn')}</span>
                  <input bind:value={held.nameEn} />
                </label>
                <label>
                  <span class="kicker">{$t('adminMotionsOccasion')}</span>
                  <select bind:value={held.occasion}>
                    {#each MOTION_OCCASIONS as o (o)}
                      <option value={o}>{$t(OCCASION_KEY[o])}</option>
                    {/each}
                  </select>
                </label>
                <button type="button" class="btn" onclick={duplicate}
                  >{$t('adminMotionsCopy')}</button
                >
                <button type="button" class="btn btn--drop" onclick={dropHeld}
                  >{$t('adminMotionsDrop')}</button
                >
              </div>
              <details class="hint-fold">
                <summary>{$t('adminBattlesHintOpen')}</summary>
                <p class="hint">{$t('adminMotionsHint')}</p>
              </details>
              <p class="kicker mt">{$t('adminMotionsWornBy')}</p>
              {#if wearers.length}
                <ul class="worn">
                  {#each wearers as w, i (`${w.name}-${w.occasion}-${i}`)}
                    <li>{w.name} · {$t(OCCASION_KEY[w.occasion])}</li>
                  {/each}
                </ul>
              {:else}
                <p class="hint">{$t('adminMotionsWornNone')}</p>
              {/if}
            </details>
          {/if}
        </div>
      </section>

      <!-- ── Одна рука: кадр ИЛИ жест, и настройки только у неё ───────── -->
      <aside class="side" onpointerdowncapture={mark} onfocusincapture={mark}>
        {#if hand === 'frame' && stripHeld}
          <p class="kicker">{$t('adminMotionsStripPose')} {stripAt + 1}</p>
          <details class="hint-fold">
            <summary>{$t('adminBattlesHintOpen')}</summary>
            <p class="hint">{$t('adminMotionsStripPoseNote')}</p>
          </details>
          <div class="row">
            <button type="button" class="btn" onclick={() => (picking = stripAt)}
              >{$t('adminMotionsFromStore')}</button
            >
            <label class="btn">
              {$t('adminMotionsUpload')}
              <input
                type="file"
                accept="image/*"
                class="hidden"
                onchange={(e) => {
                  const file = e.currentTarget.files?.[0];
                  if (file) void putCellFile(stripAt, file);
                  e.currentTarget.value = '';
                }}
              />
            </label>
            {#if stripHeld.src}
              <button type="button" class="btn" onclick={() => spreadSrc(stripAt)}
                >{$t('adminMotionsStripAll')}</button
              >
              <button type="button" class="btn btn--drop" onclick={() => clearCell(stripAt)}
                >×</button
              >
            {/if}
          </div>
          {#if stripHeld.src}
            <label class="art-size">
              {$t('adminMotionsStripTurn')}
              <span class="tabular-nums">{Math.round(stripHeld.turn)}°</span>
              <input
                type="range"
                min={-STRIP_TURN_MAX}
                max={STRIP_TURN_MAX}
                value={stripHeld.turn}
                oninput={(e) => poseCell(stripAt, { turn: Number(e.currentTarget.value) })}
                onpointerup={scheduleCompose}
              />
            </label>
            <label class="art-size">
              {$t('adminMotionsStripScale')}
              <span class="tabular-nums">{Math.round(stripHeld.size)}</span>
              <input
                type="range"
                min="20"
                max={STRIP_SCALE_MAX}
                value={stripHeld.size}
                oninput={(e) => poseCell(stripAt, { size: Number(e.currentTarget.value) })}
                onpointerup={scheduleCompose}
              />
            </label>
            <div class="nudge">
              <label>
                {$t('adminMotionsNudgeX')}
                <input
                  type="number"
                  min={-STRIP_POSE_MAX}
                  max={STRIP_POSE_MAX}
                  value={Math.round(stripHeld.x)}
                  onchange={(e) => {
                    poseCell(stripAt, { x: Number(e.currentTarget.value) });
                    scheduleCompose();
                  }}
                />
              </label>
              <label>
                {$t('adminMotionsNudgeY')}
                <input
                  type="number"
                  min={-STRIP_POSE_MAX}
                  max={STRIP_POSE_MAX}
                  value={Math.round(stripHeld.y)}
                  onchange={(e) => {
                    poseCell(stripAt, { y: Number(e.currentTarget.value) });
                    scheduleCompose();
                  }}
                />
              </label>
            </div>
          {/if}
          <button type="button" class="btn mt" onclick={() => (hand = 'gesture')}
            >{$t('adminMotionsToGesture')}</button
          >
        {:else}
          <p class="kicker">{$t('adminMotionsGesture')}</p>
          {#if gesture}
            <p class="hint">{barWord(gesture)} · {$t(WHOM_KEY[gesture.whom])}</p>
          {:else}
            <p class="hint">{$t('adminMotionsNoGesture')}</p>
          {/if}

          {#if gesture && (gesture.whom === 'flight' || gesture.whom === 'field')}
            <p class="hint">{$t('adminMotionsNoBodyHere')}</p>
          {:else if gesture?.whom === 'striker' || gesture?.whom === 'target' || track === 'striker' || track === 'target'}
            <!-- Колодец. Наведение показывает слово на сцене, не применяя его. -->
            {#each WELL as group (group.key)}
              <p class="well-label">{$t(group.key)}</p>
              <div class="well">
                {#each group.bodies as b (b)}
                  <button
                    type="button"
                    class="chip"
                    class:chip--on={activeBody.move === b || activeBody.light === b}
                    onpointerenter={() => (tasting = b)}
                    onpointerleave={() => (tasting = null)}
                    onfocus={() => (tasting = b)}
                    onblur={() => (tasting = null)}
                    onclick={() => putBody(b)}>{$t(BODY_KEY[b])}</button
                  >
                {/each}
              </div>
            {/each}
            <details class="hint-fold">
              <summary>{$t('adminBattlesHintOpen')}</summary>
              <p class="hint">{$t('adminMotionsTasteNote')}</p>
              <p class="hint">{$t('adminMotionsLightNote')}</p>
            </details>
          {/if}

          {#if gesture}
            <div class="timing">
              <p class="kicker">{$t('adminMotionsWhen')}</p>
              <div class="nudge">
                <label>
                  {$t('adminMotionsAt')}
                  <input
                    type="number"
                    min="0"
                    max={MOTION_MS_MAX}
                    bind:value={gesture.at}
                    onchange={bump}
                  />
                </label>
                <label>
                  {$t('adminMotionsDur')}
                  <input
                    type="number"
                    min="0"
                    max={MOTION_MS_MAX}
                    bind:value={gesture.dur}
                    onchange={bump}
                  />
                </label>
              </div>
              <details class="hint-fold">
                <summary>{$t('adminBattlesHintOpen')}</summary>
                <p class="hint">{$t('adminMotionsWhenNote')}</p>
              </details>
            </div>
          {/if}

          <!-- Единственная дверь на склад. Их было четыре, и величина правилась
               двумя разными ползунками с одинаковым ходом. -->
          {#if gesture && carriesArt(gesture)}
            <div class="art">
              <p class="kicker">{$t('adminMotionsArt')}</p>
              <div class="row">
                <button type="button" class="btn" onclick={openStore}
                  >{$t('adminMotionsFromStore')}</button
                >
                <label class="btn">
                  {uploading ? $t('adminMotionsUploading') : $t('adminMotionsUpload')}
                  <input
                    type="file"
                    accept="image/*"
                    class="hidden"
                    onchange={(e) => {
                      const file = e.currentTarget.files?.[0];
                      if (file) void upload(file);
                      e.currentTarget.value = '';
                    }}
                  />
                </label>
                {#if gesture.image}
                  <button type="button" class="btn btn--drop" onclick={() => setArt('')}
                    >{$t('adminMotionsClearSlot')}</button
                  >
                {/if}
              </div>
              {#if !gesture.image}
                <details class="hint-fold">
                  <summary>{$t('adminBattlesHintOpen')}</summary>
                  <p class="hint">{$t('adminMotionsSlotNote')}</p>
                </details>
              {:else}
                <label class="art-size">
                  {$t('adminMotionsSize')} <span class="tabular-nums">{gesture.size}</span>
                  <input
                    type="range"
                    min="8"
                    max={GESTURE_SIZE_MAX}
                    value={gesture.size}
                    onpointerdown={() => (pinned = true)}
                    oninput={(e) => setSize(Number(e.currentTarget.value))}
                    onpointerup={() => {
                      pinned = false;
                      bump();
                    }}
                  />
                </label>
                <details class="hint-fold">
                  <summary>{$t('adminBattlesHintOpen')}</summary>
                  <p class="hint">{$t('adminMotionsSizeNote')}</p>
                </details>
                <div class="nudge">
                  <label>
                    {$t('adminMotionsNudgeX')}
                    <input
                      type="number"
                      min={-GESTURE_NUDGE_MAX}
                      max={GESTURE_NUDGE_MAX}
                      bind:value={gesture.nudgeX}
                      onchange={bump}
                    />
                  </label>
                  <label>
                    {$t('adminMotionsNudgeY')}
                    <input
                      type="number"
                      min={-GESTURE_NUDGE_MAX}
                      max={GESTURE_NUDGE_MAX}
                      bind:value={gesture.nudgeY}
                      onchange={bump}
                    />
                  </label>
                </div>
                <label class="block">
                  {$t('adminMotionsFrames')}
                  <input
                    type="number"
                    min="1"
                    max={MOTION_FRAMES_MAX}
                    value={gesture.frames}
                    onchange={(e) => setFrames(Number(e.currentTarget.value))}
                  />
                </label>
                <details class="hint-fold">
                  <summary>{$t('adminBattlesHintOpen')}</summary>
                  <p class="hint">{$t('adminMotionsFramesNote')}</p>
                </details>
                {#if gesture.frames === STRIP_FRAMES}
                  <button
                    type="button"
                    class="btn"
                    onclick={() => {
                      stripOpen = true;
                      hand = 'frame';
                    }}>{$t('adminMotionsToStrip')}</button
                  >
                {/if}
                <label class="block">
                  {$t('adminMotionsTurn')}
                  <select bind:value={gesture.turn} onchange={bump}>
                    {#each GESTURE_TURNS as v (v)}
                      <option value={v}>{$t(TURN_KEY[v])}</option>
                    {/each}
                  </select>
                </label>
                <label class="block">
                  {$t('adminMotionsFade')}
                  <select bind:value={gesture.fade} onchange={bump}>
                    {#each GESTURE_FADES as v (v)}
                      <option value={v}>{$t(FADE_KEY[v])}</option>
                    {/each}
                  </select>
                </label>
                <label class="block">
                  {$t('adminMotionsLayer')} <span class="tabular-nums">{gesture.layer}</span>
                  <input
                    type="range"
                    min="1"
                    max={GESTURE_LAYERS}
                    bind:value={gesture.layer}
                    onpointerup={bump}
                  />
                </label>
              {/if}
            </div>
          {/if}

          {#if gesture && faceKind === 'mine'}
            <button
              type="button"
              class="btn btn--drop mt"
              onclick={() => dropGesture(gestureAt)}
              >× {barWord(gesture)}</button
            >
          {/if}
        {/if}

        {#if faceKind !== 'mine'}
          <p class="hint mt">{$t('adminMotionsArtCopyNote')}</p>
        {/if}
        {#if complaint}
          <p class="warn mt">{complaint}</p>
        {/if}
      </aside>
    </div>
  {/if}
</div>

{#if picking !== null}
  <BattleAssetPicker
    role="motion"
    onPick={(asset) => {
      if (picking === 'all') setArt(asset.url);
      else if (typeof picking === 'number') putCell(picking, asset.url);
      picking = null;
    }}
    onClose={() => (picking = null)}
  />
{/if}

<style>
  /* Размеры набора — одним местом. Стол был набран целиком десятью и
     одиннадцатью пикселями: на широком экране это не сдержанность, а мелко. */
  .desk {
    --tiny: 11px;
    --small: 12px;
    --body: 13px;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: #f8f1e7;
    color: #34251c;
  }

  /* ── Шапка ─────────────────────────────────────────────────────────────
     Три колонки чипов стояли всегда и занимали полосу во всю ширину. Ящик —
     то, чем работают каждый день; дом и заготовки берут раз в неделю, и они
     уехали за одну кнопку. */
  .faces {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 0.75rem 1rem;
    padding: 0.5rem 0.85rem;
    border-bottom: 1px solid rgba(52, 37, 28, 0.12);
  }

  .ready {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem 1.5rem;
    padding: 0.5rem 0.85rem 0.7rem;
    background: rgba(52, 37, 28, 0.04);
    border-bottom: 1px solid rgba(52, 37, 28, 0.12);
  }

  .ready-take {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    justify-content: flex-end;
    max-width: 16rem;
  }

  .face-col {
    min-width: 0;
    flex: 1;
  }

  .face-save {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-left: auto;
  }

  .face-gap {
    width: 0.75rem;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    margin-top: 0.25rem;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.25rem 0.5rem;
    font-size: var(--small);
    border: 1px solid rgba(52, 37, 28, 0.18);
    background: transparent;
    color: inherit;
    cursor: pointer;
  }

  .chip-art {
    flex-shrink: 0;
    width: 1.85rem;
    height: 0.75rem;
    background: rgba(52, 37, 28, 0.06) center / contain no-repeat;
  }

  /* Повод — то, чем движение цепляется за карту; без него два «Секира» в
     ящике не отличаются ничем. */
  .chip-when {
    font-size: 9px;
    letter-spacing: 0.04em;
    color: rgba(52, 37, 28, 0.5);
  }

  .chip--on {
    border-color: #c65f3c;
    background: rgba(198, 95, 60, 0.08);
  }

  .chip--on .chip-when {
    color: rgba(198, 95, 60, 0.85);
  }

  .chip--add {
    color: #c65f3c;
    font-size: var(--body);
    padding: 0.25rem 0.6rem;
  }

  .kicker {
    font-size: var(--tiny);
    text-transform: uppercase;
    letter-spacing: 0.14em;
    color: #6f3b24;
  }

  /* ── Тело ──────────────────────────────────────────────────────────────
     Сцена берёт всё оставшееся место и прибита; партитура под ней, тоже
     прибита; источники — в ящиках внизу. */
  .body {
    display: flex;
    flex: 1;
    min-height: 0;
  }

  .stage-wrap {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .stage-dock {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: grid;
    grid-template-columns: minmax(min-content, 1fr);
    align-content: center;
    justify-items: center;
    padding: 0.85rem 1rem;
    background: rgba(52, 37, 28, 0.02);
  }

  /* Крупнее — ШИРЕ, а не `transform`: ровно как на столе рамок. Карта тогда
     рисуется крупнее по-настоящему, а не растягивается. */
  .stage {
    position: relative;
    width: calc(var(--x) * 4.4rem * var(--zoom, 1));
    overflow: visible;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(var(--x), minmax(0, 1fr));
    gap: 2px;
  }

  .cell {
    position: relative;
    min-width: 0;
    aspect-ratio: 3 / 4;
    background: rgba(52, 37, 28, 0.04);
    border: 1px solid rgba(52, 37, 28, 0.08);
    overflow: visible;
  }

  .cell--used {
    background: rgba(52, 37, 28, 0.07);
  }

  .figure {
    display: grid;
    place-items: center;
    height: 100%;
    padding: 2px;
    overflow: visible;
    will-change: transform;
  }

  .figure > :global(.slot) {
    width: min(100%, calc(133.3333% * var(--fit, 0.714)));
  }

  .stage-bar {
    flex-shrink: 0;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem 0.9rem;
    padding: 0.45rem 1rem;
    border-top: 1px solid rgba(52, 37, 28, 0.12);
    font-size: var(--small);
  }

  .bar-gap {
    flex: 1;
    min-width: 0;
  }

  .inline {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }

  .inline input[type='range'] {
    width: 5.5rem;
  }

  .btn {
    display: inline-block;
    padding: 0.25rem 0.6rem;
    font-size: var(--small);
    border: 1px solid rgba(52, 37, 28, 0.22);
    background: transparent;
    color: inherit;
    cursor: pointer;
  }

  .btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .btn--on {
    border-color: #c65f3c;
    background: rgba(198, 95, 60, 0.08);
    color: #c65f3c;
  }

  .btn--do {
    border-color: rgba(111, 59, 36, 0.5);
    color: #6f3b24;
  }

  .btn--hold {
    border-color: #c65f3c;
    color: #c65f3c;
  }

  .btn--drop {
    border-color: rgba(143, 47, 34, 0.35);
    color: #8f2f22;
  }

  /* ── Партитура ─────────────────────────────────────────────────────── */
  .score-desk {
    flex-shrink: 0;
    border-top: 1px solid rgba(52, 37, 28, 0.12);
    padding: 0.5rem 1rem 0.6rem;
    background: #f8f1e7;
  }

  .score-head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem 1rem;
  }

  .score-beat {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: var(--small);
    min-width: 16rem;
    flex: 1 1 16rem;
  }

  .score-beat input[type='range'] {
    flex: 1;
    min-width: 8rem;
  }

  .score-beat input[type='number'] {
    width: 3.8rem;
    border: none;
    border-bottom: 1px solid rgba(52, 37, 28, 0.25);
    background: transparent;
    color: inherit;
    font: inherit;
  }

  .score {
    position: relative;
    margin-top: 0.3rem;
    --score-labels: 10.7rem;
  }

  .score-overlay {
    position: absolute;
    left: var(--score-labels);
    right: 0;
    top: 0;
    bottom: 0;
    pointer-events: none;
    z-index: 4;
  }

  .score-ruler,
  .score-row {
    display: grid;
    grid-template-columns: 5.4rem 4.6rem 1fr;
    gap: 0.35rem;
    align-items: center;
    font-size: var(--small);
  }

  .score-ruler {
    margin-bottom: 0.15rem;
    min-height: 1.05rem;
  }

  .score-row {
    margin-bottom: 0.2rem;
  }

  .score-row--on .score-whom,
  .score-row--on .score-shelf {
    color: #c65f3c;
  }

  .score-row--over .score-lane {
    outline: 1px solid #c65f3c;
    outline-offset: -1px;
  }

  .score-whom,
  .score-shelf {
    color: #6f3b24;
    line-height: 1.2;
  }

  .score-ruler .score-whom {
    font-size: 9px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: rgba(52, 37, 28, 0.42);
  }

  .score-shelf {
    font-size: var(--tiny);
    letter-spacing: 0.04em;
    text-transform: lowercase;
  }

  .score-lane {
    position: relative;
    height: 1.65rem;
    background: rgba(52, 37, 28, 0.06);
    min-width: 0;
  }

  .score-lane--art {
    background: rgba(52, 37, 28, 0.035);
  }

  /* Линейка — она же ручка времени. Была только рисунком: бегунок показывали,
     а взять его было нельзя, и остановиться на 270-й было негде. */
  .score-lane--ruler {
    height: 1.05rem;
    background: rgba(52, 37, 28, 0.03);
    cursor: ew-resize;
    touch-action: none;
  }

  .score-lane--ruler:hover {
    background: rgba(198, 95, 60, 0.07);
  }

  .score-lane--scrub {
    background: rgba(198, 95, 60, 0.12);
  }

  .score-tick {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 0;
    border-left: 1px solid rgba(52, 37, 28, 0.12);
    font-size: 9px;
    font-style: normal;
    color: rgba(52, 37, 28, 0.45);
    padding-left: 0.2rem;
    line-height: 1.05rem;
    pointer-events: none;
  }

  .score-tick--label {
    border-left-color: rgba(52, 37, 28, 0.28);
  }

  .score-wound {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 0;
    border-left: 1px dashed #c65f3c;
  }

  .score-play {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 1px;
    background: #c65f3c;
    animation: score-play var(--play-ms) linear forwards;
  }

  /* Остановленное время. Не бегунок, который бежит, а место, где держат. */
  .score-hold {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 1px;
    background: #c65f3c;
  }

  .score-hold::before {
    content: '';
    position: absolute;
    top: -2px;
    left: -3px;
    width: 7px;
    height: 7px;
    background: #c65f3c;
  }

  @keyframes score-play {
    from {
      left: 0;
    }
    to {
      left: var(--play-end);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .score-play {
      animation: none;
      left: 0;
      opacity: 0;
    }
  }

  .bar {
    position: absolute;
    top: 0.12rem;
    height: calc(100% - 0.24rem);
    min-width: 2.2rem;
    padding: 0;
    border: none;
    background: #6f3b24;
    color: #f8f1e7;
    overflow: hidden;
    cursor: grab;
    z-index: 1;
  }

  .bar--on {
    background: #c65f3c;
    z-index: 2;
  }

  .bar--light {
    background: #8a5a3a;
  }

  .bar--light.bar--on {
    background: #c65f3c;
  }

  .bar--slot {
    background: transparent;
    color: #6f3b24;
    outline: 1px dashed rgba(111, 59, 36, 0.5);
    outline-offset: -1px;
  }

  .bar--slot.bar--on {
    background: rgba(198, 95, 60, 0.12);
    color: #c65f3c;
  }

  .bar--art {
    background-color: #6f3b24;
    background-size: cover;
    background-position: center;
  }

  .bar-label {
    display: block;
    padding: 0 0.45rem 0 0.3rem;
    font-size: var(--tiny);
    line-height: 1.41rem;
    white-space: nowrap;
    overflow: hidden;
  }

  .bar-grip {
    position: absolute;
    right: 0;
    top: 0;
    width: 10px;
    height: 100%;
    cursor: ew-resize;
    background: rgba(248, 241, 231, 0.35);
  }

  .score-empty {
    padding: 0 0.45rem;
    color: rgba(52, 37, 28, 0.38);
    line-height: 1.65rem;
    font-size: var(--tiny);
    pointer-events: none;
  }

  /* ── Ящики ─────────────────────────────────────────────────────────────
     Полоса кадров стояла первой и занимала больше всех, а нужна раз в час. */
  .drawers {
    flex-shrink: 0;
    max-height: 42%;
    overflow-y: auto;
    border-top: 1px solid rgba(52, 37, 28, 0.12);
    padding: 0.35rem 1rem 0.6rem;
  }

  .box {
    border-bottom: 1px solid rgba(52, 37, 28, 0.08);
    padding-bottom: 0.35rem;
  }

  .box summary {
    padding: 0.3rem 0;
    font-size: var(--small);
    color: #6f3b24;
    cursor: pointer;
    list-style-position: outside;
  }

  .strip-cells {
    display: grid;
    grid-template-columns: repeat(6, minmax(0, 1fr));
    gap: 0;
    margin-top: 0.35rem;
    border: 1px solid rgba(52, 37, 28, 0.18);
    max-width: 34rem;
  }

  .strip-cell {
    position: relative;
    min-width: 0;
    border-right: 1px solid rgba(52, 37, 28, 0.12);
    background: rgba(52, 37, 28, 0.04);
  }

  .strip-cell:last-child {
    border-right: none;
  }

  .strip-cell--on {
    background: rgba(52, 37, 28, 0.07);
  }

  .strip-cell--held,
  .strip-cell--over {
    outline: 1px solid #c65f3c;
    outline-offset: -1px;
  }

  .strip-face {
    display: grid;
    place-items: center;
    width: 100%;
    aspect-ratio: 1;
    overflow: hidden;
    cursor: pointer;
    border: none;
    background: none;
    padding: 0;
  }

  .strip-cell--on .strip-face {
    cursor: grab;
  }

  .strip-face img {
    width: 100%;
    height: 100%;
    object-fit: contain;
    pointer-events: none;
    transform-origin: center center;
  }

  .strip-n {
    font-size: 1.1rem;
    color: rgba(52, 37, 28, 0.28);
  }

  .mt-strip {
    margin-top: 0.4rem;
  }

  /* Чужая бумага: дом и заготовки правятся только копией, и об этом должен
     говорить стол собой, а не строка в десять пикселей внизу. */
  .body--locked .score-lane,
  .body--locked .strip-cells {
    background-image: repeating-linear-gradient(
      45deg,
      rgba(52, 37, 28, 0.05) 0 4px,
      transparent 4px 9px
    );
  }

  /* ── Инспектор: одна рука ──────────────────────────────────────────── */
  .side {
    width: 23rem;
    flex-shrink: 0;
    border-left: 1px solid rgba(52, 37, 28, 0.12);
    overflow-y: auto;
    padding: 0.75rem;
    font-size: var(--body);
  }

  .names {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    align-items: end;
    margin: 0.35rem 0 0.5rem;
  }

  .names input,
  .names select,
  .stage-bar select,
  .art input,
  .art select {
    border: none;
    border-bottom: 1px solid rgba(52, 37, 28, 0.25);
    background: transparent;
    font-size: var(--body);
    padding: 0.15rem 0;
    color: inherit;
  }

  .well {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    margin-bottom: 0.35rem;
  }

  .well-label {
    margin: 0.45rem 0 0.15rem;
    font-size: var(--tiny);
    color: rgba(52, 37, 28, 0.55);
  }

  .hint,
  .note,
  .muted {
    font-size: var(--tiny);
    line-height: 1.4;
    color: rgba(52, 37, 28, 0.55);
  }

  .hint-fold {
    margin: 0.45rem 0 0;
  }
  .hint-fold summary {
    font-size: var(--tiny);
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: rgba(52, 37, 28, 0.55);
    cursor: pointer;
  }
  .hint-fold .hint {
    margin-top: 0.45rem;
  }

  .warn {
    font-size: var(--tiny);
    color: #c65f3c;
  }

  .art-size {
    display: block;
    margin-top: 0.35rem;
    font-size: var(--small);
  }

  .art-size input[type='range'] {
    display: block;
    width: 100%;
    margin-top: 0.2rem;
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
  }

  .nudge {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.4rem;
    font-size: var(--small);
  }

  .nudge input {
    width: 100%;
    border: none;
    border-bottom: 1px solid rgba(52, 37, 28, 0.25);
    background: transparent;
    font: inherit;
    color: inherit;
  }

  .block {
    display: block;
    margin-top: 0.4rem;
    font-size: var(--small);
  }

  .timing,
  .art {
    margin-top: 0.75rem;
    padding-top: 0.5rem;
    border-top: 1px solid rgba(52, 37, 28, 0.1);
  }

  .mt {
    margin-top: 0.85rem;
  }

  .worn {
    margin-top: 0.25rem;
    font-size: var(--small);
    line-height: 1.45;
  }

  .tabular-nums {
    font-variant-numeric: tabular-nums;
  }

  .hidden {
    display: none;
  }
</style>
