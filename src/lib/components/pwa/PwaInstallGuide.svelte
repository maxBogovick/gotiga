<script lang="ts">
  // iPhone и iPad не показывают окно установки: сайт добавляют вручную через «Поделиться».
  // Здесь три шага словами и знаком кнопки, чтобы её узнали в панели браузера.
  import { onMount } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { t } from '$lib/i18n';
  import { portal } from '$lib/actions/portal';
  import { focusTrap } from '$lib/actions/focusTrap';

  let { onclose }: { onclose: () => void } = $props();

  let reduceMotion = $state(false);

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
  }

  // Фокус внутрь окна, Tab по кругу и возврат фокуса на кнопку — `focusTrap`.
  onMount(() => {
    reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  });
</script>

<!-- Шапка размывает фон (`backdrop-filter`) и запирает `position: fixed` в своей высоте,
     поэтому окно переносится в `body`. -->
<div use:portal>
<button
  type="button"
  class="scrim"
  aria-label={$t('dockClose')}
  onclick={onclose}
  transition:fade|global={{ duration: reduceMotion ? 0 : 150 }}
></button>
<div
  use:focusTrap
  class="guide"
  role="dialog"
  aria-modal="true"
  aria-labelledby="pwa-guide-title"
  tabindex="-1"
  transition:fly|global={{ y: 24, duration: reduceMotion ? 0 : 200 }}
>
  <div class="head">
    <h2 id="pwa-guide-title">{$t('pwaIosTitle')}</h2>
    <button type="button" class="close" onclick={onclose} aria-label={$t('dockClose')}>✕</button>
  </div>
  <ol class="steps">
    <li>
      <span class="n" aria-hidden="true">1</span>
      <span class="txt">{$t('pwaIosStep1')}</span>
      <svg class="ico" width="22" height="22" viewBox="0 0 24 24" fill="none" aria-hidden="true">
        <path d="M12 15V3m0 0L8 7m4-4 4 4" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" />
        <path d="M7 11H6a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-6a2 2 0 0 0-2-2h-1" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
      </svg>
    </li>
    <li>
      <span class="n" aria-hidden="true">2</span>
      <span class="txt">{$t('pwaIosStep2')}</span>
      <svg class="ico" width="22" height="22" viewBox="0 0 24 24" fill="none" aria-hidden="true">
        <rect x="4" y="4" width="16" height="16" rx="3.5" stroke="currentColor" stroke-width="1.6" />
        <path d="M12 8.5v7M8.5 12h7" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
      </svg>
    </li>
    <li>
      <span class="n" aria-hidden="true">3</span>
      <span class="txt">{$t('pwaIosStep3')}</span>
    </li>
  </ol>
</div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 600;
    border: 0;
    padding: 0;
    background: rgba(52, 37, 28, 0.38);
    cursor: default;
  }

  .guide {
    position: fixed;
    z-index: 601;
    left: 12px;
    right: 12px;
    bottom: calc(12px + env(safe-area-inset-bottom, 0px));
    max-width: 26rem;
    margin: 0 auto;
    padding: 16px 18px 20px;
    background: var(--color-canvas-base, #f8f1e7);
    border: 1px solid var(--border, #d8c6b1);
    border-radius: 14px;
    box-shadow: 0 18px 44px rgba(52, 37, 28, 0.22);
    color: var(--color-ink-primary, #34251c);
  }
  .guide:focus {
    outline: none;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 8px;
  }
  h2 {
    margin: 0;
    font-family: var(--font-title, Georgia, serif);
    font-size: 1.15rem;
    font-weight: 400;
  }
  .close {
    width: 44px;
    height: 44px;
    margin-right: -12px;
    border: 0;
    background: none;
    color: var(--color-ink-secondary, #5b4638);
    font-size: 16px;
    cursor: pointer;
  }
  .close:focus-visible {
    outline: 2px solid var(--color-ember, #c65f3c);
    outline-offset: -4px;
  }

  .steps {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .steps li {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 0;
    border-top: 1px solid color-mix(in srgb, var(--color-ink-primary, #34251c) 10%, transparent);
    font-family: var(--font-body, system-ui, sans-serif);
    font-size: 14px;
    line-height: 1.4;
  }
  .n {
    flex: none;
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: 50%;
    border: 1px solid var(--color-ember-deep, #6f3b24);
    color: var(--color-ember-deep, #6f3b24);
    font-size: 13px;
  }
  .txt {
    flex: 1 1 auto;
  }
  .ico {
    flex: none;
    color: var(--color-ember-deep, #6f3b24);
  }
</style>
