<script lang="ts">
  import type { Snippet } from 'svelte';
  import { onMount, onDestroy } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { t, brandName, type TranslationKey } from '$lib/i18n';
  import { SITE_URL } from '$lib/site';
  import { createSiteAnalytics } from '$lib/analytics';

  import RakingLight from '$lib/components/RakingLight.svelte';
  import DustParticles from '$lib/components/DustParticles.svelte';
  import LivingDaguerreotype from '$lib/components/LivingDaguerreotype.svelte';
  import KeyholeVeil from '$lib/components/KeyholeVeil.svelte';
  import SecretText from '$lib/components/SecretText.svelte';
  import CandleReveal from '$lib/components/CandleReveal.svelte';

  // The source shown below is pulled straight out of the files that are running
  // this very page. Nothing is pasted by hand, so the cellar cannot drift from
  // the house the way a copied snippet does after the third refactor.
  import rakingSource from '$lib/components/RakingLight.svelte?raw';
  import dustSource from '$lib/components/DustParticles.svelte?raw';
  import plateSource from '$lib/components/LivingDaguerreotype.svelte?raw';
  import veilSource from '$lib/components/KeyholeVeil.svelte?raw';
  import cipherSource from '$lib/components/SecretText.svelte?raw';
  import candleSource from '$lib/components/CandleReveal.svelte?raw';

  const DEMO_IMAGE = '/images/main.jpg';

  // Two of the five lamps are whole-window overlays by construction (both are
  // `position: fixed`), so they cannot be demonstrated inside a bordered box.
  // They light the cellar itself instead — which is the more honest demo anyway.
  let dustLit = $state(false);
  let candleLit = $state(false);
  let veilShown = $state(true);

  // ── The plainest highlighter that could work ────────────────────────────────
  // No highlight.js: 30 KB and a palette that would fight the parchment. Comments
  // and nothing else — in these files the comments *are* the explanation.
  function esc(source: string): string {
    return source.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  }

  /** Index of a `//` that starts a comment, or -1. Protocol slashes don't count. */
  function trailingComment(line: string): number {
    for (let i = 0; i < line.length - 1; i++) {
      if (line[i] === '/' && line[i + 1] === '/' && line[i - 1] !== ':' && line[i - 1] !== '/') return i;
    }
    return -1;
  }

  function highlight(source: string): string {
    let inBlock = false;
    return source
      .split('\n')
      .map((raw) => {
        const line = esc(raw);
        if (inBlock) {
          if (raw.includes('*/')) inBlock = false;
          return `<span class="cm">${line}</span>`;
        }
        const head = raw.trimStart();
        if (head.startsWith('/*')) {
          if (!raw.includes('*/')) inBlock = true;
          return `<span class="cm">${line}</span>`;
        }
        if (head.startsWith('//')) return `<span class="cm">${line}</span>`;
        const at = trailingComment(line);
        return at === -1 ? line : `${line.slice(0, at)}<span class="cm">${line.slice(at)}</span>`;
      })
      .join('\n');
  }

  const lines = (source: string) => source.split('\n').length;

  // Lamp V is two files working together. They are shown in one panel, but with
  // the seam marked — a page about how things are built should not quietly weld
  // two sources into one.
  const fileMark = (name: string) => `// \u2500\u2500 ${name} ${'\u2500'.repeat(56 - name.length)}`;
  const cipherAndCandle =
    `${fileMark('SecretText.svelte')}\n${cipherSource}\n\n${fileMark('CandleReveal.svelte')}\n${candleSource}`;

  // ── Palette and letterforms ────────────────────────────────────────────────
  const swatches = [
    { hex: '#f8f1e7', key: 'cellarSwatchParchment' as TranslationKey },
    { hex: '#34251c', key: 'cellarSwatchIron' as TranslationKey },
    { hex: '#5f4636', key: 'cellarSwatchBark' as TranslationKey },
    { hex: '#6f3b24', key: 'cellarSwatchDeep' as TranslationKey },
    { hex: '#c65f3c', key: 'cellarSwatchEmber' as TranslationKey },
    { hex: '#d8c6b1', key: 'cellarSwatchDust' as TranslationKey }
  ];

  const letterforms = [
    { family: "'Fraunces', Georgia, serif", name: 'Fraunces', key: 'cellarFontFraunces' as TranslationKey },
    { family: 'Georgia, serif', name: 'Georgia', key: 'cellarFontGeorgia' as TranslationKey },
    { family: "'Instrument Sans', system-ui, sans-serif", name: 'Instrument Sans', key: 'cellarFontInstrument' as TranslationKey }
  ];

  const rules: TranslationKey[] = [
    'cellarRule1',
    'cellarRule2',
    'cellarRule3',
    'cellarRule4',
    'cellarRule5'
  ];

  const siteAnalytics = createSiteAnalytics();
  onDestroy(() => siteAnalytics.stop());

  onMount(() => {
    siteAnalytics.pageView();
    siteAnalytics.start();
  });
</script>

<svelte:head>
  <title>{$t('cellarTitle')} — {$brandName}</title>
  <meta name="description" content={$t('cellarLead')} />
  <meta property="og:site_name" content={$brandName} />
  <meta property="og:locale" content="en_US" />
  <meta property="og:type" content="article" />
  <meta property="og:title" content="{$t('cellarTitle')} — {$brandName}" />
  <meta property="og:description" content={$t('cellarLead')} />
  <meta property="og:url" content="{SITE_URL}/cellar" />
  <meta property="og:image" content="{SITE_URL}/images/cabinet-bg.jpeg" />
  {@html `<script type="application/ld+json">${JSON.stringify({ '@context': 'https://schema.org', '@type': 'BreadcrumbList', itemListElement: [ { '@type': 'ListItem', position: 1, name: $brandName, item: SITE_URL }, { '@type': 'ListItem', position: 2, name: 'Cellar', item: `${SITE_URL}/cellar` } ] })}<\/script>`}
  <!-- Fonts loaded once globally in app.html -->
</svelte:head>

<!-- Lamp II and Lamp V are window-wide by construction; they light this room. -->
<DustParticles opacity={dustLit ? 0.6 : 0} />
<CandleReveal isActive={candleLit} />

<div class="ground" aria-hidden="true">
  <div class="ground-grain"></div>
  <div class="ground-vignette"></div>
</div>

<main class="cellar">

  <a href="/" class="back" in:fade={{ duration: 800 }}>{$t('cellarBack')}</a>

  <!-- ── I. The threshold ─────────────────────────────────────────────────── -->
  <header class="threshold">
    <p class="kicker" in:fade={{ delay: 200, duration: 700 }}>{$t('cellarKicker')}</p>
    <h1 in:fly={{ x: -20, duration: 1000 }}>{$t('cellarTitle')}</h1>
    <p class="lead" in:fade={{ delay: 500, duration: 900 }}>{$t('cellarLead')}</p>
  </header>

  <!-- ── II. Five lamps ───────────────────────────────────────────────────── -->
  <section class="lamps" aria-labelledby="lamps-title">
    <h2 id="lamps-title" class="section-title">{$t('cellarLampsTitle')}</h2>

    {@render lamp('I', 'cellarLamp1Name', 'cellarLamp1Why', rakingSource, rakingDemo)}
    {@render lamp('II', 'cellarLamp2Name', 'cellarLamp2Why', dustSource, dustDemo)}
    {@render lamp('III', 'cellarLamp3Name', 'cellarLamp3Why', plateSource, plateDemo)}
    {@render lamp('IV', 'cellarLamp4Name', 'cellarLamp4Why', veilSource, veilDemo)}
    {@render lamp('V', 'cellarLamp5Name', 'cellarLamp5Why', cipherAndCandle, cipherDemo)}
  </section>

  <!-- ── III. Six colours, three hands ────────────────────────────────────── -->
  <section class="palette" aria-labelledby="palette-title">
    <h2 id="palette-title" class="section-title">{$t('cellarPaletteTitle')}</h2>
    <p class="section-lead">{$t('cellarPaletteLead')}</p>

    <ul class="swatches">
      {#each swatches as swatch}
        <li class="swatch">
          <span class="swatch-chip" style="background: {swatch.hex};"></span>
          <span class="swatch-name">{$t(swatch.key)}</span>
          <span class="swatch-hex">{swatch.hex}</span>
        </li>
      {/each}
    </ul>

    <h3 class="sub-title">{$t('cellarFontsTitle')}</h3>
    <ul class="letterforms">
      {#each letterforms as face}
        <li class="letterform">
          <p class="letterform-specimen" style="font-family: {face.family};">{$t('cellarSpecimen')}</p>
          <p class="letterform-note"><span class="letterform-name">{face.name}</span> — {$t(face.key)}</p>
        </li>
      {/each}
    </ul>
  </section>

  <!-- ── IV. Rules of the house ───────────────────────────────────────────── -->
  <section class="rules" aria-labelledby="rules-title">
    <h2 id="rules-title" class="section-title">{$t('cellarRulesTitle')}</h2>
    <ol class="rule-list">
      {#each rules as rule, i}
        <li class="rule">
          <span class="rule-num">{String(i + 1).padStart(2, '0')}</span>
          <p class="rule-text">{$t(rule)}</p>
        </li>
      {/each}
    </ol>
  </section>

  <!-- ── V. The quiet way out ─────────────────────────────────────────────── -->
  <footer class="outro">
    <p class="outro-text">{$t('cellarOutro')}</p>
    <a href="/" class="outro-door">{$t('cellarBack')}</a>
  </footer>

</main>

<!-- ── The four beats every lamp keeps: name, demo, why, workings ─────────── -->
{#snippet lamp(numeral: string, nameKey: TranslationKey, whyKey: TranslationKey, source: string, demo: Snippet)}
  <article class="lamp">
    <header class="lamp-head">
      <span class="lamp-num">{numeral}</span>
      <h3 class="lamp-name">{$t(nameKey)}</h3>
    </header>

    <div class="lamp-demo">{@render demo()}</div>

    <p class="lamp-why">{$t(whyKey)}</p>

    <details class="workings">
      <summary>
        <span class="workings-label">{$t('cellarCodeShow')}</span>
        <span class="workings-count">{lines(source)} {$t('cellarCodeLines')}</span>
      </summary>
      <pre class="workings-source"><code>{@html highlight(source)}</code></pre>
    </details>
  </article>
{/snippet}

{#snippet rakingDemo()}
  <div class="stage">
    <RakingLight src={DEMO_IMAGE} alt={$t('cellarLamp1Name')} intensity={0.7} />
  </div>
  <p class="stage-hint">{$t('cellarLamp1Hint')}</p>
{/snippet}

{#snippet dustDemo()}
  <div class="stage stage--empty">
    <button class="lever" class:lever--on={dustLit} onclick={() => (dustLit = !dustLit)}>
      {dustLit ? $t('cellarDustOff') : $t('cellarDustOn')}
    </button>
  </div>
  <p class="stage-hint">{$t('cellarLamp2Hint')}</p>
{/snippet}

{#snippet plateDemo()}
  <div class="stage">
    <LivingDaguerreotype src={DEMO_IMAGE} alt={$t('cellarLamp3Name')} intensity={0.6} imageFit="contain" />
  </div>
  <p class="stage-hint">{$t('cellarLamp3Hint')}</p>
{/snippet}

{#snippet veilDemo()}
  <div class="stage stage--veiled">
    <img src={DEMO_IMAGE} alt={$t('cellarLamp4Name')} class="stage-plate" />
    <KeyholeVeil show={veilShown} focalX={0.5} focalY={0.42} revealRadius={0.28} />
  </div>
  <button class="lever" class:lever--on={!veilShown} onclick={() => (veilShown = !veilShown)}>
    {veilShown ? $t('cellarVeilLift') : $t('cellarVeilDrop')}
  </button>
  <p class="stage-hint">{$t('cellarLamp4Hint')}</p>
{/snippet}

{#snippet cipherDemo()}
  <div class="stage stage--empty stage--cipher">
    <SecretText text={$t('cellarSecret')} isCandleLit={candleLit} />
    <button class="lever" class:lever--on={candleLit} onclick={() => (candleLit = !candleLit)}>
      {candleLit ? $t('cellarCandleOff') : $t('cellarCandleOn')}
    </button>
  </div>
  <p class="stage-hint">{$t('cellarLamp5Hint')}</p>
{/snippet}

<style>
  /* ── Ground ───────────────────────────────────────────────────────────── */
  .ground {
    position: fixed;
    inset: 0;
    z-index: -50;
    background: #f8f1e7;
    overflow: hidden;
    pointer-events: none;
  }
  .ground-grain {
    position: absolute;
    inset: 0;
    opacity: 0.05;
    mix-blend-mode: overlay;
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
  }
  /* A cellar is darker at its edges than at the lamp. */
  .ground-vignette {
    position: absolute;
    inset: 0;
    background: radial-gradient(ellipse at 50% 30%, transparent 0%, rgba(52, 37, 28, 0.07) 100%);
  }

  /* ── Page ─────────────────────────────────────────────────────────────── */
  .cellar {
    position: relative;
    z-index: 10;
    max-width: 46rem;
    margin: 0 auto;
    padding: 3.5rem 1.5rem 8rem;
    color: #34251c;
    font-family: 'Instrument Sans', system-ui, sans-serif;
  }
  @media (min-width: 1024px) {
    .cellar {
      padding: 5rem 2rem 10rem;
    }
  }

  .back,
  .outro-door {
    display: inline-flex;
    align-items: center;
    min-height: 44px;
    font-size: 10px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: #5f4636;
    text-decoration: none;
    transition: color 0.3s ease;
  }
  .back:hover,
  .back:active,
  .outro-door:hover,
  .outro-door:active {
    color: #34251c;
  }

  /* ── Threshold ────────────────────────────────────────────────────────── */
  .threshold {
    margin: 2.5rem 0 5rem;
  }
  .kicker {
    font-size: 10px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #5f4636;
    opacity: 0.7;
    margin-bottom: 1.25rem;
  }
  .threshold h1 {
    font-family: 'Fraunces', Georgia, serif;
    font-size: clamp(2.75rem, 9vw, 4.5rem);
    line-height: 1.02;
    color: #6f3b24;
    opacity: 0.92;
    margin-bottom: 1.75rem;
  }
  .lead {
    font-family: Georgia, serif;
    font-size: 1.0625rem;
    line-height: 1.75;
    color: #5f4636;
    border-left: 1px solid rgba(52, 37, 28, 0.2);
    padding-left: 1.5rem;
  }

  /* ── Section furniture ────────────────────────────────────────────────── */
  .section-title {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1.5rem;
    color: #6f3b24;
    letter-spacing: 0.02em;
    margin-bottom: 1rem;
  }
  .sub-title {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1.125rem;
    color: #6f3b24;
    margin: 3rem 0 1.25rem;
  }
  .section-lead {
    font-family: Georgia, serif;
    font-size: 0.9375rem;
    line-height: 1.7;
    color: #5f4636;
    margin-bottom: 2.5rem;
    max-width: 38rem;
  }

  .lamps,
  .palette,
  .rules {
    margin-bottom: 6rem;
  }

  /* ── A lamp ───────────────────────────────────────────────────────────── */
  .lamp {
    margin: 3.5rem 0 5rem;
    padding-top: 2.5rem;
    border-top: 1px solid #d8c6b1;
  }
  .lamp-head {
    display: flex;
    align-items: baseline;
    gap: 1rem;
    margin-bottom: 1.75rem;
  }
  .lamp-num {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 0.8125rem;
    letter-spacing: 0.12em;
    color: #c65f3c;
    opacity: 0.75;
  }
  .lamp-name {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1.75rem;
    line-height: 1.2;
    color: #34251c;
  }

  .lamp-why {
    font-family: Georgia, serif;
    font-size: 1rem;
    line-height: 1.8;
    color: #5f4636;
    margin: 1.75rem 0;
  }

  /* ── Stage: a demo is a framed plate, like everything else here ────────── */
  .stage {
    position: relative;
    aspect-ratio: 4 / 3;
    background: #2f2117;
    border: 1px solid #d8c6b1;
    box-shadow: 10px 10px 30px rgba(111, 59, 36, 0.14);
    overflow: hidden;
  }
  .stage--empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1.5rem;
    aspect-ratio: 16 / 7;
    background: #fff9f0;
  }
  .stage--cipher {
    gap: 2rem;
  }
  .stage-plate {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .stage-hint {
    font-size: 10px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: #5f4636;
    opacity: 0.6;
    margin-top: 0.75rem;
  }

  /* ── Lever: the only control in the room ──────────────────────────────── */
  .lever {
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 10px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #5f4636;
    background: transparent;
    border: 1px solid #d8c6b1;
    padding: 0.75rem 1.5rem;
    min-height: 44px;
    cursor: pointer;
    transition: color 0.4s ease, border-color 0.4s ease, background 0.4s ease;
  }
  .lever:hover {
    color: #34251c;
    border-color: rgba(52, 37, 28, 0.35);
  }
  .lever--on {
    color: #c65f3c;
    border-color: #c65f3c;
    background: rgba(198, 95, 60, 0.05);
  }
  .stage + .lever {
    margin-top: 1rem;
  }

  /* ── Workings: shut by default. Silence before explanation. ───────────── */
  .workings {
    border-top: 1px solid rgba(216, 198, 177, 0.6);
    padding-top: 1rem;
  }
  .workings summary {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    min-height: 44px;
    cursor: pointer;
    list-style: none;
    font-size: 10px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #5f4636;
    transition: color 0.3s ease;
  }
  .workings summary::-webkit-details-marker {
    display: none;
  }
  .workings summary:hover {
    color: #c65f3c;
  }
  .workings-count {
    opacity: 0.5;
  }
  .workings-source {
    margin-top: 1rem;
    max-height: 30rem;
    overflow: auto;
    background: #fff9f0;
    border: 1px solid #d8c6b1;
    padding: 1.25rem;
    font-family: ui-monospace, 'SF Mono', Menlo, monospace;
    font-size: 11.5px;
    line-height: 1.65;
    color: #4a3628;
    /* Wide code scrolls inside its own box; the page never scrolls sideways. */
    white-space: pre;
    tab-size: 2;
  }
  .workings-source :global(.cm) {
    color: #9a8877;
  }

  /* ── Palette ──────────────────────────────────────────────────────────── */
  .swatches {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(9rem, 1fr));
    gap: 1.5rem;
    list-style: none;
    padding: 0;
  }
  .swatch {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .swatch-chip {
    display: block;
    height: 4.5rem;
    border: 1px solid #d8c6b1;
    box-shadow: inset 0 0 24px rgba(111, 59, 36, 0.1);
  }
  .swatch-name {
    font-family: Georgia, serif;
    font-size: 0.875rem;
    font-style: italic;
    color: #34251c;
  }
  .swatch-hex {
    font-family: ui-monospace, 'SF Mono', Menlo, monospace;
    font-size: 10px;
    letter-spacing: 0.06em;
    color: #5f4636;
    opacity: 0.65;
  }

  .letterforms {
    list-style: none;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2.25rem;
  }
  .letterform-specimen {
    font-size: 1.75rem;
    line-height: 1.3;
    color: #34251c;
    margin-bottom: 0.5rem;
  }
  .letterform-note {
    font-size: 11px;
    letter-spacing: 0.08em;
    color: #5f4636;
    opacity: 0.75;
  }
  .letterform-name {
    text-transform: uppercase;
    letter-spacing: 0.12em;
  }

  /* ── Rules ────────────────────────────────────────────────────────────── */
  .rule-list {
    list-style: none;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2rem;
  }
  .rule {
    display: flex;
    gap: 1.25rem;
    align-items: baseline;
  }
  .rule-num {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 0.75rem;
    letter-spacing: 0.1em;
    color: #c65f3c;
    opacity: 0.7;
    flex-shrink: 0;
  }
  .rule-text {
    font-family: Georgia, serif;
    font-size: 1.0625rem;
    line-height: 1.7;
    font-style: italic;
    color: #34251c;
  }

  /* ── Outro ────────────────────────────────────────────────────────────── */
  .outro {
    border-top: 1px solid #d8c6b1;
    padding-top: 2.5rem;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }
  .outro-text {
    font-family: Georgia, serif;
    font-size: 0.9375rem;
    font-style: italic;
    line-height: 1.7;
    color: #5f4636;
  }

  @media (prefers-reduced-motion: reduce) {
    .lever,
    .workings summary,
    .back {
      transition: none;
    }
  }
</style>
