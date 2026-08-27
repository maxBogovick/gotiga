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
  import type { BattleCard, BattleFrame } from '$lib/types/api';
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
    parseFocal,
    pricesOf,
    cardTransitionName,
    pickImageFile,
    DEFAULT_COST_X,
    DEFAULT_COST_Y,
    DEFAULT_POWER_X,
    DEFAULT_POWER_Y,
    BADGE_SHAPES,
    applyInsetDelta,
    type InsetKey,
    type FrameOverride,
  } from '$lib/battles';
  import { api } from '$lib/api';
  import AppImage from '$lib/components/AppImage.svelte';

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
    raceIconEditable = false,
    onEditRace,
    onIconUpload,
    onError,
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
    /** The header icon alone is a live uploader, independent of `editable` —
     *  what the Races tab's own sample card sets, where nothing else here is
     *  this card's to edit. */
    raceIconEditable?: boolean;
    /** Editable but not this card's race to rename — send the keeper there. */
    onEditRace?: () => void;
    /** A new icon was chosen for the race this sample wears. */
    onIconUpload?: (url: string) => void;
    onError?: (message: string) => void;
  } = $props();

  let frame = $derived(frameForCard(card, frames));
  /** The language the card's own text reads as. A reader always sees the
   *  site's language; the keeper's desk passes its RU/EN toggle instead, so
   *  the preview shows exactly the language being typed into the sidebar
   *  form, never the other one. */
  let editLang2 = $derived((editLang ?? $lang) as 'en' | 'ru');
  let copy = $derived(cardCopy(card, editLang2));
  let focal = $derived(parseFocal(card.artFocal));
  let prices = $derived(pricesOf(card));
  let head = $derived(headerCopy(card, editLang2));
  let traits = $derived((card.traits ?? []).map((t) => traitCopy(t, editLang2)));
  let rank = $derived(frameName(frame, $lang));
  let dressed = $derived(isDressed(frame));
  let overlaid = $derived(isOverlaid(frame));
  let hasBackArt = $derived(!!frame.backImage?.trim());
  let vars = $derived(frameVars(frame));
  let varStyle = $derived(
    Object.entries(vars)
      .map(([k, v]) => `${k}:${v}`)
      .join(';'),
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
      if (art.width && art.height) patch.aspect = art.width / art.height;
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

  function shareDragStart(kind: ShareKey, event: PointerEvent & { currentTarget: HTMLElement }) {
    if (!frameEditable) return;
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
    applyInsetDelta(rankFrame(), kind, delta);
  }

  function frameDragMove(event: PointerEvent) {
    shareDragMove(event);
    insetDragMove(event);
  }

  function frameDragEnd(event: PointerEvent & { currentTarget: HTMLElement }) {
    if (!frameDragKind) return;
    frameDragKind = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
  }

  // ── Cost and power, dragged instead of dialled ─────────────────────────────
  //
  // Same tap-vs-drag gesture as the photograph: held and moved, the badge
  // follows the pointer; let go without moving it and its own numeric editor
  // opens instead — the keyboard-reachable form a drag can never be exact
  // enough to replace.

  type BadgeKind = 'cost' | 'power';
  const BADGE_KEYS: Record<
    BadgeKind,
    { x: 'costX'; y: 'costY'; shape: 'costShape' } | { x: 'powerX'; y: 'powerY'; shape: 'powerShape' }
  > = {
    cost: { x: 'costX', y: 'costY', shape: 'costShape' },
    power: { x: 'powerX', y: 'powerY', shape: 'powerShape' },
  };

  let badgeDragKind = $state<BadgeKind | null>(null);
  let badgeMoved = false;
  let badgePopoverOpen = $state<BadgeKind | null>(null);
  /** Where the popover sits, in % of `.slot` (this component's own root) —
   *  read off the badge itself when it opens rather than reusing
   *  `frame.costX`/`costY`: those are percentages of `.content`, which sits
   *  inset from `.slot` by the frame's own padding, so reusing them here would
   *  drift the popover away from the badge on any dressed or inset frame.
   *  Rendered as a sibling of `.card` rather than inside it: an unrelated
   *  global `.card { overflow: hidden }` rule (see the admin design system)
   *  would otherwise clip it, the same trap `.frame-popover` sits in. */
  let badgePopoverPos = $state<{ left: number; top: number } | null>(null);

  function badgeDragStart(kind: BadgeKind, event: PointerEvent & { currentTarget: HTMLElement }) {
    if (!frameEditable) return;
    event.preventDefault();
    event.stopPropagation();
    badgeDragKind = kind;
    badgeMoved = false;
    event.currentTarget.setPointerCapture(event.pointerId);
  }

  function badgeDragMove(event: PointerEvent) {
    if (!badgeDragKind || !contentEl) return;
    const rect = contentEl.getBoundingClientRect();
    if (!rect.width || !rect.height) return;
    badgeMoved = true;
    const { x, y } = BADGE_KEYS[badgeDragKind];
    const target = rankFrame();
    const dx = (event.movementX / rect.width) * 100;
    const dy = (event.movementY / rect.height) * 100;
    target[x] = Math.min(100, Math.max(0, (target[x] ?? 0) + dx));
    target[y] = Math.min(100, Math.max(0, (target[y] ?? 0) + dy));
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
  data-tier={card.tier}
  data-layout={frame.layout}
  style={varStyle}
  style:view-transition-name={transition ? cardTransitionName(card) : undefined}
  onpointermove={track}
  onpointerleave={rest}
  aria-label="{copy.title} — {rank}"
>
 <div
   class="card"
   class:card--down={!owned}
   class:card--still={!interactive}
   class:card--dressed={dressed && owned}
   class:card--overlaid={overlaid && owned}
   class:card--back-art={!owned && hasBackArt}
 >
  <div class="content" bind:this={contentEl}>
  {#if owned}
    <!-- 1. The header: what this is, and what kind of thing it is. -->
    <header class="band band--head">
      {#if editable || raceIconEditable || card.raceIconUrl}
        <button
          type="button"
          class="race-icon"
          class:race-icon--live={editable || raceIconEditable}
          disabled={!editable && !raceIconEditable}
          onclick={handleIconClick}
          aria-label={raceIconEditable ? $t('adminBattlesRaceIconUpload') : $t('adminBattlesRaceJump')}
        >
          {#if card.raceIconUrl}
            <img src={card.raceIconUrl} alt="" class="race-icon-img" />
          {/if}
        </button>
      {/if}

      {#if head.race}<span class="race">{head.race}</span>{/if}
      {#if head.race && head.type}<span class="head-sep">·</span>{/if}
      {#if head.type}<span class="kind">{head.type}</span>{/if}

      <!-- Notches, not a number: at shelf size a digit disappears and a row of
           marks does not. This is the level of your copy — never the card's
           rank, which is worn as the frame itself. -->
      {#if level != null}
        <span class="pips" aria-label="{$t('battlesLevelLabel')}: {level}">
          {#each [1, 2, 3, 4, 5] as step (step)}
            <span class="pip" class:pip--lit={step <= level}></span>
          {/each}
        </span>
      {/if}

      {#if frameEditable}
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
      <span class="foil" aria-hidden="true"></span>

      {#if frameEditable}
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
    <div class="band band--props">
      <div class="plate">
        <h3 class="title">{copy.title}</h3>
        <p class="rank">{rank}</p>
      </div>

      {#if traits.length}
        <ul class="traits">
          {#each traits as trait, i (i)}
            <li class="trait">
              <span class="trait-name">
                {trait.name}{#if trait.other}<span class="trait-other">({trait.other})</span>{/if}{#if trait.text}:{/if}
              </span>
              {#if trait.text}<span class="trait-text"> {trait.text}</span>{/if}
            </li>
          {/each}
        </ul>
      {/if}

      {#if copy.effect}
        <p class="effect">{copy.effect}</p>
      {/if}

      {#if copy.lore}
        <p class="lore">{copy.lore}</p>
      {/if}

      <p class="numbers">
        <span class="number">{$t('battlesHealthLabel')} <b>{card.health}</b></span>
        <span class="number">{$t('battlesManaLabel')} <b>{card.mana}</b></span>
      </p>
    </div>

    <!-- 4. The footer. -->
    <footer class="band band--foot">
      {#if frameEditable}
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
      {#if frame.layout !== 'corners'}
        <span class="stats">
          {$t('battlesCostLabel')} {card.cost} · {$t('battlesPowerLabel')} {card.power}
        </span>
      {/if}
    </footer>

    {#if isNew}
      <span class="new-mark">{$t('battlesNew')}</span>
    {/if}
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
    <!-- Cost and power, above the carving. Same box as `.content` (see
         `.badges-layer` below) but its own layer: a cut-out frame's ornament
         can bulge inward over the window, and a badge sitting inside
         `.content`'s own stacking context can never paint over a sibling
         layer no matter its local z-index — only a layer of its own can. -->
    <div class="badges-layer">
      <!-- Cost, top left by default. Held and dragged, the badge follows the
           pointer; clicked without moving, its own X/Y editor opens instead —
           in a fanned hand the left edge is the sliver you can actually see,
           which is where every game held in a hand puts it. -->
      <button
        type="button"
        class="corner corner--cost corner--shape-{frame.costShape ?? 'circle'}"
        class:corner--editable={frameEditable}
        disabled={!frameEditable}
        style="left:{frame.costX ?? DEFAULT_COST_X}%; top:{frame.costY ?? DEFAULT_COST_Y}%"
        title={$t('battlesCostLabel')}
        onpointerdown={(e) => badgeDragStart('cost', e)}
        onpointermove={badgeDragMove}
        onpointerup={badgeDragEnd}
        onpointercancel={badgeDragEnd}
      >{card.cost}</button>
      <!-- Power, dragged and edited the same way as cost. -->
      <button
        type="button"
        class="corner corner--power corner--shape-{frame.powerShape ?? 'circle'}"
        class:corner--editable={frameEditable}
        disabled={!frameEditable}
        style="left:{frame.powerX ?? DEFAULT_POWER_X}%; top:{frame.powerY ?? DEFAULT_POWER_Y}%"
        title={$t('battlesPowerLabel')}
        onpointerdown={(e) => badgeDragStart('power', e)}
        onpointermove={badgeDragMove}
        onpointerup={badgeDragEnd}
        onpointercancel={badgeDragEnd}
      >{card.power}</button>
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
    <!-- The carving, laid over the card. A cut-out frame is a picture with a
         hole in it, not a border: its ornament runs past the rectangle and its
         inner edge is meant to overlap the photograph. Last in the stack, and
         deaf to the pointer so it never swallows anything underneath.
         Never worn face down — the frame is the FRONT's own dress; a card
         lying in dust shows its back, not the front's carving. -->
    <span class="carving" aria-hidden="true"></span>
  {/if}
 </div>

 {#if badgePopoverOpen && badgePopoverPos}
   <!-- The cost/power badge's own numeric editor. Anchored off the badge's
        own screen position rather than `frame.costX`/`powerX` directly, and
        a sibling of `.card` rather than inside it — an unrelated global
        `.card { overflow: hidden }` rule would otherwise clip it. -->
   <button
     type="button"
     class="frame-backdrop"
     aria-label={$t('adminBattlesFrameClose')}
     onclick={() => (badgePopoverOpen = null)}
   ></button>
   <div class="badge-popover" style="left:{badgePopoverPos.left}%; top:{badgePopoverPos.top}%">
     <span class="badge-popover-label">
       {badgePopoverOpen === 'cost' ? $t('battlesCostLabel') : $t('battlesPowerLabel')}
     </span>
     <div class="badge-popover-shapes" role="radiogroup" aria-label={$t('adminBattlesBadgeShape')}>
       {#each BADGE_SHAPES as shape (shape)}
         <button
           type="button"
           class="badge-shape-swatch badge-shape-swatch--{shape}"
           class:active={(badgePopoverOpen === 'cost' ? frame.costShape : frame.powerShape) === shape}
           title={$t(
             shape === 'circle' ? 'adminBattlesBadgeShapeCircle' :
             shape === 'square' ? 'adminBattlesBadgeShapeSquare' :
             shape === 'diamond' ? 'adminBattlesBadgeShapeDiamond' :
             shape === 'hex' ? 'adminBattlesBadgeShapeHex' :
             'adminBattlesBadgeShapeShield',
           )}
           role="radio"
           aria-checked={(badgePopoverOpen === 'cost' ? frame.costShape : frame.powerShape) === shape}
           onclick={() => (rankFrame()[BADGE_KEYS[badgePopoverOpen!].shape] = shape)}
         ></button>
       {/each}
     </div>
     <div class="badge-popover-row">
       <label class="badge-popover-field">
         X <input
           type="number" min="0" max="100" step="1"
           value={Math.round(
             (badgePopoverOpen === 'cost' ? frame.costX : frame.powerX) ??
               (badgePopoverOpen === 'cost' ? DEFAULT_COST_X : DEFAULT_POWER_X),
           )}
           oninput={(e) => (rankFrame()[BADGE_KEYS[badgePopoverOpen!].x] = Number(e.currentTarget.value))}
         />
       </label>
       <label class="badge-popover-field">
         Y <input
           type="number" min="0" max="100" step="1"
           value={Math.round(
             (badgePopoverOpen === 'cost' ? frame.costY : frame.powerY) ??
               (badgePopoverOpen === 'cost' ? DEFAULT_COST_Y : DEFAULT_POWER_Y),
           )}
           oninput={(e) => (rankFrame()[BADGE_KEYS[badgePopoverOpen!].y] = Number(e.currentTarget.value))}
         />
       </label>
     </div>
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
  }

  .card {
    position: relative;
    height: 100%;
    background: var(--paper);
    color: var(--ink);
    border: 1px solid var(--edge);
    box-shadow:
      inset 0 0 0 2cqi var(--paper),
      inset 0 0 0 calc(2cqi + 1px) var(--edge),
      0 2px 14px rgba(52, 37, 28, 0.14);
    font-family: Georgia, 'Fraunces', serif;
    transition: transform 420ms cubic-bezier(0.22, 1, 0.36, 1);
    transform-style: preserve-3d;
    will-change: transform;
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

  /* The same box as `.content` (same inset formula, same conditional padding
     below) but a layer of its OWN, above the carving — a stacking context
     can only be beaten by a sibling with a higher z-index, never from
     inside it, which is why the badges cannot simply ask for a higher
     z-index while still living inside `.content`. Deaf to the pointer
     itself so the empty parts of the layer never steal a click from the
     photograph or the header underneath; only the badges opt back in. */
  .badges-layer {
    position: absolute;
    inset: var(--pad-top, 0) var(--pad-right, 0) var(--pad-bottom, 0) var(--pad-left, 0);
    padding: 5cqi;
    z-index: 4;
    pointer-events: none;
  }

  .card--dressed .badges-layer {
    padding: 0;
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

  .corner {
    position: absolute;
    z-index: 2;
    display: grid;
    place-items: center;
    width: 13cqi;
    height: 13cqi;
    margin: 0;
    padding: 0;
    font: inherit;
    font-size: 7cqi;
    line-height: 1;
    color: var(--paper);
    background: var(--ink);
    border: none;
    /* `left`/`top` are the badge's own CENTRE, in % of the card, so a drag can
       move it in either direction from wherever it starts without the badge's
       own size skewing the math. */
    transform: translate(-50%, -50%);
    cursor: default;
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

  .corner--power {
    background: var(--edge);
    color: var(--ink);
  }

  /* Only in the Frames tab: the badge itself becomes a handle, dragged to
     reposition and clicked (without moving) to open its own X/Y editor. */
  .corner--editable {
    cursor: grab;
    touch-action: none;
  }

  .corner--editable:active {
    cursor: grabbing;
  }

  .corner:disabled {
    opacity: 1;
  }

  /* Three bands are measured; the properties band is not, and takes the rest.
     Sliders that happen to add up can never squeeze it to nothing. */
  .band--head {
    flex: 0 0 var(--header-share, 9%);
    font-size: 3.8cqi;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--ink);
    opacity: 0.72;
  }

  .band--foot {
    flex: 0 0 var(--foot-share, 10%);
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

  /* The cost badge keeps its corner; the header starts clear of it. */
  .slot[data-layout='corners'] .band--head {
    padding-left: 17cqi;
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
    opacity: 0.5;
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
    /* Gives way before the numbers do — a clipped property still reads, a
       missing Health does not. */
    flex: 0 1 auto;
    margin: 2.5cqi 0 0;
    padding: 0;
    list-style: none;
    min-height: 0;
    overflow: hidden;
    font-size: 4.2cqi;
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
    opacity: 0.6;
  }

  .numbers {
    display: flex;
    flex: 0 0 auto;
    gap: 3cqi;
    margin: auto 0 0;
    padding-top: 2.5cqi;
    font-size: 4cqi;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--ink);
    opacity: 0.8;
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

  .plate {
    flex: 0 0 auto;
  }

  .title {
    margin: 0;
    font-family: var(--title-face, inherit);
    color: var(--title-ink, var(--ink));
    font-size: 7cqi;
    line-height: 1.15;
    font-weight: 400;
    letter-spacing: 0.01em;
  }

  .rank {
    margin: 1cqi 0 0;
    color: var(--ink);
    font-size: 4cqi;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    opacity: 0.62;
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
    font-size: 5cqi;
    line-height: 1.35;
  }

  /* On a shelf the card is a thumbnail: it carries its name, its effect and its
     numbers. The note underneath is for the card seen large. This is what the
     container query is for, rather than shipping a second "compact" component. */
  @container (max-width: 240px) {
    .lore {
      display: none;
    }

    .effect {
      -webkit-line-clamp: 3;
      line-clamp: 3;
    }
  }

  .lore {
    /* First to give way when the band is short: a note is the least of what a
       card has to say. */
    flex: 1 1 auto;
    min-height: 0;
    overflow: hidden;
    margin: 2cqi 0 0;
    color: var(--ink);
    font-size: 4.4cqi;
    line-height: 1.4;
    font-style: italic;
    opacity: 0.66;
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

  .slot[data-layout='plaque'] .plate,
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
    font-size: 4.4cqi;
    letter-spacing: 0.06em;
    color: var(--ink);
  }

  .new-mark {
    position: absolute;
    top: 3cqi;
    right: 3cqi;
    padding: 1cqi 2.4cqi;
    font-size: 3.6cqi;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: var(--paper);
    background: #c65f3c;
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
    font-size: 6cqi;
    color: var(--ink);
    opacity: 0.78;
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
    font-size: 6.4cqi;
    color: var(--ink);
  }

  .price-coin {
    font-size: 3.6cqi;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    opacity: 0.66;
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

  /* The dashed line is the whole point of the handle being visible at all —
     the invisible 6cqi hit area is only ever felt, never seen, so the seam
     itself has to be drawn or there is nothing on screen to tell the keeper
     where one band ends and the drag even starts. Faint at rest so it reads
     as a guide and not as part of the card's own ink; full strength on
     hover, focus, or mid-drag, when it is the thing being watched. */
  .share-handle::after,
  .inset-handle::after {
    content: '';
    position: absolute;
    border-style: dashed;
    border-color: color-mix(in oklab, var(--ink) 70%, transparent);
    opacity: 0.6;
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
  .badge-popover {
    position: absolute;
    z-index: 6;
    display: flex;
    flex-direction: column;
    gap: 0.4em;
    padding: 0.5em 0.6em;
    font-size: 0.65rem;
    color: var(--ink, #34251c);
    background: var(--paper, #f8f1e7);
    border: 1px solid color-mix(in oklab, var(--ink) 30%, transparent);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
    transform: translate(-50%, calc(7cqi + 6px));
    white-space: nowrap;
  }

  .badge-popover-label {
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .badge-popover-row {
    display: flex;
    align-items: center;
    gap: 0.6em;
  }

  .badge-popover-shapes {
    display: flex;
    gap: 0.35em;
  }

  .badge-shape-swatch {
    width: 1.3em;
    height: 1.3em;
    padding: 0;
    background: color-mix(in oklab, var(--ink) 55%, transparent);
    border: 1px solid transparent;
    cursor: pointer;
  }

  .badge-shape-swatch.active {
    border-color: var(--ink);
    background: var(--ink);
  }

  .badge-shape-swatch--circle {
    border-radius: 50%;
  }

  .badge-shape-swatch--square {
    border-radius: 12%;
  }

  .badge-shape-swatch--diamond {
    border-radius: 0;
    clip-path: polygon(50% 0, 100% 50%, 50% 100%, 0 50%);
  }

  .badge-shape-swatch--hex {
    border-radius: 0;
    clip-path: polygon(25% 0%, 75% 0%, 100% 50%, 75% 100%, 25% 100%, 0% 50%);
  }

  .badge-shape-swatch--shield {
    border-radius: 0;
    clip-path: polygon(50% 0%, 100% 18%, 100% 55%, 50% 100%, 0% 55%, 0% 18%);
  }

  .badge-popover-field {
    display: flex;
    align-items: center;
    gap: 0.3em;
  }

  .badge-popover-field input {
    width: 3.2em;
    padding: 0.15em 0.3em;
    font: inherit;
    color: var(--ink);
    background: color-mix(in oklab, var(--paper) 90%, var(--ink) 10%);
    border: 1px solid color-mix(in oklab, var(--ink) 25%, transparent);
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
  }
</style>
