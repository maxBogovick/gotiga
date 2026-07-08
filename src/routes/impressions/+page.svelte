<script lang="ts">
    import { fade, fly } from 'svelte/transition';
    import { cubicOut } from 'svelte/easing';
    import { t, brandName } from '$lib/i18n';
    import { SITE_URL } from '$lib/site';
    import ImpressionsForm from '$lib/components/ImpressionsForm.svelte';
</script>

<svelte:head>
    <title>{$t('impressionsPageTitle')} — {$brandName}</title>
    <meta name="description" content="Leave a quiet reaction to the exhibition — the Book of Impressions." />
    <meta property="og:site_name" content={$brandName} />
    <meta property="og:locale" content="en_US" />
    <meta property="og:type" content="website" />
    <meta property="og:title" content="{$t('impressionsPageTitle')} — {$brandName}" />
    <meta property="og:description" content="Leave a quiet reaction to the exhibition — the Book of Impressions." />
    <meta property="og:url" content="{SITE_URL}/impressions" />
    <!-- Fonts loaded once globally in app.html -->
</svelte:head>

<div class="root">
    <div class="grain" aria-hidden="true"></div>

    <div class="page">
        <nav class="back-nav" in:fade={{ duration: 600 }}>
            <a href="/" class="back-link">{$t('impressionsBack')}</a>
        </nav>

        <header class="page-header" in:fly={{ x: -20, duration: 900, delay: 100, easing: cubicOut }}>
            <p class="eyebrow">
                <span class="eyebrow-rule"></span>
                {$t('impressionsKicker')}
            </p>
            <h1 class="page-title">{$t('impressionsPageTitle')}</h1>
            <p class="page-subtitle">{$t('impressionsPageSubtitle')}</p>
        </header>

        <div in:fade={{ duration: 700, delay: 200 }}>
            <ImpressionsForm />
        </div>
    </div>
</div>

<style>
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
        0%, 100% { transform: translate(0, 0); }
        50% { transform: translate(-2%, 2%); }
    }

    .page {
        max-width: 1240px;
        margin: 0 auto;
        padding: clamp(80px, 10vw, 140px) clamp(20px, 5vw, 64px) clamp(60px, 8vw, 100px);
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

    .page-header { margin-bottom: clamp(24px, 4vw, 48px); max-width: 640px; }

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
        background: var(--ember, #c65f3c);
        opacity: 0.65;
    }

    .page-title {
        font-family: 'Cormorant Garamond', serif;
        font-size: clamp(48px, 7vw, 96px);
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

    @media (prefers-reduced-motion: reduce) {
        .grain { animation: none; }
    }
</style>
