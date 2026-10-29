<script lang="ts">
  // «Установить приложение»: одна кнопка на шапку, лист «Ещё» и полосу телефона.
  // Что она делает, решает хранилище: родной диалог там, где он есть, а на iPhone — объяснение.
  // Нет ни того, ни другого (приложение уже стоит, браузер не умеет) — кнопки нет вовсе.
  import { t } from '$lib/i18n';
  import { interact } from '$lib/analytics';
  import { pwa } from '$lib/stores/pwa.svelte';

  let {
    variant = 'header',
    where = 'header',
    onpress,
  }: {
    variant?: 'header' | 'row' | 'strip';
    /** Откуда нажали: уходит в замер (`pwa_install`). */
    where?: string;
    /** Зовётся после нажатия (лист «Ещё» закрывается). */
    onpress?: () => void;
  } = $props();

  async function press() {
    onpress?.();
    if (pwa.installMode === 'prompt') {
      interact('pwa_install_press', where, null, false);
      await pwa.install();
    } else if (pwa.installMode === 'ios') {
      interact('pwa_install_press', where, null, false);
      // Окно подсказки стоит в раскладке: эта кнопка может исчезнуть сама (лист «Ещё»
      // закрывается, полоса уходит), и окно, жившее в ней, исчезало бы с ней.
      pwa.openGuide();
    }
  }
</script>

{#if pwa.installMode !== 'none'}
  <button
    type="button"
    class="install install--{variant}"
    onclick={press}
    aria-label={$t('pwaInstall')}
    title={$t('pwaInstall')}
  >
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <path d="M12 3v12m0 0-4-4m4 4 4-4" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" />
      <path d="M4 17v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" />
    </svg>
    <span class="label">{variant === 'row' ? $t('pwaInstall') : $t('pwaInstallShort')}</span>
  </button>
{/if}

<style>
  .install {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-family: var(--font-body, 'Instrument Sans', system-ui, sans-serif);
    cursor: pointer;
    white-space: nowrap;
    -webkit-tap-highlight-color: transparent;
  }
  .install svg {
    flex: none;
  }
  .install:focus-visible {
    outline: 2px solid #a8482a;
    outline-offset: 2px;
  }

  /* Шапка (только компьютер): контурная печать с подписью; на узком окне — один знак. */
  .install--header {
    display: none;
  }
  @media (min-width: 1025px) {
    .install--header {
      display: inline-flex;
      height: 36px;
      padding: 0 14px 0 12px;
      background: transparent;
      border: 1px solid #a8482a;
      border-radius: 999px;
      color: #a8482a;
      font-size: 11px;
      font-weight: 600;
      letter-spacing: 0.08em;
      text-transform: uppercase;
      transition: background 0.25s ease, color 0.25s ease;
    }
    .install--header:hover {
      background: #a8482a;
      color: #fff9f0;
    }
  }
  @media (min-width: 1025px) and (max-width: 1359px) {
    .install--header {
      width: 36px;
      padding: 0;
      justify-content: center;
    }
    .install--header .label {
      position: absolute;
      width: 1px;
      height: 1px;
      overflow: hidden;
      clip-path: inset(50%);
    }
  }

  /* Строка листа «Ещё». */
  .install--row {
    width: 100%;
    height: 48px;
    padding: 0;
    background: none;
    border: 0;
    border-bottom: 1px solid color-mix(in srgb, var(--color-ink-primary, #34251c) 8%, transparent);
    color: #a8482a;
    font-size: 13px;
    font-weight: 500;
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }

  /* Полоса над нижней панелью. */
  .install--strip {
    flex: none;
    height: 36px;
    padding: 0 14px;
    background: #a8482a;
    border: 1px solid #a8482a;
    border-radius: 999px;
    color: #fff9f0;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
  }
</style>
