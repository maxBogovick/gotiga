<script lang="ts">
  import { t, brandName } from '$lib/i18n';

  // Single source of truth for the contact details. Phone doubles as the
  // Telegram handle (Telegram resolves +<number> links directly).
  const EMAIL = 'ritabogovick@gmail.com';
  const PHONE = '+380634153755';
  const PHONE_DISPLAY = '+380 63 415 37 55';
  const TELEGRAM = 'https://t.me/+380634153755';

  let links = $derived([
    { href: '/figurines', label: $t('navArchive') },
    { href: '/upcoming', label: $t('navUpcoming') },
    { href: '/workshop', label: $t('navWorkshop') },
    { href: '/author', label: $t('navAuthor') },
    { href: '/acquire', label: $t('navAcquire') },
  ]);

  let year = $derived(new Date().getFullYear());
</script>

<footer class="site-footer">
  <div class="grain" aria-hidden="true"></div>

  <!-- Ornamental top edge: a thin rule broken by a copper lozenge -->
  <div class="edge" aria-hidden="true">
    <span class="rule"></span>
    <span class="lozenge"></span>
    <span class="rule"></span>
  </div>

  <div class="inner">
    <!-- Colophon / brand -->
    <section class="col col-brand">
      <a href="/" class="monogram" aria-label={$brandName}>
        <span class="monogram-mark">{$brandName.charAt(0).toUpperCase()}</span>
        <span class="monogram-text">
          <span class="monogram-name">{$brandName}</span>
          <span class="monogram-sub">Cabinet of Gothic Miniatures</span>
        </span>
      </a>
      <p class="tagline">{$t('footerTagline')}</p>
      <span class="established">{$t('footerEstablished')}</span>
    </section>

    <!-- Navigation ledger -->
    <nav class="col col-nav" aria-label="Footer">
      <h2 class="col-title">{$t('footerNavTitle')}</h2>
      <ul class="nav-list">
        {#each links as link, i}
          <li>
            <a href={link.href} class="nav-row">
              <span class="nav-num">{String(i + 1).padStart(2, '0')}</span>
              <span class="nav-label">{link.label}</span>
              <span class="leader" aria-hidden="true"></span>
              <span class="nav-arrow" aria-hidden="true">→</span>
            </a>
          </li>
        {/each}
      </ul>
    </nav>

    <!-- Correspondence -->
    <section class="col col-contact">
      <span class="kicker">{$t('footerKicker')}</span>
      <h2 class="col-title">{$t('footerContactTitle')}</h2>

      <ul class="contact-list">
        <li>
          <a class="contact-row" href={`mailto:${EMAIL}`}>
            <span class="contact-icon" aria-hidden="true">
              <svg viewBox="0 0 24 24" width="16" height="16" fill="none">
                <rect x="3" y="5" width="18" height="14" rx="1.5" stroke="currentColor" stroke-width="1.3"/>
                <path d="M4 7l8 6 8-6" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
              </svg>
            </span>
            <span class="contact-text">
              <span class="contact-label">{$t('footerEmailLabel')}</span>
              <span class="contact-value">{EMAIL}</span>
            </span>
          </a>
        </li>
        <li>
          <a class="contact-row" href={`tel:${PHONE}`}>
            <span class="contact-icon" aria-hidden="true">
              <svg viewBox="0 0 24 24" width="16" height="16" fill="none">
                <path d="M6 3.5h3l1.5 4-2 1.5a11 11 0 005 5l1.5-2 4 1.5v3a2 2 0 01-2.2 2A16 16 0 014 5.7 2 2 0 016 3.5z" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/>
              </svg>
            </span>
            <span class="contact-text">
              <span class="contact-label">{$t('footerPhoneLabel')}</span>
              <span class="contact-value">{PHONE_DISPLAY}</span>
            </span>
          </a>
        </li>
        <li>
          <a class="contact-row" href={TELEGRAM} target="_blank" rel="noopener noreferrer">
            <span class="contact-icon" aria-hidden="true">
              <svg viewBox="0 0 24 24" width="16" height="16" fill="none">
                <path d="M3.5 11.5L20 5l-2.6 13.5-5-3.7-2.6 2.6-.4-4.2L17 7.5 8.6 12.2 3.5 11.5z" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/>
              </svg>
            </span>
            <span class="contact-text">
              <span class="contact-label">{$t('footerTelegramLabel')}</span>
              <span class="contact-value">{PHONE_DISPLAY}</span>
            </span>
          </a>
        </li>
      </ul>
    </section>
  </div>

  <!-- Colophon bar -->
  <div class="colophon">
    <span class="seal" aria-hidden="true">✦</span>
    <span class="colophon-text">{$t('footerColophon')}</span>
    <span class="colophon-dot" aria-hidden="true">·</span>
    <span class="colophon-rights">© {year} · {$t('footerRights')}</span>
  </div>
</footer>

<style>
  .site-footer {
    position: relative;
    isolation: isolate;
    margin-top: clamp(60px, 10vw, 140px);
    padding: clamp(48px, 7vw, 96px) clamp(20px, 6vw, 96px) clamp(20px, 3vw, 32px);
    /* Aged-leather endpaper: deep warm brown, the inside cover of an old book */
    background:
      radial-gradient(120% 80% at 50% -10%, rgba(198,95,60,0.10), transparent 60%),
      linear-gradient(180deg, #2a1810 0%, #1d100a 100%);
    color: #efe2cf;
    overflow: hidden;
    font-family: var(--font-body, Georgia, serif);
  }

  /* Paper grain + a soft vignette pressed into the leather */
  .grain {
    position: absolute;
    inset: 0;
    z-index: -1;
    pointer-events: none;
    background-image:
      radial-gradient(rgba(255,255,255,0.025) 1px, transparent 1px),
      radial-gradient(circle at 50% 120%, transparent 40%, rgba(0,0,0,0.5));
    background-size: 3px 3px, 100% 100%;
    opacity: 0.9;
    mix-blend-mode: screen;
  }

  /* ── Ornamental top edge ── */
  .edge {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: clamp(40px, 5vw, 64px);
  }
  .edge .rule {
    flex: 1;
    height: 1px;
    background: linear-gradient(90deg, transparent, rgba(216,198,177,0.28), transparent);
  }
  .edge .lozenge {
    width: 9px;
    height: 9px;
    flex-shrink: 0;
    transform: rotate(45deg);
    background: var(--color-ember, #c65f3c);
    box-shadow: 0 0 0 4px rgba(198,95,60,0.12);
  }

  /* ── Layout ── */
  .inner {
    display: grid;
    grid-template-columns: 1.4fr 0.9fr 1.2fr;
    gap: clamp(36px, 5vw, 88px);
    max-width: 1180px;
    margin: 0 auto;
  }

  .col-title {
    margin: 0 0 22px;
    font-family: var(--font-display, 'Fraunces', Georgia, serif);
    font-size: 13px;
    font-weight: 400;
    letter-spacing: 0.26em;
    text-transform: uppercase;
    color: #c9b79c;
  }

  /* ── Brand column ── */
  .monogram {
    display: flex;
    align-items: center;
    gap: 16px;
    text-decoration: none;
    color: inherit;
  }
  .monogram-mark {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 52px;
    height: 52px;
    flex-shrink: 0;
    border: 1px solid rgba(216,198,177,0.32);
    border-radius: 3px;
    font-family: 'Fraunces', Georgia, serif;
    font-size: 30px;
    line-height: 1;
    color: #f3e7d3;
    transition: border-color 0.4s ease, color 0.4s ease, background 0.4s ease;
  }
  .monogram:hover .monogram-mark {
    border-color: var(--color-ember, #c65f3c);
    color: #fff;
    background: rgba(198,95,60,0.14);
  }
  .monogram-name {
    display: block;
    font-family: 'Fraunces', Georgia, serif;
    font-size: 24px;
    letter-spacing: 0.32em;
    text-transform: uppercase;
    line-height: 1;
    color: #f3e7d3;
  }
  .monogram-sub {
    display: block;
    margin-top: 7px;
    font-size: 10.5px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #9c8970;
  }
  .tagline {
    margin: 26px 0 0;
    max-width: 34ch;
    font-size: 15px;
    line-height: 1.7;
    font-style: italic;
    color: #c4b39a;
  }
  .established {
    display: inline-block;
    margin-top: 20px;
    font-size: 10px;
    letter-spacing: 0.34em;
    text-transform: uppercase;
    color: #7f6c56;
  }

  /* ── Nav ledger ── */
  .nav-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .nav-row {
    display: flex;
    align-items: baseline;
    gap: 12px;
    padding: 13px 0;
    border-bottom: 1px solid rgba(216,198,177,0.10);
    text-decoration: none;
    color: #d8c6b1;
    font-size: 16px;
    transition: color 0.3s ease;
  }
  .nav-num {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 11px;
    letter-spacing: 0.1em;
    color: #7f6c56;
    transition: color 0.3s ease;
  }
  .nav-label {
    white-space: nowrap;
    transition: letter-spacing 0.3s ease;
  }
  .leader {
    flex: 1;
    height: 0;
    align-self: flex-end;
    margin-bottom: 4px;
    border-bottom: 1px dotted rgba(216,198,177,0.22);
  }
  .nav-arrow {
    color: var(--color-ember, #c65f3c);
    opacity: 0;
    transform: translateX(-6px);
    transition: opacity 0.3s ease, transform 0.3s ease;
  }
  .nav-row:hover {
    color: #fff;
  }
  .nav-row:hover .nav-num { color: var(--color-ember, #c65f3c); }
  .nav-row:hover .nav-label { letter-spacing: 0.04em; }
  .nav-row:hover .nav-arrow { opacity: 1; transform: translateX(0); }

  /* ── Correspondence ── */
  .kicker {
    display: block;
    margin-bottom: 8px;
    font-size: 10px;
    letter-spacing: 0.3em;
    text-transform: uppercase;
    color: var(--color-ember, #c65f3c);
  }
  .contact-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .contact-row {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 14px 16px;
    border: 1px solid rgba(216,198,177,0.16);
    border-radius: 3px;
    text-decoration: none;
    color: #efe2cf;
    background: rgba(255,247,236,0.015);
    transition: border-color 0.35s ease, background 0.35s ease, transform 0.35s cubic-bezier(0.16,1,0.3,1);
  }
  .contact-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 38px;
    height: 38px;
    flex-shrink: 0;
    border: 1px solid rgba(216,198,177,0.22);
    border-radius: 50%;
    color: #c9b79c;
    transition: color 0.35s ease, border-color 0.35s ease, transform 0.35s ease;
  }
  .contact-text {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .contact-label {
    font-size: 9.5px;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: #9c8970;
  }
  .contact-value {
    font-size: 15px;
    letter-spacing: 0.01em;
    color: #f3e7d3;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .contact-row:hover {
    border-color: rgba(198,95,60,0.55);
    background: rgba(198,95,60,0.07);
    transform: translateX(4px);
  }
  .contact-row:hover .contact-icon {
    color: var(--color-ember, #c65f3c);
    border-color: var(--color-ember, #c65f3c);
    transform: scale(1.06);
  }

  /* ── Colophon bar ── */
  .colophon {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-wrap: wrap;
    gap: 12px;
    margin: clamp(48px, 6vw, 80px) auto 0;
    padding-top: 24px;
    max-width: 1180px;
    border-top: 1px solid rgba(216,198,177,0.10);
    font-size: 11px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: #8a7660;
    text-align: center;
  }
  .seal { color: var(--color-ember, #c65f3c); }
  .colophon-dot { color: #5e4c3a; }

  @media (max-width: 880px) {
    .inner {
      grid-template-columns: 1fr 1fr;
    }
    .col-contact { grid-column: 1 / -1; }
  }

  @media (max-width: 560px) {
    .inner {
      grid-template-columns: 1fr;
      gap: 44px;
    }
    .colophon { flex-direction: column; gap: 8px; }
    .colophon-dot { display: none; }
  }

  .contact-row:active {
    border-color: rgba(198,95,60,0.55);
    background: rgba(198,95,60,0.07);
  }

  @media (prefers-reduced-motion: reduce) {
    .contact-row,
    .contact-row:hover,
    .nav-arrow { transition: none; transform: none; }
  }

  @media (pointer: coarse) {
    .contact-row:hover { transform: none; }
    .contact-row:hover .contact-icon { transform: none; }
  }
</style>
