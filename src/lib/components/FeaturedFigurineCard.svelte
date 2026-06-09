<script lang="ts">
    import { onMount } from 'svelte';
    import type { FigurineListItem } from '$lib/types/api';
    import { t } from '$lib/i18n';
    import AppImage from '$lib/components/AppImage.svelte';
    import Lightbox from '$lib/components/Lightbox.svelte';
    import OrderModal from '$lib/components/OrderModal.svelte';

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

    // ── Action state ────────────────────────────────────────────────
    let liked = $state(false);
    let showLightbox = $state(false);
    let showOrder = $state(false);
    let shareCopied = $state(false);

    const LIKED_KEY = 'gotiga_liked';

    onMount(() => {
        try {
            const ids: string[] = JSON.parse(localStorage.getItem(LIKED_KEY) ?? '[]');
            liked = ids.includes(fig.id);
        } catch {}
    });

    function saveLiked(val: boolean) {
        try {
            const ids: string[] = JSON.parse(localStorage.getItem(LIKED_KEY) ?? '[]');
            if (val) { if (!ids.includes(fig.id)) ids.push(fig.id); }
            else      { const i = ids.indexOf(fig.id); if (i !== -1) ids.splice(i, 1); }
            localStorage.setItem(LIKED_KEY, JSON.stringify(ids));
        } catch {}
    }

    function toggleLike(e: MouseEvent) {
        e.preventDefault();
        e.stopPropagation();
        liked = !liked;
        saveLiked(liked);
    }

    function openQuickView(e: MouseEvent) {
        e.preventDefault();
        e.stopPropagation();
        if (fig.faceImageUrl) showLightbox = true;
    }

    async function handleShare(e: MouseEvent) {
        e.preventDefault();
        e.stopPropagation();
        const url = `${window.location.origin}/figurines/${fig.id}`;
        try {
            if (navigator.share) {
                await navigator.share({ title: fig.name, url });
            } else {
                await navigator.clipboard.writeText(url);
                shareCopied = true;
                setTimeout(() => { shareCopied = false; }, 2000);
            }
        } catch {}
    }

    function openOrder(e: MouseEvent) {
        e.preventDefault();
        e.stopPropagation();
        showOrder = true;
    }
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

        <!-- Heart button — always visible -->
        <button
            class="card-heart"
            class:is-liked={liked}
            onclick={toggleLike}
            aria-label={liked ? $t('cardSaved') : $t('cardSave')}
            title={liked ? $t('cardSaved') : $t('cardSave')}
        >
            <svg width="13" height="13" viewBox="0 0 14 14" fill="none" aria-hidden="true">
                <path
                    d="M7 12.5C7 12.5 1 8.5 1 4.5C1 2.5 2.5 1 4.5 1C5.5 1 6.5 1.8 7 3C7.5 1.8 8.5 1 9.5 1C11.5 1 13 2.5 13 4.5C13 8.5 7 12.5 7 12.5Z"
                    fill={liked ? 'currentColor' : 'none'}
                    stroke="currentColor"
                    stroke-width="1.1"
                    stroke-linejoin="round"
                />
            </svg>
        </button>

        {#if fig.faceImageUrl}
            <AppImage src={fig.faceImageUrl} class="card-img" loading="lazy" />
        {:else}
            <div class="card-ph">?</div>
        {/if}

        <!-- Hover overlay with CTA + action buttons -->
        <div class="card-ov">
            <div class="card-ov-bar">
                <span class="card-ov-cta">
                    View work
                    <svg width="14" height="7" viewBox="0 0 14 7" fill="none" aria-hidden="true">
                        <path d="M0 3.5H13M13 3.5L9.5 1M13 3.5L9.5 6" stroke="currentColor" stroke-width="1"/>
                    </svg>
                </span>

                <div class="card-actions">
                    {#if fig.faceImageUrl}
                    <button
                        class="card-action-btn"
                        onclick={openQuickView}
                        title={$t('cardQuickView')}
                        aria-label={$t('cardQuickView')}
                    >
                        <!-- Eye -->
                        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
                            <path d="M1 7C1 7 3.5 3 7 3C10.5 3 13 7 13 7C13 7 10.5 11 7 11C3.5 11 1 7 1 7Z" stroke="currentColor" stroke-width="1" stroke-linejoin="round"/>
                            <circle cx="7" cy="7" r="2" stroke="currentColor" stroke-width="1"/>
                        </svg>
                    </button>
                    {/if}

                    <button
                        class="card-action-btn"
                        class:is-copied={shareCopied}
                        onclick={handleShare}
                        title={shareCopied ? $t('cardLinkCopied') : $t('cardShare')}
                        aria-label={$t('cardShare')}
                    >
                        <!-- Share / Copied checkmark -->
                        {#if shareCopied}
                        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
                            <path d="M2 7L5.5 10.5L12 3.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
                        </svg>
                        {:else}
                        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
                            <path d="M9 2H12V5" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"/>
                            <path d="M12 2L7 7" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
                            <path d="M6 3H3C2.4 3 2 3.4 2 4V11C2 11.6 2.4 12 3 12H10C10.6 12 11 11.6 11 11V8" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
                        </svg>
                        {/if}
                    </button>

                    <button
                        class="card-action-btn card-action-btn--req"
                        onclick={openOrder}
                        title={$t('cardRequest')}
                        aria-label={$t('cardRequest')}
                    >
                        <!-- Envelope -->
                        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
                            <rect x="2" y="4" width="10" height="7" rx="0.5" stroke="currentColor" stroke-width="1"/>
                            <path d="M2 5L7 8.5L12 5" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>
                        </svg>
                    </button>
                </div>
            </div>
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

{#if showLightbox && fig.faceImageUrl}
    <Lightbox
        images={[{ url: fig.faceImageUrl, alt: fig.name }]}
        onClose={() => { showLightbox = false; }}
    />
{/if}

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

    .card-status { color: rgba(95,70,54,0.62); }
    .card-status.status-available { color: rgba(30,112,72,0.82); }
    .card-status.status-reserved  { color: rgba(160,106,24,0.82); }
    .card-status.status-sold      { color: rgba(95,70,54,0.46); }

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

    /* ── Heart button ─────────────────────────────── */
    .card-heart {
        position: absolute;
        top: 38px;
        right: 10px;
        z-index: 9;
        width: 30px;
        height: 30px;
        display: flex;
        align-items: center;
        justify-content: center;
        background: rgba(255,249,240,0.65);
        border: 1px solid rgba(52,37,28,0.12);
        border-radius: 50%;
        backdrop-filter: blur(6px);
        cursor: pointer;
        color: rgba(95,70,54,0.55);
        transition:
            color 0.24s,
            background 0.24s,
            border-color 0.24s,
            transform 0.28s var(--ease);
    }

    .card-heart:hover {
        background: rgba(255,249,240,0.92);
        color: var(--copper, #c65f3c);
        transform: scale(1.12);
        border-color: rgba(198,95,60,0.25);
    }

    .card-heart.is-liked {
        color: var(--copper, #c65f3c);
        background: rgba(198,95,60,0.10);
        border-color: rgba(198,95,60,0.30);
    }

    .card-heart.is-liked:hover {
        background: rgba(198,95,60,0.16);
        transform: scale(1.12);
    }

    /* ── Image & overlay ──────────────────────────── */
    /* .card-img is applied to the AppImage wrapper (a child component), so it must be
       reached with :global while keeping the .card-img-wrap ancestor scoped. */
    .card-img-wrap :global(.card-img) {
        width: 100%;
        height: 100%;
        object-fit: cover;
        object-position: center 42%;
        display: block;
        filter: grayscale(0.08) saturate(0.96) contrast(0.98);
        transition: transform 0.65s var(--ease), filter 0.65s;
    }

    .card:hover :global(.card-img) {
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
            linear-gradient(to top, rgba(44,23,16,0.65) 0%, transparent 52%),
            radial-gradient(circle at 50% 18%, transparent 0%, rgba(44,23,16,0.10) 100%);
        opacity: 0;
        display: flex;
        align-items: flex-end;
        padding: 14px 14px 16px;
        transition: opacity 0.4s;
    }

    .card:hover .card-ov {
        opacity: 1;
    }

    /* ── Hover bar (CTA + action buttons) ─────────── */
    .card-ov-bar {
        display: flex;
        align-items: center;
        justify-content: space-between;
        width: 100%;
        transform: translateY(10px);
        transition: transform 0.38s var(--ease);
    }

    .card:hover .card-ov-bar {
        transform: translateY(0);
    }

    .card-ov-cta {
        display: inline-flex;
        align-items: center;
        gap: 8px;
        color: var(--cream2);
        font-size: 9px;
        letter-spacing: 0.16em;
        text-transform: uppercase;
    }

    /* ── Action icon buttons ──────────────────────── */
    .card-actions {
        display: flex;
        align-items: center;
        gap: 5px;
    }

    .card-action-btn {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 28px;
        height: 28px;
        background: rgba(255,249,240,0.11);
        border: 1px solid rgba(255,249,240,0.20);
        border-radius: 50%;
        color: rgba(255,249,240,0.62);
        cursor: pointer;
        opacity: 0;
        transform: translateY(6px) scale(0.88);
        transition:
            opacity 0.28s var(--ease),
            transform 0.28s var(--ease),
            background 0.2s,
            color 0.2s,
            border-color 0.2s;
    }

    .card-actions .card-action-btn:nth-child(1) { transition-delay: 0.04s; }
    .card-actions .card-action-btn:nth-child(2) { transition-delay: 0.08s; }
    .card-actions .card-action-btn:nth-child(3) { transition-delay: 0.12s; }

    .card:hover .card-action-btn {
        opacity: 1;
        transform: translateY(0) scale(1);
    }

    .card:hover .card-action-btn:hover {
        background: rgba(255,249,240,0.22);
        color: rgba(255,249,240,1);
        border-color: rgba(255,249,240,0.38);
        transform: translateY(0) scale(1.1);
    }

    .card-action-btn--req {
        background: rgba(198,95,60,0.18);
        border-color: rgba(198,95,60,0.32);
        color: rgba(255,200,170,0.88);
    }

    .card:hover .card-action-btn--req:hover {
        background: rgba(198,95,60,0.38);
        border-color: rgba(198,95,60,0.55);
        color: rgba(255,224,208,1);
    }

    .card-action-btn.is-copied {
        color: rgba(130,210,130,0.9);
        border-color: rgba(100,180,100,0.4);
    }

    /* Mobile: always show overlay bar */
    @media (hover: none) {
        .card-ov {
            opacity: 0.9;
            background: linear-gradient(to top, rgba(44,23,16,0.58) 0%, transparent 40%);
        }
        .card-ov-bar  { transform: none; }
        .card-action-btn {
            opacity: 1;
            transform: none;
        }
    }

    /* ── Card meta ────────────────────────────────── */
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

    .card:hover .card-name { color: var(--mid); }

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

    .card.is-sold :global(.card-img) {
        filter: grayscale(0.58) saturate(0.74);
    }

    @keyframes card-in {
        from { opacity: 0; transform: translateY(18px); }
        to   { opacity: 1; transform: none; }
    }
</style>
