<script lang="ts">
    import { onMount } from 'svelte';
    import { goto } from '$app/navigation';
    import type { FigurineListItem } from '$lib/types/api';
    import { t } from '$lib/i18n';
    import AppImage from '$lib/components/AppImage.svelte';
    import KeyholeVeil from '$lib/components/KeyholeVeil.svelte';
    import SealedDoor from '$lib/components/SealedDoor.svelte';
    import OrderModal from '$lib/components/OrderModal.svelte';
    import { savedFigurines } from '$lib/stores/saved-figurines.svelte';
    import { revealedFigurines } from '$lib/stores/revealed-figurines.svelte';
    import { themeConfig } from '$lib/stores/theme.svelte';
    import { dwellReveal } from '$lib/actions/dwell-reveal';
    import { houseClock } from '$lib/stores/house-clock.svelte';
    import { showingRooms } from '$lib/stores/showing-rooms.svelte';
    import { isGated, isShowingOpen, resolveWindow } from '$lib/showing-window';

    let {
        fig,
        index = 0,
        compact = false,
        selected = false,
    }: {
        fig: FigurineListItem;
        index?: number;
        compact?: boolean;
        selected?: boolean;
    } = $props();

    let saved = $derived(savedFigurines.has(fig.id));
    let showOrder = $state(false);
    let copied = $state(false);
    let justSaved = $state(false);
    let copyTimer: ReturnType<typeof setTimeout> | undefined;
    let pulseTimer: ReturnType<typeof setTimeout> | undefined;

    let href = $derived(`/figurines/${fig.id}`);
    let archiveNumber = $derived(`No ${String(index + 1).padStart(3, '0')}`);

    // "The house wakes": a gated work is sealed behind a carved door while the
    // visitor's local clock sits outside its showing window. The effective window
    // is the work's own hours OR its showing room's (resolveWindow). houseClock.now
    // ticks by the minute, so a door dissolves live the moment its window opens.
    let win = $derived(resolveWindow(
        { openFromMin: fig.openFromMin, openUntilMin: fig.openUntilMin, showingRoomId: fig.showingRoomId },
        showingRooms.list
    ));
    let doorClosed = $derived(isGated(win) && !isShowingOpen(win, houseClock.nowDate));

    // "Keyhole" seal: a piece stays in shadow (only its lit fragment shown) until
    // the visitor steps into its file. The reveal is forgetful — only the last
    // few opened works stay unsealed (gotiga_revealed); open more and the older
    // ones settle back into shadow, so the archive never gives itself away.
    // Loaded synchronously at init (SPA — localStorage is available in the
    // browser) so an already-revealed card never flashes dark before unsealing.
    revealedFigurines.load();
    // The seal lifts only while a work is in the rolling "recently opened" window
    // (gotiga_revealed) — not forever. See revealed-figurines.svelte.ts.
    let sealed = $derived(!revealedFigurines.has(fig.id) && !!fig.faceImageUrl);

    // Dwell-to-reveal: a sustained look (hover) thins the shadow over the globally
    // configured number of seconds, settling on a "half-lit" glance — never fully
    // clear (only opening the work does that). Armed only while still sealed and
    // not already glanced.
    let dwellSec = $derived(Number($themeConfig.effects?.keyholeDwellReveal ?? 0));
    let glanced = $state(false);   // a completed look — hold the shadow half-lit
    let dwelling = $state(false);  // a look in progress — thinning toward half-lit
    let dwellMs = $derived(sealed && !glanced && dwellSec > 0 ? dwellSec * 1000 : 0);

    // Recently catalogued — surfaced as a quiet wax mark, not a sales badge.
    let isNew = $derived.by(() => {
        if (!fig.createdAt) return false;
        const ts = new Date(fig.createdAt).getTime();
        if (Number.isNaN(ts)) return false;
        return Date.now() - ts < 21 * 24 * 60 * 60 * 1000;
    });

    let primaryFact = $derived(fig.material || fig.technique || $t('homeTrustHandmade'));
    let specimenMeta = $derived(
        fig.status === 'available'
            ? `${primaryFact} · ${$t('homeCardTransferByRequest')}`
            : primaryFact
    );

    let statusLabel = $derived(
        fig.status === 'available'
            ? $t('archiveStatusAvailableLabel')
            : fig.status === 'reserved'
                ? $t('archiveStatusReservedLabel')
                : fig.status === 'in_progress'
                    ? $t('profileWishInProgress')
                    : $t('archiveStatusSoldLabel')
    );

    // The footer action adapts to where the work actually is in its life.
    let action = $derived.by(() => {
        if (fig.status === 'available') {
            return { kind: 'request' as const, label: $t('homeCardRequestThisWork') };
        }
        if (fig.status === 'in_progress') {
            return { kind: 'link' as const, label: $t('homeViewUpcoming'), to: '/upcoming' };
        }
        return { kind: 'link' as const, label: $t('homeCardOpenFile'), to: href };
    });

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
        savedFigurines.toggle(fig.id);
        if (savedFigurines.has(fig.id)) {
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

    function openOrder(e: MouseEvent) {
        e.preventDefault();
        e.stopPropagation();
        showOrder = true;
    }

    function openSimilarCommission(e: MouseEvent) {
        e.preventDefault();
        e.stopPropagation();
        goto(`/commission?source=${encodeURIComponent(fig.id)}`);
    }
</script>

<article
    class="tile"
    class:tile-compact={compact}
    class:is-selected={selected}
    style="--i:{index}"
>
    <div class="tile-archive-bar">
        <span class="tile-index">{archiveNumber}</span>
        <span class="tile-status">
            <i class="tile-dot status-{fig.status}"></i>
            {statusLabel}
        </span>
        {#if fig.year}
            <span class="tile-year">{fig.year}</span>
        {/if}
    </div>

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
                    doorImageUrl={fig.sealedDoorImage}
                    name={fig.name}
                />
                <span class="corner corner-tl"></span>
                <span class="corner corner-tr"></span>
                <span class="corner corner-bl"></span>
                <span class="corner corner-br"></span>
            </div>
        {:else}
        <a
            {href}
            class="tile-media"
            aria-label="{$t('homeViewFigurine')}: {fig.name}"
            use:dwellReveal={{ ms: dwellMs, onStart: () => dwelling = true, onStop: () => dwelling = false, onReveal: () => { glanced = true; dwelling = false; } }}
        >
            {#if fig.faceImageUrl}
                <AppImage src={fig.faceImageUrl} thumbUrl={fig.thumbUrl} alt={fig.name} class="tile-img" loading="lazy" />
                <KeyholeVeil show={sealed} {dwelling} partial={glanced} {dwellMs} focalX={fig.focalX} focalY={fig.focalY} revealRadius={fig.revealRadius} darkness={fig.darkness} />
            {:else}
                <div class="tile-placeholder">?</div>
            {/if}
            <span class="corner corner-tl"></span>
            <span class="corner corner-tr"></span>
            <span class="corner corner-bl"></span>
            <span class="corner corner-br"></span>
        </a>
        {/if}

        {#if isNew}
            <span class="tile-seal" title={$t('archiveCardNew')}>{$t('archiveCardNew')}</span>
        {/if}

        {#if selected}
            <span class="tile-selected">{$t('homeHeroObjectLabel')}</span>
        {/if}

        <div class="tile-tools">
            <button
                class="tile-tool tile-save"
                class:is-saved={saved}
                class:just-saved={justSaved}
                onclick={toggleSaved}
                aria-pressed={saved}
                aria-label={saved ? $t('cardSaved') : $t('cardSave')}
                title={saved ? $t('cardSaved') : $t('cardSave')}
            >
                <svg width="15" height="15" viewBox="0 0 14 14" fill="none" aria-hidden="true">
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
                class="tile-tool tile-share"
                class:is-copied={copied}
                onclick={shareWork}
                aria-label={$t('cardShare')}
                title={$t('cardShare')}
            >
                {#if copied}
                    <svg width="15" height="15" viewBox="0 0 14 14" fill="none" aria-hidden="true">
                        <path d="M2.5 7.5L5.5 10.5L11.5 3.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
                    </svg>
                {:else}
                    <svg width="15" height="15" viewBox="0 0 14 14" fill="none" aria-hidden="true">
                        <circle cx="3" cy="7" r="1.6" stroke="currentColor" stroke-width="1.1"/>
                        <circle cx="11" cy="3" r="1.6" stroke="currentColor" stroke-width="1.1"/>
                        <circle cx="11" cy="11" r="1.6" stroke="currentColor" stroke-width="1.1"/>
                        <path d="M4.4 6.2L9.6 3.6M4.4 7.8L9.6 10.4" stroke="currentColor" stroke-width="1.1"/>
                    </svg>
                {/if}
            </button>
        </div>

        <span class="tile-copied" class:show={copied} role="status" aria-live="polite">
            {copied ? $t('cardLinkCopied') : ''}
        </span>
    </div>

    <div class="tile-body">
        <div class="tile-head">
            <h3>
                {#if doorClosed}
                    {fig.name}
                {:else}
                    <a {href}>{fig.name}</a>
                {/if}
            </h3>
        </div>

        <p class="tile-meta">{specimenMeta}</p>

        <div class="tile-actions">
            <span class="tile-file-hint">{archiveNumber}</span>
            {#if doorClosed}
                <span class="tile-door-hint">{$t('doorSealedHint')}</span>
            {:else if action.kind === 'request'}
                <button class="tile-cta tile-cta-similar" type="button" onclick={openSimilarCommission} title={$t('commissionCreateSimilarCta')} aria-label={$t('commissionCreateSimilarCta')}>
                    <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                        <!-- scroll body -->
                        <path d="M5 2h8a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H5"/>
                        <!-- left rolled edge: three bumps -->
                        <path d="M5 2a2 2 0 0 0 0 4"/>
                        <path d="M5 6a2 2 0 0 1 0 4"/>
                        <path d="M5 10a2 2 0 0 0 0 4"/>
                        <!-- text lines -->
                        <line x1="8" y1="5.5" x2="12" y2="5.5"/>
                        <line x1="8" y1="8" x2="12" y2="8"/>
                        <line x1="8" y1="10.5" x2="10.5" y2="10.5"/>
                    </svg>
                </button>
            {:else}
                <a class="tile-cta tile-cta-ghost" href={action.to}>
                    {action.label}
                    <svg width="14" height="7" viewBox="0 0 14 7" fill="none" aria-hidden="true">
                        <path d="M0 3.5H13M13 3.5L9.5 1M13 3.5L9.5 6" stroke="currentColor" stroke-width="1"/>
                    </svg>
                </a>
            {/if}
        </div>
    </div>
</article>

{#if showOrder}
    <OrderModal
        isOpen={showOrder}
        figurineName={fig.name}
        figurineId={fig.id}
        mode="request"
        onClose={() => { showOrder = false; }}
    />
{/if}

<style>
    .tile {
        --tile-radius: 8px;
        position: relative;
        isolation: isolate;
        display: grid;
        grid-template-rows: auto auto 1fr;
        min-width: 0;
        overflow: hidden;
        color: var(--ink, #34251c);
        text-decoration: none;
        border: 1px solid rgba(52,37,28,0.11);
        border-radius: var(--tile-radius);
        background:
            linear-gradient(150deg, rgba(255,252,246,0.94), rgba(255,247,238,0.74) 54%, rgba(246,232,213,0.50)),
            radial-gradient(circle at 16% 0%, rgba(198,95,60,0.12), transparent 34%);
        box-shadow:
            0 1px 0 rgba(255,255,255,0.82) inset,
            0 22px 54px rgba(52,37,28,0.08);
        animation: tile-in 0.5s var(--ease, cubic-bezier(0.16,1,0.3,1)) both;
        animation-delay: calc(var(--i) * 0.04s);
        transition: transform 0.26s ease, border-color 0.26s ease, box-shadow 0.26s ease, background 0.26s ease;
    }

    .tile::before {
        content: '';
        position: absolute;
        inset: 0;
        z-index: -1;
        background:
            linear-gradient(90deg, rgba(198,95,60,0.42), transparent 34%, rgba(89,99,61,0.16) 72%, transparent);
        opacity: 0;
        transition: opacity 0.26s ease;
        pointer-events: none;
    }

    .tile::after {
        content: '';
        position: absolute;
        inset: 0;
        z-index: 3;
        background: linear-gradient(115deg, transparent 0 38%, rgba(255,255,255,0.30) 48%, transparent 58% 100%);
        opacity: 0;
        transform: translateX(-42%);
        transition: opacity 0.28s ease, transform 0.58s ease;
        pointer-events: none;
    }

    .tile:hover {
        transform: translateY(-5px);
        border-color: rgba(198,95,60,0.42);
        background:
            linear-gradient(150deg, rgba(255,252,246,0.98), rgba(255,247,238,0.88) 54%, rgba(246,232,213,0.58)),
            radial-gradient(circle at 16% 0%, rgba(198,95,60,0.15), transparent 36%);
        box-shadow:
            0 1px 0 rgba(255,255,255,0.86) inset,
            0 30px 70px rgba(68,37,20,0.16);
    }

    .tile:hover::before { opacity: 1; }

    .tile:hover::after {
        opacity: 1;
        transform: translateX(48%);
    }

    .tile.is-selected {
        border-color: rgba(198,95,60,0.42);
        box-shadow:
            0 1px 0 rgba(255,255,255,0.75) inset,
            0 0 0 1px rgba(198,95,60,0.12),
            0 14px 32px rgba(68,37,20,0.11);
    }

    /* keyboard focus on any inner control lifts the whole card */
    .tile:focus-within {
        border-color: rgba(198,95,60,0.42);
    }

    .tile-archive-bar {
        min-height: 44px;
        display: grid;
        grid-template-columns: auto minmax(0, 1fr) auto;
        align-items: center;
        gap: 10px;
        padding: 0 14px;
        border-bottom: 1px solid rgba(52,37,28,0.09);
        background: rgba(255,252,246,0.44);
        color: var(--color-ink-tertiary);
        font-size: 9px;
        font-weight: 600;
        letter-spacing: 0.13em;
        line-height: 1.2;
        text-transform: uppercase;
    }

    .tile-index { font-variant-numeric: tabular-nums; }
    .tile-year { font-variant-numeric: tabular-nums; }

    .tile-status {
        justify-self: center;
        display: inline-flex;
        align-items: center;
        gap: 7px;
        min-width: 0;
        padding: 6px 9px;
        border: 1px solid rgba(52,37,28,0.08);
        border-radius: 999px;
        background: rgba(255,249,240,0.58);
        color: var(--color-ink-secondary);
    }

    .tile-media-wrap {
        position: relative;
        margin: 10px 10px 0;
        aspect-ratio: 4 / 3;
        overflow: hidden;
        border: 1px solid rgba(52,37,28,0.10);
        border-radius: 7px;
        background:
            radial-gradient(circle at 50% 42%, rgba(255,249,240,0.44), transparent 58%),
            rgba(201,168,117,0.10);
        box-shadow:
            0 1px 0 rgba(255,255,255,0.58) inset,
            0 10px 24px rgba(52,37,28,0.07);
    }

    .tile-media {
        position: absolute;
        inset: 0;
        display: block;
        overflow: hidden;
        color: inherit;
        text-decoration: none;
        border-radius: inherit;
    }

    .tile-media::before {
        content: '';
        position: absolute;
        inset: 0;
        z-index: 1;
        background:
            linear-gradient(180deg, rgba(255,249,240,0.12), transparent 34%, rgba(43,27,19,0.08)),
            radial-gradient(circle at 50% 42%, transparent 0 48%, rgba(43,27,19,0.10) 100%);
        opacity: 0.62;
        transition: opacity 0.28s ease;
        pointer-events: none;
    }

    .tile-media::after {
        content: '';
        position: absolute;
        inset: 10px;
        z-index: 2;
        border: 1px solid rgba(255,249,240,0);
        border-radius: 5px;
        box-shadow: inset 0 0 0 1px rgba(52,37,28,0);
        transition: inset 0.28s ease, border-color 0.28s ease, box-shadow 0.28s ease;
        pointer-events: none;
    }

    .tile:hover .tile-media::before { opacity: 0.32; }

    .tile:hover .tile-media::after {
        inset: 8px;
        border-color: rgba(255,249,240,0.34);
        box-shadow: inset 0 0 0 1px rgba(198,95,60,0.18);
    }

    .tile-compact .tile-media { aspect-ratio: 1 / 1; }

    .tile-media :global(.tile-img),
    .tile-media :global(.tile-img .app-image-thumb),
    .tile-media :global(.tile-img .app-image-main) {
        width: 100%;
        height: 100%;
        display: block;
        object-fit: contain;
        object-position: center;
    }

    .tile-media :global(.tile-img .app-image-main) {
        filter: grayscale(0.08) saturate(0.94) contrast(1.02);
        transition: transform 0.48s ease, filter 0.48s ease;
    }

    .tile:hover .tile-media :global(.tile-img .app-image-main) {
        transform: scale(1.055);
        filter: grayscale(0) saturate(1.04) contrast(1.04);
    }

    .corner {
        position: absolute;
        z-index: 2;
        width: 18px;
        height: 18px;
        border-color: rgba(198,95,60,0.42);
        opacity: 0.78;
        transition: width 0.24s ease, height 0.24s ease, opacity 0.24s ease;
        pointer-events: none;
    }

    .corner-tl { left: 10px; top: 10px; border-left: 1px solid; border-top: 1px solid; }
    .corner-tr { right: 10px; top: 10px; border-right: 1px solid; border-top: 1px solid; }
    .corner-bl { left: 10px; bottom: 10px; border-left: 1px solid; border-bottom: 1px solid; }
    .corner-br { right: 10px; bottom: 10px; border-right: 1px solid; border-bottom: 1px solid; }

    .tile:hover .corner {
        width: 24px;
        height: 24px;
        opacity: 1;
    }

    .tile-placeholder {
        width: 100%;
        height: 100%;
        display: grid;
        place-items: center;
        font-family: 'Cormorant Garamond', Georgia, serif;
        font-size: 30px;
        color: var(--color-ink-tertiary);
    }

    /* wax-stamp "new" mark, top-left */
    .tile-seal {
        position: absolute;
        left: 10px;
        top: 10px;
        z-index: 3;
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
        box-shadow: 0 4px 12px rgba(111,59,36,0.34);
        transform: rotate(-2deg);
    }

    /* action cluster, top-right */
    .tile-tools {
        position: absolute;
        right: 10px;
        top: 10px;
        z-index: 4;
        display: flex;
        flex-direction: column;
        gap: 8px;
    }

    /* tools sit quietly present (not popping in) — calm, discoverable, not shop-like */
    .tile-tool {
        width: 34px;
        height: 34px;
        display: grid;
        place-items: center;
        border: 1px solid rgba(52,37,28,0.16);
        border-radius: 999px;
        background: rgba(255,249,240,0.92);
        color: var(--color-ink-tertiary);
        cursor: pointer;
        opacity: 0.82;
        transition: color 0.2s, background 0.2s, border-color 0.2s, transform 0.2s, opacity 0.2s;
    }

    .tile:hover .tile-tool,
    .tile:focus-within .tile-tool,
    .tile-tool.is-saved {
        opacity: 1;
    }

    .tile-save:hover,
    .tile-save.is-saved {
        color: var(--copper, #c65f3c);
        border-color: rgba(198,95,60,0.34);
        background: rgba(255,246,239,0.92);
    }

    .tile-save.just-saved {
        animation: heart-pop 0.62s cubic-bezier(0.34,1.56,0.64,1);
    }

    .tile-share:hover {
        color: var(--copper, #c65f3c);
        border-color: rgba(198,95,60,0.34);
        background: rgba(255,246,239,0.92);
        transform: translateY(-1px);
    }

    .tile-share.is-copied {
        color: #2f7d4a;
        border-color: rgba(47,125,74,0.4);
        background: rgba(238,248,240,0.95);
        opacity: 1;
        transform: none;
    }

    .tile-copied {
        position: absolute;
        right: 52px;
        top: 18px;
        z-index: 4;
        padding: 5px 9px;
        border-radius: 999px;
        background: rgba(28,18,12,0.86);
        color: #fff7ea;
        font-family: 'Instrument Sans', system-ui, sans-serif;
        font-size: 8px;
        font-weight: 600;
        letter-spacing: 0.12em;
        line-height: 1;
        text-transform: uppercase;
        white-space: nowrap;
        opacity: 0;
        transform: translateX(6px);
        pointer-events: none;
        transition: opacity 0.2s ease, transform 0.2s ease;
    }

    .tile-copied.show {
        opacity: 1;
        transform: none;
    }

    .tile-selected {
        position: absolute;
        left: 10px;
        bottom: 10px;
        z-index: 3;
        max-width: calc(100% - 20px);
        padding: 7px 9px;
        background: rgba(43,27,19,0.88);
        color: #fff7ea;
        font-size: 8px;
        font-weight: 600;
        letter-spacing: 0.14em;
        line-height: 1;
        text-transform: uppercase;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .tile-body {
        display: grid;
        gap: 7px;
        padding: 16px 16px 15px;
        min-width: 0;
    }

    .tile-head {
        display: block;
        min-width: 0;
    }

    .tile-head h3 {
        margin: 0;
        min-width: 0;
        font-family: 'Cormorant Garamond', Georgia, serif;
        font-size: clamp(23px, 1.7vw, 31px);
        font-weight: 400;
        line-height: 0.96;
        color: inherit;
        display: -webkit-box;
        line-clamp: 2;
        -webkit-line-clamp: 2;
        -webkit-box-orient: vertical;
        overflow: hidden;
    }

    .tile-head h3 a {
        color: inherit;
        text-decoration: none;
    }

    .tile-head h3 a:hover { color: var(--copper, #c65f3c); }

    .tile-meta {
        display: block;
        margin: 0;
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        color: var(--color-ink-secondary);
        font-family: 'Cormorant Garamond', Georgia, serif;
        font-size: 16px;
        font-style: italic;
        line-height: 1.25;
    }

    .tile-actions {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 10px;
        margin-top: 7px;
        padding-top: 13px;
        border-top: 1px solid rgba(52,37,28,0.09);
    }

    .tile-file-hint,
    .tile-cta {
        min-height: 30px;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        gap: 8px;
        font-family: 'Instrument Sans', system-ui, sans-serif;
        font-size: 10px;
        font-weight: 600;
        letter-spacing: 0.1em;
        line-height: 1;
        text-transform: uppercase;
        text-decoration: none;
    }

    .tile-file-hint {
        color: var(--color-ink-tertiary);
        font-variant-numeric: tabular-nums;
    }

    /* quiet closed-door note in place of a CTA — never a countdown */
    .tile-door-hint {
        min-height: 30px;
        display: inline-flex;
        align-items: center;
        font-family: 'Cormorant Garamond', Georgia, serif;
        font-size: 14px;
        font-style: italic;
        letter-spacing: 0.02em;
        color: var(--color-ink-tertiary);
        text-transform: none;
    }

    .tile-cta {
        flex-shrink: 0;
        min-height: 34px;
        padding: 0 13px;
        border-radius: 999px;
        cursor: pointer;
        transition: color 0.2s, gap 0.2s, background 0.2s, border-color 0.2s, transform 0.2s;
    }

    .tile-cta svg { transition: transform 0.2s ease; }
    .tile-cta-ghost:hover svg { transform: translateX(2px); }

    .tile-cta-similar {
        --glow: 0;
        width: 34px;
        padding: 0;
        position: relative;
        isolation: isolate;
        border-radius: 4px;
        /* oklch: perceptual color space — hue stays true as lightness shifts */
        border: 1.5px solid oklch(38% 0.06 42 / 0.32);
        background:
            radial-gradient(ellipse 110% 55% at 50% 0%,
                oklch(100% 0 0 / 0.24) 0%,
                transparent 100%),
            oklch(94% 0.02 78 / 0.82);
        color: oklch(30% 0.055 42);
        /* floats above the card grain texture */
        box-shadow:
            inset 0 1px 0 oklch(100% 0 0 / 0.42),
            inset 0 -1px 0 oklch(30% 0.055 42 / 0.1);
        transition:
            border-color 0.28s,
            color 0.28s,
            background 0.28s,
            box-shadow 0.28s,
            /* @property enables smooth number interpolation */
            --glow 0.38s,
            /* linear() spring — physical bounce without JS */
            transform 0.55s linear(
                0, 0.43 4.7%, 0.74 9.4%, 0.86 12.1%,
                0.96 14.9%, 1.01 17.7%, 1.04 20.6%,
                1.05 23.5%, 1.04 26.5%, 1.01 30.6%,
                0.98 35.3%, 0.99 42.3%, 1 53%
            );
    }

    /* ember halo: only renderable because --glow is a typed <number> */
    .tile-cta-similar::after {
        content: '';
        position: absolute;
        inset: -5px;
        border-radius: 8px;
        background: radial-gradient(ellipse at 50% 60%,
            oklch(68% 0.23 48 / calc(var(--glow) * 0.5)),
            transparent 68%);
        z-index: -1;
        pointer-events: none;
    }

    .tile-cta-similar:hover {
        --glow: 1;
        border-color: oklch(58% 0.24 50 / 0.8);
        color: oklch(36% 0.22 42);
        background:
            radial-gradient(ellipse 110% 55% at 50% 0%,
                oklch(100% 0 0 / 0.14) 0%,
                transparent 100%),
            oklch(87% 0.1 55 / 0.38);
        box-shadow:
            inset 0 1px 0 oklch(100% 0 0 / 0.26),
            inset 0 -1px 0 oklch(36% 0.22 42 / 0.14),
            0 0 0 2px oklch(65% 0.23 48 / 0.22);
        transform: scale(1.12);
    }

    /* CSS entry animation — no JS, no library */
    @starting-style {
        .tile-cta-similar {
            opacity: 0;
            transform: scale(0.72);
        }
    }

    .tile-cta-ghost {
        border: 1px solid rgba(52,37,28,0.16);
        background: rgba(255,252,246,0.5);
        color: var(--color-ink-secondary);
    }

    .tile-cta-ghost:hover {
        transform: translateY(-1px);
        border-color: rgba(52,37,28,0.3);
        color: var(--color-ink-primary);
        background: rgba(255,252,246,0.85);
    }

    .tile-media:focus-visible,
    .tile-head h3 a:focus-visible,
    .tile-tool:focus-visible,
    .tile-cta:focus-visible {
        outline: 2px solid rgba(198,95,60,0.52);
        outline-offset: 3px;
    }

    .tile-dot {
        width: 5px;
        height: 5px;
        border-radius: 999px;
        background: rgba(95,70,54,0.42);
        flex-shrink: 0;
    }

    .tile-dot.status-available { background: rgba(30,135,75,0.72); }
    .tile-dot.status-reserved { background: rgba(175,120,20,0.74); }
    .tile-dot.status-in_progress { background: rgba(198,95,60,0.72); }

    @media (max-width: 680px) {
        /* on touch, tools and CTA should always be reachable */
        .tile-tool { opacity: 1; transform: none; backdrop-filter: none; }

        .tile-actions {
            align-items: stretch;
            flex-direction: column;
        }

        .tile-cta { width: 100%; }
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

    @media (prefers-reduced-motion: reduce) {
        .tile,
        .tile::after,
        .tile-tool,
        .tile-cta,
        .tile-cta svg,
        .tile-media :global(.tile-img .app-image-main) {
            animation: none !important;
            transition: opacity 0.2s ease, color 0.2s ease, background 0.2s ease, border-color 0.2s ease !important;
        }
        .tile:hover { transform: none; }
        .tile:hover .tile-media :global(.tile-img .app-image-main) { transform: none; }
    }
</style>
