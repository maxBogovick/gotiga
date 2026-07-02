<script lang="ts">
    import { onMount } from 'svelte';
    import { fade } from 'svelte/transition';
    import AppImage from '$lib/components/AppImage.svelte';
    import LivingDaguerreotype from '$lib/components/LivingDaguerreotype.svelte';
    import { t } from '$lib/i18n';
    import type { FigurineListItem } from '$lib/types/api';

    type Props = {
        /** The work chosen for today's vitrine (admin-pinned, else daily rotation). */
        fig: FigurineListItem | null;
        /** Curator's note — a fragment of the work's own description. */
        note?: string | null;
        /** Catalogue mark: the work's passport number, else a roman day-of-year. */
        catalogNo?: string | null;
    };

    let { fig, note = null, catalogNo = null }: Props = $props();

    // Meta line, letterpress caps — built from whatever the list payload carries.
    let metaParts = $derived(
        [fig?.year ? String(fig.year) : null, fig?.material, fig?.dimensions]
            .map((p) => p?.trim())
            .filter((p): p is string => Boolean(p))
    );

    // Trim the note to a single museum-card breath; never cut mid-word.
    let noteText = $derived((() => {
        const raw = note?.trim();
        if (!raw) return '';
        if (raw.length <= 150) return raw;
        const slice = raw.slice(0, 150);
        const cut = slice.lastIndexOf(' ');
        return `${(cut > 80 ? slice.slice(0, cut) : slice).replace(/[\s.,;:—-]+$/, '')}…`;
    })());

    // A fixed scatter of motes — stable per mount so the dust doesn't reshuffle on
    // every reactive tick. Drift up the light cone at staggered, lazy cadences.
    const MOTES = Array.from({ length: 11 }, (_, i) => ({
        left: 12 + ((i * 37) % 76),
        delay: -(i * 1.7),
        dur: 13 + ((i * 5) % 9),
        size: 1 + (i % 3) * 0.6,
    }));

    // Same "living daguerreotype" 2.5D depth parallax the specimen gets on its
    // own detail page — only on desktop pointers, with motion allowed.
    let isPointerFine = $state(false);
    let prefersReducedMotion = $state(false);
    let useDaguerreotype = $derived(isPointerFine && !prefersReducedMotion);

    onMount(() => {
        isPointerFine = window.matchMedia('(pointer: fine)').matches;
        prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    });
</script>

{#if fig}
<section class="vitrine" aria-labelledby="vitrine-title" in:fade={{ duration: 600 }}>
    <p class="vitrine-eyebrow">
        <span class="ve-rule"></span>
        {$t('vitrineEyebrow')}
        {#if catalogNo}<span class="ve-no">№ {catalogNo}</span>{/if}
        <span class="ve-rule"></span>
    </p>

    <a class="vitrine-link" href={`/figurines/${fig.id}`} aria-label="{$t('vitrineEnter')}: {fig.name}">
    <span class="vitrine-stage">
        <!-- Case + light + dust, all scaled to one box so the atmosphere hugs the object -->
        <span class="case-wrap">
            <span class="cone" aria-hidden="true"></span>
            <span class="dust" aria-hidden="true">
                {#each MOTES as m}
                    <span
                        class="mote"
                        style="left:{m.left}%;width:{m.size}px;height:{m.size}px;animation-delay:{m.delay}s;animation-duration:{m.dur}s;"
                    ></span>
                {/each}
            </span>

            <span class="case">
                <span class="case-post case-post--l" aria-hidden="true"></span>
                <span class="case-post case-post--r" aria-hidden="true"></span>

                <span class="specimen">
                    {#if useDaguerreotype && fig.faceImageUrl}
                        <LivingDaguerreotype src={fig.faceImageUrl} alt={fig.name} class="specimen-img" />
                    {:else}
                        <AppImage src={fig.faceImageUrl} thumbUrl={fig.thumbUrl} alt={fig.name} class="specimen-img" loading="eager" fetchpriority="high" />
                    {/if}
                    <span class="specimen-shadow" aria-hidden="true"></span>
                </span>

                <span class="case-floor" aria-hidden="true"></span>
                <span class="case-sheen" aria-hidden="true"></span>
            </span>
        </span>

        <!-- Glossy shelf the case stands on, holding a faint mirror of the piece -->
        <span class="shelf" aria-hidden="true">
            <AppImage src={fig.faceImageUrl} thumbUrl={fig.thumbUrl} alt="" class="shelf-reflection-img" loading="lazy" />
        </span>

        <!-- Pedestal -->
        <span class="plinth" aria-hidden="true">
            <span class="plinth-slab"></span>
        </span>
    </span>

    <div class="vitrine-caption">
        <h2 id="vitrine-title" class="vitrine-name">{fig.name}</h2>
        {#if metaParts.length}
            <p class="vitrine-meta">
                {#each metaParts as part, i}
                    {#if i > 0}<span class="dot">·</span>{/if}{part}
                {/each}
            </p>
        {/if}
        {#if noteText}
            <blockquote class="vitrine-note">{noteText}</blockquote>
        {/if}
        <span class="vitrine-enter">
            {$t('vitrineEnter')}
            <svg width="16" height="8" viewBox="0 0 16 8" fill="none" aria-hidden="true">
                <path d="M0 4H15M15 4L11 1M15 4L11 7" stroke="currentColor" stroke-width="1"/>
            </svg>
        </span>
    </div>
    </a>
</section>
{/if}

<style>
    .vitrine {
        position: relative;
        max-width: 1520px;
        margin: 0 auto;
        padding: clamp(34px, 5vw, 72px) clamp(20px, 4.5vw, 64px) clamp(40px, 5.5vw, 80px);
        display: grid;
        justify-items: center;
        text-align: center;
        border-top: 1px solid color-mix(in srgb, var(--color-ink-primary) 9%, transparent);
        border-bottom: 1px solid color-mix(in srgb, var(--color-ink-primary) 9%, transparent);
    }

    /* ── EYEBROW ─────────────────────────────────── */
    .vitrine-eyebrow {
        display: flex;
        align-items: center;
        gap: 12px;
        margin-bottom: clamp(18px, 2.4vw, 32px);
        font-size: 11px;
        font-weight: 600;
        letter-spacing: 0.18em;
        text-transform: uppercase;
        color: var(--color-ink-tertiary);
    }

    .ve-rule {
        display: inline-block;
        width: clamp(28px, 6vw, 64px);
        height: 1px;
        background: color-mix(in srgb, var(--color-ember) 55%, transparent);
    }

    .ve-no {
        font-family: 'Cormorant Garamond', serif;
        font-style: italic;
        font-size: 14px;
        letter-spacing: 0.04em;
        text-transform: none;
        color: var(--color-ember-deep);
    }

    /* ── LINK — wraps the whole card (stage + caption) as one target ── */
    .vitrine-link {
        display: grid;
        justify-items: center;
        width: 100%;
        text-decoration: none;
        color: inherit;
        border-radius: 4px;
    }

    /* ── STAGE ───────────────────────────────────── */
    .vitrine-stage {
        position: relative;
        display: grid;
        justify-items: center;
        width: min(360px, 80vw);
        isolation: isolate;
    }

    /* ── CASE-WRAP: the box the light cone + dust are scaled to ──── */
    .case-wrap {
        position: relative;
        width: min(300px, 70vw);
        aspect-ratio: 4 / 5;
    }

    /* Warm cone of light falling from above onto the specimen. */
    .cone {
        position: absolute;
        top: -6%;
        left: 50%;
        transform: translateX(-50%);
        width: 150%;
        height: 116%;
        z-index: 0;
        pointer-events: none;
        background: radial-gradient(ellipse 38% 60% at 50% 8%, rgba(214,150,86,0.20) 0%, rgba(198,95,60,0.07) 36%, transparent 68%);
        clip-path: polygon(38% 0, 62% 0, 100% 100%, 0 100%);
        mix-blend-mode: multiply;
        opacity: 0.9;
    }

    .dust {
        position: absolute;
        inset: 0;
        z-index: 1;
        pointer-events: none;
        overflow: hidden;
    }

    .mote {
        position: absolute;
        bottom: 6%;
        border-radius: 50%;
        background: rgba(232,196,140,0.85);
        box-shadow: 0 0 4px rgba(232,196,140,0.6);
        opacity: 0;
        animation-name: mote-drift;
        animation-timing-function: linear;
        animation-iteration-count: infinite;
        will-change: transform, opacity;
    }

    @keyframes mote-drift {
        0%   { transform: translateY(0) translateX(0); opacity: 0; }
        12%  { opacity: 0.7; }
        80%  { opacity: 0.5; }
        100% { transform: translateY(-150px) translateX(10px); opacity: 0; }
    }

    /* ── THE CASE ITSELF — a glass box, not a picture frame ───────── */
    .case {
        position: absolute;
        inset: 0;
        z-index: 2;
        overflow: hidden;
        border-radius: 4px;
        border: 1px solid color-mix(in srgb, var(--color-ink-primary) 16%, transparent);
        background:
            radial-gradient(ellipse 80% 55% at 50% 6%, color-mix(in srgb, var(--color-ember) 12%, transparent) 0%, transparent 62%),
            linear-gradient(180deg, color-mix(in srgb, var(--color-ink-primary) 6%, var(--color-canvas-raised)) 0%, var(--color-canvas-raised) 60%, color-mix(in srgb, var(--color-ink-primary) 5%, var(--color-canvas-raised)) 100%);
        box-shadow:
            inset 0 1px 0 rgba(255,247,234,0.55),
            inset 1px 0 0 rgba(255,247,234,0.16),
            inset -1px 0 0 rgba(28,16,10,0.10),
            inset 0 -2px 6px rgba(28,16,10,0.10);
        transition: transform 0.5s var(--ease, cubic-bezier(0.16,1,0.3,1)), border-color 0.4s ease;
    }

    /* Metal cap along the top edge of the glass */
    .case::before {
        content: "";
        position: absolute;
        top: 0; left: 0; right: 0;
        height: 4px;
        z-index: 5;
        background: linear-gradient(180deg, rgba(255,247,234,0.6), color-mix(in srgb, var(--color-ember-deep) 65%, transparent));
    }

    /* A fixed glint of polished glass, upper corner */
    .case::after {
        content: "";
        position: absolute;
        top: 8%;
        right: 9%;
        width: 24%;
        height: 42%;
        z-index: 4;
        background: linear-gradient(120deg, transparent 30%, rgba(255,247,234,0.4) 48%, transparent 64%);
        filter: blur(3px);
        opacity: 0.6;
        pointer-events: none;
    }

    /* Slim brass uprights at the case's corners, holding the panes together */
    .case-post {
        position: absolute;
        top: 0;
        bottom: 0;
        width: 2px;
        z-index: 5;
        background: linear-gradient(180deg, color-mix(in srgb, var(--color-ember) 75%, transparent), color-mix(in srgb, var(--color-ember-deep) 55%, transparent) 55%, transparent 95%);
        opacity: 0.65;
    }
    .case-post--l { left: 0; }
    .case-post--r { right: 0; }

    /* ── THE SPECIMEN — a freestanding object, not a cropped photo ── */
    .specimen {
        position: absolute;
        inset: 0;
        z-index: 2;
        display: grid;
        padding: 12% 15% 19%;
    }

    .specimen :global(.specimen-img) {
        position: relative;
        z-index: 2;
        display: block;
        width: 100%;
        height: 100%;
        object-fit: contain;
        object-position: center bottom;
        filter: drop-shadow(0 12px 14px rgba(28,16,10,0.36)) saturate(1.05) contrast(1.02);
        cursor: inherit;
    }

    /* Contact shadow the figurine casts onto the case floor */
    .specimen-shadow {
        position: absolute;
        left: 50%;
        bottom: 17%;
        transform: translateX(-50%);
        width: 54%;
        height: 9%;
        z-index: 1;
        background: radial-gradient(ellipse at center, rgba(28,16,10,0.30) 0%, rgba(28,16,10,0.14) 45%, transparent 75%);
        filter: blur(1.5px);
        pointer-events: none;
    }

    /* The shelf line inside the case where the piece actually stands */
    .case-floor {
        position: absolute;
        left: 9%;
        right: 9%;
        bottom: 16%;
        height: 1px;
        z-index: 1;
        background: linear-gradient(90deg, transparent, color-mix(in srgb, var(--color-ink-primary) 22%, transparent) 18%, color-mix(in srgb, var(--color-ink-primary) 22%, transparent) 82%, transparent);
        pointer-events: none;
    }

    /* Slow blade of reflected light crossing the glass. */
    .case-sheen {
        position: absolute;
        inset: 0;
        z-index: 3;
        pointer-events: none;
        background: linear-gradient(115deg, transparent 40%, rgba(255,247,234,0.18) 49%, rgba(255,247,234,0.03) 55%, transparent 63%);
        transform: translateX(-120%);
        animation: sheen-sweep 10s ease-in-out infinite;
    }

    @keyframes sheen-sweep {
        0%, 62%  { transform: translateX(-120%); }
        82%      { transform: translateX(120%); }
        100%     { transform: translateX(120%); }
    }

    .vitrine-stage:hover .case {
        transform: translateY(-3px);
        border-color: color-mix(in srgb, var(--color-ember) 42%, transparent);
    }

    /* ── SHELF — the glass case sits on a glossy surface, faintly mirrored ── */
    .shelf {
        position: relative;
        z-index: 1;
        width: min(300px, 70vw);
        height: clamp(34px, 10vw, 70px);
        margin-top: -2px;
        overflow: hidden;
    }

    .shelf::before {
        content: "";
        position: absolute;
        top: 0; left: 10%; right: 10%;
        height: 1px;
        z-index: 2;
        background: linear-gradient(90deg, transparent, rgba(255,247,234,0.55) 30%, rgba(255,247,234,0.55) 70%, transparent);
    }

    .shelf :global(.shelf-reflection-img) {
        position: absolute;
        top: 0;
        left: 0;
        width: 100%;
        aspect-ratio: 4 / 5;
        object-fit: contain;
        object-position: center bottom;
        transform-origin: top center;
        transform: scaleY(-1);
        opacity: 0.16;
        -webkit-mask-image: linear-gradient(to bottom, rgba(0,0,0,0.55), transparent 80%);
        mask-image: linear-gradient(to bottom, rgba(0,0,0,0.55), transparent 80%);
        filter: blur(0.4px);
        pointer-events: none;
        transition: opacity 0.4s ease;
    }

    .vitrine-stage:hover .shelf :global(.shelf-reflection-img) {
        opacity: 0.24;
    }

    /* ── PLINTH + PLAQUE ─────────────────────────── */
    .plinth {
        position: relative;
        z-index: 2;
        display: grid;
        justify-items: center;
        width: min(320px, 76vw);
    }

    .plinth-slab {
        position: relative;
        width: 100%;
        height: 12px;
        background: linear-gradient(180deg, color-mix(in srgb, var(--color-ink-primary) 14%, transparent), transparent);
        border-top: 1px solid color-mix(in srgb, var(--color-ember-deep) 42%, transparent);
    }

    .plinth-slab::after {
        content: "";
        position: absolute;
        left: 50%;
        top: 9px;
        transform: translateX(-50%);
        width: 84%;
        height: 20px;
        background: radial-gradient(ellipse at center, rgba(44,23,16,0.22) 0%, transparent 72%);
        pointer-events: none;
    }

    .plinth-plaque {
        margin-top: 7px;
        padding: 4px 15px;
        border: 1px solid color-mix(in srgb, var(--color-ember) 38%, transparent);
        border-radius: 1px;
        background: linear-gradient(180deg, color-mix(in srgb, var(--color-ember) 14%, var(--color-canvas-raised)), var(--color-canvas-raised));
        font-size: 8px;
        font-weight: 600;
        letter-spacing: 0.24em;
        text-transform: uppercase;
        color: var(--color-ember-deep);
        box-shadow: 0 1px 2px rgba(28,16,10,0.14);
        white-space: nowrap;
    }

    /* ── CAPTION ─────────────────────────────────── */
    .vitrine-caption {
        display: grid;
        justify-items: center;
        gap: 9px;
        margin-top: clamp(22px, 3vw, 34px);
        max-width: 480px;
    }

    .vitrine-name {
        font-family: 'Cormorant Garamond', serif;
        font-weight: 300;
        font-style: italic;
        font-size: clamp(28px, 3vw, 40px);
        line-height: 1;
        color: var(--color-ink-primary);
    }

    .vitrine-meta {
        display: flex;
        flex-wrap: wrap;
        justify-content: center;
        gap: 7px;
        font-size: 11px;
        font-weight: 500;
        letter-spacing: 0.12em;
        text-transform: uppercase;
        color: var(--color-ink-tertiary);
    }

    .vitrine-meta .dot { color: color-mix(in srgb, var(--color-ember) 60%, transparent); }

    .vitrine-note {
        position: relative;
        margin: 4px 0 2px;
        padding: 0 6px;
        font-family: 'Cormorant Garamond', serif;
        font-style: italic;
        font-size: clamp(16px, 1.5vw, 19px);
        font-weight: 300;
        line-height: 1.46;
        color: var(--color-ink-secondary);
        quotes: "«" "»";
    }

    .vitrine-note::before { content: open-quote; }
    .vitrine-note::after  { content: close-quote; }

    .vitrine-enter {
        display: inline-flex;
        align-items: center;
        gap: 9px;
        margin-top: 8px;
        padding-bottom: 2px;
        font-size: 11px;
        font-weight: 600;
        letter-spacing: 0.12em;
        text-transform: uppercase;
        text-decoration: none;
        color: var(--color-ember-deep);
        border-bottom: 1px solid color-mix(in srgb, var(--color-ember) 40%, transparent);
        transition: color 0.22s ease, border-color 0.22s ease;
    }

    .vitrine-enter svg { transition: transform 0.22s ease; }
    .vitrine-link:hover .vitrine-enter { color: var(--color-ember); border-color: var(--color-ember); }
    .vitrine-link:hover .vitrine-enter svg { transform: translateX(4px); }

    .vitrine-link:focus-visible {
        outline: 2px solid color-mix(in srgb, var(--color-ember) 56%, transparent);
        outline-offset: 4px;
    }

    @media (prefers-reduced-motion: reduce) {
        .mote, .case-sheen { animation: none; }
        .mote { opacity: 0; }
        .case-sheen { display: none; }
    }
</style>