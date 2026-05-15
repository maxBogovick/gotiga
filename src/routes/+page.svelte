<script lang="ts">
    import { onMount } from 'svelte';
    import { goto } from '$app/navigation';
    import { fade, fly, scale } from 'svelte/transition';
    import { cubicOut, quartOut } from 'svelte/easing';
    import { spring } from 'svelte/motion';
    import { api } from '$lib/api';
    import type { CabinetZone } from '$lib/types/api';
    import { t } from '$lib/i18n';
    import LangSwitcher from '$lib/components/LangSwitcher.svelte';

    // --- State ---
    let zones = $state<CabinetZone[]>([]);
    let isLoaded = $state(false);
    let imageLoaded = $state(false);
    let hoveredZone = $state<CabinetZone | null>(null);
    let isNavigating = $state(false);
    let loadingProgress = $state(0);
    let ambientIntensity = $state(1);

    // Physics-based cursor and parallax
    const cursorSpring = spring({ x: 50, y: 50 }, { stiffness: 0.06, damping: 0.4 });
    const parallaxSpring = spring({ x: 0, y: 0 }, { stiffness: 0.04, damping: 0.45 });

    let imageUrl = $state('/images/cabinet-room.jpg');

    const DEFAULT_ZONES: CabinetZone[] = [
        { id: 'curator', zoneType: 'curator', x: 38, y: 15, width: 24, height: 75, targetRoute: '/author' },
        { id: 'cabinet', zoneType: 'cabinet', x: 76, y: 42, width: 20, height: 45, targetRoute: '/figurines' },
        { id: 'portrait', zoneType: 'portrait', x: 8, y: 28, width: 15, height: 30, targetRoute: '/author' },
        { id: 'windows', zoneType: 'windows', x: 25, y: 2, width: 50, height: 20, targetRoute: '/workshop' },
    ];

    const ZONE_STATIC: Record<string, { icon: string; accent: string; labelKey: string; descKey: string }> = {
        showcase: { icon: '🕯', accent: '#9b8b7e', labelKey: 'zoneShowcase', descKey: 'zoneShowcaseDesc' },
        desk:     { icon: '🌙', accent: '#8b8b9b', labelKey: 'zoneDesk',     descKey: 'zoneDeskDesc'     },
        shelf:    { icon: '🗝', accent: '#a0937e', labelKey: 'zoneShelf',    descKey: 'zoneShelfDesc'    },
        note:     { icon: '⚰', accent: '#8b7355', labelKey: 'zoneNote',     descKey: 'zoneNoteDesc'     },
        curator:  { icon: '⚰', accent: '#8b7355', labelKey: 'zoneCurator',  descKey: 'zoneCuratorDesc'  },
        cabinet:  { icon: '🕯', accent: '#9b8b7e', labelKey: 'zoneCabinet',  descKey: 'zoneCabinetDesc'  },
        portrait: { icon: '🗝', accent: '#a0937e', labelKey: 'zonePortrait', descKey: 'zonePortraitDesc' },
        windows:  { icon: '🌙', accent: '#8b8b9b', labelKey: 'zoneWindows',  descKey: 'zoneWindowsDesc'  },
    };

    // Build translated ZONE_DATA reactively
    let ZONE_DATA = $derived(
        Object.fromEntries(
            Object.entries(ZONE_STATIC).map(([k, v]) => [k, {
                label: $t(v.labelKey as import('$lib/i18n').TranslationKey),
                description: $t(v.descKey as import('$lib/i18n').TranslationKey),
                icon: v.icon,
                accent: v.accent,
            }])
        ) as Record<string, { label: string; description: string; icon: string; accent: string }>
    );

    function getZoneData(zoneType: string) {
        return ZONE_DATA[zoneType] ?? { label: zoneType.toUpperCase(), description: $t('zoneExplore'), icon: '✦', accent: '#8b7355' };
    }

    // --- Logic ---
    async function init() {
        try {
            // Simulate progressive loading
            const progressInterval = setInterval(() => {
                loadingProgress = Math.min(loadingProgress + Math.random() * 12, 100);
                if (loadingProgress >= 100) clearInterval(progressInterval);
            }, 120);

            // Fetch zones and background image
            const [dbZones, bgPath] = await Promise.all([
                api.getCabinetZones().catch(() => DEFAULT_ZONES),
                api.getMainBackground().catch(() => null)
            ]);
            
            if (bgPath) {
                imageUrl = bgPath;
            }

            await preloadImage(imageUrl);

            zones = dbZones && dbZones.length > 0 ? dbZones : DEFAULT_ZONES;

            // Wait for loading animation
            await new Promise(resolve => {
                const checkComplete = setInterval(() => {
                    if (loadingProgress >= 100) {
                        clearInterval(checkComplete);
                        setTimeout(resolve, 600);
                    }
                }, 50);
            });

            isLoaded = true;
        } catch (e) {
            zones = DEFAULT_ZONES;
            isLoaded = true;
        }
    }

    function preloadImage(url: string): Promise<void> {
        return new Promise((resolve) => {
            const img = new Image();
            img.onload = () => { imageLoaded = true; resolve(); };
            img.onerror = () => { imageLoaded = true; resolve(); };
            img.src = url;
        });
    }

    function handleMouseMove(e: MouseEvent) {
        const { innerWidth, innerHeight } = window;
        cursorSpring.set({
            x: (e.clientX / innerWidth) * 100,
            y: (e.clientY / innerHeight) * 100
        });

        parallaxSpring.set({
            x: (e.clientX / innerWidth - 0.5) * 2,
            y: (e.clientY / innerHeight - 0.5) * 2
        });
    }

    function handleTouchMove(e: TouchEvent) {
        if (e.touches.length === 0) return;
        const { innerWidth, innerHeight } = window;
        const t = e.touches[0];
        parallaxSpring.set({
            x: (t.clientX / innerWidth - 0.5) * 2,
            y: (t.clientY / innerHeight - 0.5) * 2
        });
    }

    async function handleZoneInteraction(zone: CabinetZone) {
        if (isNavigating) return;
        isNavigating = true;
        await goto(zone.targetRoute);
    }

    let showHint = $state(false);
    let hintDismissed = $state(false);

    onMount(() => {
        init();

        // Show hint after 3s if user hasn't hovered any zone yet
        const hintTimer = setTimeout(() => {
            if (!hoveredZone) showHint = true;
        }, 3000);

        // Ambient flicker effect
        const flickerInterval = setInterval(() => {
            ambientIntensity = 0.92 + Math.random() * 0.08;
        }, 150);

        return () => {
            clearInterval(flickerInterval);
            clearTimeout(hintTimer);
        };
    });
</script>

<svelte:head>
    <title>Gothic Museum - Cabinet of Curiosities</title>
    <meta name="theme-color" content="#0a0806" />
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous">
    <link href="https://fonts.googleapis.com/css2?family=Cinzel:wght@400;600;700&family=Playfair+Display:wght@700;900&family=UnifrakturMaguntia&family=Cormorant+Garamond:ital,wght@0,400;0,600;1,400&display=swap" rel="stylesheet">
</svelte:head>

<svelte:window onmousemove={handleMouseMove} ontouchmove={handleTouchMove} />

<div class="main-wrapper" ontouchmove={handleTouchMove}>

    <!-- Ambient particles -->
    <div class="particles-container">
        {#each Array(35) as _, i}
            <div
                    class="particle"
                    style="
                    --delay: {i * 0.5}s;
                    --duration: {20 + i * 0.4}s;
                    --start-x: {10 + i * 2.5}%;
                    --end-x: {15 + i * 2.3}%;
                "
            ></div>
        {/each}
    </div>

    <!-- Loading Screen -->
    {#if !isLoaded}
        <div class="loading-screen" out:fade={{ duration: 1400, easing: quartOut }}>
            <div class="loading-content">
                <!-- Ornate frame -->
                <div class="loading-frame">
                    <div class="frame-corner tl"></div>
                    <div class="frame-corner tr"></div>
                    <div class="frame-corner bl"></div>
                    <div class="frame-corner br"></div>
                </div>

                <!-- Animated skull -->
                <div class="skull-wrapper">
                    <div class="skull-glow"></div>
                    <div class="skull-icon">☠</div>
                </div>

                <!-- Title -->
                <h1 class="loading-title">
                    {#each 'GOTHIC MUSEUM'.split('') as char, i}
                        {#if char === ' '}
                            <span class="letter-space"></span>
                        {:else}
                            <span class="letter" style="--index: {i}">{char}</span>
                        {/if}
                    {/each}
                </h1>

                <!-- Subtitle -->
                <div class="loading-subtitle">
                    <span class="ornament">⚜</span>
                    <span class="subtitle-text">Cabinet of Curiosities</span>
                    <span class="ornament">⚜</span>
                </div>

                <!-- Progress bar -->
                <div class="progress-container">
                    <div class="progress-bar">
                        <div class="progress-fill" style="width: {loadingProgress}%"></div>
                        <div class="progress-glow" style="left: {loadingProgress}%"></div>
                    </div>
                    <div class="progress-text">{Math.floor(loadingProgress)}%</div>
                </div>

                <!-- Mystical eyes -->
                <div class="mystical-eyes">
                    {#each Array(6) as _, i}
                        <div class="eye" style="--eye-delay: {i * 0.7}s; --eye-pos: {i}"></div>
                    {/each}
                </div>
            </div>

            <!-- Atmospheric fog -->
            <div class="loading-fog fog-1"></div>
            <div class="loading-fog fog-2"></div>
        </div>
    {/if}

    {#if imageLoaded}
        <main class="museum-main" in:fade={{ duration: 2000, delay: 200 }}>

            <!-- Cinematic bars with ornaments -->
            <div class="cinema-bar top-bar">
                <div class="bar-ornament left">◆ MDCCCXC ◆</div>
                <div class="bar-title">GOTHIC MUSEUM</div>
                <div class="bar-ornament right">◆ EST ◆</div>
                
                <div class="absolute right-16 top-1/2 -translate-y-1/2 opacity-40 hover:opacity-100 transition-opacity duration-300">
                    <LangSwitcher />
                </div>
                <a href="/admin" class="absolute right-8 opacity-10 hover:opacity-80 transition-opacity duration-500 text-xl" aria-label="Admin entrance">
                    🗝
                </a>
            </div>

            <div class="cinema-bar bottom-bar">
                {#if hoveredZone && ZONE_DATA[hoveredZone.zoneType]}
                    <!-- Hovered zone info -->
                    <div class="bar-zone-info" in:fade={{ duration: 200 }}>
                        <span class="bar-zone-icon">{ZONE_DATA[hoveredZone.zoneType].icon}</span>
                        <div class="bar-zone-text">
                            <span class="bar-zone-name">{ZONE_DATA[hoveredZone.zoneType].label}</span>
                            <span class="bar-zone-desc">{ZONE_DATA[hoveredZone.zoneType].description}</span>
                        </div>
                    </div>
                {:else}
                    <!-- Bottom navigation: unique routes -->
                    <nav class="bottom-nav" in:fade={{ duration: 300 }}>
                        {#each [...new Map(zones.map(z => [z.targetRoute, z])).values()] as zone}
                            {@const zd = ZONE_DATA[zone.zoneType]}
                            {#if zd}
                                <a href={zone.targetRoute} class="bottom-nav-item">
                                    <span class="bottom-nav-icon">{zd.icon}</span>
                                    <span class="bottom-nav-label">{zd.label}</span>
                                </a>
                            {/if}
                        {/each}
                    </nav>
                {/if}
            </div>

            <!-- Main image container with 3D parallax -->
            <div
                    class="image-container"
                    style="
                    transform:
                        perspective(2000px)
                        rotateY({$parallaxSpring.x * -2}deg)
                        rotateX({$parallaxSpring.y * 2}deg)
                        scale(1.03);
                    filter: brightness({ambientIntensity});
                "
            >
                <!-- Dynamic light following cursor -->
                <div
                        class="cursor-light"
                        style="
                        left: {$cursorSpring.x}%;
                        top: {$cursorSpring.y}%;
                        opacity: {ambientIntensity * 0.7};
                    "
                ></div>

                <!-- Depth layers -->
                <div class="depth-layer depth-1"></div>
                <div class="depth-layer depth-2"></div>
                <div class="depth-layer depth-3"></div>

                <img
                        src={imageUrl}
                        alt="Gothic Museum Interior"
                        class="museum-image"
                        draggable="false"
                />

                <!-- Animated fog layers -->
                <div class="fog-layer fog-layer-1"></div>
                <div class="fog-layer fog-layer-2"></div>
                <div class="fog-layer fog-layer-3"></div>

                <!-- Vignette effects -->
                <div class="vignette-effect"></div>
                <div class="noise-overlay"></div>

                <!-- Interactive zones -->
                {#if isLoaded}
                    <div class="zones-overlay">
                        {#each zones as zone, i (zone.id)}
                            {@render zoneButton(zone, i)}
                        {/each}
                    </div>
                {/if}
            </div>

        </main>
    {/if}

    <!-- Enhanced custom cursor -->
    <div class="cursor-system">
        <div
                class="cursor-outer"
                class:cursor-active={hoveredZone}
                style="left: {$cursorSpring.x}%; top: {$cursorSpring.y}%;"
        >
            <div class="cursor-ring"></div>
            <div class="cursor-ring-2"></div>
        </div>
        <div
                class="cursor-dot"
                style="left: {$cursorSpring.x}%; top: {$cursorSpring.y}%;"
        ></div>
    </div>

</div>

{#snippet zoneButton(zone: CabinetZone, index: number)}
    {@const zd = ZONE_DATA[zone.zoneType]}
    <button
            class="zone-button"
            style="
            left: {zone.x}%;
            top: {zone.y}%;
            width: {zone.width}%;
            height: {zone.height}%;
            --zone-delay: {index * 0.8}s;
        "
            onclick={() => handleZoneInteraction(zone)}
            onmouseenter={() => { hoveredZone = zone; hintDismissed = true; }}
            onmouseleave={() => hoveredZone = null}
            ontouchstart={() => { hintDismissed = true; }}
            disabled={isNavigating}
            aria-label={zd?.label}
    >
        <!-- Breathing guide -->
        <div class="zone-guide"></div>

        <!-- Corner markers -->
        <div class="zone-corners">
            <span class="corner tl"></span>
            <span class="corner tr"></span>
            <span class="corner bl"></span>
            <span class="corner br"></span>
        </div>

        <!-- Mystical shimmer -->
        <div class="zone-shimmer"></div>

        <!-- Hover glow -->
        <div class="zone-glow"></div>

        <!-- Runes -->
        <div class="zone-runes">
            <span class="rune r1">✦</span>
            <span class="rune r2">✦</span>
            <span class="rune r3">✦</span>
            <span class="rune r4">✦</span>
        </div>

        <!-- Floating label — always visible at low opacity, full on hover -->
        {#if zd}
            <div class="zone-label">
                <span class="zone-label-icon">{zd.icon}</span>
                <span class="zone-label-name">{zd.label}</span>
                <span class="zone-label-desc">{zd.description}</span>
            </div>
        {/if}
    </button>
{/snippet}

<style>
    @import url('https://fonts.googleapis.com/css2?family=Cinzel:wght@400;600;700&family=Playfair+Display:wght@700;900&family=UnifrakturMaguntia&family=Cormorant+Garamond:ital,wght@0,400;0,600;1,400&display=swap');

    * {
        margin: 0;
        padding: 0;
        box-sizing: border-box;
    }

    .main-wrapper {
        width: 100vw;
        height: 100vh;
        background: radial-gradient(ellipse at center, #1a1612 0%, #0a0806 70%);
        overflow: hidden;
        cursor: none;
        position: relative;
        font-family: 'Cinzel', serif;
        color: #d4c5b0;
    }

    /* === PARTICLES === */
    .particles-container {
        position: fixed;
        inset: 0;
        pointer-events: none;
        z-index: 1;
        overflow: hidden;
    }

    .particle {
        position: absolute;
        width: 2px;
        height: 2px;
        background: rgba(212, 197, 176, 0.3);
        border-radius: 50%;
        left: var(--start-x);
        top: -10px;
        animation: particle-drift var(--duration) ease-in-out infinite;
        animation-delay: var(--delay);
        box-shadow: 0 0 6px rgba(212, 197, 176, 0.4);
    }

    @keyframes particle-drift {
        0% {
            transform: translate(0, 0) rotate(0deg);
            opacity: 0;
        }
        10% {
            opacity: 0.6;
        }
        90% {
            opacity: 0.3;
        }
        100% {
            transform: translate(var(--end-x), 110vh) rotate(360deg);
            opacity: 0;
        }
    }

    /* === LOADING SCREEN === */
    .loading-screen {
        position: fixed;
        inset: 0;
        background: radial-gradient(ellipse at center, #1a1612 0%, #0a0806 70%);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 100;
    }

    .loading-content {
        position: relative;
        text-align: center;
        padding: 60px;
    }

    .loading-frame {
        position: absolute;
        inset: 0;
        pointer-events: none;
    }

    .frame-corner {
        position: absolute;
        width: 80px;
        height: 80px;
        border: 2px solid rgba(212, 197, 176, 0.3);
    }

    .frame-corner.tl {
        top: 0;
        left: 0;
        border-right: none;
        border-bottom: none;
    }

    .frame-corner.tr {
        top: 0;
        right: 0;
        border-left: none;
        border-bottom: none;
    }

    .frame-corner.bl {
        bottom: 0;
        left: 0;
        border-right: none;
        border-top: none;
    }

    .frame-corner.br {
        bottom: 0;
        right: 0;
        border-left: none;
        border-top: none;
    }

    .skull-wrapper {
        position: relative;
        display: inline-block;
        margin-bottom: 40px;
    }

    .skull-icon {
        font-size: 100px;
        position: relative;
        z-index: 2;
        animation: skull-levitate 5s ease-in-out infinite;
        filter: drop-shadow(0 0 40px rgba(212, 197, 176, 0.6));
    }

    .skull-glow {
        position: absolute;
        inset: -30px;
        background: radial-gradient(circle, rgba(212, 197, 176, 0.3), transparent 70%);
        animation: glow-breathe 3s ease-in-out infinite;
        z-index: 1;
    }

    @keyframes skull-levitate {
        0%, 100% {
            transform: translateY(0) rotate(-2deg);
        }
        50% {
            transform: translateY(-30px) rotate(2deg);
        }
    }

    @keyframes glow-breathe {
        0%, 100% {
            opacity: 0.4;
            transform: scale(0.9);
        }
        50% {
            opacity: 0.8;
            transform: scale(1.2);
        }
    }

    .loading-title {
        font-family: 'UnifrakturMaguntia', cursive;
        font-size: 56px;
        letter-spacing: 14px;
        margin-bottom: 20px;
        display: flex;
        justify-content: center;
        flex-wrap: wrap;
        gap: 4px;
    }

    .letter {
        display: inline-block;
        color: #d4c5b0;
        text-shadow: 0 0 20px rgba(212, 197, 176, 0.8);
        animation: letter-emerge 1.5s ease-out forwards;
        animation-delay: calc(var(--index) * 0.1s);
        opacity: 0;
        transform: translateY(20px);
    }

    .letter-space {
        width: 20px;
    }

    @keyframes letter-emerge {
        to {
            opacity: 1;
            transform: translateY(0);
        }
    }

    .loading-subtitle {
        font-family: 'Cormorant Garamond', serif;
        font-size: 20px;
        font-style: italic;
        letter-spacing: 4px;
        margin-bottom: 50px;
        color: rgba(212, 197, 176, 0.6);
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 15px;
    }

    .ornament {
        font-size: 24px;
        animation: ornament-spin 10s linear infinite;
    }

    @keyframes ornament-spin {
        to {
            transform: rotate(360deg);
        }
    }

    .subtitle-text {
        animation: subtitle-fade 2s ease-in-out infinite;
    }

    @keyframes subtitle-fade {
        0%, 100% {
            opacity: 0.6;
        }
        50% {
            opacity: 1;
        }
    }

    .progress-container {
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 20px;
        margin-top: 40px;
    }

    .progress-bar {
        width: 400px;
        height: 4px;
        background: rgba(212, 197, 176, 0.15);
        border-radius: 2px;
        position: relative;
        overflow: hidden;
        box-shadow: inset 0 0 10px rgba(0, 0, 0, 0.5);
    }

    .progress-fill {
        height: 100%;
        background: linear-gradient(90deg,
        #8b7355 0%,
        #d4c5b0 50%,
        #8b7355 100%
        );
        transition: width 0.3s ease;
        box-shadow: 0 0 20px rgba(212, 197, 176, 0.6);
    }

    .progress-glow {
        position: absolute;
        top: -15px;
        width: 50px;
        height: 34px;
        background: radial-gradient(circle, rgba(212, 197, 176, 0.8), transparent 70%);
        margin-left: -25px;
        filter: blur(10px);
        transition: left 0.3s ease;
        pointer-events: none;
    }

    .progress-text {
        font-family: 'Cinzel', serif;
        font-size: 20px;
        font-weight: 600;
        color: #d4c5b0;
        min-width: 60px;
        text-align: left;
    }

    .mystical-eyes {
        position: absolute;
        inset: 0;
        pointer-events: none;
    }

    .eye {
        position: absolute;
        width: 22px;
        height: 30px;
        background: radial-gradient(ellipse,
        rgba(212, 197, 176, 0.8) 20%,
        rgba(139, 115, 85, 0.6) 40%,
        transparent 70%
        );
        border-radius: 50%;
        left: calc(10% + var(--eye-pos) * 14%);
        top: calc(20% + var(--eye-pos) * 12%);
        animation: eye-watch 6s ease-in-out infinite;
        animation-delay: var(--eye-delay);
        opacity: 0.5;
    }

    @keyframes eye-watch {
        0%, 90%, 100% {
            opacity: 0.5;
            transform: scaleY(1);
        }
        94%, 96% {
            opacity: 0.1;
            transform: scaleY(0.1);
        }
    }

    .loading-fog {
        position: absolute;
        inset: 0;
        pointer-events: none;
    }

    .fog-1 {
        background: radial-gradient(ellipse at 25% 50%,
        rgba(120, 110, 100, 0.08) 0%,
        transparent 60%
        );
        animation: fog-drift-1 20s ease-in-out infinite;
    }

    .fog-2 {
        background: radial-gradient(ellipse at 75% 40%,
        rgba(100, 95, 90, 0.06) 0%,
        transparent 65%
        );
        animation: fog-drift-2 25s ease-in-out infinite;
    }

    @keyframes fog-drift-1 {
        0%, 100% {
            opacity: 0.4;
            transform: translateX(0) scale(1);
        }
        50% {
            opacity: 0.7;
            transform: translateX(40px) scale(1.1);
        }
    }

    @keyframes fog-drift-2 {
        0%, 100% {
            opacity: 0.3;
            transform: translateX(0) scale(1);
        }
        50% {
            opacity: 0.6;
            transform: translateX(-30px) scale(1.15);
        }
    }

    /* === MAIN MUSEUM === */
    .museum-main {
        width: 100%;
        height: 100%;
        display: flex;
        align-items: center;
        justify-content: center;
        position: relative;
    }

    .cinema-bar {
        position: fixed;
        left: 0;
        width: 100%;
        height: 100px;
        background: linear-gradient(to bottom,
        rgba(10, 8, 6, 0.95) 0%,
        rgba(10, 8, 6, 0.7) 100%
        );
        z-index: 10;
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 0 50px;
        backdrop-filter: blur(10px);
    }

    .top-bar {
        top: 0;
        border-bottom: 1px solid rgba(212, 197, 176, 0.2);
    }

    .bottom-bar {
        bottom: 0;
        height: 64px;
        border-top: 1px solid rgba(212, 197, 176, 0.18);
        justify-content: center;
        padding: 0;
        background: linear-gradient(to top,
            rgba(10, 8, 6, 0.98) 0%,
            rgba(10, 8, 6, 0.82) 100%
        );
    }

    .bar-ornament {
        font-family: 'Cinzel', serif;
        font-size: 12px;
        letter-spacing: 3px;
        color: rgba(212, 197, 176, 0.5);
        animation: ornament-glow 3s ease-in-out infinite;
    }

    @keyframes ornament-glow {
        0%, 100% {
            opacity: 0.5;
        }
        50% {
            opacity: 0.9;
            text-shadow: 0 0 15px rgba(212, 197, 176, 0.6);
        }
    }

    .bar-title {
        font-family: 'UnifrakturMaguntia', cursive;
        font-size: 28px;
        letter-spacing: 8px;
        color: #d4c5b0;
        text-shadow: 0 0 20px rgba(212, 197, 176, 0.5);
    }

    .bar-hint {
        display: flex;
        align-items: center;
        gap: 15px;
        font-family: 'Cormorant Garamond', serif;
        font-size: 16px;
        font-style: italic;
        letter-spacing: 2px;
        color: rgba(212, 197, 176, 0.5);
    }

    .hint-icon {
        font-size: 24px;
        animation: hint-pulse 3s ease-in-out infinite;
    }

    @keyframes hint-pulse {
        0%, 100% {
            transform: translateY(0) scale(1);
            opacity: 0.6;
        }
        50% {
            transform: translateY(-5px) scale(1.1);
            opacity: 1;
        }
    }

    .hint-text {
        animation: hint-fade 4s ease-in-out infinite;
    }

    @keyframes hint-fade {
        0%, 100% {
            opacity: 0.4;
        }
        50% {
            opacity: 0.8;
        }
    }

    /* === IMAGE CONTAINER === */
    .image-container {
        position: relative;
        width: 92vw;
        max-width: 1500px;
        height: 75vh;
        transition: transform 0.1s ease, filter 0.2s ease;
        transform-style: preserve-3d;
    }

    .museum-image {
        width: 100%;
        height: 100%;
        object-fit: contain;
        position: relative;
        z-index: 2;
        filter: drop-shadow(0 30px 80px rgba(0, 0, 0, 0.8));
    }

    /* Cursor light */
    .cursor-light {
        position: absolute;
        width: 800px;
        height: 800px;
        margin: -400px 0 0 -400px;
        background: radial-gradient(circle,
        rgba(212, 197, 176, 0.25) 0%,
        rgba(212, 197, 176, 0.12) 30%,
        transparent 60%
        );
        pointer-events: none;
        z-index: 3;
        transition: left 0.2s ease, top 0.2s ease, opacity 0.3s ease;
        mix-blend-mode: screen;
    }

    /* Depth layers */
    .depth-layer {
        position: absolute;
        inset: 0;
        pointer-events: none;
    }

    .depth-1 {
        background: radial-gradient(ellipse at 35% 45%,
        transparent 28%,
        rgba(0, 0, 0, 0.6) 90%
        );
        z-index: 4;
    }

    .depth-2 {
        background: radial-gradient(ellipse at 65% 55%,
        transparent 35%,
        rgba(0, 0, 0, 0.5) 95%
        );
        z-index: 5;
    }

    .depth-3 {
        background: linear-gradient(to bottom,
        rgba(0, 0, 0, 0.4) 0%,
        transparent 25%,
        transparent 75%,
        rgba(0, 0, 0, 0.7) 100%
        );
        z-index: 6;
    }

    /* Fog layers */
    .fog-layer {
        position: absolute;
        inset: 0;
        pointer-events: none;
        z-index: 7;
    }

    .fog-layer-1 {
        background: radial-gradient(ellipse at 20% 55%,
        rgba(140, 130, 120, 0.06) 0%,
        transparent 55%
        );
        animation: fog-move-1 30s ease-in-out infinite;
    }

    .fog-layer-2 {
        background: radial-gradient(ellipse at 80% 35%,
        rgba(120, 110, 105, 0.05) 0%,
        transparent 60%
        );
        animation: fog-move-2 35s ease-in-out infinite;
    }

    .fog-layer-3 {
        background: radial-gradient(ellipse at 50% 75%,
        rgba(130, 125, 115, 0.04) 0%,
        transparent 65%
        );
        animation: fog-move-3 40s ease-in-out infinite;
    }

    @keyframes fog-move-1 {
        0%, 100% {
            opacity: 0.5;
            transform: translate(0, 0) scale(1);
        }
        50% {
            opacity: 0.8;
            transform: translate(25px, -15px) scale(1.12);
        }
    }

    @keyframes fog-move-2 {
        0%, 100% {
            opacity: 0.4;
            transform: translate(0, 0) scale(1);
        }
        50% {
            opacity: 0.7;
            transform: translate(-30px, 20px) scale(1.15);
        }
    }

    @keyframes fog-move-3 {
        0%, 100% {
            opacity: 0.3;
            transform: translate(0, 0) scale(1);
        }
        50% {
            opacity: 0.6;
            transform: translate(15px, -10px) scale(1.08);
        }
    }

    /* Vignette and noise */
    .vignette-effect {
        position: absolute;
        inset: 0;
        background: radial-gradient(ellipse at center,
        transparent 25%,
        rgba(10, 8, 6, 0.6) 100%
        );
        z-index: 8;
        pointer-events: none;
    }

    .noise-overlay {
        position: absolute;
        inset: 0;
        background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E");
        opacity: 0.08;
        mix-blend-mode: overlay;
        z-index: 9;
        pointer-events: none;
    }

    /* Zones overlay */
    .zones-overlay {
        position: absolute;
        inset: 0;
        z-index: 20;
    }

    /* === ZONE BUTTON === */
    .zone-button {
        position: absolute;
        background: transparent;
        border: none;
        cursor: none;
        transition: all 0.5s cubic-bezier(0.4, 0, 0.2, 1);
        outline: none;
    }

    .zone-button:focus {
        outline: none;
    }

    .zone-guide {
        position: absolute;
        inset: -15%;
        background: radial-gradient(ellipse at center,
        rgba(212, 197, 176, 0.15) 0%,
        transparent 70%
        );
        opacity: 0.4;
        animation: guide-breathe 4s ease-in-out infinite;
        animation-delay: var(--zone-delay);
        border-radius: 50%;
    }

    .zone-button:hover .zone-guide {
        opacity: 0;
    }

    @keyframes guide-breathe {
        0%, 100% {
            transform: scale(0.85);
            opacity: 0.3;
        }
        50% {
            transform: scale(1.2);
            opacity: 0.7;
        }
    }

    .zone-corners {
        position: absolute;
        inset: 0;
    }

    .corner {
        position: absolute;
        width: 20px;
        height: 20px;
        border: 2px solid rgba(212, 197, 176, 0.4);
        transition: all 0.5s ease;
        opacity: 0.6;
    }

    .corner.tl {
        top: -5px;
        left: -5px;
        border-right: none;
        border-bottom: none;
    }

    .corner.tr {
        top: -5px;
        right: -5px;
        border-left: none;
        border-bottom: none;
    }

    .corner.bl {
        bottom: -5px;
        left: -5px;
        border-right: none;
        border-top: none;
    }

    .corner.br {
        bottom: -5px;
        right: -5px;
        border-left: none;
        border-top: none;
    }

    .zone-button:hover .corner {
        border-color: rgba(212, 197, 176, 0.9);
        opacity: 1;
        animation: corner-expand 0.6s ease-out forwards;
    }

    @keyframes corner-expand {
        0% {
            width: 20px;
            height: 20px;
        }
        100% {
            width: 35px;
            height: 35px;
        }
    }

    .zone-shimmer {
        position: absolute;
        inset: 0;
        overflow: hidden;
    }

    .zone-shimmer::before {
        content: '';
        position: absolute;
        inset: -100%;
        background: linear-gradient(90deg,
        transparent 0%,
        rgba(212, 197, 176, 0.25) 50%,
        transparent 100%
        );
        transform: translateX(-150%) skewX(-25deg);
        animation: shimmer-sweep 8s ease-in-out infinite;
        animation-delay: var(--zone-delay);
    }

    @keyframes shimmer-sweep {
        0%, 10% {
            transform: translateX(-150%) skewX(-25deg);
        }
        20%, 100% {
            transform: translateX(250%) skewX(-25deg);
        }
    }

    .zone-glow {
        position: absolute;
        inset: -25%;
        background: radial-gradient(ellipse at center,
        rgba(212, 197, 176, 0.3) 0%,
        transparent 65%
        );
        opacity: 0;
        transition: opacity 0.6s ease;
        filter: blur(20px);
    }

    .zone-button:hover .zone-glow {
        opacity: 1;
        animation: glow-pulse 2s ease-in-out infinite;
    }

    @keyframes glow-pulse {
        0%, 100% {
            transform: scale(0.9);
        }
        50% {
            transform: scale(1.15);
        }
    }

    .zone-runes {
        position: absolute;
        inset: 0;
    }

    .rune {
        position: absolute;
        font-size: 26px;
        color: rgba(212, 197, 176, 0.6);
        opacity: 0;
        transition: all 0.6s ease;
        text-shadow: 0 0 12px rgba(212, 197, 176, 0.8);
    }

    .rune.r1 {
        top: -15px;
        left: -15px;
    }

    .rune.r2 {
        top: -15px;
        right: -15px;
    }

    .rune.r3 {
        bottom: -15px;
        left: -15px;
    }

    .rune.r4 {
        bottom: -15px;
        right: -15px;
    }

    .zone-button:hover .rune {
        opacity: 1;
        animation: rune-spin 3s linear infinite;
    }

    @keyframes rune-spin {
        0% {
            transform: rotate(0deg) scale(1);
        }
        50% {
            transform: rotate(180deg) scale(1.2);
        }
        100% {
            transform: rotate(360deg) scale(1);
        }
    }

    /* === CURSOR SYSTEM === */
    .cursor-system {
        position: fixed;
        inset: 0;
        pointer-events: none;
        z-index: 999;
    }

    .cursor-outer {
        position: absolute;
        transition: left 0.12s ease, top 0.12s ease;
    }

    .cursor-ring {
        position: absolute;
        width: 45px;
        height: 45px;
        margin: -22.5px 0 0 -22.5px;
        border: 2px solid rgba(212, 197, 176, 0.5);
        border-radius: 50%;
        transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1);
    }

    .cursor-ring-2 {
        position: absolute;
        width: 55px;
        height: 55px;
        margin: -27.5px 0 0 -27.5px;
        border: 1px solid rgba(212, 197, 176, 0.2);
        border-radius: 50%;
        transition: all 0.5s cubic-bezier(0.4, 0, 0.2, 1);
    }

    .cursor-outer.cursor-active .cursor-ring {
        width: 70px;
        height: 70px;
        margin: -35px 0 0 -35px;
        border-color: rgba(212, 197, 176, 0.9);
        box-shadow: 0 0 25px rgba(212, 197, 176, 0.5);
        transform: rotate(45deg);
        border-radius: 0;
    }

    .cursor-outer.cursor-active .cursor-ring-2 {
        width: 85px;
        height: 85px;
        margin: -42.5px 0 0 -42.5px;
        border-color: rgba(212, 197, 176, 0.4);
        transform: rotate(-45deg);
        border-radius: 0;
    }

    .cursor-dot {
        position: absolute;
        width: 6px;
        height: 6px;
        margin: -3px 0 0 -3px;
        background: #d4c5b0;
        border-radius: 50%;
        box-shadow: 0 0 15px rgba(212, 197, 176, 0.9);
        transition: left 0.08s ease, top 0.08s ease;
        z-index: 1000;
    }

    /* Global styles */
    :global(body) {
        background: #0a0806;
    }

    /* === ZONE LABELS (Вариант 1) === */
    .zone-label {
        position: absolute;
        bottom: 12%;
        left: 50%;
        transform: translateX(-50%);
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 6px;
        opacity: 0.75;
        transition: opacity 0.4s ease, transform 0.4s ease;
        pointer-events: none;
        text-align: center;
        white-space: nowrap;
    }

    .zone-button:hover .zone-label {
        opacity: 1;
        transform: translateX(-50%) translateY(-8px);
    }

    .zone-label-icon {
        font-size: 22px;
        filter: drop-shadow(0 0 8px rgba(212, 197, 176, 0.7));
        transition: filter 0.4s ease;
    }

    .zone-button:hover .zone-label-icon {
        filter: drop-shadow(0 0 16px rgba(255, 220, 150, 0.9));
    }

    .zone-label-name {
        font-family: 'Cinzel', serif;
        font-size: 10px;
        letter-spacing: 0.35em;
        color: #e6decb;
        text-transform: uppercase;
        text-shadow:
            0 0 12px rgba(0,0,0,0.9),
            0 1px 3px rgba(0,0,0,1),
            0 0 20px rgba(212, 197, 176, 0.4);
        background: rgba(0,0,0,0.45);
        padding: 3px 10px 3px 12px;
        transition: text-shadow 0.4s ease, background 0.4s ease;
    }

    .zone-button:hover .zone-label-name {
        color: #fff;
        background: rgba(0,0,0,0.65);
        text-shadow:
            0 0 20px rgba(212, 197, 176, 0.9),
            0 1px 3px rgba(0,0,0,1);
    }

    .zone-label-desc {
        font-family: 'Cormorant Garamond', serif;
        font-size: 12px;
        letter-spacing: 0.08em;
        color: rgba(212, 197, 176, 0.85);
        font-style: italic;
        opacity: 0;
        transform: translateY(5px);
        transition: opacity 0.35s ease 0.05s, transform 0.35s ease 0.05s;
        text-shadow: 0 1px 4px rgba(0,0,0,1), 0 0 12px rgba(0,0,0,0.9);
        background: rgba(0,0,0,0.5);
        padding: 2px 8px 2px 10px;
    }

    .zone-button:hover .zone-label-desc {
        opacity: 1;
        transform: translateY(0);
    }

    /* === BOTTOM NAVIGATION (Вариант 3) === */
    .bottom-nav {
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 0;
        height: 100%;
    }

    .bottom-nav-item {
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 0 36px;
        height: 100%;
        text-decoration: none;
        color: rgba(212, 197, 176, 0.75);
        font-family: 'Cinzel', serif;
        font-size: 11px;
        letter-spacing: 0.3em;
        text-transform: uppercase;
        border-right: 1px solid rgba(212, 197, 176, 0.12);
        transition: color 0.35s ease, background 0.35s ease;
        position: relative;
    }

    .bottom-nav-item:first-child {
        border-left: 1px solid rgba(212, 197, 176, 0.12);
    }

    .bottom-nav-item::after {
        content: '';
        position: absolute;
        bottom: 0;
        left: 50%;
        width: 0;
        height: 1px;
        background: rgba(212, 197, 176, 0.6);
        transition: width 0.35s ease, left 0.35s ease;
    }

    .bottom-nav-item:hover {
        color: #e6decb;
        background: rgba(212, 197, 176, 0.05);
    }

    .bottom-nav-item:hover::after {
        width: 100%;
        left: 0;
    }

    .bottom-nav-icon {
        font-size: 16px;
    }

    /* Hovered zone info in bottom bar */
    .bar-zone-info {
        display: flex;
        align-items: center;
        gap: 18px;
        height: 100%;
        padding: 0 24px;
    }

    .bar-zone-icon {
        font-size: 22px;
    }

    .bar-zone-text {
        display: flex;
        flex-direction: column;
        gap: 3px;
    }

    .bar-zone-name {
        font-family: 'Cinzel', serif;
        font-size: 12px;
        letter-spacing: 0.35em;
        color: #e6decb;
        text-transform: uppercase;
    }

    .bar-zone-desc {
        font-family: 'Cormorant Garamond', serif;
        font-size: 14px;
        letter-spacing: 0.08em;
        color: rgba(212, 197, 176, 0.65);
        font-style: italic;
    }

    /* Disable custom cursor on touch-only devices */
    @media (hover: none) {
        .main-wrapper {
            cursor: default;
        }
        .zone-button {
            cursor: pointer;
        }
        .cursor-system {
            display: none;
        }

        /* На тач: лейблы всегда видны ярко */
        .zone-label {
            opacity: 1;
        }

        /* Нижняя навигация — на мобиле чуть компактнее */
        .bottom-nav-item {
            padding: 0 20px;
            font-size: 10px;
            letter-spacing: 0.18em;
        }

        .bottom-nav-icon {
            font-size: 18px;
        }
    }
</style>