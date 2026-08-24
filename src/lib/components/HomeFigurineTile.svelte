<script lang="ts">
    import { onMount } from 'svelte';
    import { goto } from '$app/navigation';
    import type { FigurineListItem } from '$lib/types/api';
    import { figurineHref } from '$lib/figurineHref';
    import { t, lang } from '$lib/i18n';
    import AppImage from '$lib/components/AppImage.svelte';
    import SealedDoor from '$lib/components/SealedDoor.svelte';
    import { savedFigurines } from '$lib/stores/saved-figurines.svelte';
    import { houseClock } from '$lib/stores/house-clock.svelte';
    import { showingRooms } from '$lib/stores/showing-rooms.svelte';
    import { isGated, isShowingOpen, resolveWindow, openingHeadline } from '$lib/showing-window';

    let {
        fig,
        index = 0,
        compact = false,
        selected = false,
        masonry = false,
        source,
        onLike,
    }: {
        fig: FigurineListItem;
        index?: number;
        compact?: boolean;
        selected?: boolean;
        // Pinterest layout: the media keeps its natural height (the home gallery
        // measures that height into a grid row-span). Off elsewhere (shelves,
        // hall) so those tidy auto-fit rows keep a fixed crop.
        masonry?: boolean;
        // Which named home-page block rendered this tile (e.g. "home_marked") —
        // tags the outgoing link so admin analytics can attribute the click.
        // Left undefined by every other caller (hall), so their links stay untagged.
        source?: string;
        /** Fired after a heart tap — optional analytics hook from the home page. */
        onLike?: () => void;
    } = $props();

    let saved = $derived(savedFigurines.has(fig.id));
    let copied = $state(false);
    let justSaved = $state(false);
    let copyTimer: ReturnType<typeof setTimeout> | undefined;
    let pulseTimer: ReturnType<typeof setTimeout> | undefined;

    // THE COLLECTION card-fx toolbar: a plain, live IntersectionObserver
    // toggle (same API this codebase already uses for workshopActivated in
    // +page.svelte) drives every scroll-reveal variant. Stays live — not
    // one-shot — so a card leaving view goes back to "unrevealed", meaning
    // switching variants always shows the reveal play out as the visitor
    // scrolls, rather than every already-seen card staying stuck revealed.
    let fxRevealed = $state(false);
    function revealOnEnter(node: HTMLElement) {
        if (typeof IntersectionObserver === 'undefined') { fxRevealed = true; return; }
        // rootMargin shrinks the viewport's bottom edge by 35% — a card only
        // counts as "in view" once meaningfully up into the frame, not the
        // instant its top pixel peeks over the fold (which would make it
        // trigger effectively immediately, before it's visible happening).
        const io = new IntersectionObserver(([entry]) => {
            fxRevealed = entry.isIntersecting;
        }, { rootMargin: '0px 0px -35% 0px', threshold: 0 });
        io.observe(node);
        return { destroy() { io.disconnect(); } };
    }

    let href = $derived(figurineHref(fig, source));

    // Editorial plate number — a print-catalogue flourish, distinct from the
    // functional archive number (kept in the footer, used elsewhere for sharing).
    const ROMAN_MAP: [number, string][] = [
        [1000, 'M'], [900, 'CM'], [500, 'D'], [400, 'CD'],
        [100, 'C'], [90, 'XC'], [50, 'L'], [40, 'XL'],
        [10, 'X'], [9, 'IX'], [5, 'V'], [4, 'IV'], [1, 'I'],
    ];
    function toRoman(num: number): string {
        let n = num;
        let out = '';
        for (const [value, sym] of ROMAN_MAP) {
            while (n >= value) { out += sym; n -= value; }
        }
        return out || 'I';
    }
    let romanNumeral = $derived(toRoman(index + 1));

    // Asymmetric rhythm: a work the author flags as "featured" (admin figurine
    // form → «Широкий разворот») earns a wide editorial spread — a deliberate,
    // author-curated emphasis, not a mechanical every-Nth trick that lands wide
    // plates at unpredictable grid positions.
    let feature = $derived(!compact && !!fig.isFeatured);

    // Cover-crop focal point (normalised 0..1, default centre) — keeps the
    // subject framed when the photo is cropped to fill the plate.
    let focalPos = $derived(
        `${Math.min(1, Math.max(0, fig.focalX ?? 0.5)) * 100}% ${Math.min(1, Math.max(0, fig.focalY ?? 0.5)) * 100}%`
    );

    // "The house wakes": a gated work is sealed behind a carved door while the
    // visitor's local clock sits outside its showing window. The effective window
    // is the work's own hours OR its showing room's (resolveWindow). houseClock.now
    // ticks by the minute, so a door dissolves live the moment its window opens.
    let win = $derived(resolveWindow(
        { openFromMin: fig.openFromMin, openUntilMin: fig.openUntilMin, showingRoomId: fig.showingRoomId },
        showingRooms.list
    ));
    let doorClosed = $derived(isGated(win) && !isShowingOpen(win, houseClock.nowDate));
    let doorLocale = $derived($lang === 'ru' ? 'ru-RU' : 'en-US');
    let doorHeadline = $derived(doorClosed ? openingHeadline(win, $t, doorLocale, houseClock.nowDate) : '');

    // Recently catalogued — surfaced as a quiet wax mark, not a sales badge.
    let isNew = $derived.by(() => {
        if (!fig.createdAt) return false;
        const ts = new Date(fig.createdAt).getTime();
        if (Number.isNaN(ts)) return false;
        return Date.now() - ts < 21 * 24 * 60 * 60 * 1000;
    });

    let statusLabel = $derived(
        fig.status === 'available'
            ? $t('archiveStatusAvailableLabel')
            : fig.status === 'reserved'
                ? $t('archiveStatusReservedLabel')
                : fig.status === 'in_progress'
                    ? $t('profileWishInProgress')
                    : $t('archiveStatusSoldLabel')
    );

    // The plate caption must never contradict the door beneath it: a gated
    // piece says "soon" up top too, in the door's own words — not whatever
    // its commission status happens to be while it's still sealed.
    let plateStatusKey = $derived(doorClosed ? 'soon' : fig.status);
    let plateStatusLabel = $derived(doorClosed ? $t('posterSoon') : statusLabel);

    onMount(() => {
        savedFigurines.load();
        houseClock.start();
        showingRooms.load();
        return () => {
            clearTimeout(copyTimer);
            clearTimeout(pulseTimer);
        };
    });

    function toggleSaved(e: MouseEvent) {
        e.preventDefault();
        e.stopPropagation();
        const next = !saved;
        void savedFigurines.set(fig.id, next);
        onLike?.();
        if (next) {
            justSaved = true;
            clearTimeout(pulseTimer);
            pulseTimer = setTimeout(() => { justSaved = false; }, 650);
        }
    }

    async function shareWork(e: MouseEvent) {
        e.preventDefault();
        e.stopPropagation();
        const url = typeof location !== 'undefined' ? `${location.origin}${href}` : href;
        try {
            if (typeof navigator !== 'undefined' && navigator.share) {
                await navigator.share({ title: fig.name, url });
                return;
            }
            await navigator.clipboard.writeText(url);
            copied = true;
            clearTimeout(copyTimer);
            copyTimer = setTimeout(() => { copied = false; }, 1800);
        } catch {
            /* user dismissed the share sheet — nothing to do */
        }
    }

    function openSimilarCommission(e: MouseEvent) {
        e.preventDefault();
        e.stopPropagation();
        goto(`/commission?source=${encodeURIComponent(fig.id)}`);
    }
</script>

<!--
  Image-forward "Pinterest" plate: the photograph is the whole card. A frosted
  glass plaque (status · year · roman, and the piece's name) sits inset over
  the foot of the photo; save / share / commission surface on hover (and stay
  put on touch, where there is no hover). The full showing-door / keyhole
  machinery from the ledger card it replaced is unchanged.
-->
<article
    class="tile"
    class:tile-compact={compact}
    class:tile-feature={feature}
    class:tile-masonry={masonry}
    class:is-selected={selected}
    class:fx-revealed={fxRevealed}
    style="--i:{index}"
    use:revealOnEnter
>
    <div class="tile-media-wrap" style={doorClosed ? '' : `view-transition-name: figurine-${fig.id}`}>
        {#if doorClosed}
            <!-- Showing window shut: a carved door, not a link. No view-transition
                 (there is no detail to morph into) and no dwell-reveal. -->
            <div class="tile-media tile-media-sealed">
                <SealedDoor
                    openFromMin={win.openFromMin}
                    openUntilMin={win.openUntilMin}
                    daysMask={win.daysMask}
                    monthDay={win.monthDay}
                    dateFrom={win.dateFrom}
                    dateUntil={win.dateUntil}
                    imageUrl={fig.faceImageUrl}
                    thumbUrl={fig.thumbUrl}
                    name={fig.name}
                    showSchedule={false}
                />

                <div class="tile-glass">
                    <span class="tile-cap-meta">
                        <span class="tile-cap-status status-{plateStatusKey}">{plateStatusLabel}</span>
                        {#if doorHeadline}
                            <span class="tile-cap-dot" aria-hidden="true">·</span>
                            <span class="tile-cap-soon">{doorHeadline}</span>
                        {/if}
                    </span>
                    <h3 class="tile-cap-name"><span class="tile-cap-name-text">{fig.name}</span></h3>
                </div>
            </div>
        {:else}
            <a
                {href}
                class="tile-media"
                style="--focal:{focalPos}"
                aria-label="{$t('homeViewFigurine')}: {fig.name}"
            >
                {#if fig.faceImageUrl}
                    <!-- The home grid is 4 columns on a wide screen and narrows down to one
                         on a phone; a wide (isFeatured) tile spans two of those columns.
                         `sizes` describes that so the browser can choose among the 420/900/
                         1800 renditions instead of assuming the tile fills the viewport. -->
                    <AppImage
                        src={fig.faceImageUrl}
                        thumbUrl={fig.thumbUrl}
                        alt={fig.name}
                        class="tile-img"
                        loading="lazy"
                        sizes="(max-width: 680px) 100vw, (max-width: 1024px) 50vw, 25vw"
                    />
                    {#if fig.detailImageUrl}
                        <!-- a second angle, held in reserve for a lingering look — not a
                             product-swatch swap, a closer look at the same piece. -->
                        <AppImage
                            src={fig.detailImageUrl}
                            alt=""
                            class="tile-img-alt"
                            loading="lazy"
                            sizes="(max-width: 680px) 100vw, (max-width: 1024px) 50vw, 25vw"
                        />
                    {/if}
                {:else}
                    <div class="tile-placeholder">?</div>
                {/if}

                <div class="tile-glass">
                    <span class="tile-cap-meta">
                        <span class="tile-cap-status status-{plateStatusKey}">{plateStatusLabel}</span>
                        {#if fig.year}
                            <span class="tile-cap-dot" aria-hidden="true">·</span>
                            <span class="tile-cap-year">{fig.year}</span>
                        {/if}
                        <span class="tile-cap-roman">{romanNumeral}</span>
                    </span>
                    <h3 class="tile-cap-name">
                        <span class="tile-cap-name-text">{fig.name}</span>
                        <svg class="tile-cap-arrow" width="16" height="8" viewBox="0 0 16 8" fill="none" aria-hidden="true">
                            <path d="M0 4H15M15 4L11 1M15 4L11 7" stroke="currentColor" stroke-width="1"/>
                        </svg>
                    </h3>
                </div>
            </a>
        {/if}

        <!-- top-left badge stack: today's-pick marker, house-favorite medal,
             "new" wax stamp — whichever apply, in that order of prestige -->
        <div class="tile-badges">
            {#if selected}
                <span class="tile-selected">{$t('homeHeroObjectLabel')}</span>
            {/if}
            {#if fig.houseFavorite}
                <span class="tile-favorite" title={$t('houseFavoriteBadge')}>
                    <svg width="11" height="11" viewBox="0 0 14 14" fill="none" aria-hidden="true">
                        <path d="M2 12C2 7 4 3 4 3M2 12L4 9.5M2 12L4.5 11" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"/>
                        <path d="M12 12C12 7 10 3 10 3M12 12L10 9.5M12 12L9.5 11" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"/>
                        <circle cx="7" cy="3" r="1.1" fill="currentColor"/>
                    </svg>
                    {$t('houseFavoriteBadge')}
                </span>
            {/if}
            {#if isNew}
                <span class="tile-seal" title={$t('archiveCardNew')}>{$t('archiveCardNew')}</span>
            {/if}
        </div>

        <!-- Quiet actions over the plate: hover-revealed on a fine pointer,
             always present where there is no hover (touch) so save/share keep
             full touch parity. -->
        <div class="tile-quick">
            <button
                class="tile-qbtn"
                class:is-active={saved}
                class:just-saved={justSaved}
                onclick={toggleSaved}
                aria-pressed={saved}
                aria-label={saved ? $t('cardSaved') : $t('cardSave')}
                title={saved ? $t('cardSaved') : $t('cardSave')}
            >
                <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
                    <path
                        d="M7 12.5C7 12.5 1 8.5 1 4.5C1 2.5 2.5 1 4.5 1C5.5 1 6.5 1.8 7 3C7.5 1.8 8.5 1 9.5 1C11.5 1 13 2.5 13 4.5C13 8.5 7 12.5 7 12.5Z"
                        fill={saved ? 'currentColor' : 'none'}
                        stroke="currentColor"
                        stroke-width="1.1"
                        stroke-linejoin="round"
                    />
                </svg>
            </button>

            <button
                class="tile-qbtn"
                class:is-active={copied}
                onclick={shareWork}
                aria-label={copied ? $t('cardLinkCopied') : $t('cardShare')}
                title={copied ? $t('cardLinkCopied') : $t('cardShare')}
            >
                {#if copied}
                    <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
                        <path d="M2.5 7.5L5.5 10.5L11.5 3.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
                    </svg>
                {:else}
                    <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
                        <circle cx="3" cy="7" r="1.6" stroke="currentColor" stroke-width="1.1"/>
                        <circle cx="11" cy="3" r="1.6" stroke="currentColor" stroke-width="1.1"/>
                        <circle cx="11" cy="11" r="1.6" stroke="currentColor" stroke-width="1.1"/>
                        <path d="M4.4 6.2L9.6 3.6M4.4 7.8L9.6 10.4" stroke="currentColor" stroke-width="1.1"/>
                    </svg>
                {/if}
            </button>

            {#if !doorClosed && fig.status === 'available'}
                <!-- the commission funnel, kept alive: "make me one like this" -->
                <button
                    class="tile-qbtn tile-qbtn-commission"
                    onclick={openSimilarCommission}
                    aria-label={$t('commissionCreateSimilarCta')}
                    title={$t('commissionCreateSimilarCta')}
                >
                    <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                        <path d="M5 2h8a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H5"/>
                        <path d="M5 2a2 2 0 0 0 0 4"/>
                        <path d="M5 6a2 2 0 0 1 0 4"/>
                        <path d="M5 10a2 2 0 0 0 0 4"/>
                        <line x1="8" y1="5.5" x2="12" y2="5.5"/>
                        <line x1="8" y1="8" x2="12" y2="8"/>
                        <line x1="8" y1="10.5" x2="10.5" y2="10.5"/>
                    </svg>
                </button>
            {/if}
        </div>
    </div>
</article>

<style>
    .tile {
        --tile-radius: 3px;
        position: relative;
        isolation: isolate;
        display: block;
        /* a wide feature plate must never stretch its narrower row-mates —
           each plate keeps its own natural height (masonry measures it). */
        align-self: start;
        min-width: 0;
        overflow: hidden;
        color: var(--ink, #34251c);
        text-decoration: none;
        border: 1px solid rgba(52,37,28,0.14);
        border-radius: var(--tile-radius);
        background: linear-gradient(180deg, rgba(255,253,248,0.94), rgba(248,240,227,0.86));
        animation: tile-in 0.5s var(--ease, cubic-bezier(0.16,1,0.3,1)) both;
        animation-delay: calc(var(--i) * 0.04s);
        transition: border-color 0.26s ease, transform 0.3s cubic-bezier(0.16,1,0.3,1), box-shadow 0.3s ease;
    }

    .tile:hover,
    .tile:focus-within {
        border-color: rgba(198,95,60,0.45);
        /* a vitrine lift, not a shop-card drop-shadow: a shallow rise + a low,
           soft ambient shadow, as if the piece were lifted for a closer look */
        transform: translateY(-3px);
        box-shadow: 0 14px 28px -18px rgba(20,13,9,0.4);
    }

    .tile.is-selected {
        border-color: rgba(198,95,60,0.5);
        outline: 1px solid rgba(198,95,60,0.28);
        outline-offset: -2px;
    }

    /* A featured plate spans two columns — an editorial "spread", author-curated.
       Falls back to one column on the narrowest layout, where there is no second
       track to span into. */
    .tile-feature { grid-column: span 2; }

    /* ── MEDIA: the photograph is the whole card ─────────────────────────── */
    .tile-media-wrap {
        position: relative;
        aspect-ratio: 4 / 3;
        overflow: hidden;
        background: rgba(201,168,117,0.10);
    }

    .tile-feature .tile-media-wrap { aspect-ratio: 16 / 9; }
    .tile-compact .tile-media-wrap { aspect-ratio: 1 / 1; }

    .tile-media {
        position: absolute;
        inset: 0;
        display: block;
        overflow: hidden;
        color: inherit;
        text-decoration: none;
    }

    .tile-media :global(.tile-img),
    .tile-media :global(.tile-img .app-image-thumb),
    .tile-media :global(.tile-img .app-image-main) {
        width: 100%;
        height: 100%;
        display: block;
        object-fit: cover;
        object-position: var(--focal, 50% 50%);
    }

    .tile-media :global(.tile-img .app-image-main) {
        filter: grayscale(0.08) saturate(0.94) contrast(1.02);
        transition: transform 0.48s ease, filter 0.48s ease;
    }

    .tile:hover .tile-media :global(.tile-img .app-image-main) {
        transform: scale(1.03);
        filter: grayscale(0) saturate(1.04) contrast(1.04);
    }

    /* ── SECOND ANGLE: a still layer pinned exactly over the face photo
       (independent of the masonry height rules below, which only ever
       measure the first image) — cross-fades in on a sustained hover. */
    .tile-media :global(.tile-img-alt) {
        position: absolute;
        inset: 0;
        z-index: 1;
        width: 100%;
        height: 100%;
        opacity: 0;
        pointer-events: none;
        transition: opacity 0.55s ease;
    }
    .tile-media :global(.tile-img-alt .app-image-thumb),
    .tile-media :global(.tile-img-alt .app-image-main) {
        width: 100%;
        height: 100%;
        display: block;
        object-fit: cover;
        object-position: var(--focal, 50% 50%);
    }
    .tile-media :global(.tile-img-alt .app-image-main) {
        filter: grayscale(0) saturate(1.04) contrast(1.04);
    }
    .tile:hover .tile-media :global(.tile-img-alt),
    .tile:focus-within .tile-media :global(.tile-img-alt) {
        opacity: 1;
    }
    /* Touch has no hover to trigger the reveal — keep only the face photo. */
    @media (hover: none) {
        .tile-media :global(.tile-img-alt) { display: none; }
    }

    /* ── MASONRY: regular plates keep their photo's natural height so the
       home gallery packs like Pinterest. Feature plates keep a controlled wide
       crop so a "spread" reads as a deliberate band, not a giant tower. ───── */
    .tile-masonry:not(.tile-feature) .tile-media-wrap { aspect-ratio: auto; }

    .tile-masonry:not(.tile-feature) .tile-media {
        position: relative;
        inset: auto;
    }

    .tile-masonry:not(.tile-feature) .tile-media :global(.tile-img) {
        height: auto;
    }

    .tile-masonry:not(.tile-feature) .tile-media :global(.tile-img .app-image-main) {
        position: relative;
        height: auto;
        object-fit: fill; /* height:auto → image shows at its intrinsic ratio */
    }

    /* Flatter than the default 3:2 — the "wide spread" reads through width,
       not sheer area, so the plate stays low even at full 2-column span. */
        .tile-masonry.tile-feature .tile-media-wrap { aspect-ratio: 16 / 9; }


    /* sealed-door plates carry no image height of their own — give the masonry
       wrap a portrait frame to stand in. Scoped to non-feature: a feature card's
       sealed door must instead match its own 16:9 wrap (below), or this portrait
       ratio wins and pushes the door's height (and its bottom caption plaque)
       past the wrap's shorter landscape box, clipping the plaque out of view. */
    .tile-masonry:not(.tile-feature) .tile-media-sealed {
        position: relative;
        inset: auto;
        aspect-ratio: 4 / 5;
    }

    /* A gated feature card keeps the same flat 16:9 as its unsealed counterpart —
       otherwise the sealed-door portrait ratio would win and it'd tower over its
       row-mates whenever a wide plate happens to be behind a closed door. */
    .tile-masonry.tile-feature .tile-media-sealed {
        position: relative;
        inset: auto;
        aspect-ratio: 16 / 9;
    }

    .tile-placeholder {
        width: 100%;
        height: 100%;
        min-height: 200px;
        display: grid;
        place-items: center;
        font-family: 'Cormorant Garamond', Georgia, serif;
        font-size: 30px;
        color: var(--color-ink-tertiary);
    }

    /* ── GLASS PLATE: a frosted plaque set on the photo, carrying the caption ── */
    .tile-glass {
        position: absolute;
        left: 10px;
        right: 10px;
        bottom: 10px;
        z-index: 3;
        display: grid;
        gap: 5px;
        padding: 12px 14px 11px;
        border-radius: 8px;
        /* darker floor stop (was 0.5) so caption text still clears WCAG 4.5:1
           even over the palest photo — a bone/stone/plaster piece, not just
           the average dark one this used to be tuned against */
        background: linear-gradient(165deg, rgba(52,37,28,0.4), rgba(14,9,6,0.62));
        backdrop-filter: blur(14px) saturate(150%);
        -webkit-backdrop-filter: blur(14px) saturate(150%);
        border: 1px solid rgba(255,247,234,0.28);
        box-shadow:
            0 10px 26px rgba(12,7,4,0.28),
            inset 0 1px 0 rgba(255,255,255,0.2);
        pointer-events: none;
        transition: background 0.3s ease, border-color 0.3s ease, transform 0.26s ease;
    }

    .tile:hover .tile-glass {
        background: linear-gradient(165deg, rgba(52,37,28,0.48), rgba(14,9,6,0.7));
        border-color: rgba(255,247,234,0.4);
    }

    /* A sealed door's own vignette is already near-black at the foot of the
       plate — the plaque's usual dark glass would sink into it and read as
       "no plaque at all". Lift it: a warmer, lighter glass + a firmer rule
       so it still stands proud as a plate on an already-dark ground. */
    .tile-media-sealed .tile-glass {
        background: linear-gradient(165deg, rgba(84,58,40,0.55), rgba(38,24,15,0.62));
        border-color: rgba(255,247,234,0.4);
        box-shadow:
            0 10px 26px rgba(12,7,4,0.4),
            inset 0 1px 0 rgba(255,255,255,0.24);
    }

    .tile-cap-meta {
        display: flex;
        align-items: baseline;
        flex-wrap: wrap;
        gap: 7px;
        font-family: 'Instrument Sans', system-ui, sans-serif;
        font-size: 9.5px;
        font-weight: 600;
        letter-spacing: 0.14em;
        line-height: 1.2;
        text-transform: uppercase;
        color: rgba(255,247,234,0.86);
    }

    .tile-cap-dot { opacity: 0.5; }

    /* Ledger ink, not a stock-light: every state reads in the same warm cream,
       distinguished by a small glyph rather than a green/amber "in stock"
       hue — that color-coding is inventory-UI language, not a museum's. */
    .tile-cap-status { color: rgba(255,247,234,0.86); }
    .tile-cap-status::before {
        content: '';
        display: inline-block;
        width: 5px;
        height: 5px;
        margin-right: 5px;
        border-radius: 50%;
        border: 1px solid currentColor;
        vertical-align: middle;
    }
    .tile-cap-status.status-available::before { background: currentColor; }
    .tile-cap-status.status-reserved::before,
    .tile-cap-status.status-in_progress::before { background: transparent; }
    .tile-cap-status.status-sold { color: rgba(255,247,234,0.6); }
    .tile-cap-status.status-sold::before { background: currentColor; opacity: 0.4; }
    .tile-cap-status.status-soon { color: #f0b48c; }
    .tile-cap-status.status-soon::before { background: currentColor; }

    .tile-cap-year { font-variant-numeric: tabular-nums; }

    .tile-cap-roman {
        margin-left: auto;
        font-family: 'Cormorant Garamond', Georgia, serif;
        font-style: italic;
        font-size: 14px;
        font-weight: 400;
        letter-spacing: 0.01em;
        text-transform: none;
        color: rgba(255,247,234,0.85);
    }

    .tile-cap-soon {
        font-family: 'Cormorant Garamond', Georgia, serif;
        font-size: 13px;
        font-weight: 600;
        letter-spacing: 0.02em;
        text-transform: none;
        color: #f0b48c;
    }

    .tile-cap-name {
        display: flex;
        align-items: baseline;
        gap: 10px;
        margin: 0;
        min-width: 0;
        font-family: 'Cormorant Garamond', Georgia, serif;
        font-size: clamp(21px, 1.5vw, 29px);
        font-weight: 400;
        line-height: 0.98;
        color: #fdf5e8;
        text-shadow: 0 1px 10px rgba(12,7,4,0.5);
    }

    /* A flat 21:9 feature plate has far less height to spare than the old
       3:2 crop — the plaque must stay compact (one line, smaller type) or it
       eats half the photo instead of just sitting on its foot. */
    .tile-feature .tile-glass { padding: 9px 13px 8px; }
    .tile-feature .tile-cap-name { font-size: clamp(16px, 1.3vw, 21px); }
    .tile-feature .tile-cap-name-text {
        line-clamp: 1;
        -webkit-line-clamp: 1;
    }

    .tile-cap-name-text {
        display: -webkit-box;
        line-clamp: 2;
        -webkit-line-clamp: 2;
        -webkit-box-orient: vertical;
        overflow: hidden;
    }

    .tile-cap-arrow {
        flex-shrink: 0;
        align-self: center;
        color: rgba(255,247,234,0.82);
        opacity: 0;
        transform: translateX(-4px);
        transition: opacity 0.28s ease, transform 0.28s ease;
    }

    .tile:hover .tile-cap-arrow,
    .tile:focus-within .tile-cap-arrow {
        opacity: 1;
        transform: none;
    }

    /* ── BADGES (top-left) ───────────────────────────────────────────────── */
    .tile-badges {
        position: absolute;
        left: 8px;
        top: 8px;
        z-index: 4;
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        gap: 6px;
        max-width: calc(100% - 74px);
    }

    .tile-selected {
        max-width: 100%;
        padding: 6px 9px;
        border: 1px solid rgba(255,249,240,0.4);
        background: rgba(28,18,12,0.72);
        color: #fff7ea;
        font-family: 'Instrument Sans', system-ui, sans-serif;
        font-size: 8px;
        font-weight: 600;
        letter-spacing: 0.14em;
        line-height: 1;
        text-transform: uppercase;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .tile-seal {
        padding: 5px 9px;
        border-radius: 4px;
        background: linear-gradient(150deg, rgba(198,95,60,0.94), rgba(111,59,36,0.94));
        color: #fff7ea;
        font-family: 'Instrument Sans', system-ui, sans-serif;
        font-size: 8px;
        font-weight: 700;
        letter-spacing: 0.16em;
        line-height: 1;
        text-transform: uppercase;
        transform: rotate(-2deg);
    }

    .tile-favorite {
        display: inline-flex;
        align-items: center;
        gap: 5px;
        padding: 5px 9px 5px 7px;
        border-radius: 4px;
        background: rgba(52,37,28,0.94);
        color: #f6e6c8;
        font-family: 'Instrument Sans', system-ui, sans-serif;
        font-size: 8px;
        font-weight: 700;
        letter-spacing: 0.14em;
        line-height: 1;
        text-transform: uppercase;
    }

    /* ── QUICK ACTIONS (top-right) ───────────────────────────────────────── */
    .tile-quick {
        position: absolute;
        right: 8px;
        top: 8px;
        z-index: 4;
        display: flex;
        flex-direction: column;
        gap: 6px;
        opacity: 0;
        transform: translateY(-4px);
        transition: opacity 0.26s ease, transform 0.26s ease;
    }

    .tile:hover .tile-quick,
    .tile:focus-within .tile-quick {
        opacity: 1;
        transform: none;
    }

    .tile-qbtn {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        width: 30px;
        height: 30px;
        padding: 0;
        border: 1px solid rgba(255,247,234,0.22);
        border-radius: 999px;
        /* matches the badge alpha (.tile-favorite/.tile-selected) — 0.6 could
           wash out the glyph against a bright photo */
        background: rgba(24,15,10,0.74);
        color: rgba(255,247,234,0.9);
        cursor: pointer;
        backdrop-filter: blur(3px);
        -webkit-backdrop-filter: blur(3px);
        transition: color 0.2s, background 0.2s, border-color 0.2s, transform 0.14s;
    }

    .tile-qbtn:hover {
        background: rgba(24,15,10,0.82);
        border-color: rgba(198,95,60,0.7);
        color: #f6d9c4;
    }

    .tile-qbtn:active { transform: scale(0.92); }

    .tile-qbtn.is-active {
        color: var(--copper, #c65f3c);
        border-color: rgba(198,95,60,0.7);
    }

    .tile-qbtn.is-active svg path[fill] { fill: currentColor; }

    .tile-qbtn.just-saved {
        animation: heart-pop 0.62s cubic-bezier(0.34,1.56,0.64,1);
    }

    .tile-qbtn-commission:hover {
        color: #f0b48c;
    }

    /* Touch (no hover): keep the actions and arrow visible — there is no pointer
       to reveal them. */
    @media (hover: none) {
        .tile-quick { opacity: 1; transform: none; }
        .tile-cap-arrow { opacity: 1; transform: none; }
    }

    .tile-media:focus-visible,
    .tile-qbtn:focus-visible {
        outline: 2px solid rgba(255,247,234,0.85);
        outline-offset: -3px;
    }

    @keyframes tile-in {
        from { opacity: 0; transform: translateY(10px); }
        to { opacity: 1; transform: none; }
    }

    @keyframes heart-pop {
        0% { transform: scale(1); }
        35% { transform: scale(1.32); }
        60% { transform: scale(0.92); }
        100% { transform: scale(1); }
    }

    /* ══════════════════════════════════════════════════════════════════════
       THE COLLECTION — six "rise" variants, temporary A/B toolbar. All six
       riff on the winning idea: a card climbing up into place as it enters.
       Every rule is scoped by the ancestor's data-card-fx attribute (set on
       .work-grid in +page.svelte), so only one variant's CSS is ever live at
       a time. All six trigger off .fx-revealed, toggled by a plain live
       IntersectionObserver (see revealOnEnter above) as each card crosses
       into view — not `animation-timeline: view()`, which turned out to be
       invisible whenever a card was already on screen at the moment the
       variant was picked.
       ══════════════════════════════════════════════════════════════════════ */

    /* ── 1. Rise — climbs up and settles with a slight overshoot ─────────── */
    :global(.work-grid[data-card-fx="rise"]) .tile {
        animation: none;
        opacity: 0;
        transform: translateY(52px) rotate(-2.2deg) scale(0.95);
        transition: transform 0.8s cubic-bezier(0.22, 0.9, 0.3, 1.28), opacity 0.55s ease;
    }
    :global(.work-grid[data-card-fx="rise"]) .tile.fx-revealed {
        opacity: 1;
        transform: none;
    }

    /* ── 2. Fog — rises while surfacing out of a blur, like mist clearing ── */
    :global(.work-grid[data-card-fx="fog"]) .tile {
        animation: none;
        opacity: 0;
        transform: translateY(40px) scale(0.97);
        filter: blur(9px);
        transition: transform 0.9s cubic-bezier(0.16,1,0.3,1), opacity 0.7s ease, filter 0.9s ease;
    }
    :global(.work-grid[data-card-fx="fog"]) .tile.fx-revealed {
        opacity: 1;
        transform: none;
        filter: blur(0px);
    }

    /* ── 3. Hoist — a mechanical, rope-and-pulley wobble rather than a
       smooth ease: overshoots up, dips back, then settles. ──────────────── */
    :global(.work-grid[data-card-fx="hoist"]) .tile {
        opacity: 0;
        transform: translateY(70px) rotate(-4deg);
    }
    :global(.work-grid[data-card-fx="hoist"]) .tile.fx-revealed {
        animation: fx-hoist 0.9s cubic-bezier(0.34, 1.56, 0.64, 1) forwards;
    }
    @keyframes fx-hoist {
        0%   { opacity: 0; transform: translateY(70px) rotate(-4deg); }
        55%  { opacity: 1; transform: translateY(-10px) rotate(1.5deg); }
        75%  { transform: translateY(4px) rotate(-0.5deg); }
        100% { opacity: 1; transform: translateY(0) rotate(0deg); }
    }

    /* ── 4. Drift — surfaces on a diagonal, alternating left/right by column
       so the whole wall feels like it's floating up unevenly. ───────────── */
    :global(.work-grid[data-card-fx="drift"]) .tile {
        animation: none;
        opacity: 0;
        transform: translate(-34px, 46px) rotate(-3deg);
        transition: transform 0.85s cubic-bezier(0.16,1,0.3,1), opacity 0.6s ease;
    }
    :global(.work-grid[data-card-fx="drift"]) .tile:nth-child(even) {
        transform: translate(34px, 46px) rotate(3deg);
    }
    :global(.work-grid[data-card-fx="drift"]) .tile.fx-revealed {
        opacity: 1;
        transform: none;
    }

    /* ── 5. Unfold — a blind lifting from the bottom edge, combined with the
       same climb, rather than a plain fade. ──────────────────────────────── */
    :global(.work-grid[data-card-fx="unfold"]) .tile {
        animation: none;
        opacity: 1;
        transform: translateY(18px);
        clip-path: inset(100% 0 0 0);
        transition: transform 0.75s cubic-bezier(0.16,1,0.3,1), clip-path 0.75s cubic-bezier(0.16,1,0.3,1);
    }
    :global(.work-grid[data-card-fx="unfold"]) .tile.fx-revealed {
        transform: none;
        clip-path: inset(0 0 0 0);
    }

    /* ── 6. Shadow — climbs up while surfacing out of near-darkness ──────── */
    :global(.work-grid[data-card-fx="shadow"]) .tile {
        animation: none;
        opacity: 0;
        transform: translateY(38px) scale(0.96);
        filter: brightness(0.25) saturate(0.4);
        transition: transform 0.85s cubic-bezier(0.16,1,0.3,1), opacity 0.6s ease, filter 0.9s ease;
    }
    :global(.work-grid[data-card-fx="shadow"]) .tile.fx-revealed {
        opacity: 1;
        transform: none;
        filter: none;
    }

    @media (prefers-reduced-motion: reduce) {
        .tile,
        .tile-qbtn,
        .tile-cap-arrow,
        .tile-media :global(.tile-img .app-image-main) {
            animation: none !important;
            transition: opacity 0.2s ease, color 0.2s ease, background 0.2s ease, border-color 0.2s ease !important;
        }
        .tile:hover .tile-media :global(.tile-img .app-image-main) { transform: none; }
        .tile:hover, .tile:focus-within { transform: none; box-shadow: none; }
        /* an alternating photo is itself motion — withhold it under reduced-motion */
        .tile-media :global(.tile-img-alt) { display: none !important; }

        /* Every card-fx variant collapses to a plain, static, fully-visible plate. */
        :global(.work-grid[data-card-fx]) .tile,
        :global(.work-grid[data-card-fx]) .tile-media-wrap::after,
        :global(.work-grid[data-card-fx]) .tile-media :global(.tile-img .app-image-main) {
            animation: none !important;
            transition: none !important;
            filter: none !important;
            clip-path: none !important;
            transform: none !important;
            opacity: 1 !important;
            box-shadow: none !important;
        }
    }
</style>
