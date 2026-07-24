<script lang="ts">
    import { fade, fly } from 'svelte/transition';
    import { cubicOut } from 'svelte/easing';
    import { t, brandName } from '$lib/i18n';
    import { SITE_URL } from '$lib/site';

    // Single source of truth for the contact address — kept in sync with SiteFooter.svelte.
    const EMAIL = 'margoritunia@gmail.com';

    let sections = $derived([
        { title: $t('privacySectionDataTitle'), paragraphs: [
            $t('privacyDataNewsletter'),
            $t('privacyDataForms'),
            $t('privacyDataContact'),
            $t('privacyDataImpressions'),
            $t('privacyDataAccount'),
        ] },
        { title: $t('privacySectionAnalyticsTitle'), paragraphs: [$t('privacyAnalyticsBody')] },
        { title: $t('privacySectionCookiesTitle'), paragraphs: [$t('privacyCookiesBody')] },
        { title: $t('privacySectionThirdPartiesTitle'), paragraphs: [$t('privacyThirdPartiesBody')] },
        { title: $t('privacySectionRetentionTitle'), paragraphs: [$t('privacyRetentionBody')] },
        { title: $t('privacySectionRightsTitle'), paragraphs: [$t('privacyRightsBody')] },
        { title: $t('privacySectionChildrenTitle'), paragraphs: [$t('privacyChildrenBody')] },
        { title: $t('privacySectionChangesTitle'), paragraphs: [$t('privacyChangesBody')] },
    ]);
</script>

<svelte:head>
    <title>{$t('privacyTitle')} — {$brandName}</title>
    <meta name="description" content="What personal data this site collects, why, and for how long." />
    <meta property="og:site_name" content={$brandName} />
    <meta property="og:type" content="website" />
    <meta property="og:title" content="{$t('privacyTitle')} — {$brandName}" />
    <meta property="og:url" content="{SITE_URL}/privacy" />
    <!-- Fonts loaded once globally in app.html -->
</svelte:head>

<div class="root">
    <div class="grain" aria-hidden="true"></div>

    <div class="page">
        <nav class="back-nav" in:fade={{ duration: 600 }}>
            <a href="/" class="back-link">{$t('privacyBack')}</a>
        </nav>

        <header class="page-header" in:fly={{ x: -20, duration: 900, delay: 100, easing: cubicOut }}>
            <p class="eyebrow">
                <span class="eyebrow-rule"></span>
                {$t('privacyEyebrow')}
            </p>
            <h1 class="page-title">{$t('privacyTitle')}</h1>
            <p class="page-subtitle">{$t('privacyUpdated')}</p>
        </header>

        <div class="body" in:fade={{ duration: 600, delay: 150 }}>
            <p class="intro">{$t('privacyIntro')}</p>

            {#each sections as section}
                <section class="policy-section">
                    <h2 class="section-title">{section.title}</h2>
                    {#each section.paragraphs as p}
                        <p class="section-text">{p}</p>
                    {/each}
                </section>
            {/each}

            <section class="policy-section">
                <h2 class="section-title">{$t('privacySectionContactTitle')}</h2>
                <p class="section-text">{$t('privacyContactBody')}</p>
                <a class="contact-email" href={`mailto:${EMAIL}`}>{EMAIL}</a>
            </section>
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
    }

    .page {
        max-width: 760px;
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

    .page-header { margin-bottom: clamp(40px, 6vw, 64px); }

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
        font-size: clamp(38px, 6vw, 64px);
        font-weight: 300;
        line-height: 1;
        color: var(--ink);
        margin-bottom: 14px;
        letter-spacing: -0.01em;
    }

    .page-subtitle {
        font-size: 12px;
        letter-spacing: 0.08em;
        text-transform: uppercase;
        color: var(--muted);
    }

    .body {
        font-family: var(--font-body, 'DM Sans', system-ui, sans-serif);
    }

    .intro {
        font-size: 16px;
        line-height: 1.7;
        color: var(--muted);
        margin-bottom: clamp(36px, 5vw, 56px);
        max-width: 62ch;
    }

    .policy-section {
        padding: 26px 0;
        border-top: 1px solid var(--border);
    }
    .policy-section:last-child {
        border-bottom: 1px solid var(--border);
    }

    .section-title {
        font-family: 'Cormorant Garamond', serif;
        font-size: 22px;
        font-weight: 500;
        color: var(--ink);
        margin: 0 0 14px;
    }

    .section-text {
        font-size: 14.5px;
        line-height: 1.75;
        color: var(--muted);
        max-width: 64ch;
        margin: 0 0 12px;
    }
    .section-text:last-child { margin-bottom: 0; }

    .contact-email {
        display: inline-block;
        margin-top: 4px;
        font-size: 15px;
        color: var(--copper);
        text-decoration: none;
        border-bottom: 1px solid var(--border-ember, var(--copper));
    }
    .contact-email:hover { opacity: 0.75; }

    @media (prefers-reduced-motion: reduce) {
        .grain { display: none; }
    }
</style>
