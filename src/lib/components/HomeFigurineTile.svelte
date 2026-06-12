<script lang="ts">
    import { onMount } from 'svelte';
    import type { FigurineListItem } from '$lib/types/api';
    import { t } from '$lib/i18n';
    import AppImage from '$lib/components/AppImage.svelte';
    import { savedFigurines } from '$lib/stores/saved-figurines.svelte';

    let {
        fig,
        index = 0,
        compact = false,
    }: {
        fig: FigurineListItem;
        index?: number;
        compact?: boolean;
    } = $props();

    let saved = $derived(savedFigurines.has(fig.id));
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
</script>

<a
    href={`/figurines/${fig.id}`}
    class="tile"
    class:tile-compact={compact}
    style="--i:{index}"
    aria-label="{$t('homeViewFigurine')}: {fig.name}"
>
    <div class="tile-media">
        {#if fig.faceImageUrl}
            <AppImage src={fig.faceImageUrl} thumbUrl={fig.thumbUrl} alt={fig.name} class="tile-img" loading="lazy" />
        {:else}
            <div class="tile-placeholder">?</div>
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
            <h3>{fig.name}</h3>
            {#if fig.year}
                <span>{fig.year}</span>
            {/if}
        </div>
        <p class="tile-meta">
            <span class="tile-dot status-{fig.status}"></span>
            {statusLabel}
        </p>
    </div>
</a>

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

    .tile-media {
        position: relative;
        aspect-ratio: 4 / 3;
        overflow: hidden;
        background: rgba(201,168,117,0.10);
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
        object-fit: cover;
        object-position: center 42%;
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
        color: rgba(95,70,54,0.36);
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
        color: rgba(95,70,54,0.58);
        backdrop-filter: blur(8px);
        cursor: pointer;
        transition: color 0.2s, background 0.2s, border-color 0.2s, transform 0.2s;
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

    .tile-head span {
        flex-shrink: 0;
        padding-top: 3px;
        font-size: 9px;
        letter-spacing: 0.16em;
        color: rgba(95,70,54,0.42);
    }

    .tile-meta {
        display: flex;
        align-items: center;
        gap: 7px;
        margin: 0;
        font-size: 8.5px;
        letter-spacing: 0.15em;
        text-transform: uppercase;
        color: rgba(95,70,54,0.52);
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

    @keyframes tile-in {
        from { opacity: 0; transform: translateY(10px); }
        to { opacity: 1; transform: none; }
    }
</style>
