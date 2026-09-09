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
    defaultSlices,
    dressWindowMissing,
    frameName,
    kindOf,
    livePiece,
    newOrnament,
    pickImageFile,
    sliceSigns,
    type InsetKey,
  } from "$lib/battles";
  import { SITE_FONTS } from "$lib/fonts";
  import { selectOnFocus, blurOnWheel } from "$lib/utils/fields";
  import BattleCard from "$lib/components/BattleCard.svelte";
  import BattleIcon from "$lib/components/BattleIcon.svelte";
  import BattleFramePicker from "$lib/components/admin/BattleFramePicker.svelte";
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
      {#if frames[frameIndex] && presets}
        <div
          class="flex flex-wrap items-center gap-x-3 gap-y-2 px-4 py-2.5 border-b border-[#34251c]/10 bg-[#f8f1e7]"
        >
          <div class="w-[22rem] max-w-full">
            <BattleFramePicker
              presets={presets ?? []}
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
                onclick={() => presetWorn && forgetPreset(presetWorn)}
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
          <!-- Полоса чинов — только когда чинов больше одного. У гостя рамка
               одна, и «выберите чин» из одной кнопки это не выбор. -->
          {#if frames.length > 1}
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
          {/if}

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
              card={sample}
              {frames}
              owned={true}
              transition={false}
              interactive={false}
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
          {#if worn}
          <div class="p-4 border-b border-[#34251c]/10">
            <p class="mb-2 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">
              {$t("adminBattlesWornBy")}
            </p>
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
          {/if}

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
                      pickFromStore(
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