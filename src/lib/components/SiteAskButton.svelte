<script lang="ts">
  // Кнопка «Спросить сайт» в шапке. Окно и логика ответов грузятся при первом
  // нажатии (`SiteAskHost`), здесь только знак и подпись.
  import { t } from '$lib/i18n';
  import { siteAskUi } from '$lib/stores/site-ask-ui.svelte';

  let { variant = 'pill' }: { variant?: 'pill' | 'icon' } = $props();
</script>

<button
  type="button"
  class="ask-btn"
  class:icon={variant === 'icon'}
  aria-haspopup="dialog"
  aria-expanded={siteAskUi.open}
  title={$t('askTitle')}
  aria-label={variant === 'icon' ? $t('askTitle') : undefined}
  onclick={() => siteAskUi.toggle('button')}
>
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" aria-hidden="true">
    <path d="M10 3.5l1.9 5.6 5.6 1.9-5.6 1.9L10 18.5l-1.9-5.6L2.5 11l5.6-1.9z" fill="currentColor" />
    <path d="M18.5 3l.8 2.2 2.2.8-2.2.8-.8 2.2-.8-2.2-2.2-.8 2.2-.8z" fill="currentColor" opacity="0.7" />
  </svg>
  {#if variant === 'pill'}<span class="label">{$t('askTitle')}</span>{/if}
</button>

<style>
  .ask-btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 36px;
    padding: 0 14px 0 11px;
    border: 1px solid var(--color-border-soft, #d8c6b1);
    border-radius: 18px;
    background: none;
    color: var(--color-ink-secondary, #5a3420);
    font-family: var(--font-display, Fraunces, Georgia, serif);
    font-style: italic;
    font-size: 14px;
    white-space: nowrap;
    cursor: pointer;
    transition: color 0.2s ease, background 0.2s ease, border-color 0.2s ease;
  }
  .ask-btn.icon {
    width: 36px;
    padding: 0;
    justify-content: center;
    border-color: transparent;
    border-radius: 50%;
  }
  .ask-btn:hover,
  .ask-btn[aria-expanded='true'] {
    color: var(--color-ember-deep, #9a4120);
    border-color: color-mix(in srgb, var(--color-ember, #b44e24) 40%, transparent);
    background: color-mix(in srgb, var(--color-ember, #b44e24) 8%, transparent);
  }
  .ask-btn:focus-visible {
    outline: 2px solid rgba(180, 78, 36, 0.6);
    outline-offset: 3px;
  }

  /* Шапка свёрнута прокруткой или окно уже: остаётся один знак. */
  :global(.site-header.is-scrolled) .ask-btn:not(.icon) {
    width: 36px;
    padding: 0;
    justify-content: center;
    border-color: transparent;
    border-radius: 50%;
  }
  :global(.site-header.is-scrolled) .ask-btn:not(.icon) .label {
    display: none;
  }
  @media (max-width: 1360px) {
    .ask-btn:not(.icon) {
      width: 36px;
      padding: 0;
      justify-content: center;
      border-color: transparent;
      border-radius: 50%;
    }
    .ask-btn:not(.icon) .label {
      display: none;
    }
  }
</style>
