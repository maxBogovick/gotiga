<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import QRCode from 'qrcode';
  import NotFound from '$lib/components/NotFound.svelte';
  import { brandName, t } from '$lib/i18n';
  import type { Figurine, FigurineStatus } from '$lib/types/api';

  let { data } = $props();
  let figurine = $derived(data.figurine as Figurine | null);
  let id = $derived(page.params.id ?? '');
  let qrDataUrl = $state('');

  type Fact = { label: string; value: string };
  type Note = { label: string; value: string };

  function hasText(value: string | null | undefined): value is string {
    return Boolean(value?.trim());
  }

  function oneLine(value: string | null | undefined): string {
    return value?.replace(/\s+/g, ' ').trim() ?? '';
  }

  function statusLabel(status: FigurineStatus): string {
    switch (status) {
      case 'available': return $t('figurineStatusAvailable');
      case 'reserved': return $t('figurineStatusReserved');
      case 'in_progress': return $t('figurineStatusInProgress');
      case 'sold': return $t('figurineStatusSold');
    }
  }

  let passportNo = $derived(
    oneLine(figurine?.passportNumber) || (id ? `ARC-${id.slice(0, 8).toUpperCase()}` : '')
  );
  let edition = $derived(oneLine(figurine?.edition) || $t('passportEditionUnique'));
  let created = $derived(oneLine(figurine?.createdPeriod) || (figurine?.year ? String(figurine.year) : ''));
  let heroImage = $derived.by(() => {
    const images = figurine?.images ?? [];
    const image = images.find((i) => i.imageType === 'face')
      ?? images.find((i) => i.imageType === 'full')
      ?? images[0];
    return image ? (image.originalUrl ?? image.url) : '';
  });
  let heroAlt = $derived(
    figurine?.images?.find((i) => (i.originalUrl ?? i.url) === heroImage)?.altText
      ?? figurine?.name
      ?? ''
  );

  let facts = $derived.by<Fact[]>(() => {
    if (!figurine) return [];
    return [
      { label: $t('passportNumber'), value: passportNo },
      { label: $t('passportStatus'), value: statusLabel(figurine.status) },
      { label: $t('passportEdition'), value: edition },
      ...(created ? [{ label: $t('passportCreated'), value: created }] : []),
      ...(hasText(figurine.dimensions) ? [{ label: $t('figurineDimensions'), value: figurine.dimensions.trim() }] : []),
      ...(hasText(figurine.material) ? [{ label: $t('figurineMaterial'), value: figurine.material.trim() }] : []),
      ...(hasText(figurine.technique) ? [{ label: $t('figurineTechnique'), value: figurine.technique.trim() }] : []),
      ...(hasText(figurine.series) ? [{ label: $t('detailTrustSeries'), value: figurine.series.trim() }] : []),
    ];
  });

  let notes = $derived.by<Note[]>(() => {
    if (!figurine) return [];
    return [
      ...(hasText(figurine.authenticityNote) ? [{ label: $t('passportAuthenticity'), value: figurine.authenticityNote.trim() }] : []),
      ...(hasText(figurine.provenanceNote) ? [{ label: $t('passportProvenance'), value: figurine.provenanceNote.trim() }] : []),
      ...(hasText(figurine.includedItems) ? [{ label: $t('passportIncluded'), value: figurine.includedItems.trim() }] : []),
      ...(hasText(figurine.careInstructions) ? [{ label: $t('passportCare'), value: figurine.careInstructions.trim() }] : []),
    ];
  });

  onMount(async () => {
    if (!figurine) return;
    const passportUrl = `${window.location.origin}/figurines/${id}/passport`;
    qrDataUrl = await QRCode.toDataURL(passportUrl, {
      errorCorrectionLevel: 'M',
      margin: 2,
      scale: 7,
      color: { dark: '#2b1812', light: '#fffdf8' },
    });
  });
</script>

<svelte:head>
  <title>{figurine ? `${$t('passportTitle')} — ${figurine.name}` : $t('passportTitle')} — {$brandName}</title>
  <meta name="description" content={figurine ? `${$t('passportTitle')}: ${figurine.name}` : $t('passportTitle')} />
</svelte:head>

{#if data.loadError || !figurine}
  <NotFound message={$t('notFoundMessage')} />
{:else}
  <main class="passport-page">
    <nav class="passport-nav" aria-label="Passport navigation">
      <a href="/figurines/{id}">{$t('passportBackToWork')}</a>
      <button type="button" onclick={() => window.print()}>{$t('passportPrint')}</button>
    </nav>

    <article class="passport-sheet" aria-labelledby="passport-title">
      <header class="ps-head">
        <span class="ps-brand">{$brandName}</span>
        <span class="ps-kicker">{$t('passportKicker')}</span>
      </header>

      <div class="ps-body">
        <div class="ps-seal" aria-hidden="true">GT</div>

        <p class="ps-eyebrow">{$t('passportNumber')} · {passportNo}</p>
        <h1 id="passport-title">{$t('passportTitle')}</h1>
        <p class="ps-name">{figurine.name}</p>

        <figure class="ps-plate">
          <div class="ps-plate-mat">
            {#if heroImage}
              <img src={heroImage} alt={heroAlt} />
            {:else}
              <span aria-hidden="true">GT</span>
            {/if}
          </div>
        </figure>

        <hr class="ps-rule" aria-hidden="true" />

        <dl class="ps-ledger" aria-label="Passport facts">
          {#each facts as fact}
            <div class="ps-row">
              <dt>{fact.label}</dt>
              <span class="ps-leader" aria-hidden="true"></span>
              <dd>{fact.value}</dd>
            </div>
          {/each}
        </dl>

        {#if notes.length}
          <hr class="ps-rule" aria-hidden="true" />
          <section class="ps-notes" aria-label="Passport notes">
            {#each notes as note}
              <div class="ps-note">
                <h2>{note.label}</h2>
                <p>{note.value}</p>
              </div>
            {/each}
          </section>
        {/if}

        <div class="ps-validation">
          {#if qrDataUrl}
            <img src={qrDataUrl} alt="Passport verification QR code" />
          {/if}
          <div class="ps-validation-text">
            <span class="ps-validation-mark">{$t('passportFooterVerified')}</span>
            <strong>{passportNo}</strong>
          </div>
        </div>
      </div>
    </article>
  </main>
{/if}

<style>
  /* ── Page: parchment table the document rests on ───────────────── */
  .passport-page {
    position: relative;
    isolation: isolate;
    min-height: 100vh;
    padding: clamp(20px, 4vw, 56px) clamp(16px, 4vw, 40px) clamp(40px, 6vw, 80px);
    color: var(--color-ink-primary);
    background:
      radial-gradient(120% 70% at 50% -10%, color-mix(in srgb, var(--color-ember) 8%, transparent), transparent 60%),
      var(--color-canvas-base);
  }

  .passport-page::before,
  .passport-page::after {
    content: "";
    position: fixed;
    inset: 0;
    z-index: -1;
    pointer-events: none;
  }

  /* dust grain */
  .passport-page::before {
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='180' height='180'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.82' numOctaves='2' stitchTiles='stitch'/%3E%3CfeColorMatrix type='saturate' values='0'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
    background-size: 180px 180px;
    opacity: 0.05;
    mix-blend-mode: multiply;
  }

  /* old-house edge vignette */
  .passport-page::after {
    background:
      radial-gradient(130% 90% at 50% 16%, transparent 52%, color-mix(in srgb, var(--color-ink-primary) 9%, transparent) 100%);
  }

  /* ── Controls (chrome — not part of the document) ──────────────── */
  .passport-nav {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    width: min(100%, 760px);
    margin: 0 auto 1.4rem;
    font-family: var(--font-body);
    font-size: 0.64rem;
    font-weight: 700;
    letter-spacing: 0.16em;
    text-transform: uppercase;
  }

  .passport-nav a,
  .passport-nav button {
    min-height: 2rem;
    padding: 0;
    border: 0;
    border-bottom: 1px solid color-mix(in srgb, var(--color-ember) 40%, transparent);
    background: transparent;
    color: var(--color-ember-deep);
    cursor: pointer;
    font: inherit;
    letter-spacing: inherit;
    text-transform: inherit;
    text-decoration: none;
    transition: color 0.2s ease, border-color 0.2s ease;
  }

  .passport-nav a:hover,
  .passport-nav button:hover {
    color: var(--color-ink-primary);
    border-color: var(--color-ember);
  }

  /* ── The document sheet (portrait certificate) ─────────────────── */
  .passport-sheet {
    position: relative;
    width: min(100%, 760px);
    margin: 0 auto;
    padding: clamp(14px, 2.4vw, 22px);
    border-radius: 3px;
    background:
      linear-gradient(177deg, color-mix(in srgb, var(--color-canvas-raised) 94%, white), color-mix(in srgb, var(--color-canvas-base) 86%, white));
    /* soft presence-shadow: a document resting on the table (allowed) */
    box-shadow:
      0 1px 0 color-mix(in srgb, white 70%, transparent) inset,
      0 30px 60px -28px color-mix(in srgb, var(--color-ink-primary) 40%, transparent);
  }

  /* engraved double-rule frame */
  .passport-sheet::before {
    content: "";
    position: absolute;
    inset: clamp(8px, 1.4vw, 13px);
    pointer-events: none;
    border: 1px solid color-mix(in srgb, var(--color-ink-primary) 22%, transparent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-canvas-base) 90%, transparent),
                0 0 0 4px color-mix(in srgb, var(--color-ink-primary) 12%, transparent);
    border-radius: 1px;
  }

  /* ── Header rail: brand / kicker ───────────────────────────────── */
  .ps-head {
    position: relative;
    z-index: 1;
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 1rem;
    padding: clamp(0.8rem, 2vw, 1.4rem) clamp(1rem, 3vw, 2rem) clamp(0.7rem, 1.6vw, 1rem);
    border-bottom: 1px solid color-mix(in srgb, var(--color-ink-primary) 14%, transparent);
    font-family: var(--font-body);
    font-size: 0.62rem;
    font-weight: 700;
    letter-spacing: 0.22em;
    text-transform: uppercase;
  }

  .ps-brand {
    color: var(--color-ink-primary);
  }

  .ps-kicker {
    color: var(--color-ember);
  }

  /* ── Body column ───────────────────────────────────────────────── */
  .ps-body {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: clamp(1.6rem, 4vw, 2.8rem) clamp(1.2rem, 4vw, 3rem) clamp(1.4rem, 3vw, 2.2rem);
    text-align: center;
  }

  /* wax seal — the house stamp */
  .ps-seal {
    display: grid;
    place-items: center;
    width: 3.5rem;
    height: 3.5rem;
    margin-bottom: clamp(1rem, 2.4vw, 1.5rem);
    border-radius: 999px;
    color: color-mix(in srgb, var(--color-canvas-raised) 92%, white);
    font-family: var(--font-display);
    font-size: 1.15rem;
    font-weight: 600;
    letter-spacing: 0.04em;
    background:
      radial-gradient(circle at 38% 32%, color-mix(in srgb, var(--color-ember-mid) 80%, white), var(--color-ember) 52%, var(--color-ember-deep) 100%);
    box-shadow:
      inset 0 1px 2px color-mix(in srgb, white 40%, transparent),
      inset 0 -3px 6px color-mix(in srgb, var(--color-ember-ink) 60%, transparent),
      0 4px 10px -3px color-mix(in srgb, var(--color-ember-ink) 55%, transparent);
  }

  .ps-eyebrow {
    margin: 0 0 0.85rem;
    color: color-mix(in srgb, var(--color-ink-secondary) 72%, transparent);
    font-family: var(--font-body);
    font-size: 0.62rem;
    font-weight: 700;
    letter-spacing: 0.18em;
    text-transform: uppercase;
  }

  h1 {
    margin: 0;
    max-width: 16ch;
    color: var(--color-ink-primary);
    font-family: var(--font-display);
    font-size: clamp(1.85rem, 5.5vw, 2.85rem);
    font-weight: 500;
    line-height: 1.04;
  }

  .ps-name {
    margin: 0.7rem 0 0;
    max-width: 34ch;
    color: color-mix(in srgb, var(--color-ink-secondary) 90%, transparent);
    font-family: var(--font-serif);
    font-size: clamp(1.05rem, 2.4vw, 1.3rem);
    font-style: italic;
    line-height: 1.45;
    overflow-wrap: anywhere;
  }

  /* ── Mounted photo plate (passe-partout mat) ───────────────────── */
  .ps-plate {
    width: min(100%, 22rem);
    margin: clamp(1.5rem, 4vw, 2.4rem) 0 clamp(0.5rem, 2vw, 1rem);
    padding: clamp(0.7rem, 2vw, 1.1rem);
    background:
      linear-gradient(160deg, color-mix(in srgb, var(--color-canvas-raised) 96%, white), color-mix(in srgb, var(--color-canvas-sunken) 70%, white));
    border: 1px solid color-mix(in srgb, var(--color-ink-primary) 16%, transparent);
    border-radius: 2px;
    box-shadow: inset 0 1px 0 color-mix(in srgb, white 80%, transparent);
  }

  .ps-plate-mat {
    position: relative;
    aspect-ratio: 4 / 5;
    overflow: hidden;
    border: 1px solid color-mix(in srgb, var(--color-ink-primary) 20%, transparent);
    background:
      radial-gradient(circle at 50% 38%, color-mix(in srgb, var(--color-ember) 9%, transparent), transparent 62%),
      color-mix(in srgb, var(--color-canvas-sunken) 70%, white);
  }

  .ps-plate-mat::after {
    content: "";
    position: absolute;
    inset: 0;
    pointer-events: none;
    box-shadow: inset 0 0 22px color-mix(in srgb, var(--color-ink-primary) 16%, transparent);
  }

  .ps-plate-mat img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center 42%;
  }

  .ps-plate-mat span {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    color: color-mix(in srgb, var(--color-ember-deep) 60%, transparent);
    font-family: var(--font-display);
    font-size: 3rem;
  }

  /* ── Ornamental divider with ember lozenge ─────────────────────── */
  .ps-rule {
    width: 100%;
    height: 0;
    margin: clamp(1.6rem, 4vw, 2.4rem) 0;
    border: 0;
    border-top: 1px solid color-mix(in srgb, var(--color-ink-primary) 14%, transparent);
    overflow: visible;
  }

  .ps-rule::after {
    content: "";
    display: block;
    width: 7px;
    height: 7px;
    margin: -4px auto 0;
    rotate: 45deg;
    background: var(--color-ember);
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--color-canvas-raised) 92%, white);
  }

  /* ── Ledger of facts (museum dotted leaders) ───────────────────── */
  .ps-ledger {
    width: 100%;
    max-width: 30rem;
    margin: 0;
    text-align: left;
  }

  .ps-row {
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
    padding: 0.5rem 0;
  }

  .ps-row + .ps-row {
    border-top: 1px solid color-mix(in srgb, var(--color-ink-primary) 7%, transparent);
  }

  dt {
    flex: none;
    color: color-mix(in srgb, var(--color-ink-secondary) 78%, transparent);
    font-family: var(--font-body);
    font-size: 0.66rem;
    font-weight: 700;
    letter-spacing: 0.13em;
    text-transform: uppercase;
  }

  .ps-leader {
    flex: 1 1 auto;
    align-self: stretch;
    min-width: 1.5rem;
    border-bottom: 1px dotted color-mix(in srgb, var(--color-ink-primary) 30%, transparent);
    transform: translateY(-0.28rem);
  }

  dd {
    flex: none;
    max-width: 60%;
    margin: 0;
    color: var(--color-ink-primary);
    font-family: var(--font-serif);
    font-size: 1.02rem;
    line-height: 1.4;
    text-align: right;
    overflow-wrap: anywhere;
  }

  /* ── Notes (flowing archival prose) ────────────────────────────── */
  .ps-notes {
    display: flex;
    flex-direction: column;
    gap: clamp(1.3rem, 3vw, 1.9rem);
    width: 100%;
    max-width: 34rem;
    text-align: left;
  }

  .ps-note h2 {
    margin: 0 0 0.55rem;
    color: var(--color-ember-deep);
    font-family: var(--font-body);
    font-size: 0.64rem;
    font-weight: 700;
    letter-spacing: 0.16em;
    text-transform: uppercase;
  }

  .ps-note p {
    margin: 0;
    color: color-mix(in srgb, var(--color-ink-primary) 86%, transparent);
    font-family: var(--font-serif);
    font-size: 1.04rem;
    line-height: 1.62;
  }

  /* ── Validation stamp footer ───────────────────────────────────── */
  .ps-validation {
    display: flex;
    align-items: center;
    gap: clamp(1rem, 3vw, 1.5rem);
    width: 100%;
    max-width: 30rem;
    margin-top: clamp(1.8rem, 4vw, 2.6rem);
    padding-top: clamp(1.4rem, 3vw, 2rem);
    border-top: 1px solid color-mix(in srgb, var(--color-ink-primary) 14%, transparent);
    text-align: left;
  }

  .ps-validation img {
    flex: none;
    width: 5.5rem;
    height: 5.5rem;
    padding: 0.3rem;
    border: 1px solid color-mix(in srgb, var(--color-ink-primary) 16%, transparent);
    border-radius: 2px;
    background: color-mix(in srgb, var(--color-canvas-raised) 96%, white);
  }

  .ps-validation-text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .ps-validation-mark {
    color: color-mix(in srgb, var(--color-ink-secondary) 72%, transparent);
    font-family: var(--font-body);
    font-size: 0.62rem;
    font-weight: 700;
    letter-spacing: 0.13em;
    line-height: 1.4;
    text-transform: uppercase;
  }

  .ps-validation-text strong {
    color: var(--color-ink-primary);
    font-family: var(--font-display);
    font-size: 1.15rem;
    font-weight: 500;
    letter-spacing: 0.06em;
  }

  /* ── Responsive ────────────────────────────────────────────────── */
  @media (max-width: 560px) {
    .passport-page {
      padding: 1rem 0.75rem 2.5rem;
    }

    .passport-nav {
      padding-inline: 0.25rem;
    }

    dd {
      max-width: 52%;
    }
  }

  /* ── Print ─────────────────────────────────────────────────────── */
  @media print {
    .passport-page {
      padding: 0;
      background: white;
    }

    .passport-page::before,
    .passport-page::after,
    .passport-nav {
      display: none;
    }

    .passport-sheet {
      box-shadow: none;
    }

    .passport-sheet::before {
      box-shadow: none;
      border-color: rgba(0, 0, 0, 0.35);
    }
  }
</style>
