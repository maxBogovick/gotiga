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
    let hasFacts = $derived(Boolean(fig.series || fig.material || fig.technique || fig.status === 'available'));
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
            {#if fig.year}
                <span class="tile-year">{fig.year}</span>
            {/if}
        </div>

        <p class="tile-meta">
            <span class="tile-dot status-{fig.status}"></span>
            {statusLabel}
        </p>

        {#if hasFacts}
            <dl class="tile-facts" aria-label="{fig.name} details">
                {#if fig.series}
                    <div>
                        <dt>{$t('archiveSeriesLabel')}</dt>
                        <dd>{fig.series}</dd>
                    </div>
                {/if}
                {#if fig.material}
                    <div>
                        <dt>{$t('archiveMaterialLabel')}</dt>
                        <dd>{fig.material}</dd>
                    </div>
                {:else if fig.technique}
                    <div>
                        <dt>{$t('archiveTechniqueLabel')}</dt>
                        <dd>{fig.technique}</dd>
                    </div>
                {/if}
                {#if fig.status === 'available'}
                    <div>
                        <dt>{$t('homeWorkPriceLabel')}</dt>
                        <dd>{$t('figurinePriceOnRequest')}</dd>
                    </div>
                {/if}
            </dl>
        {/if}

        <div class="tile-actions">
            <a href={`/figurines/${fig.id}`} class="tile-open">
                {$t('cardViewWork')}
                <svg width="14" height="7" viewBox="0 0 14 7" fill="none" aria-hidden="true">
                    <path d="M0 3.5H13M13 3.5L9.5 1M13 3.5L9.5 6" stroke="currentColor" stroke-width="1"/>
                </svg>
            </a>
            {#if fig.status === 'available'}
                <button class="tile-request" type="button" onclick={openOrder}>
                    {$t('cardRequest')}
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
        display: grid;
        grid-template-rows: auto 1fr;
        min-width: 0;
        color: var(--ink, #34251c);
        text-decoration: none;
        border: 1px solid rgba(52,37,28,0.12);
        background: rgba(255,252,246,0.82);
        box-shadow: 0 1px 0 rgba(255,255,255,0.75) inset;
        animation: tile-in 0.5s var(--ease, cubic-bezier(0.16,1,0.3,1)) both;
        animation-delay: calc(var(--i) * 0.04s);
        transition: transform 0.24s ease, border-color 0.24s ease, box-shadow 0.24s ease;
    }

    .tile:hover {
        transform: translateY(-2px);
        border-color: rgba(198,95,60,0.32);
        box-shadow: 0 12px 28px rgba(68,37,20,0.10);
    }

    .tile.is-selected {
        border-color: rgba(198,95,60,0.42);
        box-shadow:
            0 1px 0 rgba(255,255,255,0.75) inset,
            0 0 0 1px rgba(198,95,60,0.12),
            0 14px 32px rgba(68,37,20,0.11);
    }

    .tile-media-wrap,
    .tile-media {
        position: relative;
        aspect-ratio: 4 / 3;
        overflow: hidden;
        background: rgba(201,168,117,0.10);
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
        filter: grayscale(0.08) saturate(0.96);
        transition: transform 0.38s ease, filter 0.38s ease;
    }

    .tile:hover .tile-media :global(.tile-img .app-image-main) {
        transform: scale(1.035);
        filter: grayscale(0) saturate(1.02);
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
        right: 9px;
        top: 9px;
        z-index: 2;
        width: 30px;
        height: 30px;
        display: grid;
        place-items: center;
        border: 1px solid rgba(52,37,28,0.14);
        border-radius: 999px;
        background: rgba(255,249,240,0.76);
        color: var(--color-ink-tertiary);
        backdrop-filter: blur(8px);
        cursor: pointer;
        transition: color 0.2s, background 0.2s, border-color 0.2s, transform 0.2s;
    }

    .tile-selected {
        position: absolute;
        left: 9px;
        top: 9px;
        z-index: 2;
        max-width: calc(100% - 56px);
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
        transform: scale(1.08);
    }

    .tile-body {
        display: grid;
        gap: 8px;
        padding: 12px 13px 13px;
        min-width: 0;
    }

    .tile-head {
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        gap: 10px;
        min-width: 0;
    }

    .tile-head h3 {
        margin: 0;
        min-width: 0;
        font-family: 'Cormorant Garamond', Georgia, serif;
        font-size: clamp(19px, 1.45vw, 25px);
        font-weight: 400;
        line-height: 1;
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

    .tile-year {
        flex-shrink: 0;
        padding-top: 3px;
        font-size: 10px;
        letter-spacing: 0.08em;
        color: var(--color-ink-tertiary);
    }

    .tile-facts {
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 7px 10px;
        margin: 2px 0 0;
        padding: 9px 0 0;
        border-top: 1px solid rgba(52,37,28,0.08);
    }

    .tile-facts div {
        min-width: 0;
    }

    .tile-facts dt {
        margin: 0 0 3px;
        font-size: 8px;
        font-weight: 600;
        letter-spacing: 0.12em;
        text-transform: uppercase;
        color: var(--color-ink-tertiary);
    }

    .tile-facts dd {
        margin: 0;
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        font-family: 'Cormorant Garamond', Georgia, serif;
        font-size: 14px;
        font-style: italic;
        line-height: 1.15;
        color: var(--color-ink-secondary);
    }

    .tile-actions {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 10px;
        margin-top: 3px;
    }

    .tile-open,
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

    .tile-open {
        color: var(--color-ink-secondary);
        transition: color 0.2s, gap 0.2s;
    }

    .tile-open:hover {
        gap: 12px;
        color: var(--copper, #c65f3c);
    }

    .tile-request {
        flex-shrink: 0;
        padding: 0 10px;
        border: 1px solid rgba(198,95,60,0.28);
        background: rgba(255,246,239,0.72);
        color: var(--copper, #c65f3c);
        cursor: pointer;
        transition: background 0.2s, border-color 0.2s, color 0.2s;
    }

    .tile-request:hover {
        border-color: rgba(198,95,60,0.52);
        background: rgba(255,246,239,0.98);
        color: var(--color-ink-primary);
    }

    .tile-media:focus-visible,
    .tile-head h3 a:focus-visible,
    .tile-save:focus-visible,
    .tile-open:focus-visible,
    .tile-request:focus-visible {
        outline: 2px solid rgba(198,95,60,0.52);
        outline-offset: 3px;
    }

    .tile-meta {
        display: flex;
        align-items: center;
        gap: 7px;
        margin: 0;
        font-size: 11px;
        letter-spacing: 0.09em;
        text-transform: uppercase;
        color: var(--color-ink-tertiary);
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
        .tile-facts {
            grid-template-columns: 1fr;
        }

        .tile-actions {
            align-items: stretch;
            flex-direction: column;
        }

        .tile-open,
        .tile-request {
            width: 100%;
        }
    }

    @keyframes tile-in {
        from { opacity: 0; transform: translateY(10px); }
        to { opacity: 1; transform: none; }
    }
</style>
