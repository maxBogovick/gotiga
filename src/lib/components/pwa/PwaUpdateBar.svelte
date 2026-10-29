<script lang="ts">
  // Две тихие строки внизу страницы: «доступна новая версия» и «нет сети».
  // Область объявлена сразу и живёт всегда: читалка объявляет то, что в неё положили,
  // а область, появившаяся вместе с текстом, молчит.
  import { t } from '$lib/i18n';
  import { pwa } from '$lib/stores/pwa.svelte';
</script>

<div class="bar" role="status" aria-live="polite" aria-atomic="false">
  {#if pwa.updateReady}
    <div class="card">
      <span class="txt">{$t('pwaUpdateReady')}</span>
      <button type="button" class="act" onclick={() => pwa.applyUpdate()}>{$t('pwaUpdateNow')}</button>
      <button type="button" class="later" onclick={() => pwa.postponeUpdate()}>{$t('pwaUpdateLater')}</button>
    </div>
  {/if}
  {#if pwa.offline}
    <div class="card card--offline">
      <span class="txt">{$t('pwaOffline')}</span>
    </div>
  {/if}
</div>

<style>
  .bar {
    position: fixed;
    z-index: 520;
    left: 12px;
    bottom: calc(var(--dock-visible-h, var(--section-bar-h, 0px)) + 12px);
    transition: bottom 0.22s ease;
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: calc(100vw - 24px);
    pointer-events: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .bar {
      transition: none;
    }
  }
  .card {
    pointer-events: auto;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px 10px;
    padding: 8px 8px 8px 14px;
    background: var(--color-canvas-base, #f8f1e7);
    border: 1px solid var(--border, #d8c6b1);
    border-radius: 12px;
    box-shadow: 0 8px 24px rgba(52, 37, 28, 0.16);
    font-family: var(--font-body, system-ui, sans-serif);
    font-size: 13px;
    color: var(--color-ink-primary, #34251c);
  }
  .card--offline {
    padding: 10px 14px;
  }
  .txt {
    line-height: 1.35;
  }
  .act,
  .later {
    min-height: 44px;
    padding: 0 14px;
    border-radius: 999px;
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    cursor: pointer;
  }
  .act {
    background: #a8482a;
    border: 1px solid #a8482a;
    color: #fff9f0;
  }
  .later {
    background: none;
    border: 1px solid transparent;
    color: var(--color-ink-secondary, #5b4638);
  }
  .act:focus-visible,
  .later:focus-visible {
    outline: 2px solid #a8482a;
    outline-offset: 2px;
  }
</style>
