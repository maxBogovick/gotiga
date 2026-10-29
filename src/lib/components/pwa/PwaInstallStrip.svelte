<script lang="ts">
  // Полоса «Установить» над нижней панелью телефона. Один раз за визит: появляется через
  // две секунды, уходит при прокрутке дальше экрана и по крестику; закрытую крестиком
  // не показываем месяц. Это не окно поверх страницы: ничего не загораживает ввод.
  import { onMount } from 'svelte';
  import { fly } from 'svelte/transition';
  import { t } from '$lib/i18n';
  import { pwa } from '$lib/stores/pwa.svelte';
  import PwaInstallButton from './PwaInstallButton.svelte';

  let ready = $state(false);
  let scrolled = $state(false);
  let reduceMotion = $state(false);

  // Сообщения об обновлении и о сети важнее приглашения и стоят на том же месте: пока они
  // на экране, полоса уступает.
  let visible = $derived(
    ready && !scrolled && pwa.installMode !== 'none' && !pwa.updateReady && !pwa.offline,
  );

  function close() {
    pwa.quietStrip();
    ready = false;
  }

  onMount(() => {
    reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    if (pwa.stripQuiet()) return;
    const timer = setTimeout(() => (ready = true), 2000);
    const onScroll = () => {
      if (window.scrollY > Math.max(window.innerHeight, 400)) scrolled = true;
    };
    window.addEventListener('scroll', onScroll, { passive: true });
    return () => {
      clearTimeout(timer);
      window.removeEventListener('scroll', onScroll);
    };
  });
</script>

{#if visible}
  <aside class="strip" aria-label={$t('pwaInstall')} transition:fly={{ y: 16, duration: reduceMotion ? 0 : 200 }}>
    <p class="ask">{$t('pwaInstallAsk')}</p>
    <PwaInstallButton variant="strip" where="strip" onpress={() => (ready = false)} />
    <button type="button" class="x" onclick={close} aria-label={$t('dockClose')}>✕</button>
  </aside>
{/if}

<style>
  .strip {
    display: none;
  }

  @media (max-width: 1024px) {
    .strip {
      position: fixed;
      z-index: 185;
      left: 8px;
      right: 8px;
      bottom: calc(var(--dock-visible-h, var(--section-bar-h, 64px)) + 8px);
      transition: bottom 0.22s ease;
      display: flex;
      align-items: center;
      gap: 10px;
      padding: 8px 4px 8px 14px;
      background: var(--color-canvas-base, #f8f1e7);
      border: 1px solid var(--border, #d8c6b1);
      border-radius: 14px;
      box-shadow: 0 8px 24px rgba(52, 37, 28, 0.14);
    }
    @media (prefers-reduced-motion: reduce) {
      .strip {
        transition: none;
      }
    }
    .ask {
      flex: 1 1 auto;
      margin: 0;
      font-family: var(--font-body, system-ui, sans-serif);
      font-size: 12px;
      line-height: 1.35;
      color: var(--color-ink-secondary, #5b4638);
    }
    .x {
      flex: none;
      width: 44px;
      height: 44px;
      border: 0;
      background: none;
      color: var(--color-ink-secondary, #5b4638);
      font-size: 15px;
      cursor: pointer;
    }
    .x:focus-visible {
      outline: 2px solid #a8482a;
      outline-offset: -4px;
    }
  }
</style>
