<script lang="ts">
    import { onMount } from 'svelte';
    import { fade, fly } from 'svelte/transition';
    import { cubicOut } from 'svelte/easing';
    import { api } from '$lib/api';
    import type { FigurineListItem } from '$lib/types/api';
    import { t } from '$lib/i18n';
    import OrderModal from '$lib/components/OrderModal.svelte';
    import AppImage from '$lib/components/AppImage.svelte';

    let items = $state<FigurineListItem[]>([]);
    let isLoading = $state(true);
    let error = $state<string | null>(null);

    let modalOpen = $state(false);
    let modalFigurineId = $state('');
    let modalFigurineName = $state('');

    function openBook(fig: FigurineListItem) {
        modalFigurineId = fig.id;
        modalFigurineName = fig.name;
        modalOpen = true;
    }

    onMount(async () => {
        try {
            items = await api.getInProgressFigurines();
        } catch (e) {
            error = 'load_error';
        } finally {
            isLoading = false;
        }
    });
</script>

<svelte:head>
    <title>{$t('upcomingTitle')} — Gotiga</title>
    <meta name="description" content="Авторские фигурки в процессе создания. Можно забронировать." />
    <!-- Fonts loaded once globally in app.html -->
</svelte:head>

<div class="root">
    <div class="grain" aria-hidden="true"></div>

    <div class="page">
        <nav class="back-nav" in:fade={{ duration: 600 }}>
            <a href="/" class="back-link">{$t('upcomingBack')}</a>
        </nav>

        <header class="page-header" in:fly={{ x: -20, duration: 900, delay: 100, easing: cubicOut }}>
            <p class="eyebrow">
                <span class="eyebrow-rule"></span>
                {$t('upcomingKicker')}
            </p>
            <h1 class="page-title">{$t('upcomingTitle')}</h1>
            <p class="page-subtitle">{$t('upcomingSubtitle')}</p>
        </header>

        {#if isLoading}
            <div class="loading" in:fade>
                <span class="loading-dot"></span>
                <span class="loading-dot"></span>
                <span class="loading-dot"></span>
            </div>
        {:else if error || items.length === 0}
            <div class="empty" in:fade={{ duration: 700, delay: 200 }}>
                <p class="empty-title">{$t('upcomingEmpty')}</p>
                <p class="empty-hint">{$t('upcomingEmptyHint')}</p>
                <a href="/commission" class="petition-link">{$t('commissionInvite')}</a>
            </div>
        {:else}
            <div class="grid" in:fade={{ duration: 600, delay: 150 }}>
                {#each items as fig, i (fig.id)}
                    <article
                        class="card"
                        in:fly={{ y: 24, duration: 700, delay: 80 * i, easing: cubicOut }}
                    >
                        <div class="card-img-wrap">
                            {#if fig.faceImageUrl}
                                <AppImage
                                    src={fig.faceImageUrl}
                                    thumbUrl={fig.thumbUrl}
                                    alt={fig.name}
                                    class="card-img"
                                    loading="lazy"
                                />
                            {:else}
                                <div class="card-img-placeholder" aria-hidden="true"></div>
                            {/if}
                            <div class="card-img-overlay"></div>
                            <span class="wip-badge">{$t('upcomingWip')}</span>
                        </div>

                        <div class="card-body">
                            <h2 class="card-name">{fig.name}</h2>

                            <dl class="card-meta">
                                {#if fig.technique}
                                    <div class="meta-row">
                                        <dt class="meta-key">{$t('upcomingTechnique')}</dt>
                                        <dd class="meta-val">{fig.technique}</dd>
                                    </div>
                                {/if}
                                {#if fig.material}
                                    <div class="meta-row">
                                        <dt class="meta-key">{$t('upcomingMaterial')}</dt>
                                        <dd class="meta-val">{fig.material}</dd>
                                    </div>
                                {/if}
                                {#if fig.year}
                                    <div class="meta-row">
                                        <dt class="meta-key">{$t('upcomingYear')}</dt>
                                        <dd class="meta-val">{fig.year}</dd>
                                    </div>
                                {/if}
                            </dl>

                            <button class="book-btn" onclick={() => openBook(fig)}>
                                {$t('upcomingBook')}
                                <svg class="btn-arrow" width="16" height="8" viewBox="0 0 16 8" fill="none">
                                    <path d="M0 4H15M15 4L11 1M15 4L11 7" stroke="currentColor" stroke-width="1"/>
                                </svg>
                            </button>
                            <p class="book-hint">{$t('upcomingBookHint')}</p>
                        </div>
                    </article>
                {/each}
            </div>

            <div class="invitation" in:fade={{ duration: 700, delay: 300 }}>
                <span class="invitation-rule" aria-hidden="true"></span>
                <p class="invitation-text">{$t('commissionInviteHint')}</p>
                <a href="/commission" class="petition-link">{$t('commissionInvite')}</a>
            </div>
        {/if}
    </div>
</div>

<OrderModal
    isOpen={modalOpen}
    figurineId={modalFigurineId}
    figurineName={modalFigurineName}
    mode="request"
    onClose={() => { modalOpen = false; }}
/>

<style>
    :root {
        --cream:   var(--color-canvas-base);
        --cream2:  var(--color-canvas-raised);
        --ink:     var(--color-ink-primary);
        --brown:   var(--color-ink-primary);
        --mid:     var(--color-ember-deep);
        --tan:     var(--color-ember-ink);
        --copper:  var(--color-ember);
        --gold:    var(--color-ochre);
        --muted:   color-mix(in srgb, var(--color-ink-secondary) 68%, transparent);
        --muted2:  color-mix(in srgb, var(--color-ink-secondary) 40%, transparent);
        --border:  color-mix(in srgb, var(--color-ink-primary) 10%, transparent);
        --border2: color-mix(in srgb, var(--color-ink-primary) 18%, transparent);
        --ease:    cubic-bezier(0.16,1,0.3,1);
    }

    * { margin: 0; padding: 0; box-sizing: border-box; }

    :global(body) {
        background: var(--cream);
        color: var(--brown);
        font-family: 'Instrument Sans', sans-serif;
        -webkit-font-smoothing: antialiased;
    }

    .root {
        width: 100vw;
        min-height: 100svh;
        background:
            radial-gradient(ellipse 70% 55% at 72% 38%, rgba(198,95,60,0.06) 0%, transparent 65%),
            var(--cream);
        position: relative;
        overflow-x: hidden;
    }

    .grain {
        position: fixed;
        inset: -50%;
        width: 200%;
        height: 200%;
        opacity: 0.028;
        pointer-events: none;
        z-index: 500;
        background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
        animation: grain-anim 6s steps(1) infinite;
    }
    @keyframes grain-anim {
        0%   { transform: translate(0,0); }
        16%  { transform: translate(-5%,-8%); }
        33%  { transform: translate(8%,4%); }
        50%  { transform: translate(-3%,10%); }
        66%  { transform: translate(10%,-4%); }
        83%  { transform: translate(-8%,6%); }
        100% { transform: translate(0,0); }
    }

    .page {
        max-width: 1400px;
        margin: 0 auto;
        padding: clamp(28px, 5vw, 72px) clamp(20px, 4.5vw, 72px) clamp(60px, 8vw, 120px);
        position: relative;
        z-index: 1;
    }

    .back-nav { margin-bottom: clamp(28px, 4vw, 56px); }

    .back-link {
        font-size: 9px;
        letter-spacing: 0.22em;
        text-transform: uppercase;
        color: var(--muted2);
        text-decoration: none;
        transition: color 0.25s;
    }
    .back-link:hover { color: var(--brown); }

    /* Header */
    .page-header { margin-bottom: clamp(40px, 6vw, 80px); max-width: 640px; }

    .eyebrow {
        display: flex;
        align-items: center;
        gap: 12px;
        font-size: 9px;
        letter-spacing: 0.22em;
        text-transform: uppercase;
        color: var(--muted2);
        margin-bottom: 18px;
    }
    .eyebrow-rule {
        display: inline-block;
        width: 26px;
        height: 1px;
        background: var(--copper);
        opacity: 0.65;
        flex-shrink: 0;
    }

    .page-title {
        font-family: 'Cormorant Garamond', serif;
        font-size: clamp(48px, 7vw, 112px);
        font-weight: 300;
        line-height: 0.9;
        color: var(--ink);
        margin-bottom: 22px;
        letter-spacing: -0.015em;
    }

    .page-subtitle {
        font-family: 'Cormorant Garamond', serif;
        font-size: clamp(16px, 1.8vw, 20px);
        font-weight: 300;
        font-style: italic;
        line-height: 1.55;
        color: var(--muted);
    }

    /* Loading */
    .loading {
        display: flex;
        gap: 8px;
        padding: 80px 0;
    }
    .loading-dot {
        width: 5px;
        height: 5px;
        border-radius: 50%;
        background: var(--copper);
        animation: dot-pulse 1.4s ease-in-out infinite;
    }
    .loading-dot:nth-child(2) { animation-delay: 0.2s; }
    .loading-dot:nth-child(3) { animation-delay: 0.4s; }
    @keyframes dot-pulse {
        0%,80%,100% { opacity: 0.25; transform: scale(0.8); }
        40% { opacity: 1; transform: scale(1); }
    }

    /* Empty */
    .empty {
        padding: 100px 0;
        border-top: 1px solid var(--border);
        border-bottom: 1px solid var(--border);
    }
    .empty-title {
        font-family: 'Cormorant Garamond', serif;
        font-size: clamp(22px, 3vw, 36px);
        font-weight: 300;
        font-style: italic;
        color: var(--muted);
        margin-bottom: 10px;
    }
    .empty-hint {
        font-size: 11px;
        letter-spacing: 0.12em;
        text-transform: uppercase;
        color: var(--muted2);
    }

    /* Petition invitation */
    .petition-link {
        display: inline-block;
        margin-top: 1.5rem;
        font-family: 'Cormorant Garamond', Georgia, serif;
        font-style: italic;
        font-size: 1.15rem;
        color: #c65f3c;
        text-decoration: none;
        border-bottom: 1px solid rgba(198, 95, 60, 0.4);
        padding-bottom: 2px;
        transition: color 0.2s, border-color 0.2s;
    }
    .petition-link:hover {
        color: #6f3b24;
        border-color: #6f3b24;
    }
    .invitation {
        text-align: center;
        margin-top: clamp(48px, 7vw, 96px);
    }
    .invitation-rule {
        display: block;
        width: 40px;
        height: 1px;
        background: #c65f3c;
        margin: 0 auto 1.5rem;
    }
    .invitation-text {
        font-family: 'Cormorant Garamond', Georgia, serif;
        font-style: italic;
        font-size: 1.1rem;
        color: #6f3b24;
        max-width: 38ch;
        margin: 0 auto;
    }

    /* Grid */
    .grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
        gap: clamp(24px, 3vw, 42px);
    }

    /* Card */
    .card {
        display: flex;
        flex-direction: column;
        border: 1px solid var(--border);
        background: var(--cream2);
        transition: border-color 0.3s;
    }
    .card:hover { border-color: var(--border2); }

    .card-img-wrap {
        position: relative;
        aspect-ratio: 3/4;
        overflow: hidden;
        background: rgba(52,37,28,0.06);
    }

    :global(.card-img) {
        width: 100%;
        height: 100%;
        object-fit: cover;
        display: block;
        transition: transform 0.8s var(--ease), filter 0.5s;
        filter: saturate(0.72) contrast(1.06);
    }
    .card:hover :global(.card-img) {
        transform: scale(1.04);
        filter: saturate(0.9) contrast(1.04);
    }

    .card-img-placeholder {
        width: 100%;
        height: 100%;
        background: repeating-linear-gradient(
            45deg,
            rgba(52,37,28,0.03) 0px,
            rgba(52,37,28,0.03) 1px,
            transparent 1px,
            transparent 8px
        );
    }

    .card-img-overlay {
        position: absolute;
        inset: 0;
        background: linear-gradient(180deg, transparent 50%, rgba(44,23,16,0.36) 100%);
        pointer-events: none;
    }

    .wip-badge {
        position: absolute;
        top: 14px;
        left: 14px;
        font-size: 7.5px;
        letter-spacing: 0.24em;
        text-transform: uppercase;
        color: var(--cream2);
        background: rgba(198,95,60,0.88);
        padding: 4px 10px;
        backdrop-filter: blur(4px);
    }

    .card-body {
        padding: clamp(18px, 2.5vw, 26px);
        display: flex;
        flex-direction: column;
        gap: 14px;
        flex: 1;
    }

    .card-name {
        font-family: 'Cormorant Garamond', serif;
        font-size: clamp(20px, 2.2vw, 26px);
        font-weight: 400;
        line-height: 1.2;
        color: var(--ink);
    }

    .card-meta { display: flex; flex-direction: column; gap: 6px; }

    .meta-row { display: flex; align-items: baseline; gap: 8px; }

    .meta-key {
        font-size: 8px;
        letter-spacing: 0.2em;
        text-transform: uppercase;
        color: var(--muted2);
        flex-shrink: 0;
        min-width: 64px;
    }

    .meta-val {
        font-family: 'Cormorant Garamond', serif;
        font-size: 14px;
        font-style: italic;
        color: var(--muted);
        line-height: 1.3;
    }

    /* Book button */
    .book-btn {
        display: inline-flex;
        align-items: center;
        gap: 12px;
        height: 42px;
        padding: 0 20px;
        margin-top: auto;
        background: var(--ink);
        color: var(--cream2);
        font-family: 'Instrument Sans', sans-serif;
        font-size: 9px;
        letter-spacing: 0.18em;
        text-transform: uppercase;
        border: none;
        cursor: pointer;
        transition: background 0.25s, gap 0.25s;
        clip-path: polygon(0 0, calc(100% - 6px) 0, 100% 6px, 100% 100%, 6px 100%, 0 calc(100% - 6px));
        align-self: flex-start;
    }
    .book-btn:hover {
        background: var(--mid);
        gap: 18px;
    }

    .btn-arrow {
        flex-shrink: 0;
        transition: transform 0.25s;
    }
    .book-btn:hover .btn-arrow { transform: translateX(3px); }

    .book-hint {
        font-family: 'Cormorant Garamond', serif;
        font-size: 12px;
        font-style: italic;
        color: var(--muted2);
        margin: 0;
        line-height: 1.4;
    }

    /* Responsive */
    @media (max-width: 640px) {
        .grid { grid-template-columns: 1fr; }
    }
</style>
