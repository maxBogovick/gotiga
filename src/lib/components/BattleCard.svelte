<script lang="ts">
  // One card, rendered one way. The shelf, the keeper's preview and (later) the
  // moment of taking all draw this same component, because a preview that has
  // its own renderer is a preview that eventually lies.
  //
  // Size is never passed in. The card fills its container and reads its own
  // width with container queries, so the same component is a spine on a shelf
  // and a full card in a frame without a second set of styles.
  //
  // `editable` is for what only makes sense done ON the card: dragging and
  // zooming the photograph, choosing the frame, jumping to a race. Text and
  // numbers were tried here too, typed straight into the decorative bands,
  // but the keeper's desk found that unusable at card size — those live in
  // an ordinary form next to the card instead, writing the same `card`
  // object this component reads, so the preview still never lies. It is off
  // everywhere except the admin card editor.
  import type {
    BattleBadgeShape,
    BattleCard,
    BattleFrame,
    SheetBand,
    SheetRow,
    SheetSlot,
    SliceSide,
  } from '$lib/types/api';
  import { t, lang } from '$lib/i18n';
  import {
    cardCopy,
    frameFor,
    frameForCard,
    headerCopy,
    traitCopy,
    frameName,
    frameVars,
    isDressed,
    isOverlaid,
    isSliced,
    parseFocal,
    pricesOf,
    cardTransitionName,
    pickImageFile,
    DEFAULT_COST_X,
    DEFAULT_COST_Y,
    DEFAULT_POWER_X,
    DEFAULT_POWER_Y,
    type BadgeKind,
    BADGE_SHAPES,
    BADGE_FIELDS,
    BADGE_KINDS,
    badgeAt,
    KIND_SIDES,
    SLICE_GROW_MAX,
    SLICE_SIDE_AXES,
    applyInsetDelta,
    carvedCopies,
    kindOf,
    kindLabelKey,
    channelLabelKey,
    bodyPassport,
    badgeReserve,
    badgeSpot,
    badgeStyle,
    badgeExtent,
    badgePlate,
    badgeShape,
    badgeText,
    badgeUnfilled,
    sealWear,
    DEFAULT_ASPECT,
    sheetOf,
    sheetBand,
    sheetShows,
    isStatSlot,
    statGroupShow,
    SHEET_SLOT_BANDS,
    SHEET_STATS,
    BODY_STAT_LABELS,
    livePiece,
    paperClip,
    scrapFlight,
    sliceResizeDelta,
    sliceSigns,
    type InsetKey,
    type FrameOverride,
    type ScrapFly,
    type SliceResizeX,
    type SliceResizeY,
  } from '$lib/battles';
  import { api } from '$lib/api';
  import AppImage from '$lib/components/AppImage.svelte';
  import BattleBadgeInspector from '$lib/components/BattleBadgeInspector.svelte';

  let {
    card = $bindable(),
    frames = null,
    owned = false,
    level = null,
    isNew = false,
    interactive = true,
    transition = true,
    editable = false,
    editLang = null,
    frameEditable = false,
    frameEditTarget = null,
    sliceHeld = $bindable(null),
    onEditStart,
    onEditEnd,
    raceIconEditable = false,
    onEditRace,
    onIconUpload,
    onBadgeArtUpload,
    onBadgeArtStore,
    onError,
    hurt = 1,
    wearSeed = 0,
    struck = null,
    scrap = null,
    onfit,
    alive = null,
    rowsEditable = false,
    rowHeld = $bindable(null),
    onRowMove,
  }: {
    card: BattleCard;
    frames?: BattleFrame[] | null;
    /** Face up or face down. A card you do not have lies in dust, price up. */
    owned?: boolean;
    /** The level of *your* copy, 1..5. Null while nobody owns anything. */
    level?: number | null;
    isNew?: boolean;
    /** Off in dense admin lists, where forty tilting cards help nobody. */
    interactive?: boolean;
    /**
     * Whether this card claims its shared-element name. Exactly one element per
     * page may carry it — a second one aborts the whole view transition — so a
     * preview rendered beside the shelf passes `false`.
     */
    transition?: boolean;
    /** Every band becomes a live editor: the keeper writes on the card itself. */
    editable?: boolean;
    /** Which language the inline fields read and write. Falls back to the
     *  site's reader language if the desk hasn't set its own toggle. */
    editLang?: 'en' | 'ru' | null;
    /** The rank's own shape becomes draggable: the header/art/footer seams and
     *  the four edges of the window, each a handle onto that one frame's own
     *  numbers. Set only by the Frames tab, on the sample it dresses — never
     *  together with `editable`, which is a different card's content. */
    frameEditable?: boolean;
    /** Redirects the four inset handles onto a race's own per-level patch
     *  instead of the tier's shared frame — set only by the Races tab, on the
     *  sample it dresses, together with `frameEditable`. The header/art/foot
     *  seams and the badges stay off in this mode: a picture chosen for one
     *  level isn't the place to also retune the tier's bands. Insets here
     *  don't mirror to the opposite side either — a race's own picture is
     *  rarely symmetric. */
    frameEditTarget?: FrameOverride | null;
    /** Which COPY of which piece is currently in hand — the top-left corner,
     *  say, and not the corner picture in general, since the four corners are
     *  placed apart. `id` is a named slot or an added ornament's own id, which
     *  is the whole reason it is a string: the keeper's flourishes and the
     *  frame's anatomy are dragged by one piece of code. Bindable, because it
     *  is picked both ways — on the card by taking hold of the copy, and in the
     *  sidebar by choosing a side — and one value cannot disagree with itself. */
    sliceHeld?: { id: string; side: SliceSide } | null;
    /** Called once at the START of every gesture that edits the frame — a
     *  slice taken hold of, an inset handle, a band seam, a badge. The desk
     *  takes its undo snapshot here, which is the only moment the state before
     *  the edit still exists. */
    onEditStart?: () => void;
    /** Called when a gesture is done. The desk moves the held piece's own
     *  toolbar here rather than every frame of the drag: a bar jumping under
     *  the pointer is worse than one that catches up. */
    onEditEnd?: () => void;
    /** The header icon alone is a live uploader, independent of `editable` —
     *  what the Races tab's own sample card sets, where nothing else here is
     *  this card's to edit. */
    raceIconEditable?: boolean;
    /** Editable but not this card's race to rename — send the keeper there. */
    onEditRace?: () => void;
    /** A new icon was chosen for the race this sample wears. */
    onIconUpload?: (url: string) => void;
    /** Две двери за картинкой жетона, для стола значка. Карта их только
     *  ПРОВОЗИТ: где лежит склад и как туда стучаться, знает стол хранителя, а
     *  карта рисуется и на полке, где ни того, ни другого нет. */
    onBadgeArtUpload?: (apply: (url: string) => void) => void;
    onBadgeArtStore?: (apply: (url: string) => void) => void;
    onError?: (message: string) => void;
    /** Remaining health, 0..1. 1 — целая. Полк и стол карт это не передают. */
    hurt?: number;
    /** Id тела: те же выщербы при том же здоровье, лечение снимает те же. */
    wearSeed?: number;
    /** Синяк или чернила — только такт удара, не метр здоровья. Родитель
     *  вешает это В МОМЕНТ удара, не в начале замаха. */
    struck?: 'bruise' | 'ink' | null;
    /** Один обломок, улетает с карты. Тот же миг, что синяк. */
    scrap?: ScrapFly | null;
    /**
     * Сколько здоровья ОСТАЛОСЬ. Числом, а не долей: `hurt` рвёт бумагу и
     * этого ему хватает, а кружок здоровья говорит число, и посчитанное из
     * доли оно врало бы округлением. `null` — партии нет: полка и стол
     * печатают то, что написано на бумаге.
     */
    alive?: number | null;
    /**
     * Влезло ли написанное в окно — и на сколько не влезло, в пикселях.
     *
     * Полоса свойств переполняется МОЛЧА: у неё `overflow-y: auto`, и лишние
     * числа уезжают в прокрутку, которой на карте никто не видит и не ищет.
     * Мерит тот, кто рисует, потому что мерить это можно только после укладки,
     * и только на той ширине, на которой карта стоит. Стол зажигает по этому
     * лампу; полка не передаёт ничего и ничего не меряет.
     */
    onfit?: (over: number) => void;
    /**
     * Опись правится НА КАРТЕ: строку берут там, где она напечатана, и
     * отпускают там, где ей стоять. Ставится только столом рамок — на полке
     * карта не редактор, а карта.
     */
    rowsEditable?: boolean;
    /** Какая строка сейчас в руке. Связка, потому что берут её двумя путями —
     *  на карте и в списке сбоку, — а одно значение не может разойтись само с
     *  собой во мнении о том, что несут. */
    rowHeld?: SheetSlot | null;
    /** Строку отпустили: вот полоса и вот место в ней. Карта не пишет в опись
     *  сама — она читает раму, а рама принадлежит столу. */
    onRowMove?: (slot: SheetSlot, band: SheetBand, before: SheetSlot | null) => void;
  } = $props();

  let frame = $derived(frameForCard(card, frames, level));
  /** The language the card's own text reads as. A reader always sees the
   *  site's language; the keeper's desk passes its RU/EN toggle instead, so
   *  the preview shows exactly the language being typed into the sidebar
   *  form, never the other one. */
  let editLang2 = $derived((editLang ?? $lang) as 'en' | 'ru');
  let copy = $derived(cardCopy(card, editLang2));
  let torn = $derived(owned ? paperClip(hurt, wearSeed) : null);
  let flake = $derived(scrap ? scrapFlight(scrap.blow, scrap.remain, scrap.seed) : null);
  let focal = $derived(parseFocal(card.artFocal));
  let prices = $derived(pricesOf(card));
  let head = $derived(headerCopy(card, editLang2));
  let traits = $derived(
    (card.traits ?? []).map((t) => traitCopy(t, editLang2)).filter((t) => t.name),
  );
  let kindWord = $derived($t(kindLabelKey(card.kind)));
  let channelWord = $derived(
    (() => {
      const key = channelLabelKey(card.attackChannel);
      return key ? $t(key) : '';
    })(),
  );
  let passport = $derived(bodyPassport(card));
  /** A colon-and-text trait is a rule. Until `abilities` names one, the face
   *  must not look as if the engine will honour it. The keeper's form still
   *  holds the prose; the card they preview is the card the guest will see. */
  let printTraitRule = $derived((card.abilities?.length ?? 0) > 0);
  let printEffectAsRule = $derived((card.abilities?.length ?? 0) > 0);
  let rank = $derived(frameName(frame, $lang));
  let dressed = $derived(isDressed(frame));
  let overlaid = $derived(isOverlaid(frame));
  let sliced = $derived(isSliced(frame));
  let hasBackArt = $derived(!!frame.backImage?.trim());
  let vars = $derived(frameVars(frame));

  /* ── Опись ────────────────────────────────────────────────────────────────
     Что печатается, в какой полосе и в каком порядке, решает РАМА. Полосы —
     один `{#each}` по её строкам. До этого содержимое полос было прибито к
     разметке, и «убрать байку с клетки боя» значило дописать в компонент ещё
     одно условие; теперь это строка описи, и она едет вместе с нарядом на
     чин, на уровень расы и на одну карту тем же кодом, что и всё остальное. */

  let rows = $derived(sheetOf(frame));

  /** Есть ли ЭТОЙ строке что сказать на ЭТОЙ карте. Пустая раса и пустой
   *  канал не должны оставлять после себя точку-разделитель, а «стоимость ·
   *  сила» строкой печатается только там, где значков по углам нет. */
  function speaks(slot: SheetSlot): boolean {
    switch (slot) {
      case 'raceIcon': return editable || raceIconEditable || !!card.raceIconUrl;
      case 'race': return !!head.race;
      case 'channel': return !!channelWord;
      case 'pips': return level != null;
      case 'title': return !!copy.title;
      case 'rank': return !!rank;
      case 'traits': return traits.length > 0;
      case 'effect': return !!copy.effect;
      case 'lore': return !!copy.lore;
      case 'stats': return frame.layout !== 'corners';
      case 'cost':
      case 'power': return frame.layout === 'corners';
      case 'new': return isNew;
      // Значки и их подписи — единственные строки, которые рисует не полоса, а
      // свой слой: они держатся за кружок, а не за поток. В описи они стоят
      // затем, чтобы их можно было включить и выключить там же, где всё
      // остальное, — но полосе их отдавать нечего.
      case 'costWord':
      case 'powerWord': return false;
      default:
        // Число паспорта печатается только тогда, когда оно у карты есть:
        // ноль на бумагу не идёт, как и раньше.
        if (isStatSlot(slot)) return passport.some((one) => one.field === slot);
        return true;
    }
  }

  /**
   * Одна напечатанная вещь: строка описи, её точка-разделитель и — только у
   * паспорта — числа, собранные в одну коробку.
   */
  type Cell = {
    row: SheetRow;
    sep: 'none' | 'large' | 'always';
    stats?: SheetRow[];
  };

  /** Строки, между которыми в шапке ставится точка. Иконка расы — квадрат, а
   *  насечки уровня — насечки: точка возле них ничего не разделяет. */
  const SEP_SLOTS = new Set<SheetSlot>(['race', 'kind', 'channel', 'title', 'rank', 'stats']);

  /**
   * Строки одной полосы вместе с их разделителем.
   *
   * Разделитель принадлежит строке, ПЕРЕД которой стоит, и прячется вместе с
   * ней — иначе строка «только крупно» уносила бы с полки слово, но оставляла
   * точку. Своя ступень у него потому, что перед первой ВИДИМОЙ строкой точки
   * быть не должно: если всё, что стоит слева, показывается только крупно, то
   * и точка появляется только крупно.
   */
  function cellsOf(band: SheetBand): Cell[] {
    const list = sheetBand(rows, band).filter((row) => speaks(row.slot));
    const stats = list.filter((row) => isStatSlot(row.slot));
    const out: Cell[] = [];
    let told = false;
    list.forEach((row, i) => {
      // Семь чисел печатаются ОДНОЙ коробкой: семь отдельных абзацев в колонке
      // свойств это семь строк высотой в карту. Коробку ставит первое из них,
      // порядок внутри — порядок описи, а видна она с той величины, с какой
      // видно самое щедрое из чисел, иначе полная скрытых чисел коробка
      // оставляла бы на карте свой отступ.
      if (isStatSlot(row.slot)) {
        if (told) return;
        told = true;
        out.push({ row: { ...row, show: statGroupShow(stats) }, sep: 'none', stats });
        return;
      }
      const sep = ((): Cell['sep'] => {
        if (band !== 'head' || !SEP_SLOTS.has(row.slot)) return 'none';
        const before = list.slice(0, i).filter((one) => SEP_SLOTS.has(one.slot));
        if (!before.length) return 'none';
        return before.some((one) => one.show === 'always' || one.show === 'cell')
          ? 'always'
          : 'large';
      })();
      out.push({ row, sep });
    });
    return out;
  }

  let headCells = $derived(cellsOf('head'));
  let propCells = $derived(cellsOf('props'));
  let footCells = $derived(cellsOf('foot'));
  let overCells = $derived(cellsOf('over'));
  let costWord = $derived(sheetShows(rows, 'costWord'));
  let powerWord = $derived(sheetShows(rows, 'powerWord'));
  /** Ступени самих кружков. Раньше их гасил медиазапрос на 160 px — мимо
   *  описи и мимо хранителя; теперь решает опись, как и про всё остальное. */
  let costRow = $derived(rows.find((row) => row.slot === 'cost'));
  let powerRow = $derived(rows.find((row) => row.slot === 'power'));
  /** Кружок здоровья. Дом даёт ему клетку боя и только её. */
  let healthRow = $derived(rows.find((row) => row.slot === 'healthMark'));
  let badgeRow = $derived<Record<BadgeKind, SheetRow | undefined>>({
    cost: costRow,
    power: powerRow,
    health: healthRow,
  });
  /** Метка «новая» лежит поверх карты — тогда, и только тогда, шапка ей
   *  уступает место. Поставленная в полосу, она стоит в потоке и не мешает. */
  let newOver = $derived(rows.some((row) => row.slot === 'new' && row.band === 'over' && row.show !== 'never'));

  /** Партия, а не полка: сцена передаёт, сколько здоровья осталось. Ширина
   *  клетки при этом может перешагнуть порог полки (161 px) — иначе фотография
   *  на доске нечитаема, — и опись, читая одну ширину, напечатала бы стоимость
   *  вместо здоровья. `alive` и есть признак «это тело в бою», вторым обликом
   *  карты это не становится: отрисовщик тот же, плотность как у клетки. */
  let inMatch = $derived(alive != null);
  /** Сургуч на кружке здоровья. Тот же `wearSeed`, что рвёт бумагу, и это не
   *  экономия: печать и лист — одно тело, и трещины на них обязаны стоять по
   *  одному счёту, иначе лечение снимало бы выщерб с края и оставляло трещину
   *  на воске. Только в партии: на полке `hurt` — единица, карта цела. */
  let sealed = $derived(inMatch ? sealWear(hurt, wearSeed) : null);

  /** Сколько уступают значкам и метке шапка и текст свойств. Считается из
   *  того, где значки СТОЯТ, а не из того, где они стояли в первый день. */
  let reserve = $derived(
    badgeReserve(frame, {
      isNew,
      costOn: !inMatch && !!costRow && costRow.show !== 'never',
      powerOn: !!powerRow && powerRow.show !== 'never',
      healthOn: !!healthRow && healthRow.show !== 'never' && (inMatch || healthRow.show === 'cell' || healthRow.show === 'cellOnly'),
      costWord: costWord && !inMatch,
      powerWord,
      newOver,
    }),
  );

  /* ── Опись под мышью ─────────────────────────────────────────────────────
     Строку берут там, где она напечатана, и отпускают там, где ей стоять. По
     тому же закону, что и детали рамки: место назначается на самом предмете, а
     не числом в колонке сбоку. Числа и списки остаются — но вторым входом. */

  /** Куда попадёт строка, если отпустить сейчас. `ok` — можно ли ей туда:
   *  полоса, в которой строке стоять нельзя, не берётся вовсе. */
  let rowAim = $state<{ band: SheetBand; before: SheetSlot | null; ok: boolean } | null>(null);
  /** Метка вставки, в координатах карты. Меряется по соседям, а не считается
   *  вторым способом: у полосы свойств строки разной высоты, и вычисленная
   *  черта разошлась бы с настоящей на первой же длинной черте. */
  let rowMark = $state<{ left: number; top: number; width: number; height: number } | null>(null);
  let rowMoved = false;

  function rowTake(slot: SheetSlot, event: PointerEvent) {
    if (!rowsEditable || event.button !== 0) return;
    event.preventDefault();
    // Ниже лежит карта со своими нажатиями — наведение фотографии, взятие
    // детали резьбы. Строку берут поверх них.
    event.stopPropagation();
    onEditStart?.();
    rowMoved = false;
    rowHeld = slot;
    rowAim = null;
    rowMark = null;
    aimRow(event);
  }

  /** Где строка окажется, если отпустить здесь. */
  function aimRow(event: PointerEvent) {
    const held = rowHeld;
    if (!held || !root) return;
    rowMoved = true;
    const allowed = SHEET_SLOT_BANDS[held];
    let band: SheetBand = 'over';
    for (const one of ['head', 'props', 'foot'] as const) {
      const box = root.querySelector(`.band--${one}`)?.getBoundingClientRect();
      // Полосы лежат стопкой во всю ширину окна, поэтому спрашивается только
      // высота: вбок из полосы не выйти, не выйдя из карты.
      if (box && box.height > 0 && event.clientY >= box.top && event.clientY <= box.bottom) {
        band = one;
        break;
      }
    }
    const ok = allowed.includes(band);
    const seat = ok ? seatIn(band, held, event) : null;
    rowAim = { band, before: seat?.before ?? null, ok };
    rowMark = seat && ok ? seat.mark : null;
  }

  /**
   * Место в полосе и черта, которой оно показано.
   *
   * Шапка и подвал идут вдоль, свойства — вниз, поэтому спрашивается разная
   * ось; таблицы для этого не нужно, полос всего четыре и направление у них
   * то же, что у их вёрстки.
   */
  function seatIn(
    band: SheetBand,
    held: SheetSlot,
    event: PointerEvent,
  ): {
    before: SheetSlot | null;
    mark: { left: number; top: number; width: number; height: number };
  } | null {
    if (!root) return null;
    const card = root.getBoundingClientRect();
    // «Поверх» — не полоса, а сама коробка окна: строки, стоящие поверх карты,
    // лежат в ней рядом с полосами, а не внутри одной из них.
    const host =
      band === 'over' ? contentEl : (root.querySelector(`.band--${band}`) as HTMLElement | null);
    if (!host) return null;
    // На любой глубине, а не только среди прямых детей: числа паспорта лежат
    // внутри своей коробки, и без этого их нельзя было бы ни взять, ни
    // положить между собой.
    const boxes = Array.from(host.querySelectorAll<HTMLElement>('[data-row]'))
      .filter((one) => one.dataset.row !== held)
      .map((one) => ({ slot: one.dataset.row!, box: one.firstElementChild?.getBoundingClientRect() }))
      .filter((one): one is { slot: string; box: DOMRect } => !!one.box && one.box.width > 0);
    const along = band === 'props' ? 'vertical' : 'horizontal';
    const at = (box: DOMRect) =>
      along === 'vertical' ? box.top + box.height / 2 : box.left + box.width / 2;
    const here = along === 'vertical' ? event.clientY : event.clientX;
    let index = boxes.findIndex((one) => here < at(one.box));
    if (index < 0) index = boxes.length;
    // Соседом, а не номером: карта видит только напечатанное, а опись держит и
    // то, чему на этой карте сказать нечего, — «третье место» значило бы у них
    // разное. Сосед один и тот же у обоих.
    const before = (boxes[index]?.slot as SheetSlot | undefined) ?? null;
    const near = boxes[Math.min(index, boxes.length - 1)]?.box;
    const room = host.getBoundingClientRect();
    if (!near) {
      return {
        before,
        mark: { left: room.left - card.left, top: room.top - card.top, width: room.width, height: 2 },
      };
    }
    const after = index >= boxes.length;
    const mark =
      along === 'vertical'
        ? {
            left: near.left - card.left,
            top: (after ? near.bottom : near.top) - card.top,
            width: near.width,
            height: 2,
          }
        : {
            left: (after ? near.right : near.left) - card.left,
            top: near.top - card.top,
            width: 2,
            height: near.height,
          };
    return { before, mark };
  }

  function rowLet() {
    const held = rowHeld;
    const aim = rowAim;
    rowHeld = null;
    rowAim = null;
    rowMark = null;
    onEditEnd?.();
    // Нажали и отпустили не двинувшись — это выбор строки, а не перекладка.
    if (!held || !aim || !aim.ok || !rowMoved) return;
    onRowMove?.(held, aim.band, aim.before);
  }

  // Слушает окно, а не саму строку: у обёртки строки нет своей коробки
  // (`display: contents`), захватывать указатель ей нечем, а рука, потерявшая
  // строку на полпути к соседней полосе, — это перекладка, которой не было.
  $effect(() => {
    if (!rowHeld || !rowsEditable) return;
    const move = (e: PointerEvent) => aimRow(e);
    const done = () => rowLet();
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', done);
    window.addEventListener('pointercancel', done);
    return () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', done);
      window.removeEventListener('pointercancel', done);
    };
  });

  let varStyle = $derived(
    Object.entries(vars)
      .map(([k, v]) => `${k}:${v}`)
      .join(';') +
      `;--head-pad-left:${reserve.headLeft.toFixed(1)}cqi` +
      `;--head-pad-right:${reserve.headRight.toFixed(1)}cqi` +
      `;--body-pad-left:${reserve.bodyLeft.toFixed(1)}cqi` +
      `;--body-pad-right:${reserve.bodyRight.toFixed(1)}cqi`,
  );

  // The photo's pan/zoom. `object-fit: cover` at a FIXED, centred
  // `object-position` is what guarantees no gaps at zoom 1 — that part never
  // moves. Panning and zooming both live in one `transform`, applied on top
  // of that already-covering image, which is the only way panning can ever
  // reach both axes: `object-position` computes its crop against the
  // element's own un-scaled box, so a `transform: scale()` layered on top of
  // it can enlarge what's already chosen but can never reveal more of the
  // source — an image whose own proportions happen to match the card in one
  // axis would have nothing to pan into on that axis at ANY zoom. Doing both
  // through one transform instead means zooming in always opens up room to
  // pan in both directions, on every photo.
  //
  // `translate()` is listed before `scale()` — composed right-to-left, scale
  // applies to the point first and translate after, so a percentage in
  // `translate()` lands at that exact fraction of the box regardless of the
  // zoom level, and the max pan at a given zoom is simply half of what the
  // zoom overshoots the box by.
  let artTx = $derived(((focal.x - 0.5) * (focal.zoom - 1) * 100).toFixed(2));
  let artTy = $derived(((focal.y - 0.5) * (focal.zoom - 1) * 100).toFixed(2));

  // Pointer tilt and the foil sweep. Written as two custom properties rather
  // than an inline transform so the CSS below owns the whole effect: it can be
  // switched off wholesale by a media query, which an inline style cannot.
  let root = $state<HTMLElement | null>(null);
  /** The window's own box — bound only so a frame-shape drag can read its
   *  live pixel height, the one thing `--header-share` etc. are a fraction
   *  of. Never written to. */
  let contentEl = $state<HTMLElement | null>(null);

  /** Сама карта — коробка, в долях которой стоят значки стоимости и силы.
   *  Не окно: значок носится ПОВЕРХ резьбы так же охотно, как внутри окна,
   *  и в долях окна место на раме невыразимо. */
  let cardEl = $state<HTMLElement | null>(null);

  /** Полоса свойств — единственная, у которой высота не задана, и
   *  единственная, которая поэтому умеет переполниться. */
  let propsEl = $state<HTMLElement | null>(null);

  /**
   * Сторож переполнения.
   *
   * Считает не «похоже ли, что много», а разницу между написанным и окном, и
   * пересчитывает её на каждое изменение ширины: та же карта на листе взятия
   * влезает, а на полке нет, и вопрос «влезло ли» без ширины не имеет ответа.
   * Ставится только там, где о нём спросили: `ResizeObserver` на каждой карте
   * полки — сорок наблюдателей ради лампы, которую там никто не зажигает.
   */
  $effect(() => {
    const tell = onfit;
    const box = propsEl;
    if (!tell || !box) return;
    // Прочитано нарочно: полоса меряется заново не только когда меняется
    // ширина, но и когда меняется написанное. Иначе лампа гасла бы на карте,
    // у которой хранитель только что стёр половину строк, и загоралась бы
    // через раз на той, где дописал.
    void rows;
    void copy;
    void traits;
    void passport;
    void vars;
    let last = -1;
    const measure = () => {
      const over = Math.max(0, box.scrollHeight - box.clientHeight);
      if (over === last) return;
      last = over;
      tell(over);
    };
    const watch = new ResizeObserver(measure);
    watch.observe(box);
    const settle = requestAnimationFrame(measure);
    return () => {
      cancelAnimationFrame(settle);
      watch.disconnect();
    };
  });
  let frameId = 0;

  // `container-type: inline-size` together with `aspect-ratio` on the same
  // element (`.slot`, below) is a combination some browsers need a second
  // layout pass to settle — the first paint can size the bands from a
  // not-yet-resolved aspect ratio, so a click right after mount can land on
  // the wrong band until something else forces a reflow. Forcing one here,
  // whenever the frame's own vars change, closes that window without waiting
  // for the keeper's first click to be the thing that fixes it.
  $effect(() => {
    if (!root) return;
    void varStyle;
    void root.getBoundingClientRect();
  });

  function track(event: PointerEvent) {
    if (!interactive || !root) return;
    const el = root;
    const rect = el.getBoundingClientRect();
    const x = (event.clientX - rect.left) / rect.width;
    const y = (event.clientY - rect.top) / rect.height;
    // One write per frame. Pointermove fires far faster than the screen paints,
    // and every write here invalidates layout on a card that may be one of forty.
    cancelAnimationFrame(frameId);
    frameId = requestAnimationFrame(() => {
      el.style.setProperty('--mx', x.toFixed(3));
      el.style.setProperty('--my', y.toFixed(3));
    });
  }

  function rest() {
    cancelAnimationFrame(frameId);
    root?.style.setProperty('--mx', '0.5');
    root?.style.setProperty('--my', '0.5');
  }

  // ── Editing ─────────────────────────────────────────────────────────────

  async function editArt() {
    const file = await pickImageFile();
    if (!file) return;
    try {
      const imported = await api.importMediaWithVariants(file, 'images', 'battle-card-art');
      // Both: `artUrl` is what renders, `artUrlOverride` is what marks this as
      // the card's own picture rather than a borrowed one — an upload through
      // the card face is always the former, never a coincidence of the latter.
      card.artUrl = imported.url;
      card.artUrlOverride = imported.url;
    } catch (e) {
      onError?.(String(e));
    }
  }

  let framePopoverOpen = $state(false);

  /** A picture for this one card, worn instead of the tier's shared frame. */
  async function uploadCardFrame() {
    const file = await pickImageFile();
    if (!file) return;
    try {
      const art = await api.adminUploadBattleFrameArt(file);
      const patch: FrameOverride = {
        frameImage: art.url,
        frameMode: art.hasAlpha ? 'overlay' : 'behind',
      };
      card.frameOverride = JSON.stringify(patch);
    } catch (e) {
      onError?.(String(e));
    } finally {
      framePopoverOpen = false;
    }
  }

  async function uploadRaceIcon() {
    const file = await pickImageFile();
    if (!file) return;
    try {
      const art = await api.adminUploadBattleFrameArt(file);
      onIconUpload?.(art.url);
    } catch (e) {
      onError?.(String(e));
    }
  }

  function handleIconClick() {
    if (raceIconEditable) {
      uploadRaceIcon();
    } else if (editable) {
      onEditRace?.();
    }
  }

  // ── The window on the photograph ──────────────────────────────────────────
  //
  // A click with no movement replaces the picture; a click that moves aims the
  // window instead — the same gesture reader of "tap vs. drag" any photo app
  // uses, so the art band needs no separate controls to do both jobs the
  // sketch asked of it. The drag itself is relative, not a jump-to-cursor:
  // the picture is grabbed and follows the pointer, the way it works
  // everywhere this gesture exists — a click a mouse-width off-centre must
  // not snap the photo across the frame.
  //
  // `focal.x`/`focal.y` stay in the stored 0..1 shape (0.5 = centred, same
  // JSON as before), but now read as a FRACTION of how far the photo can
  // currently be panned rather than an absolute crop coordinate — the actual
  // pixel range that fraction spans grows with zoom and is always exactly
  // what keeps the photo covering the window, no more and no less, at every
  // zoom level automatically.

  let dragging = $state(false);
  let dragMoved = false;

  function clamp01(v: number): number {
    return Math.min(1, Math.max(0, v));
  }

  function aimDown(event: PointerEvent & { currentTarget: HTMLElement }) {
    if (!editable) return;
    dragging = true;
    dragMoved = false;
    // Captured so the drag survives leaving the band — letting go outside it
    // would otherwise strand the picture wherever the pointer last was seen.
    event.currentTarget.setPointerCapture(event.pointerId);
  }

  function aimMove(event: PointerEvent & { currentTarget: HTMLElement }) {
    if (!dragging) return;
    dragMoved = true;
    const box = event.currentTarget.getBoundingClientRect();
    if (!box.width || !box.height) return;
    // How far the photo can be pushed off-centre at this zoom, in percent of
    // the window — zero at zoom 1, where `object-fit: cover` already has no
    // slack to move into on either axis.
    const maxPercent = 50 * (focal.zoom - 1);
    if (maxPercent <= 0) return;
    const tx = (focal.x - 0.5) * 2 * maxPercent + (event.movementX / box.width) * 100;
    const ty = (focal.y - 0.5) * 2 * maxPercent + (event.movementY / box.height) * 100;
    const x = clamp01(tx / (2 * maxPercent) + 0.5);
    const y = clamp01(ty / (2 * maxPercent) + 0.5);
    card.artFocal = JSON.stringify({ ...focal, x, y });
  }

  function aimUp(event: PointerEvent & { currentTarget: HTMLElement }) {
    if (!dragging) return;
    dragging = false;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    if (!dragMoved) editArt();
  }

  function aimZoom(event: WheelEvent) {
    if (!editable) return;
    event.preventDefault();
    const zoom = Math.min(3, Math.max(1, focal.zoom - event.deltaY * 0.002));
    card.artFocal = JSON.stringify({ ...focal, zoom });
  }

  // ── The frame's own shape, dragged instead of dialled ─────────────────────
  //
  // Same idea as aiming the photograph: a handle sits right on the seam it
  // moves, and a pointer-capture drag reads the one number that seam is.
  // `frame` is the actual object living in the keeper's `frames` array (see
  // `frameFor` in `battles.ts` — it returns that array's own entry, not a
  // copy), so writing to it here is the same write the Frames tab's sliders
  // make, just aimed by hand instead of by number.

  type ShareKey = 'headerShare' | 'artShare' | 'footShare';
  const SHARE_BOUNDS: Record<ShareKey, [number, number]> = {
    headerShare: [0, 0.3],
    artShare: [0.12, 0.85],
    footShare: [0, 0.3],
  };

  let frameDragKind = $state<ShareKey | InsetKey | null>(null);

  /** The rank's own shared frame, ignoring this one card's `frameOverride` —
   *  `frame` above is override-aware because rendering should show what a
   *  dressed card actually wears, but a drag here is meant for the whole
   *  rank, the same target the Frames tab's own sliders write to. Without
   *  this, dragging a handle on a card that happens to carry a picture of
   *  its own would edit a throwaway copy instead of the five-frame dictionary. */
  function rankFrame() {
    return frameFor(card.tier, frames);
  }

  /** What the inset handles actually mutate: a race's own per-level patch
   *  when `frameEditTarget` names one, else the tier's shared frame — the
   *  same fallback `rankFrame` uses, kept separate because `frameEditTarget`
   *  is missing the header/art/foot/badge fields `rankFrame`'s other callers
   *  need. */
  function insetTarget() {
    return frameEditTarget ?? rankFrame();
  }

  function shareDragStart(kind: ShareKey, event: PointerEvent & { currentTarget: HTMLElement }) {
    if (!frameEditable) return;
    onEditStart?.();
    event.preventDefault();
    event.stopPropagation();
    frameDragKind = kind;
    event.currentTarget.setPointerCapture(event.pointerId);
  }

  function shareDragMove(event: PointerEvent) {
    if (!frameDragKind || !(frameDragKind in SHARE_BOUNDS)) return;
    const kind = frameDragKind as ShareKey;
    const h = contentEl?.getBoundingClientRect().height;
    if (!h) return;
    const [min, max] = SHARE_BOUNDS[kind];
    const target = rankFrame();
    const current = target[kind] ?? 0;
    target[kind] = Math.min(max, Math.max(min, current + event.movementY / h));
  }

  function insetDragStart(kind: InsetKey, event: PointerEvent & { currentTarget: HTMLElement }) {
    if (!frameEditable) return;
    onEditStart?.();
    event.preventDefault();
    event.stopPropagation();
    frameDragKind = kind;
    event.currentTarget.setPointerCapture(event.pointerId);
  }

  /** Top and left grow the inset in the direction the handle is dragged;
   *  bottom and right sit on the far edge, so growing their inset means
   *  dragging the handle the other way. */
  const INSET_SIGN: Record<InsetKey, 1 | -1> = {
    insetTop: 1,
    insetLeft: 1,
    insetBottom: -1,
    insetRight: -1,
  };

  function insetDragMove(event: PointerEvent) {
    if (!frameDragKind || !(frameDragKind in INSET_SIGN)) return;
    const kind = frameDragKind as InsetKey;
    const rect = root?.getBoundingClientRect();
    if (!rect) return;
    const vertical = kind === 'insetTop' || kind === 'insetBottom';
    const size = vertical ? rect.height : rect.width;
    if (!size) return;
    const movement = vertical ? event.movementY : event.movementX;
    const delta = ((movement / size) * 100) * INSET_SIGN[kind];
    applyInsetDelta(insetTarget(), kind, delta, !frameEditTarget);
  }

  function frameDragMove(event: PointerEvent) {
    shareDragMove(event);
    insetDragMove(event);
  }

  function frameDragEnd(event: PointerEvent & { currentTarget: HTMLElement }) {
    if (!frameDragKind) return;
    frameDragKind = null;
    onEditEnd?.();
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
  }

  // ── Резьба, собираемая руками ─────────────────────────────────────────────
  //
  // Число в поле сбоку — точная запись того, что глаз проверяет на картинке:
  // сходится ли угол со стороной. Поэтому деталь берут прямо на карте, а поля
  // остаются вторым, точным входом — как у ценника и силы, где перетаскивание
  // и числовой редактор живут на одной кнопке.
  //
  // Берут ОТДЕЛЬНУЮ КОПИЮ: левый верхний угол — не то же самое, что правый
  // нижний, и левая сторона — не зеркало правой. Пока все копии ходили одним
  // числом, подогнать раму, у которой верх шире низа, было нельзя. Копии всё
  // так же ходят вместе, пока связка включена, — просто теперь это выбор, а не
  // устройство.

  /** Off while dressing a race's level, for the same reason the band seams
   *  are: a picture chosen for one level is not the place to re-cut the rank's
   *  carving. The pieces stay chrome everywhere else on the site. */
  let sliceEditable = $derived(frameEditable && !frameEditTarget && sliced);

  /** Every copy of every piece the frame is built from — named slots and the
   *  keeper's own flourishes in ONE list, each already carrying the inline
   *  style that places it. One list and one style so there is exactly one
   *  renderer: a preview with a second one is a preview that eventually lies. */
  let copies = $derived(sliced ? carvedCopies(frame) : []);

  /** What a drag on the held copy is doing: moving it, or growing it past its
   *  band. Two gestures, two grips, no modifier key to remember. A size drag
   *  names which edges of the box are in hand, so a side changes only that
   *  axis and a corner still changes both. */
  let sliceDrag = $state<'move' | 'size' | null>(null);
  let sliceSizeX = $state<SliceResizeX | null>(null);
  let sliceSizeY = $state<SliceResizeY | null>(null);
  /** Held for the length of a size drag so the arrows stay under the pointer
   *  even after it leaves the thin hit strip. */
  let sliceCursor = $state<string | null>(null);
  /** A press that never moved is a pick, not a drag — the way through to a
   *  copy lying under another. */
  let sliceMoved = false;
  /** Whether this press landed on a spot the copy already in hand covers.
   *  Two things hang off it: what stays in hand for the drag, and whether
   *  letting go without moving asks for the copy underneath. */
  let sliceAgain = false;

  /** One copy on the card, found by what it IS rather than by a class table
   *  that would have to be kept in step with the markup — and that could not
   *  name an ornament the keeper invented five seconds ago at all. */
  function copyEl(id: string, side: SliceSide): Element | null {
    return root?.querySelector(`[data-piece="${CSS.escape(id)}"][data-side="${side}"]`) ?? null;
  }

  /**
   * Where the held copy's box is, in % of the card — the one place the size
   * handles may sit, one per edge and one per corner.
   *
   * Measured rather than computed a second time, and for a reason that is not
   * laziness: the handles cannot live INSIDE the copy they size. A piece
   * carries a `layer`, a `z-index` on a positioned element opens a stacking
   * context, and a child can never climb out of its parent's — so a handle
   * inside a corner at layer 2 sits under the accent at layer 5 that covers
   * the same band, visible and untouchable. Out here, above the whole
   * assembly, they are always reachable; and measuring keeps their place
   * honest however the copy is grown, slid or turned.
   */
  let gripBox = $state<{ left: number; top: number; width: number; height: number } | null>(null);

  $effect(() => {
    if (!sliceEditable || !sliceHeld) {
      gripBox = null;
      return;
    }
    // Re-measured whenever the assembly moves — including every frame of a
    // drag, since that is exactly when the handles must keep up with their copy.
    void copies;
    const el = root;
    const at = copyEl(sliceHeld.id, sliceHeld.side);
    if (!el || !at) {
      gripBox = null;
      return;
    }
    const card = el.getBoundingClientRect();
    const box = at.getBoundingClientRect();
    if (!card.width || !card.height) return;
    gripBox = {
      left: ((box.left - card.left) / card.width) * 100,
      top: ((box.top - card.top) / card.height) * 100,
      width: (box.width / card.width) * 100,
      height: (box.height / card.height) * 100,
    };
  });

  /** The eight places a box can be asked to grow from. Edges first, corners
   *  on top of them (higher z-index in the markup's own CSS) so a corner
   *  press is never stolen by the side it sits on. */
  const SLICE_RESIZE_HANDLES: {
    id: string;
    x: SliceResizeX | null;
    y: SliceResizeY | null;
    cursor: string;
  }[] = [
    { id: 'n', x: null, y: 'top', cursor: 'ns-resize' },
    { id: 's', x: null, y: 'bottom', cursor: 'ns-resize' },
    { id: 'e', x: 'right', y: null, cursor: 'ew-resize' },
    { id: 'w', x: 'left', y: null, cursor: 'ew-resize' },
    { id: 'nw', x: 'left', y: 'top', cursor: 'nwse-resize' },
    { id: 'ne', x: 'right', y: 'top', cursor: 'nesw-resize' },
    { id: 'sw', x: 'left', y: 'bottom', cursor: 'nesw-resize' },
    { id: 'se', x: 'right', y: 'bottom', cursor: 'nwse-resize' },
  ];

  function resizeHandleStyle(
    box: { left: number; top: number; width: number; height: number },
    handle: (typeof SLICE_RESIZE_HANDLES)[number],
  ): string {
    const parts = [`cursor:${handle.cursor}`];
    if (handle.x && handle.y) {
      parts.push(
        `left:${handle.x === 'left' ? box.left : box.left + box.width}%`,
        `top:${handle.y === 'top' ? box.top : box.top + box.height}%`,
      );
    } else if (handle.x) {
      parts.push(
        `left:${handle.x === 'left' ? box.left : box.left + box.width}%`,
        `top:${box.top}%`,
        `height:${box.height}%`,
      );
    } else {
      parts.push(
        `top:${handle.y === 'top' ? box.top : box.top + box.height}%`,
        `left:${box.left}%`,
        `width:${box.width}%`,
      );
    }
    return parts.join(';');
  }

  function sliceTake(
    id: string,
    side: SliceSide,
    how: 'move' | 'size',
    event: PointerEvent & { currentTarget: HTMLElement },
    sizeX: SliceResizeX | null = null,
    sizeY: SliceResizeY | null = null,
  ) {
    if (!sliceEditable) return;
    onEditStart?.();
    event.preventDefault();
    // The grip is its own element above the piece: without this, one press
    // would start both a size drag and a move drag on the same pointer.
    event.stopPropagation();
    // What is in hand STAYS in hand while the keeper keeps pressing where it
    // lies. Without this a base corner could never be moved once its accent
    // was uploaded: the accent covers the whole band, so every press would
    // take the accent back, and the copy underneath would be visible and
    // untouchable. Pressing anywhere the held copy does not reach picks up
    // whatever is there instead, so a far copy still takes one press.
    sliceAgain = !!sliceHeld && partsUnder(event.clientX, event.clientY).some(sameAsHeld);
    if (!sliceAgain) sliceHeld = { id, side };
    sliceDrag = how;
    sliceSizeX = how === 'size' ? sizeX : null;
    sliceSizeY = how === 'size' ? sizeY : null;
    sliceCursor =
      how === 'size'
        ? (SLICE_RESIZE_HANDLES.find((h) => h.x === sizeX && h.y === sizeY)?.cursor ?? null)
        : null;
    sliceMoved = false;
    event.currentTarget.setPointerCapture(event.pointerId);
  }

  function sameAsHeld(part: { id: string; side: SliceSide }) {
    return !!sliceHeld && part.id === sliceHeld.id && part.side === sliceHeld.side;
  }

  /**
   * Both gestures are the same two numbers read the same way — but which WAY
   * is the copy's own: dragging a top-left corner rightward widens it, and
   * dragging a top-right corner rightward has to narrow it, or the ornament
   * would run off the card while the pointer said "bigger".
   *
   * With the link on, the delta is read ONCE through the held copy's signs and
   * then written into every copy of the piece — the same number in all of them,
   * which is exactly what the pieces had when they shared one, and therefore a
   * frame that stays symmetric. (Passing the raw pointer delta to each copy's
   * own signs instead would slide all four the same way across the screen,
   * which is a frame sliding off its card, not a frame being widened.) With
   * the link off, only the copy in hand moves.
   */
  function sliceDragMove(event: PointerEvent) {
    if (!sliceDrag || !sliceHeld) return;
    const rect = root?.getBoundingClientRect();
    if (!rect?.width || !rect.height) return;
    const dx = (event.movementX / rect.width) * 100;
    const dy = (event.movementY / rect.height) * 100;
    if (!dx && !dy) return;
    const target = rankFrame();
    const piece = livePiece(target, sliceHeld.id);
    const kind = kindOf(target, sliceHeld.id);
    if (!piece || !kind) return;
    sliceMoved = true;
    const moving = piece.linked !== false ? KIND_SIDES[kind] : [sliceHeld.side];
    const sign = sliceSigns(sliceHeld.side);
    const held = (v: number) => Math.min(SLICE_GROW_MAX, Math.max(-SLICE_GROW_MAX, v));
    const delta =
      sliceDrag === 'size'
        ? sliceResizeDelta(kind, sliceHeld.side, sliceSizeX, sliceSizeY, dx, dy)
        : null;
    for (const side of moving) {
      if (!piece.places[side]) {
        piece.places[side] = { growX: 0, growY: 0, nudgeX: 0, nudgeY: 0, shown: true };
      }
      const at = piece.places[side];
      if (delta) {
        at.growX = held(at.growX + delta.growX);
        at.growY = held(at.growY + delta.growY);
        at.nudgeX = held(at.nudgeX + delta.nudgeX);
        at.nudgeY = held(at.nudgeY + delta.nudgeY);
      } else {
        at.nudgeX = held(at.nudgeX + dx * sign.nudgeX);
        at.nudgeY = held(at.nudgeY + dy * sign.nudgeY);
      }
    }
  }

  /** Every drawn copy whose box covers this point, the topmost first. Read off
   *  the rendered boxes rather than recomputed from the insets, so it can never
   *  disagree with what the keeper is actually looking at — and a copy the
   *  keeper put out is not drawn, so it is not here either. */
  function partsUnder(x: number, y: number): { id: string; side: SliceSide }[] {
    if (!root) return [];
    return copies
      .filter((copy) => {
        const box = copyEl(copy.id, copy.side)?.getBoundingClientRect();
        return !!box && x >= box.left && x <= box.right && y >= box.top && y <= box.bottom;
      })
      .sort((a, b) => b.layer - a.layer)
      .map(({ id, side }) => ({ id, side }));
  }

  /** Letting go without having moved anything steps DOWN through the copies
   *  stacked under the pointer, and wraps. The first press on a spot takes
   *  the topmost copy there, as a press should; it is the SECOND that asks
   *  for the one beneath — which is how an accent and the corner under it are
   *  told apart at all, sharing as they do exactly one box. */
  function sliceDragEnd(event: PointerEvent & { currentTarget: HTMLElement }) {
    if (!sliceDrag) return;
    const wasSize = sliceDrag === 'size';
    sliceDrag = null;
    sliceSizeX = null;
    sliceSizeY = null;
    sliceCursor = null;
    onEditEnd?.();
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    // A press on a size handle is asking the box, not the stack: cycling
    // through to the copy underneath would steal the very gesture that
    // just named an edge.
    if (sliceMoved || !sliceAgain || wasSize) return;
    const stack = partsUnder(event.clientX, event.clientY);
    if (stack.length < 2) return;
    const at = stack.findIndex(sameAsHeld);
    sliceHeld = stack[(at + 1) % stack.length];
  }

  // ── Cost and power, dragged instead of dialled ─────────────────────────────
  //
  // Same tap-vs-drag gesture as the photograph: held and moved, the badge
  // follows the pointer; let go without moving it and its own numeric editor
  // opens instead — the keyboard-reachable form a drag can never be exact
  // enough to replace.



  let badgeDragKind = $state<BadgeKind | null>(null);
  let badgeMoved = false;
  let badgePopoverOpen = $state<BadgeKind | null>(null);
  /** Where the popover sits, in % of `.slot` (this component's own root) —
   *  read off the badge itself when it opens rather than reusing
   *  `frame.costX`/`costY`: the badge hangs off its own centre
   *  (`translate(-50%, -50%)`) and carries a caption of unknown width, so the
   *  numbers name a point the popover must not simply repeat.
   *  Rendered as a sibling of `.card` rather than inside it: an unrelated
   *  global `.card { overflow: hidden }` rule (see the admin design system)
   *  would otherwise clip it, the same trap `.frame-popover` sits in. */
  let badgePopoverPos = $state<{ left: number; top: number } | null>(null);

  /** Сама панель — чтобы её можно было ИЗМЕРИТЬ. Место ей назначено в долях
   *  карты, а влезает она или нет — вопрос к экрану, и ответить на него можно
   *  только меркой. */
  let badgePopoverEl = $state<HTMLElement | null>(null);

  /**
   * Панель, приведённая в видимое.
   *
   * Открывается она под значком, и это верно ровно до нижнего значка: под ним
   * до края экрана остаётся полсантиметра, панель уезжает вниз, и добраться до
   * неё нечем — карта не прокручивается вслед за тем, чего у неё нет.
   *
   * Сперва ПЕРЕВОРОТ: не влезло под значком — открываем над ним, потому что
   * подпихивать вверх панель, которая тогда закроет полкарты, значит спорить с
   * тем, где её открыли. И только потом, если и так не встало, — сдвиг в
   * пределы экрана.
   *
   * Пишет прямо в стиль элемента, а не в состояние: состояние меняло бы
   * разметку, разметка — мерку, и мерка гоняла бы саму себя по кругу. Проход
   * ровно один: сбросить, измерить, назначить.
   */
  function fitBadgePopover() {
    const el = badgePopoverEl;
    if (!el) return;
    const pad = 8;
    el.classList.remove('badge-popover--up');
    el.style.setProperty('--bi-shift-x', '0px');
    el.style.setProperty('--bi-shift-y', '0px');
    let box = el.getBoundingClientRect();
    if (box.bottom > window.innerHeight - pad && box.height + pad * 2 <= window.innerHeight) {
      el.classList.add('badge-popover--up');
      box = el.getBoundingClientRect();
    }
    let dx = 0;
    let dy = 0;
    if (box.bottom > window.innerHeight - pad) dy = window.innerHeight - pad - box.bottom;
    if (box.top + dy < pad) dy = pad - box.top;
    if (box.right > window.innerWidth - pad) dx = window.innerWidth - pad - box.right;
    if (box.left + dx < pad) dx = pad - box.left;
    el.style.setProperty('--bi-shift-x', `${dx}px`);
    el.style.setProperty('--bi-shift-y', `${dy}px`);
  }

  // Мерить приходится и после открытия, и после всякого движения под панелью:
  // стол хранителя прокручивается, и панель, поставленная один раз, уехала бы
  // вместе со значком, но уже без права на своё место.
  $effect(() => {
    if (!badgePopoverOpen || !badgePopoverPos || !badgePopoverEl) return;
    fitBadgePopover();
    const again = () => fitBadgePopover();
    window.addEventListener('resize', again);
    window.addEventListener('scroll', again, true);
    return () => {
      window.removeEventListener('resize', again);
      window.removeEventListener('scroll', again, true);
    };
  });

  function badgeDragStart(kind: BadgeKind, event: PointerEvent & { currentTarget: HTMLElement }) {
    if (!frameEditable) return;
    onEditStart?.();
    event.preventDefault();
    event.stopPropagation();
    badgeDragKind = kind;
    badgeMoved = false;
    event.currentTarget.setPointerCapture(event.pointerId);
  }

  function badgeDragMove(event: PointerEvent) {
    if (!badgeDragKind || !cardEl) return;
    const rect = cardEl.getBoundingClientRect();
    if (!rect.width || !rect.height) return;
    badgeMoved = true;
    const { x, y } = BADGE_FIELDS[badgeDragKind];
    const target = rankFrame();
    const dx = (event.movementX / rect.width) * 100;
    const dy = (event.movementY / rect.height) * 100;
    // Прижато к карте тем же `badgeSpot`, которым значок рисуется: иначе число
    // уезжало бы дальше кружка, и обратный ход начинался бы не сразу.
    const spot = badgeSpot(
      (target[x] ?? 0) + dx,
      (target[y] ?? 0) + dy,
      frame.aspect || DEFAULT_ASPECT,
      badgeExtent(frame, badgeDragKind),
    );
    target[x] = spot.x;
    target[y] = spot.y;
  }

  function badgeDragEnd(event: PointerEvent & { currentTarget: HTMLElement }) {
    if (!badgeDragKind) return;
    const kind = badgeDragKind;
    const badgeEl = event.currentTarget;
    badgeDragKind = null;
    if (badgeEl.hasPointerCapture(event.pointerId)) {
      badgeEl.releasePointerCapture(event.pointerId);
    }
    // A click that never moved opens the badge's own editor; a drag has
    // already placed it, and popping the editor up too would just be in the way.
    if (badgeMoved) return;
    if (badgePopoverOpen === kind) {
      badgePopoverOpen = null;
      return;
    }
    const rootRect = root?.getBoundingClientRect();
    if (!rootRect || !rootRect.width || !rootRect.height) return;
    const badgeRect = badgeEl.getBoundingClientRect();
    badgePopoverPos = {
      left: ((badgeRect.left + badgeRect.width / 2 - rootRect.left) / rootRect.width) * 100,
      top: ((badgeRect.top + badgeRect.height / 2 - rootRect.top) / rootRect.height) * 100,
    };
    badgePopoverOpen = kind;
  }
</script>

<article
  bind:this={root}
  class="slot"
  class:slice-sizing={!!sliceCursor}
  class:slot--match={inMatch}
  class:slot--flush-foot={frame.layout === 'corners' && !frameEditable && footCells.length === 0}
  data-tier={card.tier}
  data-layout={frame.layout}
  style={varStyle}
  style:cursor={sliceCursor}
  style:view-transition-name={transition ? cardTransitionName(card) : undefined}
  onpointermove={track}
  onpointerleave={rest}
  aria-label="{copy.title || rank} — {rank}"
>
 <div
   class="card"
   bind:this={cardEl}
   class:card--down={!owned}
   class:card--still={!interactive}
   class:card--dressed={dressed && owned}
   class:card--overlaid={overlaid && owned}
   class:card--back-art={!owned && hasBackArt}
   class:card--torn={!!torn}
   style:clip-path={torn}
 >
  <div class="content" bind:this={contentEl}>
  {#if owned}
    <!-- Опись. Один `{#each}` на полосу: что стоит в шапке, что в свойствах,
         в каком порядке и с какой величины видно — это строки рамы, а не
         разметка. Обёртка каждой строки — `display: contents`, то есть своей
         коробки у неё нет: `.numbers` остаётся тем же элементом флекса полосы,
         каким был, и ни одно правило ниже не переписано. Прячется строка
         «только крупно» тем, что гаснет обёртка вместе со всем, что внутри. -->
    {#snippet say(row: SheetRow)}
      {#if row.slot === 'raceIcon'}
        {#if editable || raceIconEditable}
          <button
            type="button"
            class="race-icon race-icon--live"
            onclick={handleIconClick}
            aria-label={raceIconEditable ? $t('adminBattlesRaceIconUpload') : $t('adminBattlesRaceJump')}
          >
            {#if card.raceIconUrl}
              <img src={card.raceIconUrl} alt="" class="race-icon-img" />
            {/if}
          </button>
        {:else}
          <span class="race-icon">
            <img src={card.raceIconUrl} alt="" class="race-icon-img" />
          </span>
        {/if}
      {:else if row.slot === 'race'}
        <span class="race">{head.race}</span>
      {:else if row.slot === 'kind'}
        <span class="kind">{kindWord}</span>
      {:else if row.slot === 'channel'}
        <span class="kind">{channelWord}</span>
      {:else if row.slot === 'pips'}
        <!-- Насечки, не число: на полке цифра пропадает, а ряд меток нет. Это
             уровень ВАШЕЙ копии — никогда не чин карты, который надет рамой. -->
        <span class="pips" aria-label="{$t('battlesLevelLabel')}: {level}">
          {#each [1, 2, 3, 4, 5] as step (step)}
            <span class="pip" class:pip--lit={step <= (level ?? 0)}></span>
          {/each}
        </span>
      {:else if row.slot === 'title'}
        <h3 class="title">{copy.title}</h3>
      {:else if row.slot === 'rank'}
        <p class="rank">{rank}</p>
      {:else if row.slot === 'traits'}
        <ul class="traits">
          {#each traits as trait, i (i)}
            <li class="trait">
              <span class="trait-name">
                {trait.name}{#if trait.other}<span class="trait-other">({trait.other})</span>{/if}{#if printTraitRule && trait.text}:{/if}
              </span>
              {#if printTraitRule && trait.text}<span class="trait-text"> {trait.text}</span>{/if}
            </li>
          {/each}
        </ul>
      {:else if row.slot === 'effect'}
        <p class="effect" class:effect--voice={!printEffectAsRule}>{copy.effect}</p>
      {:else if row.slot === 'lore'}
        <p class="lore">{copy.lore}</p>
      {:else if row.slot === 'stats'}
        <span class="stats">
          {$t('battlesCostLabel')} {card.cost} · {$t('battlesPowerLabel')} {card.power}
        </span>
      {:else if row.slot === 'new'}
        <span class="new-mark" class:new-mark--over={row.band === 'over'}>{$t('battlesNew')}</span>
      {/if}
    {/snippet}

    {#snippet stripe(cells: Cell[])}
      {#each cells as cell (cell.row.slot)}
        {#if cell.stats}
          <!-- Паспорт. Обёртка коробки не берётся в руку и своего `data-row`
               не имеет: тянут ОТДЕЛЬНОЕ число, а не весь паспорт разом. -->
          <span class="row" class:row--large={cell.row.show === 'large'} class:row--shelf={cell.row.show === 'always'}
          class:row--only-cell={cell.row.show === 'cellOnly'}>
            <p class="numbers">
              {#each cell.stats as stat (stat.slot)}
                {@const value = passport.find((one) => one.field === stat.slot)?.value}
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <span
                  class="row"
                  class:row--large={stat.show === 'large'}
                  class:row--shelf={stat.show === 'always'}
                  class:row--only-cell={stat.show === 'cellOnly'}
                  class:row--live={rowsEditable}
                  class:row--held={rowHeld === stat.slot}
                  data-row={stat.slot}
                  onpointerdown={(e) => rowTake(stat.slot, e)}
                >
                  <span class="number"
                    >{$t(BODY_STAT_LABELS[stat.slot as keyof typeof BODY_STAT_LABELS])}
                    <b>{value}</b></span
                  >
                </span>
              {/each}
            </p>
          </span>
        {:else}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <span
            class="row"
            class:row--large={cell.row.show === 'large'}
            class:row--shelf={cell.row.show === 'always'}
          class:row--only-cell={cell.row.show === 'cellOnly'}
            class:row--live={rowsEditable}
            class:row--held={rowHeld === cell.row.slot}
            data-row={cell.row.slot}
            onpointerdown={(e) => rowTake(cell.row.slot, e)}
          >
            {#if cell.sep !== 'none'}<span
                class="head-sep"
                class:row--large={cell.sep === 'large'}
              >·</span>{/if}{@render say(cell.row)}
          </span>
        {/if}
      {/each}
    {/snippet}

    <!-- 1. The header: what this is, and what kind of thing it is. -->
    <header
      class="band band--head"
      class:band--drop={rowAim?.band === 'head' && rowAim.ok}
      class:band--deny={rowAim?.band === 'head' && !rowAim.ok}
    >
      {@render stripe(headCells)}

      {#if frameEditable && !frameEditTarget}
        <div
          class="share-handle share-handle--head"
          class:active={frameDragKind === 'headerShare'}
          onpointerdown={(e) => shareDragStart('headerShare', e)}
          onpointermove={frameDragMove}
          onpointerup={frameDragEnd}
          onpointercancel={frameDragEnd}
          role="slider"
          aria-orientation="horizontal"
          aria-label={$t('adminBattlesHeaderShare')}
          aria-valuenow={Math.round(frame.headerShare * 100)}
          tabindex="0"
        ></div>
      {/if}
    </header>

    <!-- 2. The work, seen through the window. Click replaces it; drag aims it. -->
    <div
      class="art band--art"
      class:art--editable={editable}
      class:art--dragging={dragging}
      style="--art-tx:{artTx}%;--art-ty:{artTy}%;--art-zoom:{focal.zoom}"
      onpointerdown={aimDown}
      onpointermove={aimMove}
      onpointerup={aimUp}
      onpointercancel={aimUp}
      onwheel={aimZoom}
      ondragstart={(e) => e.preventDefault()}
      role={editable ? 'button' : undefined}
      tabindex={editable ? 0 : undefined}
      aria-label={editable ? $t('adminBattlesAim') : undefined}
    >
      {#if card.artUrl}
        <AppImage src={card.artUrl} alt={copy.title} class="art-image" sizes="(max-width: 640px) 45vw, 260px" />
      {:else}
        <div class="art--absent" aria-hidden="true"></div>
      {/if}
      {#if struck}
        <i class="struck struck--{struck}" aria-hidden="true"></i>
      {/if}
      <span class="foil" aria-hidden="true"></span>

      {#if frameEditable && !frameEditTarget}
        <div
          class="share-handle share-handle--art"
          class:active={frameDragKind === 'artShare'}
          onpointerdown={(e) => shareDragStart('artShare', e)}
          onpointermove={frameDragMove}
          onpointerup={frameDragEnd}
          onpointercancel={frameDragEnd}
          role="slider"
          aria-orientation="horizontal"
          aria-label={$t('adminBattlesArtShare')}
          aria-valuenow={Math.round(frame.artShare * 100)}
          tabindex="0"
        ></div>
      {/if}
    </div>

    <!-- 3. The properties. The band with no fixed share: it takes whatever the
            other three leave, because it is the one holding prose. -->
    <div
      class="band band--props"
      class:band--drop={rowAim?.band === 'props' && rowAim.ok}
      class:band--deny={rowAim?.band === 'props' && !rowAim.ok}
      bind:this={propsEl}
    >
      {@render stripe(propCells)}
    </div>

    <!-- 4. The footer. -->
    <footer
      class="band band--foot"
      class:band--drop={rowAim?.band === 'foot' && rowAim.ok}
      class:band--deny={rowAim?.band === 'foot' && !rowAim.ok}
    >
      {#if frameEditable && !frameEditTarget}
        <div
          class="share-handle share-handle--foot"
          class:active={frameDragKind === 'footShare'}
          onpointerdown={(e) => shareDragStart('footShare', e)}
          onpointermove={frameDragMove}
          onpointerup={frameDragEnd}
          onpointercancel={frameDragEnd}
          role="slider"
          aria-orientation="horizontal"
          aria-label={$t('adminBattlesFootShare')}
          aria-valuenow={Math.round(frame.footShare * 100)}
          tabindex="0"
        ></div>
      {/if}
      {@render stripe(footCells)}
    </footer>

    <!-- Поверх карты: то, что не стоит ни в одной полосе. -->
    {@render stripe(overCells)}

  {:else}
    <!-- Face down. Not greyed out: a card you do not have is a card lying in
         dust with its price still legible, which is also the room's price list. -->
    <div class="back" aria-hidden="true"></div>
    <div class="back-copy">
      <p class="rank rank--down">{rank}</p>
      <h3 class="title title--down">{copy.title}</h3>
      <ul class="prices">
        {#each prices as price (price.coin)}
          <li class="price">
            <span class="price-amount">{price.amount}</span>
            <span class="price-coin">
              {price.coin === 'dust' ? $t('battlesCoinDust') : $t('battlesCoinFeed')}
            </span>
          </li>
        {/each}
      </ul>
    </div>
  {/if}

  {#if frameEditable}
    <div
      class="inset-handle inset-handle--top"
      class:active={frameDragKind === 'insetTop'}
      onpointerdown={(e) => insetDragStart('insetTop', e)}
      onpointermove={frameDragMove}
      onpointerup={frameDragEnd}
      onpointercancel={frameDragEnd}
      role="slider"
      aria-orientation="vertical"
      aria-label={$t('adminBattlesInsetTop')}
      aria-valuenow={Math.round(frame.insetTop)}
      tabindex="0"
    ></div>
    <div
      class="inset-handle inset-handle--right"
      class:active={frameDragKind === 'insetRight'}
      onpointerdown={(e) => insetDragStart('insetRight', e)}
      onpointermove={frameDragMove}
      onpointerup={frameDragEnd}
      onpointercancel={frameDragEnd}
      role="slider"
      aria-orientation="horizontal"
      aria-label={$t('adminBattlesInsetRight')}
      aria-valuenow={Math.round(frame.insetRight)}
      tabindex="0"
    ></div>
    <div
      class="inset-handle inset-handle--bottom"
      class:active={frameDragKind === 'insetBottom'}
      onpointerdown={(e) => insetDragStart('insetBottom', e)}
      onpointermove={frameDragMove}
      onpointerup={frameDragEnd}
      onpointercancel={frameDragEnd}
      role="slider"
      aria-orientation="vertical"
      aria-label={$t('adminBattlesInsetBottom')}
      aria-valuenow={Math.round(frame.insetBottom)}
      tabindex="0"
    ></div>
    <div
      class="inset-handle inset-handle--left"
      class:active={frameDragKind === 'insetLeft'}
      onpointerdown={(e) => insetDragStart('insetLeft', e)}
      onpointermove={frameDragMove}
      onpointerup={frameDragEnd}
      onpointercancel={frameDragEnd}
      role="slider"
      aria-orientation="horizontal"
      aria-label={$t('adminBattlesInsetLeft')}
      aria-valuenow={Math.round(frame.insetLeft)}
      tabindex="0"
    ></div>
  {/if}
  </div>

  {#if owned && frame.layout === 'corners'}
    <!-- Cost and power, above the carving and placed against the whole card
         (see `.badges-layer` below): a badge sitting inside `.content`'s own
         stacking context can never paint over a sibling layer no matter its
         local z-index, and inside `.content`'s BOX it could never be dragged
         out onto the frame at all. -->
    <div class="badges-layer">
      {#snippet badge(kind: BadgeKind)}
        {@const keys = BADGE_FIELDS[kind]}
        {@const label = $t(
          kind === 'cost'
            ? 'battlesCostLabel'
            : kind === 'power'
              ? 'battlesPowerLabel'
              : 'battlesHealthLabel',
        )}
        <!-- Здоровье — единственное число, которое в бою МЕНЯЕТСЯ, поэтому
             оно и приходит извне. Нет партии — нет и `alive`, и кружок
             говорит то, что напечатано на бумаге. -->
        {@const value =
          kind === 'cost' ? card.cost : kind === 'power' ? card.power : (alive ?? card.health)}
        {@const spot = badgeAt(frame, kind)}
        {@const shape = badgeShape(frame, kind)}
        {@const fill = badgeText(frame, kind, 'fill')}
        {@const plate = badgePlate(frame, kind)}
        {@const paint = badgeStyle(frame, kind)}
        <!-- Сургуч только у здоровья, и только у него он и может быть: это
             единственное число карты, которое в бою меняется. Стоимость и сила
             отпечатаны на бумаге раз и навсегда, трескаться им не с чего. -->
        {@const wear = kind === 'health' ? sealed : null}
        {@const word = kind === 'cost' ? costWord : kind === 'power' ? powerWord : false}
        {@const step = badgeRow[kind]?.show ?? 'always'}
        <!-- В партии порог ширины не решает: клетка крупнее полки всё равно
             фигура, стоимость на ней молчит, здоровье и сила — нет. -->
        {@const byWidth = !inMatch}
        <!-- Берутся в руку все три. Здоровье не бралось, пока у него не было
             своих полей: править его значило бы править стоимость под видом
             здоровья. Поля появились — исчезла и причина. -->
        {@const live = frameEditable && !frameEditTarget}
        {@const named = `${label} ${value}`}
        <span
          class="corner-mark"
          class:corner-mark--power={kind === 'power'}
          class:row--large={byWidth && step === 'large'}
          class:row--shelf={byWidth && step === 'always'}
          class:row--only-cell={byWidth && step === 'cellOnly'}
          style="left:{spot.x}%; top:{spot.y}%"
        >
          {#if live}
            <button
              type="button"
              class="corner corner--{kind} corner--shape-{shape} corner--editable"
              class:corner--unfilled={!!fill && badgeUnfilled(fill)}
              class:corner--plate={!!plate}
              style={paint}
              aria-label={named}
              onpointerdown={(e) => badgeDragStart(kind, e)}
              onpointermove={badgeDragMove}
              onpointerup={badgeDragEnd}
              onpointercancel={badgeDragEnd}
            >
              {#if wear}<span class="corner-wear" style={wear}></span>{/if}
              <span class="corner-num">{value}</span>
            </button>
          {:else}
            <span
              class="corner corner--{kind} corner--shape-{shape}"
              class:corner--unfilled={!!fill && badgeUnfilled(fill)}
              class:corner--plate={!!plate}
              style={paint}
              role="img"
              aria-label={named}
            >
              {#if wear}<span class="corner-wear" style={wear}></span>{/if}
              <span class="corner-num">{value}</span>
            </span>
          {/if}
          {#if word}<span class="corner-word" aria-hidden="true">{label}</span>{/if}
        </span>
      {/snippet}
      {#each BADGE_KINDS as kind (kind)}
        {#if badgeRow[kind] && badgeRow[kind].show !== 'never' && !(inMatch && kind === 'cost')}{@render badge(kind)}{/if}
      {/each}
    </div>
  {/if}

  {#if editable}
    <!-- The frame: pick which of the five ranks dresses this card, or wear a
         picture just for this one card instead. Floats outside `.content` so
         a cut-out frame's overflow:hidden window never clips it. -->
    <div class="frame-control">
      <button type="button" class="frame-btn" onclick={() => (framePopoverOpen = !framePopoverOpen)}>
        {$t('adminBattlesTier')} {card.tier}
      </button>
      {#if framePopoverOpen}
        <button
          type="button"
          class="frame-backdrop"
          aria-label={$t('adminBattlesFrameClose')}
          onclick={() => (framePopoverOpen = false)}
        ></button>
        <div class="frame-popover">
          <div class="frame-tier-row">
            {#each [1, 2, 3, 4, 5] as t (t)}
              <button
                type="button"
                class="frame-tier"
                class:active={card.tier === t}
                onclick={() => {
                  card.tier = t;
                  framePopoverOpen = false;
                }}
              >{t}</button>
            {/each}
          </div>
          <button type="button" class="frame-own" onclick={uploadCardFrame}>
            {$t('adminBattlesFrameOwnPicture')}
          </button>
          {#if card.frameOverride}
            <button
              type="button"
              class="frame-own"
              onclick={() => {
                card.frameOverride = null;
                framePopoverOpen = false;
              }}
            >{$t('adminBattlesFrameResetCard')}</button>
          {/if}
        </div>
      {/if}
    </div>
  {/if}

  {#if overlaid && owned}
    {#if sliced}
      <!-- The carving, built from a corner and two side pictures instead of
           one stretched whole. Same job as `.carving` below — a hole the card
           shows through, deaf to the pointer — but assembled from sixteen
           pieces, each of which takes its band from the frame's four insets
           only as a STARTING point and is then grown past it, slid along it
           and layered over its neighbour by that slot's own placement. Carving
           does not tile: a corner sits ON its edge, and that overlap is the
           whole reason these numbers exist.

           One container, not two, and that is load-bearing: `z-index` on a
           positioned element opens a stacking context, so two layers would put
           every accent above every base piece forever, and a corner could never
           be asked to sit under the edge it meets. Inside one context the six
           `layer` numbers decide the whole order. -->
      <div class="sliced-carving" class:sliced-carving--live={sliceEditable} aria-hidden="true">
        {#each copies as copy (copy.id + ':' + copy.side)}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <span
            class="carve"
            data-piece={copy.id}
            data-side={copy.side}
            style={copy.style}
            class:slice-live={sliceEditable}
            class:slice-mate={sliceHeld?.id === copy.id}
            class:slice-shown={sliceHeld?.id === copy.id && sliceHeld.side === copy.side}
            onpointerdown={(e) => sliceTake(copy.id, copy.side, 'move', e)}
            onpointermove={sliceDragMove}
            onpointerup={sliceDragEnd}
            onpointercancel={sliceDragEnd}
          ></span>
        {/each}
        {#if sliceEditable && sliceHeld && gripBox}
          <!-- Восемь мест той же коробки: четыре стороны и четыре угла.
               Соседи сборки, а не дети — см. `gripBox`. Квадратик во
               внутреннем углу остаётся тем, чем был: второе число копии,
               видимое без наведения. `@const` не для красоты: `sliceHeld`
               — изменяемая связка, и внутри замыкания сужение до
               непустого значения теряется. -->
          {@const held = sliceHeld}
          {@const box = gripBox}
          {@const axes = SLICE_SIDE_AXES[held.side]}
          {#each SLICE_RESIZE_HANDLES as handle (handle.id)}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <span
              class="slice-resize"
              class:slice-resize--corner={!!handle.x && !!handle.y}
              class:slice-resize--edge-x={!handle.y}
              class:slice-resize--edge-y={!handle.x}
              class:slice-resize--knob={handle.x === axes.gripX && handle.y === axes.gripY}
              style={resizeHandleStyle(box, handle)}
              title={$t('adminBattlesSliceResize')}
              onpointerdown={(e) => sliceTake(held.id, held.side, 'size', e, handle.x, handle.y)}
              onpointermove={sliceDragMove}
              onpointerup={sliceDragEnd}
              onpointercancel={sliceDragEnd}
            ></span>
          {/each}
        {/if}
      </div>
    {:else}
      <!-- The carving, laid over the card. A cut-out frame is a picture with a
           hole in it, not a border: its ornament runs past the rectangle and its
           inner edge is meant to overlap the photograph. Last in the stack, and
           deaf to the pointer so it never swallows anything underneath.
           Never worn face down — the frame is the FRONT's own dress; a card
           lying in dust shows its back, not the front's carving. -->
      <span class="carving" aria-hidden="true"></span>
    {/if}
  {/if}
 </div>

 <!-- Куда сядет строка. Черта меряется по соседям, а не считается вторым
      способом: у полосы свойств строки разной высоты, и вычисленная черта
      разошлась бы с настоящей на первой же длинной черте.

      Лежит В КАРТЕ, а не в окне, и числа у неё от карты: окно — коробка,
      сдвинутая врезками рамы и обрезающая всё, что вышло за неё, а место
      выбирают ровно по краям. -->
 {#if rowMark}
   <i
     class="row-mark"
     style="left:{rowMark.left}px; top:{rowMark.top}px; width:{rowMark.width}px; height:{rowMark.height}px"
     aria-hidden="true"
   ></i>
 {/if}

 {#if flake}
   <i
     class="scrap"
     style="--scrap-x:{flake.x};--scrap-y:{flake.y};--scrap-spin:{flake.spin};--scrap-size:{flake.size}cqi"
     aria-hidden="true"
   ></i>
 {/if}

 {#if badgePopoverOpen && badgePopoverPos}
   <!-- Стол значка. Прицеплен к экранному месту самого значка, а не к
        `frame.costX`/`powerX`: у значка есть подпись неизвестной ширины, и он
        висит на своём центре. Сосед `.card`, а не его ребёнок — посторонее
        правило `.card { overflow: hidden }` иначе обрезало бы стол. -->
   <button
     type="button"
     class="frame-backdrop"
     aria-label={$t('adminBattlesFrameClose')}
     onclick={() => (badgePopoverOpen = null)}
   ></button>
   <div
     class="badge-popover"
     bind:this={badgePopoverEl}
     style="left:{badgePopoverPos.left}%; top:{badgePopoverPos.top}%"
   >
     <BattleBadgeInspector
       kind={badgePopoverOpen}
       {frame}
       write={rankFrame}
       {onEditStart}
       onArtUpload={onBadgeArtUpload}
       onArtStore={onBadgeArtStore}
       onclose={() => (badgePopoverOpen = null)}
     />
   </div>
 {/if}
</article>

<style>
  /* The card reads its own width, so one component serves every size it is ever
     drawn at, and every measurement below is in cqi for the same reason.
     The container must be a SEPARATE element from the one that uses the units:
     an element cannot size itself with its own container units, so a padding in
     cqi on the container itself silently resolves against the page instead. */
  .slot {
    container-type: inline-size;
    position: relative;
    aspect-ratio: var(--aspect, 0.714);
    --mx: 0.5;
    --my: 0.5;
    overflow: visible;
  }

  .card {
    position: relative;
    height: 100%;
    background: var(--paper);
    color: var(--ink);
    border: 1px solid var(--edge);
    border-radius: 0;
    box-shadow:
      inset 0 0 0 2cqi var(--paper),
      inset 0 0 0 calc(2cqi + 1px) var(--edge),
      0 2px 14px rgba(52, 37, 28, 0.14);
    font-family: Georgia, 'Fraunces', serif;
    transition: transform 420ms cubic-bezier(0.22, 1, 0.36, 1);
    transform-style: preserve-3d;
    will-change: transform;
  }

  .card--torn {
    filter: drop-shadow(0 0 0.6px #6f3b24);
  }

  /* Any card wearing a picture. The painted rings and the hairline border are
     the renderer's own frame — they must not be drawn on top of a carved one. */
  .card--dressed {
    border: none;
    box-shadow: 0 2px 14px rgba(52, 37, 28, 0.14);
  }

  /* Worn BEHIND: the picture is the card's ground. For a frame with no hole in
     it, where laying it on top would simply cover the card. */
  .card--dressed:not(.card--overlaid) {
    background-image: var(--frame-image);
    background-size: 100% 100%;
    background-repeat: no-repeat;
  }

  /* Worn ON TOP. The card is a plain rectangle of paper; the carving is a
     separate layer above everything, and the paper shows through its hole.
     Stretched rather than fitted: the keeper sets the card's ratio from the
     picture on upload, so the two already agree. */
  .carving {
    position: absolute;
    inset: 0;
    z-index: 3;
    background-image: var(--frame-image);
    background-size: 100% 100%;
    background-repeat: no-repeat;
    /* Chrome, not a surface: it must never take a click, a hover or a
       text selection away from the card underneath. */
    pointer-events: none;
  }

  /* The sliced carving's own layer — same box and same place in the stack as
     `.carving` above, built from as many pieces as the frame has. A stacking
     context of its own: the `layer` numbers inside it order the pieces against
     EACH OTHER and can never reach past the badges above. */
  .sliced-carving {
    position: absolute;
    inset: 0;
    z-index: 3;
    pointer-events: none;
  }

  /* One rule for every copy of every piece. Where each sits — its box, its
     picture, its layer, its turn — arrives as an inline style from
     `carvedCopies`, because the six named slots are five shapes between them
     and an ornament the keeper adds is one of the same five: sixteen CSS rules
     could not name a piece invented five seconds ago, and a second renderer
     for the added ones would be a preview that eventually lies.

     Every copy starts from its band — the same four insets that place
     `.content`'s window — and is then grown past it, slid along it and layered
     over its neighbour by ITS OWN four numbers. Its own, and not its piece's:
     the left side of a carving is rarely the mirror of its right, and one
     number for both would put that fit out of reach the same way one number
     for all four insets would. */
  .carve {
    position: absolute;
    background-repeat: no-repeat;
  }

  /* ── Резьба под рукой хранителя ──────────────────────────────────────────
     Всё ниже живёт только в предпросмотре: на полке резьба остаётся хромом и
     не берёт ни щелчка. Указатель отдан каждой копии по отдельности — берут
     ту, которую видят, и едет она одна или со своими, смотря по связке. */
  .sliced-carving--live .slice-live {
    pointer-events: auto;
    cursor: move;
  }

  .slice-sizing,
  .slice-sizing .slice-live,
  .slice-sizing .slice-resize {
    cursor: inherit;
  }

  /* Пунктир по коробке копии. Взятая обведена сплошнее своих: пока связка
     включена, поедут все, и это должно быть видно до того, как поедут. */
  .sliced-carving--live .carve::after {
    content: '';
    position: absolute;
    inset: 0;
    border: 1px dashed color-mix(in oklab, var(--ink) 60%, transparent);
    opacity: 0;
    pointer-events: none;
    transition: opacity 150ms ease;
  }

  .sliced-carving--live .slice-live:hover::after {
    opacity: 0.5;
  }

  .sliced-carving--live .slice-mate::after {
    opacity: 0.45;
  }

  .sliced-carving--live .slice-shown::after {
    opacity: 1;
    border-style: solid;
    border-color: color-mix(in oklab, var(--ink) 75%, transparent);
  }

  /* Восемь мест той же коробки. Соседи деталей, а не дети: `layer` открывает
     контекст наложения, и захват внутри угла со слоем 2 ушёл бы под акцент
     со слоем 5. Стороны невидимы до наведения — курсор и есть подсказка;
     внутренний угол остаётся квадратиком, каким был. */
  .slice-resize {
    position: absolute;
    z-index: 20;
    pointer-events: auto;
    touch-action: none;
    user-select: none;
  }

  .slice-resize--edge-x {
    width: 3.6cqi;
    margin-left: -1.8cqi;
  }

  .slice-resize--edge-y {
    height: 3.6cqi;
    margin-top: -1.8cqi;
  }

  .slice-resize--corner {
    z-index: 21;
    width: 3.4cqi;
    height: 3.4cqi;
    margin: -1.7cqi 0 0 -1.7cqi;
  }

  .slice-resize--corner:hover,
  .slice-resize--knob {
    background: var(--paper);
    border: 1px solid color-mix(in oklab, var(--ink) 70%, transparent);
  }

  .slice-resize--edge-x:hover::after,
  .slice-resize--edge-y:hover::after {
    content: '';
    position: absolute;
    background: color-mix(in oklab, var(--ink) 55%, transparent);
  }

  .slice-resize--edge-x:hover::after {
    top: 0;
    bottom: 0;
    left: 50%;
    width: 1px;
    margin-left: -0.5px;
  }

  .slice-resize--edge-y:hover::after {
    left: 0;
    right: 0;
    top: 50%;
    height: 1px;
    margin-top: -0.5px;
  }

  /* The WHOLE card, not the window — a layer of its own above the carving.
     A stacking context can only be beaten by a sibling with a higher
     z-index, never from inside it, which is why the badges cannot simply ask
     for a higher z-index while still living inside `.content`.

     The box is the card's because `costX`/`costY` are percentages of THE
     CARD: a badge belongs on the frame's corner as readily as inside the
     window, and in percentages of the window a place on the carving cannot
     be written down at all. It is also the reading `badgeReserve` has always
     used — it measures the badge against `insetLeft` and the header's own
     share — so a window-relative box here made the reserve wrong by the
     insets on every dressed frame.

     Deaf to the pointer itself so the empty parts of the layer never steal a
     click from the photograph or the header underneath; only the badges opt
     back in. */
  .badges-layer {
    position: absolute;
    inset: 0;
    z-index: 4;
    pointer-events: none;
  }

  .badges-layer .corner {
    pointer-events: auto;
  }

  /* The paper the card is written on, under everything. A cut-out frame has
     nothing behind it but this. */
  .card--dressed.card--overlaid {
    background-color: var(--paper);
    background-image: var(--paper-image);
    background-size: cover;
    background-position: center;
    /* A carving casts its own shadow in the picture; a second one under the
       rectangle would show as a hard edge outside the ornament. */
    box-shadow: none;
  }

  /* The opening in the frame. Absolutely positioned so the top and bottom
     insets measure against the card's HEIGHT — a percentage padding would
     measure all four sides against its width, and a tall card would wear a
     window in the wrong place. */
  .content {
    position: absolute;
    inset: var(--pad-top, 0) var(--pad-right, 0) var(--pad-bottom, 0) var(--pad-left, 0);
    display: flex;
    flex-direction: column;
    padding: 5cqi;
    /* The window is a fixed box. Whatever the keeper writes, nothing may spill
       out over the carving. */
    overflow: hidden;
  }

  /* The insets already stand the content off the carving; a second inset of
     the renderer's own would push it into the middle of the window. */
  .card--dressed .content {
    padding: 0;
  }

  /* Under the carving, never over it. */
  .content {
    z-index: 1;
  }

  /* Rank shows as heavier paper and a heavier edge, never as a brighter colour:
     nothing in this house glows. */
  .slot[data-tier='5'] .card {
    box-shadow:
      inset 0 0 0 2cqi var(--paper),
      inset 0 0 0 calc(2cqi + 1.5px) var(--edge),
      0 3px 22px rgba(52, 37, 28, 0.3);
  }

  .slot:hover .card:not(.card--still) {
    /* Small on purpose. A card that leaps is a card in a shop window. */
    transform: perspective(900px)
      rotateY(calc((var(--mx) - 0.5) * 7deg))
      rotateX(calc((0.5 - var(--my)) * 7deg));
  }

  .corner-mark {
    position: absolute;
    z-index: 2;
    display: flex;
    flex-direction: column;
    align-items: center;
    /* Centre of the disc, same as the old `.corner` left/top: a drag writes
       the badge's own centre, and the caption hangs off that, not off a
       second pair of numbers. */
    transform: translate(-50%, -50%);
    pointer-events: none;
  }

  /* Power sits on the sill: hanging the word below it would leave the card. */
  .corner-mark--power {
    flex-direction: column-reverse;
  }

  /* Заливка кружка — своя у каждого значка, и назначается она НЕ ЗДЕСЬ:
     `--badge-fill` приходит инлайновым стилем, только когда хранитель выбрал
     цвет. Умолчания остаются в откате `var()`, поэтому нетронутый значок
     печатается ровно тем, чем печатался: стоимость — чернилами карты, сила —
     цветом каймы (см. `.corner--power`). Цифру выбирает `badgeInk`. */
  .corner {
    position: relative;
    z-index: 2;
    display: grid;
    place-items: center;
    width: calc(10.5cqi * var(--badge-size, 1));
    height: calc(10.5cqi * var(--badge-size, 1));
    margin: 0;
    padding: 0;
    font: inherit;
    line-height: 1;
    color: var(--badge-ink, var(--paper));
    /* Заливка остаётся ЗАЛИВКОЙ — `background-color`, а не сокращение: свет
       ложится вторым слоем поверх неё, и выбранный хранителем цвет никуда не
       девается. Сокращение `background` стёрло бы этот слой у любого, кто
       переназначит цвет (см. `.corner--power`). */
    background-color: var(--badge-fill, var(--ink));
    /* Значок — вещь, а не заливка. Свет падает сверху, той же лампой, при
       которой снята фотография рамы: блик по верхней кромке, тень по нижней,
       волосяная кайма цветом рамы. Ничего не светится — плашка просто
       перестаёт быть плоской наклейкой поверх резьбы. */
    background-image: linear-gradient(
      to bottom,
      color-mix(in oklab, #fff 20%, transparent),
      transparent 45%,
      color-mix(in oklab, #000 14%, transparent)
    );
    box-shadow:
      inset 0 0.4cqi 0.6cqi color-mix(in oklab, #fff 26%, transparent),
      inset 0 -0.45cqi 0.7cqi color-mix(in oklab, #000 22%, transparent),
      inset 0 0 0 0.28cqi color-mix(in oklab, var(--edge) 60%, transparent);
    /* Тень СНАРУЖИ — фильтром, а не четвёртой строкой `box-shadow`: у ромба,
       шестиугольника и щита форма вырезана `clip-path`, а он обрезает и
       внешнюю тень тоже. Фильтр читает готовый силуэт и потому верен всем
       пяти формам разом, а не одним только круглой и квадратной. */
    filter: drop-shadow(0 0.28cqi 0.4cqi color-mix(in oklab, var(--ink) 40%, transparent));
    border: none;
    cursor: default;
    pointer-events: auto;
  }

  /* Цифра растёт вместе с кружком: величина значка — это величина ЗНАЧКА, а
     подложка без цифры или цифра без подложки были бы двумя ручками там, где
     хранитель тянет одну. Толщина своя: у неё ступени начертаний, а не ход. */
  .corner-num {
    /* Кегль почти прежний (7 → 6.6), кружок вокруг него — заметно меньше: то,
       что убрали, было мёртвым полем, а не цифрой. Зеркало `BADGE_BARE`. */
    font-size: calc(6.6cqi * var(--type-scale, 1) * var(--badge-size, 1));
    /* Дом печатает цифру полужирной: на монете в 10.5cqi светлое начертание
       читается хуже, чем читалось на плашке в тринадцать. Хранитель, назначив
       толщину, по-прежнему побеждает. */
    font-weight: var(--badge-weight, 600);
    line-height: 1;
    /* Цифра лежит НА металле, а не рядом с ним: волосяная тень под ней — то же
       самое освещение, что и у кружка, и без неё число висит отдельно от
       вещи, на которой оно вычеканено. */
    text-shadow: 0 0.1cqi 0.15cqi color-mix(in oklab, #000 26%, transparent);
  }

  .corner-word {
    margin-top: 0.7cqi;
    font-size: calc(3.2cqi * var(--type-scale, 1));
    letter-spacing: 0.08em;
    text-transform: uppercase;
    line-height: 1;
    color: var(--ink);
    white-space: nowrap;
    opacity: min(1, calc(0.78 * var(--ink-fade, 1)));
  }

  .corner-mark--power .corner-word {
    margin-top: 0;
    margin-bottom: 0.7cqi;
  }

  /* On the taking sheet the passport names cost and power. Hanging the word
     off the disc at 400px puts СИЛА in the numbers and СТОИМОСТЬ in the art. */
  @container (min-width: 281px) {
    .corner-word {
      display: none;
    }
  }

  /* The badge's own outline — a coin is only the default, not the only shape
     a cost or a power has ever worn. Picked per badge in its own popover. */
  .corner--shape-circle {
    border-radius: 50%;
  }

  .corner--shape-square {
    border-radius: 12%;
  }

  .corner--shape-diamond {
    border-radius: 0;
    clip-path: polygon(50% 0, 100% 50%, 50% 100%, 0 50%);
  }

  .corner--shape-hex {
    border-radius: 0;
    clip-path: polygon(25% 0%, 75% 0%, 100% 50%, 75% 100%, 25% 100%, 0% 50%);
  }

  .corner--shape-shield {
    border-radius: 0;
    clip-path: polygon(50% 0%, 100% 18%, 100% 55%, 50% 100%, 0% 55%, 0% 18%);
  }

  /* Не шестая форма, а её отсутствие: подложка не печатается, на карте одна
     цифра. Заливку это НЕ стирает — сняли форму, вернули, и цвет на месте, —
     поэтому побеждает объявление, а не пустое поле. Коробка остаётся: она
     ручка, по которой значок берут, и то, из чего считается отступ шапки. */
  .corner--shape-none,
  .corner--power.corner--shape-none {
    background: none;
    border-radius: 0;
    clip-path: none;
    /* Нет формы — нет и КОРОБКИ. Кружок вокруг цифры кегля 6.6cqi это поле
       пустоты почти в самую цифру шириной, и пока коробка стояла, она мешала
       дважды: держала цифру в своём центре и не подпускала её к краю карты на
       свою половину. Значок становится ровно тем, что нарисовано. */
    width: auto;
    height: auto;
  }

  /* Сургуч у здоровья: трещины и выщербы, посчитанные `sealWear` по тому,
     сколько здоровья осталось. Картинка приходит инлайновым стилем — одна
     строка на весь слой, как у резьбы и у движений.

     `multiply`, а не сплошная краска: трещина обязана темнить то, что под ней,
     — крашеный воск, фотографию жетона, чужой рисунок, — а не класть поверх
     всего этого одинаковую чёрную черту. Слой лежит ВНУТРИ значка, поэтому
     `clip-path` формы обрезает его сам: сургуч не вылезает за край печати,
     какой бы формы она ни была, и об этом не надо помнить отдельно. */
  .corner-wear {
    position: absolute;
    inset: 0;
    background-repeat: no-repeat;
    background-position: center;
    background-size: 100% 100%;
    mix-blend-mode: multiply;
    pointer-events: none;
  }

  /* Материал — это материал ПОДЛОЖКИ. Нет её (снята форма) или она нарочно
     прозрачна — светить и отбрасывать тень нечему: блик, кайма и внешняя тень
     обвели бы кружок, которого хранитель как раз и не заказывал. Цифра при
     этом остаётся со своей волосяной тенью: она лежит на бумаге, на резьбе или
     на фотографии, и без неё теряется на любой из трёх. */
  .corner--shape-none:not(.corner--plate),
  .corner--unfilled:not(.corner--plate) {
    background-image: none;
    box-shadow: none;
    filter: none;
  }

  .corner--power {
    /* `background-color`, а не сокращение: сокращение стёрло бы слой света,
       который `.corner` кладёт вторым фоном, и сила осталась бы единственным
       плоским значком из трёх. */
    background-color: var(--badge-fill, var(--edge));
    color: var(--badge-ink, var(--ink));
  }

  /* Жетон — картинка со склада, надетая вместо крашеной подложки. Он и есть
     нарисованная подложка, поэтому краска, форма и весь домашний материал под
     ним молчат: блик и кайма, положенные поверх чужого рисунка, обвели бы его
     кружком, которого на рисунке нет. А внешняя тень остаётся — и остаётся
     ФИЛЬТРОМ: `drop-shadow` читает альфу картинки, то есть настоящий силуэт
     жетона, и медальон с фигурным краем отбрасывает тень своей формы, а не
     формы своей коробки. Ради одного этого фильтр тут и стоял.

     Ничего не выбрано хранителем — ничего и не назначено: `--badge-plate`
     приходит инлайновым стилем только когда жетон надет, и `.corner--plate`
     ставится по тому же условию, так что нетронутый значок этих правил не
     видит вовсе. */
  .corner--plate {
    /* Коробка возвращается: форма могла быть снята, а под жетоном она есть —
       он и есть нарисованная подложка. */
    width: calc(10.5cqi * var(--badge-size, 1));
    height: calc(10.5cqi * var(--badge-size, 1));
    background-color: transparent;
    background-image: var(--badge-plate);
    background-repeat: no-repeat;
    background-position: center;
    /* `contain`, а не `cover`: жетон рисуют целиком, и обрезанный по коробке
       медальон — это медальон без края. */
    background-size: contain;
    box-shadow: none;
    /* Форму задаёт сам рисунок. Оставленный `clip-path` ромба или щита срезал
       бы у чужой картинки углы по чужой мерке. */
    clip-path: none;
    border-radius: 0;
  }


  /* У здоровья нет ни своего правила, ни своих полей: `BADGE_FIELDS.health`
     указывает на поля стоимости, и кружок приходит сюда её формой, её
     заливкой, её величиной и на её месте. Разница ровно одна — число. */

  /* Only in the Frames tab: the badge itself becomes a handle, dragged to
     reposition and clicked (without moving) to open its own X/Y editor. */
  .corner--editable {
    cursor: grab;
    touch-action: none;
  }

  /* Со снятой заливкой хватать нечего: цифра мельче своей коробки, и хранитель
     тянул бы за воздух. Волосяная обводка — ТОЛЬКО на столе (`--editable`), на
     полке значка без заливки не видно, и в этом весь смысл. */
  .corner--editable.corner--unfilled,
  .corner--editable.corner--shape-none {
    outline: 1px dashed color-mix(in oklab, var(--ink) 35%, transparent);
    outline-offset: -1px;
  }

  .corner--editable:active {
    cursor: grabbing;
  }

  .corner:disabled {
    opacity: 1;
  }

  /* Обёртка строки описи. Своей коробки у неё нет — `display: contents`, —
     поэтому `.numbers` остаётся тем же элементом флекса своей полосы, каким
     был, и ни одно правило ниже переписывать не пришлось. */
  .row {
    display: contents;
  }

  /* Две ступени, два порога, и оба — те самые числа, по которым карта уже
     делит себя сама. Класс говорит, где строку ВИДНО ЕЩЁ, а не где её прячут:
     «с листа взятия» гаснет на полке, «с полки» — в клетке боя, а строка без
     класса («везде») не гаснет нигде. `never` до разметки не доходит вовсе.

     Написано модификаторами, а не через `.row:not(...)`, потому что эти же два
     класса носит кружок значка, у которого своя коробка и своё место, и
     `display: contents` от `.row` отняло бы у него и то и другое.

     Гаснет вместе с обёрткой и точка-разделитель внутри неё: строка, которую
     хранитель оставил «только крупно», не должна уносить с полки слово, но
     оставлять после себя точку. */
  @container (max-width: 280px) {
    .row--large {
      display: none;
    }
  }

  @container (max-width: 160px) {
    .row--shelf {
      display: none;
    }
  }

  /* Единственная ступень с потолком, а не с порогом: «только в клетке боя».
     Заведена не ради полноты лестницы, а ради одного случая, который иначе
     невыразим, — кружок здоровья появляется ровно там, где исчезает кружок
     стоимости, и стоят они в одном углу. */
  @container (min-width: 161px) {
    .row--only-cell {
      display: none;
    }
  }

  /* Партия на широкой клетке. Порог 160 px — это полка, а не «бой»; клетка
     этюда крупнее полки, иначе фотография нечитаема. Опись, читая ширину,
     напечатала бы документ полки (стоимость, черты, действие) и спрятала бы
     здоровье. Класс ставит сцена через `alive`: плотность как у клетки,
     отрисовщик тот же. */
  .slot--match .row--large,
  .slot--match .row--shelf {
    display: none;
  }

  .slot--match .row--only-cell {
    display: contents;
  }

  /* Строку берут там, где она напечатана. Обёртка своей коробки не имеет, но
     курсор и подсветку наследует то, что внутри неё, а нажатие всплывает
     сквозь неё — этого хватает, чтобы взять строку целиком. */
  .row--live > :global(*) {
    cursor: grab;
    touch-action: none;
  }

  /* `.card` в селекторе не для красоты: у `.numbers` и `.rank` своя
     непрозрачность с тем же весом, и без предка взятая строка бледнела бы
     через раз — смотря какое правило написано ниже. */
  .card .row--held > :global(*) {
    opacity: 0.4;
    cursor: grabbing;
  }

  /* Полоса, над которой рука. Пунктир изнутри, а не рамка снаружи: полосы
     стоят вплотную, и внешняя рамка сдвинула бы соседнюю на пиксель. */
  .band--drop {
    box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--ink) 45%, transparent);
  }

  /* Сюда этой строке нельзя. Не «ничего не происходит», а сказанный отказ:
     проза в шапке в девять процентов карты — это обрезанная проза. */
  .band--deny {
    box-shadow: inset 0 0 0 1px rgba(143, 47, 34, 0.55);
    cursor: not-allowed;
  }

  /* Куда сядет. Считается по соседям и кладётся в координатах карты — как
     рукоять детали, и по той же причине: черта внутри полосы унаследовала бы
     её обрезку и пропала бы ровно на краю, где место и выбирают. */
  .row-mark {
    position: absolute;
    z-index: 6;
    background: #c65f3c;
    pointer-events: none;
  }

  /* Three bands are measured; the properties band is not, and takes the rest.
     Sliders that happen to add up can never squeeze it to nothing. */
  .band--head {
    flex: 0 0 var(--header-share, 9%);
    font-size: calc(3.8cqi * var(--type-scale, 1));
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--ink);
    opacity: min(1, calc(0.72 * var(--ink-fade, 1)));
  }

  .band--foot {
    flex: 0 0 var(--foot-share, 10%);
  }

  /* Уголки носят стоимость и силу вне окна, и подвал резервировал десятую
     часть карты ни подо что. Полоса остаётся на вкладке рамок, где шов — это
     ручка, и остаётся всюду, где опись всё же поставила в подвал строку:
     сплющивать полосу, в которой что-то стоит, значит прятать то, что
     хранитель только что туда положил. */
  .slot--flush-foot .band--foot {
    flex: 0 0 0;
    min-height: 0;
    overflow: hidden;
    pointer-events: none;
  }

  .band--head,
  .band--foot {
    position: relative;
    display: flex;
    align-items: center;
    gap: 2cqi;
    min-height: 0;
    overflow: hidden;
  }

  /* Шапка отступает от того, что на ней лежит, — от значков и от метки
     «новая». Числа приходят из `badgeReserve`, из того, ГДЕ ЗНАЧОК СТОИТ:
     до этого отступ был один и жёсткий (17cqi слева) и держался на том, что
     значок стоимости не двигали. Подпись под кружком в этот отступ не входила
     вовсе — отсюда «СТОИМОСТЬ ДОМОВЫЕ · ТЕ…» на каждой карте полки. */
  .band--head {
    padding-left: var(--head-pad-left, 0);
    padding-right: var(--head-pad-right, 0);
  }

  .band--props {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    min-height: 0;
    /* Content is clamped to fit (line-clamp, a capped trait list), so this
       scroll rarely triggers — kept as a backstop against the flex box
       quietly overlapping its neighbours if it ever doesn't. */
    overflow-y: auto;
    overflow-x: hidden;
  }

  .head-sep {
    opacity: min(1, calc(0.5 * var(--ink-fade, 1)));
  }

  /* The race icon: a small square before the header text, always the same
     slot whether it holds a picture, an empty frame waiting for one, or
     nothing at all on an ordinary read-only card. */
  .race-icon {
    flex: 0 0 auto;
    width: 6.5cqi;
    height: 6.5cqi;
    padding: 0;
    background: color-mix(in oklab, var(--ink) 6%, transparent);
    border: 1px solid color-mix(in oklab, var(--ink) 30%, transparent);
    border-radius: 20%;
    overflow: hidden;
    cursor: default;
  }

  .race-icon--live {
    cursor: pointer;
  }

  .race-icon-img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .traits {
    /* One line of its own, never a zero-height flex leftover. The numbers sit
       below with `margin-top: auto`; shrinking this band to nothing was how
       Creature and Granny lost their traits on the shelf. */
    flex: 0 0 auto;
    margin: 2.5cqi 0 0;
    padding: 0;
    list-style: none;
    min-height: 0;
    overflow: hidden;
    font-size: calc(4.2cqi * var(--type-scale, 1));
    line-height: 1.34;
    color: var(--ink);
  }

  .trait + .trait {
    margin-top: 1.4cqi;
  }

  .trait-name {
    font-weight: 600;
  }

  /* The other language, kept alongside rather than hidden: the keeper writes
     both names on the card and reads them together. Spaced in CSS because
     Svelte trims a leading space inside an element. */
  .trait-other {
    margin-left: 0.35ch;
    font-weight: 400;
    opacity: min(1, calc(0.6 * var(--ink-fade, 1)));
  }

  .numbers {
    display: flex;
    flex: 0 0 auto;
    flex-wrap: wrap;
    gap: 1.2cqi 3cqi;
    margin: auto 0 0;
    padding-top: 2.5cqi;
    font-size: calc(4cqi * var(--type-scale, 1));
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--ink);
    opacity: min(1, calc(0.88 * var(--ink-fade, 1)));
  }

  /* Текст уступает значку силы так же, как шапка — значку стоимости, и по
     тому же числу: без этого последнее число (Шаг, Лечение) стоит под кружком
     и обрезается, а действие пробегает сквозь него. Значок, поднятый в шапку,
     тексту уже не мешает, и `badgeReserve` отдаёт здесь ноль. */
  .numbers,
  .effect,
  .lore,
  .traits {
    padding-right: var(--body-pad-right, 0);
    padding-left: var(--body-pad-left, 0);
  }

  .number b {
    font-weight: 600;
    letter-spacing: 0;
  }

  .art {
    position: relative;
    flex: 0 0 var(--art-share, 44%);
    overflow: hidden;
    background: color-mix(in oklab, var(--ink) 8%, var(--paper));
  }

  /* A picture to grab and slide, not just a button to press — the cursor
     says so before the keeper even touches it. */
  .art--editable {
    cursor: grab;
    touch-action: none;
  }

  .art--dragging {
    cursor: grabbing;
  }

  .art :global(.art-image) {
    width: 100%;
    height: 100%;
  }

  .art :global(.app-image-main) {
    width: 100%;
    height: 100%;
    object-fit: cover;
    /* Fixed and centred on purpose — see the note by `artTx`/`artTy` above.
       Pan and zoom both live in the transform below instead. */
    object-position: 50% 50%;
    transform: translate(var(--art-tx, 0%), var(--art-ty, 0%)) scale(var(--art-zoom, 1));
  }

  /* An <img> is natively draggable — without this, the first move of a drag
     hands the gesture to the browser's own "drag this picture out" behaviour
     (the small ghost thumbnail with its caption) instead of the pointer
     handlers above. Belt and suspenders alongside `ondragstart` above: this
     also stops a stray drag from selecting the image as text. */
  .art--editable :global(.art-image),
  .art--editable :global(.app-image-main) {
    -webkit-user-drag: none;
    user-select: none;
    -webkit-user-select: none;
  }

  .art--absent {
    width: 100%;
    height: 100%;
    background: repeating-linear-gradient(
      45deg,
      color-mix(in oklab, var(--ink) 6%, var(--paper)) 0 6px,
      var(--paper) 6px 12px
    );
  }

  /* One slow sweep, following the pointer. Blank at rank 1 and 2, where --foil
     is transparent — a humble card has no foil at all. */
  .foil {
    position: absolute;
    inset: 0;
    pointer-events: none;
    background: radial-gradient(
      circle at calc(var(--mx) * 100%) calc(var(--my) * 100%),
      var(--foil) 0%,
      transparent 55%
    );
    mix-blend-mode: soft-light;
    opacity: 0;
    transition: opacity 500ms ease;
  }

  .slot:hover .card:not(.card--still) .foil {
    opacity: 1;
  }

  /* Синяк на фотографии: сургуч, не неон. От сердца миниатюры наружу, такт,
     потом нет. Чернила — чара, тот же жест, другой цвет. */
  .struck {
    position: absolute;
    inset: 0;
    pointer-events: none;
    transform-origin: 50% 42%;
    animation: struck-spread 420ms ease both;
  }

  .struck--bruise {
    background: radial-gradient(
      circle at 50% 42%,
      rgba(139, 48, 36, 0.48) 0%,
      rgba(139, 48, 36, 0.22) 32%,
      transparent 68%
    );
    mix-blend-mode: multiply;
  }

  .struck--ink {
    background: radial-gradient(
      circle at 48% 40%,
      rgba(52, 37, 28, 0.4) 0%,
      rgba(52, 37, 28, 0.16) 38%,
      transparent 70%
    );
    mix-blend-mode: multiply;
  }

  @keyframes struck-spread {
    0% { opacity: 0; transform: scale(0.28); }
    38% { opacity: 1; transform: scale(0.82); }
    100% { opacity: 0; transform: scale(1.18); }
  }

  /* Обломок — ребёнок слота, не карты: у `.card` в app.css overflow:hidden,
     и кусок бумаги не вылетел бы. Размер в cqi, от самой карты. */
  .scrap {
    position: absolute;
    left: 50%;
    top: 42%;
    z-index: 8;
    width: var(--scrap-size, 24cqi);
    height: calc(var(--scrap-size, 24cqi) * 0.78);
    margin-left: calc(var(--scrap-size, 24cqi) / -2);
    pointer-events: none;
    background: #f8f1e7;
    border: 1px solid #6f3b24;
    box-shadow: 1px 1px 0 #34251c;
    clip-path: polygon(12% 6%, 90% 0%, 100% 70%, 72% 100%, 0% 86%, 14% 40%);
    animation: scrap-fly 560ms cubic-bezier(0.2, 0.8, 0.25, 1) both;
  }

  @keyframes scrap-fly {
    0% {
      transform: translate(0, 0) rotate(-8deg);
      opacity: 1;
    }
    100% {
      transform: translate(var(--scrap-x, 40cqi), var(--scrap-y, -28cqi)) rotate(var(--scrap-spin, 70deg));
      opacity: 0;
    }
  }

  .title,
  .rank {
    /* Имя и чин были одной плашкой, пока стояли рядом всегда. Опись их
       развела: каждый теперь сам себе строка и сам не сжимается. */
    flex: 0 0 auto;
  }

  .title {
    margin: 0;
    font-family: var(--title-face, inherit);
    color: var(--title-ink, var(--ink));
    font-size: calc(7cqi * var(--type-scale, 1));
    line-height: 1.15;
    font-weight: 400;
    letter-spacing: 0.01em;
  }

  .rank {
    margin: 1cqi 0 0;
    color: var(--ink);
    font-size: calc(4cqi * var(--type-scale, 1));
    letter-spacing: 0.18em;
    text-transform: uppercase;
    opacity: min(1, calc(0.62 * var(--ink-fade, 1)));
  }

  .effect {
    /* Ends on a whole line instead of being sliced mid-letter by the card edge. */
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 6;
    line-clamp: 6;
    flex: 0 0 auto;
    overflow: hidden;
    margin: 3cqi 0 0;
    color: var(--ink);
    padding-top: 2.5cqi;
    border-top: 1px solid color-mix(in oklab, var(--edge) 70%, transparent);
    font-size: calc(5cqi * var(--type-scale, 1));
    line-height: 1.35;
  }

  /* No ability behind the line: this is the house's voice, not a rule.
     Granny's hen counts turns in the lore register until a verb exists. */
  .effect--voice {
    padding-top: 0;
    border-top: none;
    font-size: calc(4.4cqi * var(--type-scale, 1));
    line-height: 1.4;
    font-style: italic;
    opacity: min(1, calc(0.78 * var(--ink-fade, 1)));
  }

  /* On a shelf the card is a thumbnail: name, a line of traits, the effect,
     the body passport. The note underneath is for the card seen large.
     280px, not 240: the shelf slot is ~261px, and 240 left the lore fighting
     the traits for the last 81px of the window. The taking sheet is 400px,
     so lore and the full trait list still live there. */
  @container (max-width: 280px) {
    /* Байка отсюда ушла: «печатать ли её на полке» — это ступень описи, а не
       медиазапрос, и держать один и тот же выбор в двух местах значит однажды
       дать им разойтись. Домашняя ступень байки — «с листа взятия», то есть
       ровно то, что делало это правило. Здесь осталась только ПЛОТНОСТЬ:
       сколько черт влезает и как обрезается строка. */

    .trait-other {
      display: none;
    }

    .traits {
      max-height: calc(4.2cqi * var(--type-scale, 1) * 1.34);
    }

    .trait:not(:first-child) {
      display: none;
    }

    .trait {
      display: -webkit-box;
      -webkit-box-orient: vertical;
      -webkit-line-clamp: 1;
      line-clamp: 1;
      overflow: hidden;
    }

    .effect {
      -webkit-line-clamp: 3;
      line-clamp: 3;
    }
  }

  /* Smaller still: the card standing on a board cell. At this width it is a
     figure, not a document — frame, photograph, name, health, power.
     Cost belongs to the shelf; health belongs to the body that stands here.
     The same density is applied by `.slot--match` when the cell is wider than
     160 px: otherwise a readable board card would print the shelf document.

     Here rather than behind a `compact` prop for the same reason the preview
     lives in this component: a board that draws its own smaller card is a
     board that will drift from the shelf, and then one of the two is lying. */
  @container (max-width: 160px) {
    /* Что здесь ПЕЧАТАЕТСЯ, решает опись, а не этот медиазапрос. Раньше он
       гасил шапку, подвал, черты, действие, паспорт и оба кружка списком — и
       ступень «всегда» не значила «всегда»: хранитель ставил её и не понимал,
       почему в клетке боя ничего нет. Здесь осталось только про ВЕЛИЧИНУ. */

    /* The photograph takes everything the name leaves: the three shares are a
       frame's way of dividing four bands, and at this size there are two. */
    .art {
      flex: 1 1 auto;
    }

    /* Число, оставленное в клетке, должно читаться. Домашние 4cqi — это пять
       с половиной точек на карте в 140 px: не мелкий шрифт, а грязь. */
    .numbers {
      gap: 0.8cqi 2cqi;
      padding-top: 1.6cqi;
      font-size: calc(7cqi * var(--type-scale, 1));
    }

    .band--props {
      flex: 0 0 auto;
      padding-top: 2cqi;
      overflow: visible;
    }

    .title {
      display: -webkit-box;
      -webkit-box-orient: vertical;
      -webkit-line-clamp: 2;
      line-clamp: 2;
      overflow: hidden;
      font-size: calc(9cqi * var(--type-scale, 1));
      line-height: 1.1;
    }

    /* Шапка в клетке боя высотой в девять процентов карты — это полоска в
       восемнадцать точек, и уступать в ней значку нечего: он сам с неё ростом.
       Кто её здесь показывает, тот показывает и кружок поверх неё. */
    .band--head {
      padding-left: 0;
      padding-right: 0;
    }
  }

  .slot--match .content {
    padding: 0;
  }

  .slot--match .band--head,
  .slot--match .band--foot {
    flex: 0 0 0;
    min-height: 0;
    padding: 0;
    overflow: hidden;
    pointer-events: none;
  }

  .slot--match .art {
    flex: 1 1 auto;
    min-height: 0;
  }

  .slot--match .numbers {
    gap: 0.8cqi 2cqi;
    padding-top: 1.6cqi;
    font-size: calc(7cqi * var(--type-scale, 1));
  }

  .slot--match .band--props {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 2;
    flex: 0 0 auto;
    padding: 1.6cqi 2.4cqi 2cqi;
    overflow: visible;
    background: color-mix(in oklab, var(--paper, #f8f1e7) 78%, transparent);
  }

  .slot--match .title {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
    font-size: calc(8.5cqi * var(--type-scale, 1));
    line-height: 1.1;
  }

  .slot--match .band--head {
    padding-left: 0;
    padding-right: 0;
  }

  .lore {
    /* First to give way when the band is short: a note is the least of what a
       card has to say. */
    flex: 1 1 auto;
    min-height: 0;
    overflow: hidden;
    margin: 2cqi 0 0;
    color: var(--ink);
    font-size: calc(4.4cqi * var(--type-scale, 1));
    line-height: 1.4;
    font-style: italic;
    opacity: min(1, calc(0.66 * var(--ink-fade, 1)));
  }

  .pips {
    display: flex;
    gap: 1.4cqi;
  }

  /* Pushed to the far side of the header, opposite the race and type — the
     plaque layout centres its header as one group instead, so an auto margin
     there would fight the centring rather than sit at its edge. */
  .slot[data-layout='corners'] .band--head .pips {
    margin-left: auto;
  }

  .pip {
    width: 5cqi;
    height: 1.4cqi;
    background: color-mix(in oklab, var(--ink) 18%, transparent);
  }

  .pip--lit {
    background: var(--ink);
  }

  .slot[data-layout='plaque'] .band--head,
  .slot[data-layout='plaque'] .numbers {
    justify-content: center;
  }

  .slot[data-layout='plaque'] .title,
  .slot[data-layout='plaque'] .rank,
  .slot[data-layout='plaque'] .effect,
  .slot[data-layout='plaque'] .lore,
  .slot[data-layout='plaque'] .traits {
    text-align: center;
  }

  .slot[data-layout='plaque'] .effect {
    border-top: none;
  }

  .slot[data-layout='plaque'] .band--foot {
    justify-content: center;
  }

  .stats {
    font-size: calc(4.4cqi * var(--type-scale, 1));
    letter-spacing: 0.06em;
    color: var(--ink);
  }

  .new-mark {
    /* Поставленная в полосу, метка стоит в потоке и места ни у кого не
       отнимает; лежащая поверх карты — в правом верхнем углу, и шапка ей
       уступает ровно на её ширину (`--head-pad-right`). */
    align-self: center;
    padding: 1cqi 2.4cqi;
    font-size: calc(3.6cqi * var(--type-scale, 1));
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: var(--paper);
    background: #c65f3c;
  }

  .new-mark--over {
    position: absolute;
    top: 3cqi;
    right: 3cqi;
  }

  /* The dusty back. */
  .card--down {
    background: color-mix(in oklab, var(--ink) 10%, var(--paper));
  }

  /* The keeper's own picture for the reverse, in place of the plain dusty
     tint above. The dust texture and the rank/price text stay on top of it —
     see `.back` / `.back-copy` below — the same way they read over the tint. */
  .card--back-art {
    background-image: var(--back-image);
    background-size: cover;
    background-position: center;
  }

  .back {
    position: absolute;
    inset: 0;
    background:
      repeating-linear-gradient(
        135deg,
        color-mix(in oklab, var(--ink) 7%, transparent) 0 2px,
        transparent 2px 9px
      );
    opacity: 0.7;
  }

  .back-copy {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    height: 100%;
    text-align: center;
  }

  .title--down {
    font-size: calc(6cqi * var(--type-scale, 1));
    color: var(--ink);
    opacity: min(1, calc(0.78 * var(--ink-fade, 1)));
  }

  .rank--down {
    margin: 0 0 auto;
    padding-top: 6cqi;
  }

  .prices {
    display: flex;
    flex-direction: column;
    gap: 1.2cqi;
    margin: 4cqi 0 0;
    padding: 0;
    list-style: none;
  }

  .price {
    display: flex;
    align-items: baseline;
    justify-content: center;
    gap: 1.6cqi;
  }

  .price-amount {
    font-size: calc(6.4cqi * var(--type-scale, 1));
    color: var(--ink);
  }

  .price-coin {
    font-size: calc(3.6cqi * var(--type-scale, 1));
    letter-spacing: 0.14em;
    text-transform: uppercase;
    opacity: min(1, calc(0.66 * var(--ink-fade, 1)));
  }

  /* The rank's own shape, dragged instead of dialled — see `frameEditable` in
     the script. Each handle is a child of the very band it resizes rather
     than a floating overlay measured from outside: its parent already clips
     to exactly the right box, so a handle flush against that parent's own
     edge can never sit anywhere but the true seam, in any unit the frame
     happens to be sized in. Only the drag *math* needs a real measurement
     (`contentEl` / `root`, in the script) — the drag *position* needs none. */
  .share-handle,
  .inset-handle {
    position: absolute;
    z-index: 4;
    touch-action: none;
  }

  /* The dashed line is only for the moment of dragging — the frame's own
     picture is the card's border, so the guide stays invisible at rest and
     appears only on hover, focus, or mid-drag, when the keeper is actually
     looking for the seam. The invisible 6cqi hit area is felt regardless. */
  .share-handle::after,
  .inset-handle::after {
    content: '';
    position: absolute;
    border-style: dashed;
    border-color: color-mix(in oklab, var(--ink) 70%, transparent);
    opacity: 0;
    transition: opacity 150ms ease;
  }

  .share-handle:hover::after,
  .inset-handle:hover::after,
  .share-handle:focus-visible::after,
  .inset-handle:focus-visible::after,
  .share-handle.active::after,
  .inset-handle.active::after {
    opacity: 1;
  }

  /* Header/art and art/props seams: flush with the band's own bottom edge,
     the hit area reaching up into that same band so `overflow:hidden` on it
     never clips the handle away. */
  .share-handle--head,
  .share-handle--art {
    left: 0;
    right: 0;
    bottom: 0;
    height: 6cqi;
    cursor: row-resize;
  }

  .share-handle--head::after,
  .share-handle--art::after {
    left: 0;
    right: 0;
    bottom: 0;
    height: 0;
    border-top-width: 2px;
  }

  /* Props/footer seam: flush with the footer's own top edge instead, since
     the footer — not the properties band above it — is the one with a fixed
     height to drag. */
  .share-handle--foot {
    left: 0;
    right: 0;
    top: 0;
    height: 6cqi;
    cursor: row-resize;
  }

  .share-handle--foot::after {
    left: 0;
    right: 0;
    top: 0;
    height: 0;
    border-top-width: 2px;
  }

  /* The window's own four edges — the frame's carved opening, not a band
     seam. Flush against `.content`'s own edges for the same reason. */
  .inset-handle--top,
  .inset-handle--bottom {
    left: 0;
    right: 0;
    height: 6cqi;
    cursor: row-resize;
  }

  .inset-handle--top { top: 0; }
  .inset-handle--bottom { bottom: 0; }

  .inset-handle--top::after,
  .inset-handle--bottom::after {
    left: 0;
    right: 0;
    height: 0;
    border-top-width: 2px;
  }

  .inset-handle--top::after { top: 0; }
  .inset-handle--bottom::after { bottom: 0; }

  .inset-handle--left,
  .inset-handle--right {
    top: 0;
    bottom: 0;
    width: 6cqi;
    cursor: col-resize;
  }

  .inset-handle--left { left: 0; }
  .inset-handle--right { right: 0; }

  .inset-handle--left::after,
  .inset-handle--right::after {
    top: 0;
    bottom: 0;
    width: 0;
    border-left-width: 2px;
  }

  .inset-handle--left::after { left: 0; }
  .inset-handle--right::after { right: 0; }

  /* The frame: sits outside `.content` on purpose, so a cut-out frame's own
     overflow:hidden window can never clip it, and above the carving so it is
     always reachable even on a dressed card. */
  .frame-control {
    position: absolute;
    top: 2cqi;
    right: 2cqi;
    z-index: 5;
  }

  .frame-btn {
    padding: 0.25em 0.6em;
    font-size: 0.65rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--ink);
    background: color-mix(in oklab, var(--paper) 85%, var(--ink) 15%);
    border: 1px solid color-mix(in oklab, var(--ink) 35%, transparent);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.15);
    cursor: pointer;
  }

  .frame-backdrop {
    position: fixed;
    inset: 0;
    z-index: 5;
    padding: 0;
    background: transparent;
    border: none;
  }

  .frame-popover {
    position: absolute;
    top: 100%;
    right: 0;
    z-index: 6;
    display: flex;
    flex-direction: column;
    gap: 0.4em;
    min-width: 9rem;
    margin-top: 0.3em;
    padding: 0.5em;
    font-size: 0.7rem;
    color: var(--ink, #34251c);
    background: var(--paper, #f8f1e7);
    border: 1px solid color-mix(in oklab, var(--ink) 30%, transparent);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
  }

  .frame-tier-row {
    display: flex;
    gap: 0.25em;
  }

  .frame-tier {
    flex: 1 1 auto;
    padding: 0.3em 0;
    text-align: center;
    color: var(--ink);
    background: transparent;
    border: 1px solid color-mix(in oklab, var(--ink) 30%, transparent);
    cursor: pointer;
  }

  .frame-tier.active {
    color: var(--paper);
    background: var(--ink);
  }

  .frame-own {
    padding: 0.35em 0.5em;
    text-align: left;
    color: var(--ink);
    background: transparent;
    border: 1px solid color-mix(in oklab, var(--ink) 30%, transparent);
    cursor: pointer;
  }

  /* The cost/power badge's own numeric editor — a click on the badge that
     never moved opens this instead of dragging it. Anchored on the badge's
     own centre, same as the badge itself, and dropped just clear of it. */
  /* Только ПРИВЯЗКА: где стоит стол значка. Бумага, кайма и отступы — его
     собственные, и вторая их пара здесь однажды разошлась бы с первой.
     Отвод от значка назван в cqi, потому что отводят от кружка, а кружок
     мерян шириной карты, и число это — его ПОЛОВИНА (5.75 при 10.5cqi) плюс
     волосок: считать отвод от старой величины значит отходить на пустое
     место, которого на карте больше нет. Сдвиг — в пикселях: он не про карту, а про экран,
     и назначает его `fitBadgePopover`. */
  .badge-popover {
    position: absolute;
    z-index: 6;
    transform: translate(
      calc(-50% + var(--bi-shift-x, 0px)),
      calc(5.75cqi + 6px + var(--bi-shift-y, 0px))
    );
  }

  /* Над значком — когда под ним до края экрана не осталось места. */
  .badge-popover--up {
    transform: translate(
      calc(-50% + var(--bi-shift-x, 0px)),
      calc(-100% - 5.75cqi - 6px + var(--bi-shift-y, 0px))
    );
  }

  /* A tilting, sweeping card is decoration; the card without it is the whole
     card. So the effect is removed rather than slowed. */
  @media (prefers-reduced-motion: reduce) {
    .card,
    .slot:hover .card:not(.card--still) {
      transform: none;
      transition: none;
    }

    .foil {
      display: none;
    }

    .struck,
    .scrap {
      animation: none;
      display: none;
    }
  }
</style>
