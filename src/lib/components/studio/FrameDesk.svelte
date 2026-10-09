<script lang="ts">
  // Стол резчика — вкладка «Рамки», вынутая из админки целиком.
  //
  // Вынута ради студии: тот же стол должен работать и вне `/admin`, под
  // обычной сессией человека. Поэтому здесь НЕТ ни одного обращения к `api` —
  // всё, что уходит наружу, приходит пропсами (`uploadArt`, `importMedia`,
  // `pickFromStore`, `save`). Админка передаёт админский путь, студия — свой.
  //
  // У гостя лишнее прячется не флагом, а ОТСУТСТВИЕМ функции: не дан `presets`
  // — нет и ящика нарядов. Флаг забывают; отсутствующий проп — нет.
  //
  // Состояние, которое стол делит с хозяином (`frames`, отмена, ящик нарядов),
  // остаётся у хозяина и приходит сюда `$bindable`: вкладка «Лицо карты»
  // правит те же самые рамы, и второй их копии в доме быть не должно.
  import { onMount } from "svelte";
  import { t, lang, type TranslationKey } from "$lib/i18n";
  import {
    BADGE_SCALE_MIN,
    BADGE_SCALE_MAX,
    CARD_WIDTHS,
    DEFAULT_ASPECT,
    FRAME_MODES,
    KIND_SIDES,
    LAYOUTS,
    SLICE_FITS,
    SLICE_GROW_MAX,
    SLICE_KIND,
    SLICE_KINDS,
    SLICE_LAYERS,
    SLICE_SLOTS,
    SLICE_TURNS,
    applyInsetDelta,
    clampScale,
    cardTallAt,
    defaultSlices,
    dressWindowMissing,
    frameName,
    freeMarkOf,
    kindOf,
    livePiece,
    setFreeMark,
    newOrnament,
    pickImageFile,
    sliceSigns,
    widthShow,
    type InsetKey,
  } from "$lib/battles";
  import { SITE_FONTS } from "$lib/fonts";
  import { selectOnFocus, blurOnWheel } from "$lib/utils/fields";
  import BattleCard from "$lib/components/BattleCard.svelte";
  import BattleIcon from "$lib/components/BattleIcon.svelte";
  import BattleFramePicker from "$lib/components/admin/BattleFramePicker.svelte";
  import BattleFrameFace from "$lib/components/BattleFrameFace.svelte";
  import type {
    BattleAssetRole,
    BattleCard as BattleCardDto,
    BattleFrame,
    BattleFramePreset,
    SliceFit,
    SliceKind,
    SliceOrnament,
    SlicePiece,
    SliceSide,
    SliceSlot,
    SliceTurn,
    SheetBand,
    SheetSlot,
  } from "$lib/types/api";

  interface Props {
    /** Рамы дома. Правятся на месте — это те же объекты, что у хозяина. */
    frames: BattleFrame[];
    frameIndex: number;
    /** Деталь в руке: что двигают мышью прямо на карте. */
    sliceHeld: { id: string; side: SliceSide } | null;
    /** Строка описи в руке. Её держит вкладка «Лицо карты», но полоса под
     *  рукой обводится и здесь. */
    rowHeld: SheetSlot | null;
    /** Где приколота панель детали. Помнится между заходами. */
    barPin: { x: number; y: number } | null;
    pokedAt: { x: number; y: number } | null;
    uploading: boolean;
    /** Карта-манекен: на ком примеряют раму. Считает хозяин — у него `draft`
     *  и полка карт, а у стола их нет и быть не должно. */
    sample: BattleCardDto;
    /** Кто носит этот чин: все, в своём наряде, переодетые расой. Не дан —
     *  полки «кто это носит» на столе нет вовсе: у гостя нет ни полки карт, ни
     *  чинов, и пустая полка сказала бы ему неправду. */
    worn?: {
      mine: BattleCardDto[];
      own: BattleCardDto[];
      byRace: BattleCardDto[];
      plain: BattleCardDto[];
    };
    saving: boolean;
    dirty: boolean;
    history: unknown[];
    ahead: unknown[];
    /** Ящик нарядов. Не дан — ящика на столе нет. */
    /** Ящик нарядов. Не дан — ящика на столе нет вовсе (студия). */
    presets?: BattleFramePreset[];
    presetOpen?: string | null;
    presetName?: string;
    /** Своя мерка: рама, как её прислал сервер, — чтобы видеть несохранённое. */
    presetWorn?: BattleFramePreset | null;
    presetChanged?: boolean;
    /** Поле имени наряда — по нему бьют фокусом после «сохранить как». */
    frameNameBox?: HTMLInputElement | null;
    /** Пометить начало жеста — для отмены. */
    mark: () => void;
    /** Не дан — своей кнопки сохранения у стола нет: у студии она в шапке. */
    save?: () => void;
    flash: (text: string, ms?: number) => void;
    titleOf: (card: BattleCardDto) => string;
    moveRow: (slot: SheetSlot, band: SheetBand, before: SheetSlot | null) => void;
    stepBack: () => void;
    stepOn: () => void;
    /** Дверь на склад: роль и куда положить выбранное. */
    pickFromStore: (role: BattleAssetRole, apply: (url: string) => void) => void;
    /** Загрузить деталь рамы: лоссовый WebP с альфой. */
    uploadArt: (file: File) => Promise<{ url: string; hasAlpha: boolean }>;
    /** Загрузить обычную картинку — бумагу под картой. */
    importMedia: (
      file: File,
      kind: "images" | "videos" | "audio",
      note: string,
    ) => Promise<{ url: string }>;
    onOpenCard?: (card: BattleCardDto) => void;
    keepFrameAsPreset?: () => void;
    keepFrameAsNew?: () => void;
    forgetPreset?: (preset: BattleFramePreset) => void;
    updateOpenPreset?: () => void;
    beginNewFrame?: () => void;
    wearPresetOnRank?: (preset: BattleFramePreset) => void;
  }

  let {
    frames = $bindable(),
    frameIndex = $bindable(),
    sliceHeld = $bindable(),
    rowHeld = $bindable(),
    barPin = $bindable(),
    pokedAt = $bindable(),
    uploading = $bindable(),
    presetOpen = $bindable(null),
    presetName = $bindable(""),
    frameNameBox = $bindable(null),
    sample,
    worn,
    saving,
    dirty,
    history,
    ahead,
    presets,
    presetWorn = null,
    presetChanged = false,
    mark,
    save,
    flash,
    titleOf,
    moveRow,
    stepBack,
    stepOn,
    pickFromStore,
    uploadArt,
    importMedia,
    onOpenCard = () => {},
    keepFrameAsPreset = () => {},
    keepFrameAsNew = () => {},
    forgetPreset = () => {},
    updateOpenPreset = () => {},
    beginNewFrame = () => {},
    wearPresetOnRank = () => {},
  }: Props = $props();

  /** A frame picture, kept transparent, stretched to the card's fixed ratio. */
  async function uploadFrameArt() {
    const file = await pickImageFile();
    if (!file) return;
    uploading = true;
    try {
      const art = await uploadArt(file);
      const frame = frames[frameIndex];
      frame.frameImage = art.url;
      // The card's ratio is fixed game-wide; the picture is stretched to fit
      // it, not the other way around, so different frame uploads can never
      // leave cards different shapes. The aspect slider still overrides it.
      //
      // `freeform` is a mode the keeper picked on purpose, before uploading
      // anything — the whole point of it is one ready-made illustration, and
      // guessing `overlay`/`behind` from the file's own alpha would silently
      // undo that choice on the very next re-upload.
      if (frame.frameMode !== "freeform") {
        if (art.hasAlpha) {
          frame.frameMode = "overlay";
        } else {
          // No hole in it: worn on top it would cover the card completely.
          frame.frameMode = "behind";
          flash($t("adminBattlesFrameNoAlpha"), 8000);
        }
      }
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      uploading = false;
    }
  }

  /** The same picture, taken off the shelf instead of uploaded fresh — a
   *  frame illustration is often reused across ranks or presets, and
   *  re-uploading the same file each time would just pile up copies of it.
   *  Unlike `uploadFrameArt`, this never guesses `frameMode` from the file's
   *  alpha: a shelf asset carries no such flag, and the keeper picking one
   *  has already chosen a mode on purpose. */
  function pickFrameArt() {
    pickFromStore("art", (url) => {
      frames[frameIndex].frameImage = url;
    });
  }

  /** The paper under the card. An ordinary photograph — no transparency needed. */
  async function uploadPaperArt() {
    const file = await pickImageFile();
    if (!file) return;
    uploading = true;
    try {
      const imported = await importMedia(
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
      const art = await uploadArt(file);
      frames[frameIndex].backImage = art.url;
    } catch (e) {
      flash(String(e), 6000);
    } finally {
      uploading = false;
    }
  }

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
      const art = await uploadArt(file);
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
      const art = await uploadArt(file);
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
    pickFromStore('accent', (url) => {
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

  /** Кратности стекла. Полутора среди них больше нет: оно стояло умолчанием
   *  ровно потому, что при 1× карта терялась, а «вписать» отвечает на это
   *  лучше и само. Лишняя кнопка на ленте стоит дороже редкой кратности. */
  const ZOOMS = [1, 2, 3, 4];

  /**
   * Ширина колонки. Тянется за ручку и помнится между заходами.
   *
   * Была жёсткой (27rem), и место на столе оказалось роздано наоборот: сцене
   * доставалось на полтысячи точек больше, чем занимает карта, а в колонке
   * ползунки и поля переносились по два в ряд и обрезали свои подписи. Кто
   * режет крупную резьбу, тому нужна сцена; кто правит числа — колонка; и
   * выбирать это должен тот, кто работает, а не тот, кто размечал.
   */
  const SIDE_MIN = 380;
  const SIDE_MAX = 640;
  let sideWide = $state(432);

  /**
   * Какую долю колонки занимает верстак детали.
   *
   * Была жёсткая доля, и она не могла быть верной: у детали без украшения
   * настроек на треть меньше, чем у украшения, а список бывает и из шести
   * строк, и из восемнадцати. Долю выбирает тот, кто работает, — как и ширину
   * колонки.
   */
  const PANE_MIN = 0.3;
  const PANE_MAX = 0.82;
  let paneShare = $state(0.66);

  /** Обе мерки стола в одной памяти: два ключа под две половины одной
   *  настройки разошлись бы на первом же забытом обновлении. */
  const DESK_KEY = "gotiga_battle_desk";

  onMount(() => {
    try {
      const put = JSON.parse(localStorage.getItem(DESK_KEY) || "null");
      if (put && Number.isFinite(put.side)) sideWide = clampSide(put.side);
      if (put && Number.isFinite(put.pane)) paneShare = clampPane(put.pane);
    } catch {
      /* стол просто останется домашних мерок */
    }
  });

  function keepDesk() {
    try {
      localStorage.setItem(
        DESK_KEY,
        JSON.stringify({ side: sideWide, pane: paneShare }),
      );
    } catch {
      /* не запомнилось — мерки живут до конца сеанса */
    }
  }

  function clampSide(px: number): number {
    return Math.min(SIDE_MAX, Math.max(SIDE_MIN, Math.round(px)));
  }

  function clampPane(share: number): number {
    return Math.min(PANE_MAX, Math.max(PANE_MIN, Math.round(share * 100) / 100));
  }

  /** Верстак тянут за ручку над ним. Считается долей КОЛОНКИ, а не точками:
   *  колонку тоже тянут, и доля переживает это сама. */
  let paneGrab = $state(false);
  let paneFrom = 0;
  let paneAt = 0;
  let asideBox = $state<HTMLElement | null>(null);
  function paneTake(event: PointerEvent & { currentTarget: HTMLElement }) {
    paneGrab = true;
    paneFrom = paneShare;
    paneAt = event.clientY;
    event.currentTarget.setPointerCapture(event.pointerId);
    event.preventDefault();
  }
  function paneDrag(event: PointerEvent) {
    if (!paneGrab || !asideBox?.clientHeight) return;
    paneShare = clampPane(
      paneFrom - (event.clientY - paneAt) / asideBox.clientHeight,
    );
  }
  function paneDrop(event: PointerEvent & { currentTarget: HTMLElement }) {
    if (!paneGrab) return;
    paneGrab = false;
    if (event.currentTarget.hasPointerCapture(event.pointerId))
      event.currentTarget.releasePointerCapture(event.pointerId);
    keepDesk();
  }

  /** Тянут за ручку на левой кромке колонки. Считается ПРИРАЩЕНИЕМ от того,
   *  где взяли, а не расстоянием до края окна: стол не обязан упираться в
   *  край — у студии над ним своя шапка комнаты, — и ширина, отмеренная от
   *  окна, врала бы ровно на неё. */
  let sideGrab = $state(false);
  let sideFrom = 0;
  let sideAt = 0;
  function sideTake(event: PointerEvent & { currentTarget: HTMLElement }) {
    sideGrab = true;
    sideFrom = sideWide;
    sideAt = event.clientX;
    event.currentTarget.setPointerCapture(event.pointerId);
    event.preventDefault();
  }
  function sideDrag(event: PointerEvent) {
    if (!sideGrab) return;
    sideWide = clampSide(sideFrom - (event.clientX - sideAt));
  }
  function sideDrop(event: PointerEvent & { currentTarget: HTMLElement }) {
    if (!sideGrab) return;
    sideGrab = false;
    if (event.currentTarget.hasPointerCapture(event.pointerId))
      event.currentTarget.releasePointerCapture(event.pointerId);
    keepDesk();
  }

  /** Сколько места отведено мерке слева. Одно число на обе половины: верхняя
   *  подпись отступает ровно на него, иначе мерка ширины встала бы не по краям
   *  карты, а по краям стола, и перестала бы что-либо мерить. */
  const RULER_GUTTER = 34;

  /**
   * Какой ширины карта на столе — и это НАСТОЯЩАЯ её ширина, а не величина
   * предпросмотра.
   *
   * До этого стол ставил карту в 320 px и умножал это число на увеличение:
   * 320 · 480 · 640 · 960 · 1280. Ни одно из них не та ширина, на которой
   * карта где-нибудь стоит, и резчик, сажавший уголок «на глаз по краю», не
   * мог узнать ни сколько это точек у гостя, ни какие строки описи на такой
   * карте вообще печатаются, — а от ширины зависит и то, и другое.
   *
   * Теперь ширина выбирается из тех трёх, на которых карта стоит в комнате
   * (`CARD_WIDTHS`), и лист взятия — по умолчанию: рамку строят там, где видно
   * всё, а проверяют на полке и в клетке, где видно не всё.
   */
  let stageWidth = $state<number>(CARD_WIDTHS[0]);

  /**
   * Во сколько раз стол увеличивает карту. СТЕКЛО, а не ширина: `zoom` не
   * трогает собственную ширину карты, поэтому контейнерные запросы, врезки и
   * кегль остаются ровно теми, какие будут у гостя, — крупнее делается только
   * то, что видит резчик.
   *
   * Не `transform`: тот увеличил бы картинку поверх раскладки, и стол перестал
   * бы под неё отводить место, а `getBoundingClientRect` под перетаскиванием
   * остался бы честным лишь наполовину. И не ширина контейнера, как было: та
   * увеличивала резьбу и кегль по-настоящему, но вместе с ними — и саму карту,
   * то есть меняла ответ на вопрос «что на ней напечатано».
   *
   * «Вписать» — не шестая кратность, а ОТКАЗ называть её: столько, сколько
   * нужно, чтобы карта поместилась целиком. Оно же умолчание, потому что стол
   * открывают, чтобы посмотреть на карту, а не чтобы сперва подобрать число,
   * при котором её видно.
   */
  let glass = $state<number | "fit">("fit");

  /** Высота карты при выбранной ширине: отношение сторон у каждого чина своё,
   *  и высота — не вторая ручка, а следствие первой. */
  let cardTall = $derived(
    cardTallAt(stageWidth, frames[frameIndex]?.aspect || DEFAULT_ASPECT),
  );

  /** Сколько места у сцены на самом деле. Меряется САМА сцена, а не окно:
   *  между ними стоят вкладки, известие комнаты, лента стола и колонка, и
   *  число, посчитанное от окна, врёт ровно на их сумму. */
  let stageWide = $state(0);
  let stageTall = $state(0);

  /** Что сцена тратит не на карту: свой отступ (`p-8`), мерка сверху, подпись
   *  снизу и жёлоб мерки слева. Названы числами, потому что мерить их в рантайме
   *  значило бы мерить то, что сам же и поставил. */
  const STAGE_PAD = 32;
  const RULER_TALL = 24;
  const CAPTION_TALL = 30;
  /** Запас, ради которого «вписать» не колеблется. Без него подогнанная точно
   *  в край карта вызывает полосу прокрутки, полоса отнимает ширину, ширина
   *  уменьшает подгонку, полоса пропадает — и так до бесконечности. */
  const FIT_SLACK = 14;

  /**
   * Во сколько раз карта влезает целиком.
   *
   * Заведено потому, что умолчание стола ГАРАНТИРОВАЛО обратное: над сценой
   * стояли четыре полосы, карта листа взятия под полуторным стеклом — 840 px
   * ростом, и подвал (насечки уровня, стоимость, сила) не показывался ни на
   * одном разумном экране. Увидеть карту целиком можно было, только угадав,
   * что для этого надо отказаться от увеличения.
   */
  let fitGlass = $derived.by(() => {
    const room = stageTall - STAGE_PAD * 2 - RULER_TALL - CAPTION_TALL - FIT_SLACK;
    const across = stageWide - STAGE_PAD * 2 - RULER_GUTTER - FIT_SLACK;
    if (room <= 0 || across <= 0) return 1;
    const fits = Math.min(room / cardTall, across / stageWidth);
    // До сотых: сцена меняет ширину на пиксель от полосы прокрутки колонки, и
    // незакруглённое число пересобирало бы карту на каждом таком пикселе.
    return Math.min(4, Math.max(0.25, Math.round(fits * 100) / 100));
  });

  /**
   * Ящик нарядов. Держится ссылкой ровно затем, чтобы ЗАКРЫВАТЬСЯ: наряд
   * достают, чтобы посмотреть на него на карте, а открытый ящик стоит поверх
   * ленты и половины сцены — то есть поверх того, ради чего его открывали.
   */
  let presetDrawer = $state<HTMLDetailsElement | null>(null);
  function shutDrawer() {
    if (presetDrawer) presetDrawer.open = false;
  }

  /** Стекло числом — то, чем меряют все остальные. «Вписать» здесь уже
   *  разрешилось в число: у отрисовщика, у мерки и у подписи не должно быть
   *  второго случая, про который надо помнить. */
  let stageZoom = $derived(glass === "fit" ? fitGlass : glass);

  /** Сколько карта занимает НА СТОЛЕ. Отличается от её собственной ширины ровно
   *  стеклом, и говорится об этом вслух: иначе «400» на кнопке и полметра
   *  карты на экране спорили бы друг с другом. */
  let stageShown = $derived(Math.round(stageWidth * stageZoom));

  /** Как зовут каждую из трёх ширин. Местом, а не числом: «400» ничего не
   *  говорит, «лист взятия» говорит всё, — а число стоит рядом, потому что
   *  резчик готовит картинки в точках. */
  const WIDTH_KEY: Record<number, TranslationKey> = {
    400: "adminBattlesWidthSheet",
    261: "adminBattlesWidthShelf",
    140: "adminBattlesWidthCell",
  };

  /** То же имя одним словом. На кнопке ленты стоит оно, полное — всплывающей
   *  подписью: «взятия» и «боя» вместе стоили шестидесяти точек ленты и не
   *  различали ни одной пары. */
  const WIDTH_SHORT: Record<number, TranslationKey> = {
    400: "adminBattlesWidthSheetShort",
    261: "adminBattlesWidthShelfShort",
    140: "adminBattlesWidthCellShort",
  };

  /** До какой ступени описи дотягивается карта такой ширины. Та же лестница,
   *  что на вкладке «Лицо карты»: ступень — не про стол, а про карту. */
  const STAGE_SHOW_KEY = {
    large: "adminBattlesStageShowLarge",
    always: "adminBattlesStageShowAlways",
    cell: "adminBattlesStageShowCell",
  } as const satisfies Record<"large" | "always" | "cell", TranslationKey>;

  /**
   * Сколько это в точках на той карте, которую сейчас показывают.
   *
   * Врезки названы в процентах, а режут они КАРТИНКУ, и картинку готовят в
   * точках: «12 %» не отвечает ни на «какой ширины рисовать уголок», ни на
   * «хватит ли у него разрешения», а «48 px» отвечает на оба. Проценты вбок
   * читаются от ширины, вниз — от высоты, ровно как их читает `inset` в CSS;
   * считать их от одного числа значило бы соврать на всякой карте, кроме
   * квадратной.
   */
  /** Четыре врезки в том порядке, в каком их обходят по часовой стрелке. */
  const INSETS = [
    { key: "insetTop", label: "adminBattlesInsetTop" },
    { key: "insetRight", label: "adminBattlesInsetRight" },
    { key: "insetBottom", label: "adminBattlesInsetBottom" },
    { key: "insetLeft", label: "adminBattlesInsetLeft" },
  ] as const satisfies readonly { key: InsetKey; label: TranslationKey }[];

  function insetPx(key: InsetKey, pct: number): number {
    const across = key === "insetLeft" || key === "insetRight";
    return Math.round((pct / 100) * (across ? stageWidth : cardTall));
  }

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

  function rememberBar() {
    try {
      if (barPin) localStorage.setItem(BAR_PIN_KEY, JSON.stringify(barPin));
      else localStorage.removeItem(BAR_PIN_KEY);
    } catch {
      // Приватное окно или запрет на хранение. Полоска работает и без памяти.
    }
  }

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

  /** Стрелки двигают взятую копию. Мышь на карте листа взятия даёт 0.25 % на
   *  точку экрана (а под стеклом — и того меньше); точнее неё клавиатура и
   *  должна быть, а не грубее, как было при шаге в полпроцента. Alt — не
   *  двигает, а наращивает нахлёст. */
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

  /** Свежий завиток из уже нарезанной детали — обычный путь: детали приходят
   *  листами, и перезаливать по одной то, что лежит на складе, значит плодить
   *  копии одного файла. */
  function addOrnamentFromStore() {
    pickFromStore("accent", (url) => {
      frames[frameIndex].ornaments.push(newOrnament(url));
    });
  }

  async function addOrnamentUpload() {
    const file = await pickImageFile();
    if (!file) return;
    uploading = true;
    try {
      const art = await uploadArt(file);
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


  // ── Полоска детали: где она стоит и где приколота ───────────────────────
  //
  // `shot` — слепок рам строкой. Полоска пересчитывается на любую правку, потому
  // что деталь под ней могла как раз поехать или вырасти.
  let shot = $derived(JSON.stringify(frames));
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

</script>

  <!-- Как лежит одна деталь. Один и тот же набор для всех шести слотов, потому
       что нестыковка была у всех шести одна и та же: полоса знала, где деталь
       начинается, и на этом всё кончалось. -->

  {#snippet placement(id: string, kind: SliceKind, piece: SlicePiece)}
    {@const sides = KIND_SIDES[kind]}
    {@const side = sliceHeld?.id === id ? sliceHeld.side : sides[0]}
    {@const at = piece.places[side]}
    <div
      class="mb-1 pl-3 border-l {sliceHeld?.id === id
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
          class="flex items-center gap-1.5 ml-2 text-[11px] text-[#8a6a55] cursor-pointer"
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
      <!-- Заполнение и разворот — в один ряд по половине. Стояли друг под
           другом во всю ширину, и на каждой детали это была лишняя строка
           верстака, у которого высота на счету. -->
      <div class="grid grid-cols-2 items-end gap-2 mb-2">
        <label class="block min-w-0">
          <span
            class="block mb-1 text-[11px] text-[#8a6a55]"
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
        <label class="block min-w-0">
          <span
            class="block mb-1 text-[11px] text-[#8a6a55]"
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
                class="block mb-1 text-[11px] text-[#8a6a55]"
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
        </div>
      {/if}
    </div>
  {/snippet}

    <!-- ── Пять рам, по одной на ранг ──────────────────────────────────────
         Стол резчика: карта занимает место, сбоку — список деталей и настройки
         ТОЛЬКО той, что в руке. До этого было наоборот, и полтораста органов
         управления стояли столбиком возле миниатюры в 320 px.

         Список рамок — на всю ширину, над сценой и колонкой: это и есть то,
         с чем работают. Подпись «эта рамка» имени не выбирает, а ящик в
         колонке прятал тот же список за прокруткой. -->
    <div class="flex-1 flex flex-col min-h-0">
      <!-- Ящик нарядов и его соседи — только у хозяина. У гостя рамка одна,
           своя, и «взять наряд из ящика» ему нечего: прячется это не флагом
           `guest`, а отсутствием самого ящика. -->
      <!-- ЛЕНТА СТОЛА. Одна, а не две, и над ОБЕИМИ колонками.
           Полос было две — ящик нарядов отдельной строкой над столом, — и
           вместе они съедали 136 px высоты у сцены, которой высоты и так не
           хватало: карта листа взятия не помещалась целиком НИ ПРИ КАКОМ
           разумном окне. Ящик ушёл сюда же, в свой ящик справа: его
           открывают раз в сеанс, а место он занимал постоянно.
           Заодно это развело полномочия, которые стояли в 40 px друг от
           друга без всякой границы: всё, что делает ящик, делается ВНУТРИ
           ящика и подписано нарядом, а лента снаружи — про чин. -->
      <div
        class="flex items-center gap-3 px-4 py-1.5 border-b border-[#34251c]/10"
      >
        <!-- Левая половина переносится ВНУТРИ СЕБЯ, правая не переносится
             никогда. Переносилась вся лента разом, и с `ml-auto` на правой
             половине это давало худший из возможных исходов: первая строка
             наполовину пустая, «ящик» и «сохранить рамки» сиротами на второй.
             Свободное место теперь достаётся тому, что его переживёт. -->
        <div class="flex-1 min-w-0 flex flex-wrap items-center gap-x-3 gap-y-1.5">
        <!-- Пять чинов ЛИЦАМИ. Только когда чинов больше одного: у гостя
             рамка одна, и «выберите чин» из одной кнопки это не выбор.

             Были пять кнопок с именами, и по именам чин не выбирают: рамки —
             семья, и правят их на согласованность («второй темнее первого,
             пятый — золото»), а слова «Крепкая» и «Памятная» про это не
             говорят ничего. Лицо говорит всё и стоит вчетверо меньше места;
             имя печатается у одного, выбранного, остальные названы всплывающей
             подписью — прочитать из пяти имён нужно ровно одно: какой чин
             сейчас на столе.

             Это же и единственное на столе место, где семью видно ЦЕЛИКОМ:
             карта всегда одна, а бумага, кайма и угол пяти чинов стоят здесь
             рядом. -->
        {#if frames.length > 1}
          <div
            class="flex items-center border border-[#34251c]/15"
            title={$t("adminBattlesTier")}
          >
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
                title="{frame.tier} · {frameName(frame, $lang)}"
                class="flex items-center gap-1.5 px-1.5 py-1 {frameIndex === i
                  ? 'bg-[#34251c] text-[#f8f1e7]'
                  : 'hover:bg-[#34251c]/5'}"
              >
                <BattleFrameFace
                  {frame}
                  class="w-5 h-7 {frameIndex === i
                    ? 'shadow-[0_0_0_1px_#f8f1e7]'
                    : ''}"
                />
                {#if frameIndex === i}
                  <span class="text-[11px] whitespace-nowrap"
                    >{frame.tier} · {frameName(frame, $lang)}</span
                  >
                {/if}
              </button>
            {/each}
          </div>
        {/if}

        <!-- Наряд, надетый на этот чин. Стоит НА ленте, а не в ящике:
             `BattleFramePicker` сам по себе выдвижной — одна строка с лицом
             рамки и её именем, — и прятать его за кнопкой значило спрятать
             ровно то, по чему наряд узнают. Высокой полосу делал не он, а
             пятеро его соседей; они и уехали в ящик. -->
        {#if frames[frameIndex] && presets}
          <div class="w-[12rem]">
            <BattleFramePicker
              presets={presets ?? []}
              bind:chosen={presetOpen}
              onchoose={wearPresetOnRank}
              onforget={forgetPreset}
              disabled={saving}
              size="slim"
              label={$t("adminBattlesPresetChoose")}
            />
          </div>
        {/if}

        <!-- Ширина карты. Три настоящие, а не величина предпросмотра: от
             ширины зависит и то, какие строки описи печатаются, и сколько
             точек приходится на врезку, — и резчику надо знать оба числа
             до того, как он нарисует уголок, а не после. -->
        <div
          class="flex border border-[#34251c]/15"
          title={$t("adminBattlesCardWidthHint")}
        >
          {#each CARD_WIDTHS as w (w)}
            <button
              onclick={() => (stageWidth = w)}
              title={$t(WIDTH_KEY[w])}
              class="px-2.5 py-1 text-[10px] whitespace-nowrap {stageWidth ===
              w
                ? 'bg-[#34251c] text-[#f8f1e7]'
                : 'hover:bg-[#34251c]/5'}"
              >{$t(WIDTH_SHORT[w])} · <span class="tabular-nums">{w}</span
              ></button
            >
          {/each}
        </div>

        <!-- Стекло. Только увеличивает показ: собственная ширина карты от
             него не меняется, поэтому и опись, и резьба, и кегль остаются
             ровно теми, что будут у гостя.
             «Вписать» стоит ПЕРВЫМ и оно же умолчание: до него карта не
             помещалась на столе целиком ни разу, и подвал — насечки уровня,
             стоимость, сила — был виден только прокруткой вслепую. -->
        <div
          class="flex border border-[#34251c]/15"
          title={$t("adminBattlesStageGlassHint")}
        >
          <button
            onclick={() => (glass = "fit")}
            class="px-2 py-1 text-[10px] whitespace-nowrap {glass === 'fit'
              ? 'bg-[#34251c] text-[#f8f1e7]'
              : 'hover:bg-[#34251c]/5'}">{$t("adminBattlesStageFit")}</button
          >
          {#each ZOOMS as z (z)}
            <button
              onclick={() => (glass = z)}
              class="px-2 py-1 text-[10px] {glass === z
                ? 'bg-[#34251c] text-[#f8f1e7]'
                : 'hover:bg-[#34251c]/5'}">{z}×</button
            >
          {/each}
        </div>

        <!-- Клетка боя. Стоит рядом с увеличением, а не в колонке справа:
             это способ СМОТРЕТЬ на карту, как и увеличение, а не её
             свойство. Без него кружок здоровья на столе недостижим — он
             выходит только в бою, а стол не бой. -->
        <div
          class="flex items-center gap-2 border border-[#34251c]/15 px-2 py-0.5"
        >
          <label
            class="flex items-center gap-1.5 text-[10px] uppercase tracking-[0.14em] cursor-pointer"
          >
            <input
              type="checkbox"
              bind:checked={stageInMatch}
              class="accent-[#34251c]"
            />
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

        </div>

        <div class="flex-shrink-0 flex items-center gap-3">
          <!-- ЯЩИК: что с нарядом ДЕЛАЮТ — отложить, обновить, забыть,
               начать новую рамку. Занимало целую полосу над столом, а нужно
               раз в сеанс. Заодно это дало двум областям полномочий границу,
               которой у них не было: «удалить» больше не стоит в сорока
               точках от «сохранить рамки», относясь при этом к другому.
               ВЫБОР наряда сюда не входит — он на ленте, лицом вверх: сам
               `BattleFramePicker` и есть выдвижной список, и спрятать его за
               кнопкой значило спрятать то, по чему наряд узнают. -->
          {#if frames[frameIndex] && presets}
            <details class="relative" bind:this={presetDrawer}>
              <summary
                class="flex items-center gap-1.5 px-2.5 py-1 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/25 cursor-pointer hover:bg-[#34251c]/5 list-none [&::-webkit-details-marker]:hidden"
              >
                <BattleIcon name="keep" />
                {$t("adminBattlesPresetDrawer")}
                {#if presetChanged}
                  <span class="w-1.5 h-1.5 rounded-full bg-[#c65f3c]"></span>
                {/if}
              </summary>
              <div
                class="absolute right-0 top-full z-30 mt-1 w-[32rem] max-w-[90vw] flex flex-col gap-3 p-4 bg-[#f8f1e7] border border-[#34251c]/25 shadow-[0_6px_24px_rgba(52,37,28,0.18)]"
              >
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
                  onclick={() => {
                keepFrameAsNew?.();
                shutDrawer();
              }}
                  disabled={saving}
                  title={$t("adminBattlesPresetKeepNewHint")}
                  class="flex items-center gap-1.5 px-2.5 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/25 hover:bg-[#34251c]/5 disabled:opacity-40"
                  ><BattleIcon name="twin" />{$t(
                    "adminBattlesPresetKeepNew",
                  )}</button
                >
                <button
                  onclick={() => {
                updateOpenPreset?.();
                shutDrawer();
              }}
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
                  onclick={() => {
                if (presetWorn) forgetPreset?.(presetWorn);
                shutDrawer();
              }}
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
                  onclick={() => {
                keepFrameAsPreset?.();
                shutDrawer();
              }}
                  disabled={saving || !presetName.trim()}
                  title={$t("adminBattlesPresetKeep")}
                  class="flex items-center gap-1.5 px-2.5 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/25 hover:bg-[#34251c]/5 disabled:opacity-40"
                  ><BattleIcon name="keep" />{$t("adminBattlesPresetKeep")}</button
                >
              {/if}
              <button
                onclick={() => beginNewFrame?.()}
                title={$t("adminBattlesFrameNewHint")}
                class="flex items-center gap-1.5 px-2.5 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/25 hover:bg-[#34251c]/5"
                ><BattleIcon name="plus" />{$t("adminBattlesFrameNew")}</button
              >
            </div>
              </div>
            </details>
          {/if}

          {#if dirty}
            <span
              class="flex items-center gap-1.5 text-[10px] uppercase tracking-[0.16em] text-[#8f2f22]"
            >
              <span class="w-1.5 h-1.5 rounded-full bg-[#c65f3c]"></span>
              {$t("adminBattlesUnsaved")}
            </span>
          {/if}
          <!-- Своя кнопка сохранения — только если её дали. У студии она в
               шапке комнаты, и вторая здесь была бы вторым способом сделать
               одно и то же. Отметка «не сохранено» остаётся в обоих случаях:
               она не кнопка, а известие. -->
          {#if save}
            <button
              onclick={save}
              disabled={saving}
              class="px-4 py-1.5 text-[10px] uppercase tracking-[0.16em] {dirty
                ? 'bg-[#34251c] text-[#f8f1e7]'
                : 'border border-[#34251c]/25'} disabled:opacity-40"
              >{$t("adminBattlesFramesSave")}</button
            >
          {/if}
        </div>
      </div>

      <div class="flex-1 flex min-h-0">
      <section class="flex-1 min-w-0 flex flex-col bg-[#f1e8db]">
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
          bind:clientWidth={stageWide}
          bind:clientHeight={stageTall}
          onscroll={() =>
            (stageScroll = {
              x: stageBox?.scrollLeft ?? 0,
              y: stageBox?.scrollTop ?? 0,
            })}
          class="relative flex-1 overflow-auto p-8 outline-none"
        >
          <!-- Мерка. Стоит вплотную к карте, по её настоящим краям, и называет
               настоящие точки — те, в которых резчик готовит картинки.
               Наверху ширина, слева высота; обе снаружи карты, чтобы ничего не
               заслонить, и обе вне стекла, чтобы подпись не росла вместе с
               резьбой. -->
          <div class="mx-auto w-fit">
            <div
              class="flex items-center gap-2 pb-1.5 text-[9px] uppercase tracking-[0.14em] text-[#8a6a55]"
              style="margin-left:{RULER_GUTTER}px; width:{stageShown}px"
            >
              <span
                class="flex-1 border-t border-[#34251c]/20 border-l border-l-[#34251c]/35 h-[5px]"
              ></span>
              <span class="tabular-nums whitespace-nowrap">{stageWidth} px</span>
              <span
                class="flex-1 border-t border-[#34251c]/20 border-r border-r-[#34251c]/35 h-[5px]"
              ></span>
            </div>
            <div class="flex items-stretch">
              <div
                class="shrink-0 flex items-center justify-center text-[9px] uppercase tracking-[0.14em] text-[#8a6a55]"
                style="width:{RULER_GUTTER}px"
              >
                <span
                  class="flex-1 self-stretch border-r border-[#34251c]/20 border-t border-t-[#34251c]/35 border-b border-b-[#34251c]/35 mr-1.5"
                ></span>
                <span class="tabular-nums [writing-mode:vertical-rl] rotate-180"
                  >{cardTall} px</span
                >
              </div>
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div
                class="relative"
                bind:this={cardBox}
                onpointerdowncapture={poke}
                style="width:{stageWidth}px; zoom:{stageZoom}"
              >
                <BattleCard
                  card={sample}
                  {frames}
                  owned={true}
                  transition={false}
                  interactive={false}
                  editable={true}
                  frameEditable={true}
                  rowsEditable={true}
                  hurt={stageInMatch ? stageHurt : 1}
                  alive={stageInMatch
                    ? Math.max(0, Math.round((sample.health || 10) * stageHurt))
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
            </div>

            <!-- Чем эта ширина отличается от соседней: отношением сторон и
                 тем, что на ней печатается. Стоит ПОД КАРТОЙ, а не в полосе
                 кнопок: это сказано про карту, а не про то, чем её выбирают,
                 — и места под ней ровно столько, сколько нужно словам. -->
            <p
              class="pt-2 text-[9px] uppercase tracking-[0.14em] text-[#8a6a55]"
              style="margin-left:{RULER_GUTTER}px; width:{stageShown}px"
            >
              <span class="tabular-nums"
                >1 : {(1 / (frames[frameIndex]?.aspect || DEFAULT_ASPECT)).toFixed(
                  2,
                )}</span
              >
              · {$t(STAGE_SHOW_KEY[widthShow(stageWidth)])}{#if stageZoom !== 1}
                · <span class="tabular-nums"
                  >{$t("adminBattlesStageOnDesk")} {stageShown} px</span
                >{/if}
            </p>
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
              title={heldRow ? undefined : $t("adminBattlesBarIdle")}
              class="absolute z-20 flex -translate-x-1/2 items-center gap-0.5 p-1 bg-[#f8f1e7] border border-[#34251c]/25 shadow-[0_2px_10px_rgba(52,37,28,0.18)] {heldRow
                ? ''
                : 'opacity-45 hover:opacity-100'}"
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
                    pickFromStore(
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
                <!-- Прибитая полоска с пустой рукой. Место — это и есть то, за
                     чем её прибивали: пусть стоит и ждёт, а не пропадает,
                     чтобы появиться в другом углу.
                     Но ждёт она КОРЕШКОМ: слова «В РУКЕ НИЧЕГО» делали её
                     полосой в полкарты шириной, и прибитая у верхнего края
                     она закрывала собой угол — то самое место, ради которого
                     на этот стол приходят. Что она пуста, сказано её
                     бледностью и всплывающей подписью; места это не стоит. -->
              {/if}
            </div>
          {/if}
        </div>
      </section>

      <!-- Колонка. Перехват нажатия и фокуса на ней целиком снимает слепок для
           отмены перед любой правкой — иначе каждый из полутораста органов
           управления пришлось бы оборачивать руками. -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <!-- КОЛОНКА. Две панели, а не одна прокрутка.
           Была одна: разделы рамы, список деталей и настройки взятой детали
           лежали в общем потоке без единой прибитой вещи (`sticky` — ноль,
           `max-h` — ноль). Оборот работы у резчика такой: взять деталь в
           списке — поправить число — посмотреть; а список и числа НЕ
           помещались на экран вместе, и каждый оборот начинался с прокрутки
           колонки то вверх, то вниз. Теперь верстак детали прибит внизу и
           прокручивается сам, а разделы рамы — сами.

           Порядок разделов — порядок работы, а не порядок появления: чем
           рамку начинают (как надета, бумага, окно), то и сверху, и первые
           две открыты. Справка о носителях сложена и ушла вниз. -->
      <!-- Ручка колонки. Своя полоска в шесть точек, а не кромка самой
           колонки: кромка — это граница, за неё промахиваются, и всякое
           нажатие рядом с ней уходило бы в первый же орган под ней. -->
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <div
        role="separator"
        aria-orientation="vertical"
        aria-label={$t("adminBattlesSideGrip")}
        title={$t("adminBattlesSideGrip")}
        onpointerdown={sideTake}
        onpointermove={sideDrag}
        onpointerup={sideDrop}
        onpointercancel={sideDrop}
        ondblclick={() => (sideWide = clampSide(432))}
        class="flex-shrink-0 w-1.5 cursor-col-resize touch-none border-l border-[#34251c]/10 {sideGrab
          ? 'bg-[#c65f3c]/40'
          : 'hover:bg-[#34251c]/10'}"
      ></div>

      <aside
        bind:this={asideBox}
        class="flex-shrink-0 flex flex-col min-h-0"
        style="width:{sideWide}px"
        onpointerdowncapture={mark}
        onfocusincapture={mark}
      >
        {#if frames[frameIndex]}
          <div class="flex-1 min-h-0 overflow-y-auto">
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
                        : mode === "sliced"
                          ? $t("adminBattlesFrameSliced")
                          : $t("adminBattlesFrameFreeform")}</button
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
              {:else if frames[frameIndex].frameMode === "freeform"}
                <details class="mt-2" open>
                  <summary
                    class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
                    >{$t("adminBattlesHintOpen")}</summary
                  >
                  <p
                    class="mt-2 text-[11px] leading-relaxed italic text-[#8a6a55]"
                  >
                    {$t("adminBattlesFrameFreeformHint")}
                  </p>
                </details>
              {/if}
            </div>


            <details open class="border-b border-[#34251c]/10">
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
                      class="block mb-1 text-[11px] text-[#8a6a55]"
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
                      class="block mb-1 text-[11px] text-[#8a6a55]"
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
                        class="block mb-1 text-[11px] text-[#8a6a55]"
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
                      class="block mb-1 text-[11px] text-[#8a6a55]"
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

            <details open class="border-b border-[#34251c]/10">
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
                    {#if frames[frameIndex].frameMode !== "freeform"}
                      <p
                        class="max-w-[62ch] mt-2 text-[11px] leading-relaxed italic text-[#8a6a55]"
                      >
                        {$t("adminBattlesBandsHint")}
                      </p>
                    {/if}
                  </details>
                  <!-- В один столбец, а не в два по 160 px: у ползунка в
                       колонке 432 px нет причин быть шириной в треть её, а
                       подпись при такой ширине обрезалась на «ВРЕЗКА СВЕ…».
                       Имя слева, число справа — по числам эти ручки и ищут. -->
                  <div class="space-y-3">
                    <!-- `freeform`: та же самая рама, что у «поверх» — своя
                         дыра в своём месте на КАЖДОЙ загруженной картинке, и
                         врезки — единственный язык, на котором место дыры
                         вообще можно назвать. Три доли полос ниже ничего не
                         значат — полос в этом режиме нет ни одной. -->
                    <!-- Четыре врезки одним списком: разница между ними — одно
                         слово и одна ось, и четыре списанных друг с друга блока
                         расходились бы по одному. Рядом с процентом стоят
                         ТОЧКИ — те самые, что на выбранной ширине карты: резать
                         картинку по процентам нельзя. -->
                    {#each INSETS as row (row.key)}
                      <label class="block">
                        <span
                          class="flex items-baseline justify-between gap-2 mb-1 text-[11px] text-[#8a6a55]"
                        >
                          <span class="truncate">{$t(row.label)}</span>
                          <span class="flex-shrink-0 tabular-nums"
                            >{frames[frameIndex][row.key].toFixed(0)}% ·
                            <b class="font-normal text-[#34251c]/70"
                              >{insetPx(
                                row.key,
                                frames[frameIndex][row.key],
                              )} px</b
                            ></span
                          >
                        </span>
                        <input
                          type="range"
                          min="0"
                          max="45"
                          step="0.5"
                          value={frames[frameIndex][row.key]}
                          oninput={(e) =>
                            setInset(row.key, Number(e.currentTarget.value))}
                          class="w-full"
                        />
                      </label>
                    {/each}
                    <label class="block">
                      <span
                        class="block mb-1 text-[11px] text-[#8a6a55]"
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
                    {#if frames[frameIndex].frameMode !== "freeform"}
                      <label class="block">
                        <span
                          class="block mb-1 text-[11px] text-[#8a6a55]"
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
                      <label class="block">
                        <span
                          class="block mb-1 text-[11px] text-[#8a6a55]"
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
                      <label class="block">
                        <span
                          class="block mb-1 text-[11px] text-[#8a6a55]"
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
                    {/if}
                  </div>
                </div>
              </div>
            </details>


            <details class="border-b border-[#34251c]/10">
              <summary
                class="px-4 py-2.5 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
                >{$t("adminBattlesFrameArt")}</summary
              >
              <div class="px-4 pb-4 space-y-4">
                <div class="flex flex-wrap items-end gap-4">
                  <label class="block">
                    <span
                      class="block mb-1 text-[11px] text-[#8a6a55]"
                      >{$t("adminBattlesFrameName")} · EN</span
                    >
                    <input
                      bind:value={frames[frameIndex].nameEn}
                      class="px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                    />
                  </label>
                  <label class="block">
                    <span
                      class="block mb-1 text-[11px] text-[#8a6a55]"
                      >{$t("adminBattlesFrameName")} · RU</span
                    >
                    <input
                      bind:value={frames[frameIndex].nameRu}
                      class="px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                    />
                  </label>
                  <label class="block">
                    <span
                      class="block mb-1 text-[11px] text-[#8a6a55]"
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
                    <button
                      onclick={pickFrameArt}
                      disabled={uploading}
                      class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5 disabled:opacity-40"
                      >{$t("adminAssetsPick")}</button
                    >
                    <label class="block flex-1 min-w-[12rem]">
                      <span
                        class="block mb-1 text-[11px] text-[#8a6a55]"
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
                  <details class="mt-3">
                    <summary
                      class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
                      >{$t("adminBattlesHintOpen")}</summary
                    >
                    <p
                      class="max-w-[62ch] mt-2 text-[11px] leading-relaxed italic text-[#8a6a55]"
                    >
                      {$t("adminBattlesFrameScaleHint")}
                    </p>
                  </details>
                  <div class="grid grid-cols-2 gap-3 mt-3">
                    <label class="block">
                      <span
                        class="block mb-1 text-[11px] text-[#8a6a55]"
                        >{$t("adminBattlesFrameScaleX")} · {clampScale(
                          frames[frameIndex].frameScaleX,
                          BADGE_SCALE_MIN,
                          BADGE_SCALE_MAX,
                        ).toFixed(2)}×</span
                      >
                      <input
                        type="range"
                        min={BADGE_SCALE_MIN}
                        max={BADGE_SCALE_MAX}
                        step="0.01"
                        value={clampScale(
                          frames[frameIndex].frameScaleX,
                          BADGE_SCALE_MIN,
                          BADGE_SCALE_MAX,
                        )}
                        oninput={(e) =>
                          (frames[frameIndex].frameScaleX = Number(
                            e.currentTarget.value,
                          ))}
                        class="w-full"
                      />
                    </label>
                    <label class="block">
                      <span
                        class="block mb-1 text-[11px] text-[#8a6a55]"
                        >{$t("adminBattlesFrameScaleY")} · {clampScale(
                          frames[frameIndex].frameScaleY,
                          BADGE_SCALE_MIN,
                          BADGE_SCALE_MAX,
                        ).toFixed(2)}×</span
                      >
                      <input
                        type="range"
                        min={BADGE_SCALE_MIN}
                        max={BADGE_SCALE_MAX}
                        step="0.01"
                        value={clampScale(
                          frames[frameIndex].frameScaleY,
                          BADGE_SCALE_MIN,
                          BADGE_SCALE_MAX,
                        )}
                        oninput={(e) =>
                          (frames[frameIndex].frameScaleY = Number(
                            e.currentTarget.value,
                          ))}
                        class="w-full"
                      />
                    </label>
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
                      class="block mb-1 text-[11px] text-[#8a6a55]"
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
                        class="block mb-1 text-[11px] text-[#8a6a55]"
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

            {#if frames[frameIndex].frameMode === "freeform"}
              <!-- Место имени и приписки задаётся не здесь: их берут в руку
                   прямо на карте рядом (тот же приём, что у значков стоимости
                   и силы) и тащат, куда нужно, на пустую бумагу иллюстрации.
                   Здесь — только то, что каскадом не перетащишь: начертание,
                   чернила, кегль. -->
              <details open class="border-b border-[#34251c]/10">
                <summary
                  class="px-4 py-2.5 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
                  >{$t("adminBattlesFreeText")}</summary
                >
                <div class="px-4 pb-4">
                  <div class="pt-5 border-t border-[#34251c]/10">
                    <details class="mb-3">
                      <summary
                        class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
                        >{$t("adminBattlesHintOpen")}</summary
                      >
                      <p
                        class="max-w-[62ch] mt-2 text-[11px] leading-relaxed italic text-[#8a6a55]"
                      >
                        {$t("adminBattlesFreeTextHint")}
                      </p>
                    </details>
                    <p
                      class="mb-2 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                    >
                      {$t("adminBattlesFreeName")}
                    </p>
                    <div class="flex flex-wrap items-end gap-4 mb-5">
                      <label class="block">
                        <span
                          class="block mb-1 text-[11px] text-[#8a6a55]"
                          >{$t("adminBattlesFreeNameSize")} · {freeMarkOf(
                            frames[frameIndex],
                            "title",
                          ).size.toFixed(2)}×</span
                        >
                        <input
                          type="range"
                          min="0.5"
                          max="3"
                          step="0.05"
                          value={freeMarkOf(frames[frameIndex], "title").size}
                          oninput={(e) =>
                            setFreeMark(frames[frameIndex], "title", {
                              size: Number(e.currentTarget.value),
                            })}
                          class="w-full"
                        />
                      </label>
                    </div>
                    <p
                      class="mb-2 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]"
                    >
                      {$t("adminBattlesFreeLoreFont")}
                    </p>
                    <div class="flex flex-wrap items-end gap-4">
                      <label class="block">
                        <span
                          class="block mb-1 text-[11px] text-[#8a6a55]"
                          >{$t("adminBattlesFreeLoreFont")}</span
                        >
                        <select
                          value={freeMarkOf(frames[frameIndex], "lore").font}
                          onchange={(e) =>
                            setFreeMark(frames[frameIndex], "lore", {
                              font: e.currentTarget.value || undefined,
                            })}
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
                          class="block mb-1 text-[11px] text-[#8a6a55]"
                          >{$t("adminBattlesFreeLoreInk")}</span
                        >
                        <input
                          type="color"
                          value={freeMarkOf(frames[frameIndex], "lore").ink}
                          oninput={(e) =>
                            setFreeMark(frames[frameIndex], "lore", {
                              ink: e.currentTarget.value,
                            })}
                          class="w-12 h-8 bg-transparent border border-[#34251c]/15"
                        />
                      </label>
                      <label class="block flex-1 min-w-[14rem]">
                        <span
                          class="block mb-1 text-[11px] text-[#8a6a55]"
                          >{$t("adminBattlesFreeLoreSize")} · {freeMarkOf(
                            frames[frameIndex],
                            "lore",
                          ).size.toFixed(2)}×</span
                        >
                        <input
                          type="range"
                          min="0.5"
                          max="3"
                          step="0.05"
                          value={freeMarkOf(frames[frameIndex], "lore").size}
                          oninput={(e) =>
                            setFreeMark(frames[frameIndex], "lore", {
                              size: Number(e.currentTarget.value),
                            })}
                          class="w-full"
                        />
                      </label>
                    </div>
                  </div>
                </div>
              </details>
            {/if}

            <!-- Кто это носит. Справка, а не настройка: стояла ПЕРВОЙ и своими
                 десятью строками толкала вниз всё, чем работают. Оставлена на
                 столе (без неё чин красят вслепую и не узнают, что его не видно
                 ни на одной карте), но сложена и убрана в конец — туда, где ей
                 и место по частоте обращения. -->
            {#if worn}
              <details class="border-b border-[#34251c]/10">
                <summary
                  class="flex items-center gap-2 px-4 py-2.5 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer list-none [&::-webkit-details-marker]:hidden"
                >
                  {#if !worn.plain.length && worn.mine.length}
                    <span class="w-1.5 h-1.5 rounded-full bg-[#c65f3c]"></span>
                  {/if}
                  {$t("adminBattlesWornBy")}
                  <span class="tabular-nums">· {worn.mine.length}</span>
                </summary>
                <div class="px-4 pb-4">
                {#if !worn.mine.length}
                  <p class="text-[11px] italic text-[#8a6a55]">
                    {$t("adminBattlesWornNone")}
                  </p>
                {:else}
                  <p class="text-[11px] leading-relaxed text-[#6f3b24]">
                    {$t("adminBattlesWornPlain")}
                    <b class="tabular-nums">{worn.plain.length}</b>
                    {$t("adminBattlesWornOf")}
                    <b class="tabular-nums">{worn.mine.length}</b>
                  </p>
                  {#if !worn.plain.length}
                    <p
                      class="mt-1 flex items-start gap-1.5 text-[11px] leading-relaxed text-[#8f2f22]"
                    >
                      <span class="mt-1.5 w-1.5 h-1.5 flex-shrink-0 rounded-full bg-[#c65f3c]"
                      ></span>
                      {$t("adminBattlesWornBlind")}
                    </p>
                  {/if}
                  {#each [{ list: worn.own, word: "adminBattlesWornOwn" as TranslationKey }, { list: worn.byRace, word: "adminBattlesWornRace" as TranslationKey }] as group (group.word)}
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
                              onclick={() => onOpenCard(one)}
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
              </details>
            {/if}
          </div>

          <!-- ВЕРСТАК ДЕТАЛИ. Прибит книзу и держит ровно то, чем работают
               каждую минуту: список деталей своей прокруткой и настройки той,
               что в руке, — под ним, всегда на виду. -->
          {#if frames[frameIndex].frameMode === "sliced"}
            <!-- Ручка верстака: им делят колонку по высоте. -->
            <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
            <div
              role="separator"
              aria-orientation="horizontal"
              aria-label={$t("adminBattlesPaneGrip")}
              title={$t("adminBattlesPaneGrip")}
              onpointerdown={paneTake}
              onpointermove={paneDrag}
              onpointerup={paneDrop}
              onpointercancel={paneDrop}
              ondblclick={() => ((paneShare = 0.66), keepDesk())}
              class="flex-shrink-0 h-1.5 cursor-row-resize touch-none border-t border-[#34251c]/10 {paneGrab
                ? 'bg-[#c65f3c]/40'
                : 'hover:bg-[#34251c]/10'}"
            ></div>

            <section
              class="flex-shrink-0 flex flex-col min-h-0 border-t border-[#34251c]/15 bg-[#f4ede2]"
              style="max-height:{Math.round(paneShare * 100)}%"
            >
              <!-- ШАПКА ВЕРСТАКА. Прибита, а не прокручивается вместе со
                   списком: заголовок, подсказка и две кнопки «добавить» стояли
                   ВНУТРИ прокрутки и съедали у неё шестьдесят точек из ста
                   сорока — то есть от списка оставалась одна строка с
                   половиной. Кнопки стали значками по той же причине: два
                   слова в разрядку на «добавить — со склада» стоили строки. -->
              <div
                class="flex-shrink-0 flex items-center gap-2 px-3 py-1.5 border-b border-[#34251c]/12"
              >
                <p
                  class="min-w-0 truncate text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]"
                >
                  {$t("adminBattlesStack")}
                </p>
                <div class="ml-auto flex-shrink-0 flex items-center gap-0.5">
                  <button
                    onclick={addOrnamentUpload}
                    disabled={uploading}
                    title={$t("adminBattlesOrnamentAdd")}
                    class="p-1.5 text-[#8a6a55] hover:bg-[#34251c]/8 hover:text-[#34251c] disabled:opacity-40"
                    ><BattleIcon name="upload" /></button
                  >
                  <button
                    onclick={addOrnamentFromStore}
                    title={$t("adminBattlesOrnamentAddStore")}
                    class="p-1.5 text-[#8a6a55] hover:bg-[#34251c]/8 hover:text-[#34251c]"
                    ><BattleIcon name="store" /></button
                  >
                  <details class="relative">
                    <summary
                      title={$t("adminBattlesHintOpen")}
                      class="p-1.5 text-[10px] leading-none text-[#8a6a55] cursor-pointer list-none [&::-webkit-details-marker]:hidden hover:text-[#34251c]"
                      >?</summary
                    >
                    <p
                      class="absolute right-0 top-full z-30 mt-1 w-[22rem] max-w-[80vw] p-3 text-[11px] leading-relaxed italic text-[#8a6a55] bg-[#f8f1e7] border border-[#34251c]/25 shadow-[0_6px_24px_rgba(52,37,28,0.18)]"
                    >
                      {$t("adminBattlesStackHint")}
                    </p>
                  </details>
                </div>
              </div>

              <div class="flex-1 min-h-[9rem] overflow-y-auto">
                <!-- Список деталей. Сверху то, что рисуется поверх; порядок задают
                     здесь, и только здесь, поэтому невидимых ничьих между равными
                     слоями больше нет. -->
                <div class="p-3">
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
                            class="w-7 h-7 flex-shrink-0 border border-[#34251c]/15 bg-[#34251c]/[0.04] bg-center bg-contain bg-no-repeat"
                            style:background-image={row.image
                              ? `url("${row.image}")`
                              : "none"}
                          ></span>
                          <span
                            class="min-w-0 flex-1 text-[11px] truncate {row.image
                              ? ''
                              : 'text-[#8a6a55] italic'}">{row.label}</span
                          >
                          <span
                            class="flex-shrink-0 text-[11px] text-[#8a6a55] truncate max-w-[9rem]"
                          >
                            {row.image
                              ? $t(KIND_KEY[row.kind])
                              : $t("adminBattlesPieceEmpty")}
                          </span>
                        </button>
                      </div>
                    {/each}
                  </div>
                </div>
              </div>
              <!-- Настройки ужимаются и прокручиваются, а не выталкивают
                   список: на низком окне верстак упирается в свой потолок, и
                   неужимаемые настройки вылезли бы за край колонки — то есть
                   из виду ушло бы ровно то, ради чего верстак прибит. -->
              <div
                class="min-h-0 max-h-[20rem] overflow-y-auto border-t border-[#34251c]/12 bg-[#f8f1e7]"
              >
                <!-- Настройки ТОЛЬКО взятой детали. Шесть блоков разом были прежде
                     всегда открыты, и колонка не помещалась на экран. -->
                {#if heldRow}
                  <div class="p-4">
                    <!-- Не `<p>`: ниже стоит `<details>`, а абзац его в себе
                         держать не может — браузер закрывает абзац перед ним,
                         и разметка расходится с той, которую обходит Svelte.
                         Стоило это не съехавшей вёрстки, а падения на
                         `get_first_child`: весь верстак переставал слушать
                         мышь, и ни одна кнопка списка не работала. -->
                    <div
                      class="flex items-center gap-2 mb-3 text-[10px] uppercase tracking-[0.16em] text-[#c65f3c]"
                    >
                      <span class="min-w-0 truncate">{heldRow.label}</span>
                      <!-- Подсказка значком, а не строкой внизу: нужна она
                           однажды, а место занимала всегда. -->
                      <details class="relative ml-auto flex-shrink-0">
                        <summary
                          title={$t("adminBattlesHintOpen")}
                          class="px-1 text-[10px] leading-none text-[#8a6a55] cursor-pointer list-none [&::-webkit-details-marker]:hidden hover:text-[#34251c]"
                          >?</summary
                        >
                        <span
                          class="absolute right-0 top-full z-30 mt-1 block w-[22rem] max-w-[80vw] p-3 text-[11px] normal-case tracking-normal leading-relaxed italic text-[#8a6a55] bg-[#f8f1e7] border border-[#34251c]/25 shadow-[0_6px_24px_rgba(52,37,28,0.18)]"
                        >
                          {$t("adminBattlesSliceHint")}
                        </span>
                      </details>
                    </div>
                    {#if heldRow.ornament}
                      <label class="block w-full mb-3">
                        <span
                          class="block mb-1 text-[11px] text-[#8a6a55]"
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
                    <!-- Картинка детали. Здесь стояли четыре кнопки — загрузить ·
                         со склада · убрать картинку · убрать украшение, — и все
                         четыре до одной есть на полоске, которая висит у самой
                         детали: один поступок был напечатан дважды, и хранитель
                         всякий раз выбирал, каким из двух его сделать. Полоске —
                         ДЕЙСТВИЯ, колонке — слова и числа. Осталось поле пути:
                         оно и есть ответ на «что за картинка», и второй строкой
                         с именем файла над ним было бы то же самое, сказанное
                         дважды. -->
                    <label class="block w-full mb-2">
                      <span class="block mb-1 text-[11px] text-[#8a6a55]"
                        >{$t("adminBattlesPieceArt")}</span
                      >
                      <input
                        value={heldRow.image}
                        oninput={(e) =>
                          setPieceImage(heldRow!, e.currentTarget.value)}
                        placeholder={$t("adminBattlesPieceEmpty")}
                        class="w-full px-2 py-1.5 text-xs bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35"
                      />
                    </label>
                    {@render placement(heldRow.id, heldRow.kind, heldRow.piece)}
                  </div>
                {:else}
                  <p
                    class="p-4 text-[11px] leading-relaxed italic text-[#8a6a55]"
                  >
                    {$t("adminBattlesStackNothingHeld")}
                  </p>
                {/if}
              </div>
            </section>
          {/if}
        {/if}
      </aside>
      </div>
    </div>