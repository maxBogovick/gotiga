<script lang="ts">
    import type { FigurineListItem } from '$lib/types/api';
    import { t } from '$lib/i18n';
    import AppImage from '$lib/components/AppImage.svelte';

    let {
        fig,
        index,
    }: {
        fig: FigurineListItem;
        index: number;
    } = $props();

    let statusLabel = $derived(
        fig.status === 'available'
            ? $t('archiveStatusAvailableLabel')
            : fig.status === 'reserved'
                ? $t('archiveStatusReservedLabel')
                : $t('archiveStatusSoldLabel')
    );

    let displayIndex = $derived(String(index + 1).padStart(2, '0'));
</script>

<a
    href={`/figurines/${fig.id}`}
    class="card"
    class:is-sold={fig.status === 'sold'}
    style="--i:{index}"
    aria-label="{$t('homeViewFigurine')}: {fig.name}"
>
    <div class="card-img-wrap" style="view-transition-name: figurine-{fig.id}">
        <div class="card-register">
            <span>№ {displayIndex}</span>
            <span
                class="card-status"
                class:status-available={fig.status === 'available'}
                class:status-reserved={fig.status === 'reserved'}
                class:status-sold={fig.status === 'sold'}
            >
                {statusLabel}
            </span>
        </div>

        {#if fig.faceImageUrl}
            <AppImage src={fig.faceImageUrl} class="card-img" loading="lazy" />
        {:else}
            <div class="card-ph">?</div>
        {/if}

        <div class="card-ov">
            <span class="card-ov-cta">
                View work
                <svg width="14" height="7" viewBox="0 0 14 7" fill="none" aria-hidden="true">
                    <path d="M0 3.5H13M13 3.5L9.5 1M13 3.5L9.5 6" stroke="currentColor" stroke-width="1"/>
                </svg>
            </span>
        </div>
    </div>

    <div class="card-meta">
        <div class="card-meta-top">
            <h3 class="card-name">{fig.name}</h3>
            {#if fig.year}
                <span class="card-year">{fig.year}</span>
            {/if}
        </div>
        <span class="card-kind">Archive specimen</span>
    </div>
</a>

<style>
    .card {
        display: block;
        text-decoration: none;
        color: inherit;
        overflow: hidden;
        background:
            linear-gradient(180deg, rgba(255,251,244,0.96), rgba(250,242,230,0.9));
        border: 1px solid rgba(52,37,28,0.12);
        border-radius: 18px;
        box-shadow:
            0 1px 0 rgba(255,255,255,0.85) inset,
            0 18px 44px rgba(68,37,20,0.09);
        animation: card-in 0.65s var(--ease) both;
        animation-delay: calc(var(--i) * 0.08s + 0.15s);
        transition:
            border-color 0.35s var(--ease),
            box-shadow 0.35s var(--ease),
            transform 0.35s var(--ease);
    }

    .card:hover {
        border-color: rgba(111,59,36,0.28);
        box-shadow:
            0 1px 0 rgba(255,255,255,0.85) inset,
            0 24px 56px rgba(68,37,20,0.14);
        transform: translateY(-3px);
    }

    .card-register {
        position: absolute;
        top: 0;
        left: 0;
        right: 0;
        z-index: 8;
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 12px;
        min-height: 30px;
        padding: 0 10px;
        background: linear-gradient(180deg, rgba(255,249,240,0.86), rgba(255,249,240,0.58));
        border-bottom: 1px solid rgba(52,37,28,0.10);
        backdrop-filter: blur(8px);
        font-size: 8.5px;
        letter-spacing: 0.18em;
        text-transform: uppercase;
        color: rgba(95,70,54,0.52);
        pointer-events: none;
    }

    .card-status {
        color: rgba(95,70,54,0.62);
    }

    .card-status.status-available {
        color: rgba(30,112,72,0.82);
    }

    .card-status.status-reserved {
        color: rgba(160,106,24,0.82);
    }

    .card-status.status-sold {
        color: rgba(95,70,54,0.46);
    }

    .card-img-wrap {
        position: relative;
        aspect-ratio: 4 / 5;
        overflow: hidden;
        background: rgba(201,168,117,0.08);
        border-bottom: 1px solid rgba(52,37,28,0.12);
    }

    .card-img-wrap::before,
    .card-img-wrap::after {
        content: '';
        position: absolute;
        z-index: 5;
        width: 22px;
        height: 22px;
        pointer-events: none;
        opacity: 0.62;
    }

    .card-img-wrap::before {
        top: 9px;
        left: 9px;
        border-top: 1px solid rgba(255,249,240,0.55);
        border-left: 1px solid rgba(255,249,240,0.55);
    }

    .card-img-wrap::after {
        right: 9px;
        bottom: 9px;
        border-right: 1px solid rgba(255,249,240,0.48);
        border-bottom: 1px solid rgba(255,249,240,0.48);
    }

    .card-img {
        width: 100%;
        height: 100%;
        object-fit: cover;
        object-position: center 42%;
        display: block;
        filter: grayscale(0.08) saturate(0.96) contrast(0.98);
        transition: transform 0.65s var(--ease), filter 0.65s;
    }

    .card:hover .card-img {
        transform: scale(1.035);
        filter: grayscale(0) saturate(1.02) contrast(1);
    }

    .card-ph {
        width: 100%;
        height: 100%;
        display: grid;
        place-items: center;
        font-family: 'Cormorant Garamond', serif;
        font-size: 38px;
        color: var(--muted2);
    }

    .card-ov {
        position: absolute;
        inset: 0;
        background:
            linear-gradient(to top, rgba(44,23,16,0.62) 0%, transparent 50%),
            radial-gradient(circle at 50% 18%, transparent 0%, rgba(44,23,16,0.12) 100%);
        opacity: 0;
        display: flex;
        align-items: flex-end;
        padding: 16px;
        transition: opacity 0.4s;
    }

    .card:hover .card-ov {
        opacity: 1;
    }

    .card-ov-cta {
        display: inline-flex;
        align-items: center;
        gap: 8px;
        color: var(--cream2);
        font-size: 9px;
        letter-spacing: 0.16em;
        text-transform: uppercase;
        transform: translateY(8px);
        transition: transform 0.35s var(--ease);
    }

    .card:hover .card-ov-cta {
        transform: translateY(0);
    }

    .card-meta {
        position: relative;
        min-height: 122px;
        padding: 22px 24px 18px;
        background:
            linear-gradient(180deg, rgba(255,252,246,0.94), rgba(249,240,227,0.92));
    }

    .card-meta-top {
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        gap: 18px;
        min-height: 54px;
        padding-bottom: 16px;
        border-bottom: 1px solid rgba(52,37,28,0.11);
    }

    .card-name {
        font-family: 'Cormorant Garamond', serif;
        font-size: clamp(22px, 1.65vw, 30px);
        font-weight: 400;
        line-height: 1.04;
        color: var(--ink);
        margin: 0;
        min-width: 0;
        display: -webkit-box;
        line-clamp: 2;
        -webkit-line-clamp: 2;
        -webkit-box-orient: vertical;
        overflow: hidden;
        transition: color 0.28s;
    }

    .card:hover .card-name {
        color: var(--mid);
    }

    .card-year {
        flex-shrink: 0;
        padding-top: 5px;
        font-size: 9px;
        letter-spacing: 0.18em;
        text-transform: uppercase;
        color: var(--muted2);
    }

    .card-kind {
        display: flex;
        align-items: center;
        gap: 10px;
        margin-top: 14px;
        font-size: 8.5px;
        letter-spacing: 0.18em;
        text-transform: uppercase;
        color: rgba(95,70,54,0.42);
    }

    .card-kind::before {
        content: '';
        width: 18px;
        height: 1px;
        background: rgba(95,70,54,0.22);
    }

    .card.is-sold .card-img {
        filter: grayscale(0.58) saturate(0.74);
    }

    @keyframes card-in {
        from {
            opacity: 0;
            transform: translateY(18px);
        }

        to {
            opacity: 1;
            transform: none;
        }
    }
</style>
