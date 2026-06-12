<script lang="ts">
    import { onMount } from 'svelte';
    import type { FigurineListItem } from '$lib/types/api';
    import { t } from '$lib/i18n';
    import AppImage from '$lib/components/AppImage.svelte';
    import OrderModal from '$lib/components/OrderModal.svelte';
    import { savedFigurines } from '$lib/stores/saved-figurines.svelte';

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
    let archiveNumber = $derived(`No ${String(index + 1).padStart(3, '0')}`);
    let primaryFact = $derived(fig.material || fig.technique || fig.series || $t('homeTrustHandmade'));
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

    onMount(() => {
        savedFigurines.load();
    });

    function toggleSaved(e: MouseEvent) {
        e.preventDefault();
        e.stopPropagation();
        savedFigurines.toggle(fig.id);
    }

    function openOrder(e: MouseEvent) {
        e.preventDefault();
        e.stopPropagation();
        showOrder = true;
    }
</script>

    <article
        class="tile"
        class:tile-compact={compact}
        class:is-selected={selected}
        style="--i:{index}"
    >
    <div class="tile-archive-bar">
        <span>{archiveNumber}</span>
        <span class="tile-status">
            <i class="tile-dot status-{fig.status}"></i>
            {statusLabel}
        </span>
        {#if fig.year}
            <span>{fig.year}</span>
        {/if}
    </div>

    <div class="tile-media-wrap">
        <a
            href={`/figurines/${fig.id}`}
            class="tile-media"
            aria-label="{$t('homeViewFigurine')}: {fig.name}"
        >
            {#if fig.faceImageUrl}
                <AppImage src={fig.faceImageUrl} thumbUrl={fig.thumbUrl} alt={fig.name} class="tile-img" loading="lazy" />
            {:else}
                <div class="tile-placeholder">?</div>
            {/if}
            <span class="corner corner-tl"></span>
            <span class="corner corner-tr"></span>
            <span class="corner corner-bl"></span>
            <span class="corner corner-br"></span>
        </a>

        {#if selected}
            <span class="tile-selected">{$t('homeHeroObjectLabel')}</span>
        {/if}

        <button
            class="tile-save"
            class:is-saved={saved}
            onclick={toggleSaved}
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
    </div>

    <div class="tile-body">
        <div class="tile-head">
            <h3>
                <a href={`/figurines/${fig.id}`}>{fig.name}</a>
            </h3>
        </div>

        <p class="tile-meta">{specimenMeta}</p>

        <div class="tile-actions" class:single-action={fig.status !== 'available'}>
            <span class="tile-file-hint">{archiveNumber}</span>
            {#if fig.status === 'available'}
                <button class="tile-request" type="button" onclick={openOrder}>
                    {$t('homeCardRequestThisWork')}
                    <svg width="14" height="7" viewBox="0 0 14 7" fill="none" aria-hidden="true">
                        <path d="M0 3.5H13M13 3.5L9.5 1M13 3.5L9.5 6" stroke="currentColor" stroke-width="1"/>
                    </svg>
                </button>
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

    .tile:hover::before {
        opacity: 1;
    }

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
        overflow: hidden;
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

    .tile:hover .tile-media::before {
        opacity: 0.32;
    }

    .tile:hover .tile-media::after {
        inset: 8px;
        border-color: rgba(255,249,240,0.34);
        box-shadow: inset 0 0 0 1px rgba(198,95,60,0.18);
    }

    .tile-media {
        display: block;
        color: inherit;
        text-decoration: none;
    }

    .tile-compact .tile-media {
        aspect-ratio: 1 / 1;
    }

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

    .corner-tl {
        left: 10px;
        top: 10px;
        border-left: 1px solid;
        border-top: 1px solid;
    }

    .corner-tr {
        right: 10px;
        top: 10px;
        border-right: 1px solid;
        border-top: 1px solid;
    }

    .corner-bl {
        left: 10px;
        bottom: 10px;
        border-left: 1px solid;
        border-bottom: 1px solid;
    }

    .corner-br {
        right: 10px;
        bottom: 10px;
        border-right: 1px solid;
        border-bottom: 1px solid;
    }

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

    .tile-save {
        position: absolute;
        right: 10px;
        top: 10px;
        z-index: 2;
        width: 34px;
        height: 34px;
        display: grid;
        place-items: center;
        border: 1px solid rgba(52,37,28,0.16);
        border-radius: 999px;
        background: rgba(255,249,240,0.70);
        color: var(--color-ink-tertiary);
        backdrop-filter: blur(8px);
        cursor: pointer;
        transition: color 0.2s, background 0.2s, border-color 0.2s, transform 0.2s;
    }

    .tile-selected {
        position: absolute;
        left: 10px;
        bottom: 10px;
        z-index: 2;
        max-width: calc(100% - 20px);
        padding: 7px 9px;
        background: rgba(43,27,19,0.78);
        color: #fff7ea;
        font-size: 8px;
        font-weight: 600;
        letter-spacing: 0.14em;
        line-height: 1;
        text-transform: uppercase;
        backdrop-filter: blur(8px);
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .tile-save:hover,
    .tile-save.is-saved {
        color: var(--copper, #c65f3c);
        border-color: rgba(198,95,60,0.34);
        background: rgba(255,246,239,0.92);
    }

    .tile-save:hover {
        transform: translateY(-1px);
    }

    .tile-body {
        display: grid;
        gap: 9px;
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

    .tile-head h3 a:hover {
        color: var(--copper, #c65f3c);
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

    .tile-actions.single-action {
        justify-content: flex-start;
    }

    .tile-file-hint,
    .tile-request {
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
    }

    .tile-request {
        flex-shrink: 0;
        min-height: 34px;
        padding: 0 11px;
        border: 1px solid rgba(198,95,60,0.24);
        border-radius: 999px;
        background: rgba(255,246,239,0.66);
        color: var(--copper, #c65f3c);
        cursor: pointer;
        transition: color 0.2s, gap 0.2s, background 0.2s, border-color 0.2s, transform 0.2s;
    }

    .tile-request:hover {
        gap: 12px;
        transform: translateY(-1px);
        border-color: rgba(198,95,60,0.48);
        background: rgba(255,246,239,0.94);
        color: var(--color-ink-primary);
    }

    .tile-media:focus-visible,
    .tile-head h3 a:focus-visible,
    .tile-save:focus-visible,
    .tile-request:focus-visible {
        outline: 2px solid rgba(198,95,60,0.52);
        outline-offset: 3px;
    }

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
        .tile-actions {
            align-items: stretch;
            flex-direction: column;
        }

        .tile-request {
            width: 100%;
        }
    }

    @keyframes tile-in {
        from { opacity: 0; transform: translateY(10px); }
        to { opacity: 1; transform: none; }
    }
</style>
