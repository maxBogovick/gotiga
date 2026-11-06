<script lang="ts">
  // Режимы игры одной полосой в строке двери боя: обучение, прохождение, забег.
  // Стоит и на странице битв, и на странице забега, чтобы между режимами можно
  // было переходить в одно нажатие.
  import { t } from '$lib/i18n';

  let {
    current,
    runOpen,
    base = '',
  }: {
    current: 'training' | 'campaign' | 'run';
    /** Есть ли опубликованный маршрут: без него пункта «Забег» нет. */
    runOpen: boolean;
    /** Куда вести режимы битв: на их странице пусто (тот же адрес, другой `?mode=`). */
    base?: string;
  } = $props();
</script>

<nav class="modes" aria-label={$t('battleModesLabel')}>
  <a
    href="{base}?mode=training"
    class:on={current === 'training'}
    aria-current={current === 'training' ? 'page' : undefined}
    data-sveltekit-noscroll
    data-sveltekit-replacestate={base ? undefined : ''}>{$t('battleModeTraining')}</a
  >
  <a
    href="{base}?mode=campaign"
    class:on={current === 'campaign'}
    aria-current={current === 'campaign' ? 'page' : undefined}
    data-sveltekit-noscroll
    data-sveltekit-replacestate={base ? undefined : ''}>{$t('battleModeCampaign')}</a
  >
  {#if runOpen}
    <a href="/battles/run" class:on={current === 'run'} aria-current={current === 'run' ? 'page' : undefined}
      >{$t('battlesRoomRun')}</a
    >
  {/if}
</nav>

<style>
  .modes {
    display: inline-flex;
    margin-left: 0.4rem;
    border: 1px solid #d8c6b1;
  }

  .modes a {
    padding: 0.32rem 0.85rem;
    color: #6f3b24;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.74rem;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    text-decoration: none;
  }

  .modes a + a {
    border-left: 1px solid #d8c6b1;
  }

  .modes a:hover:not(.on) {
    color: #c65f3c;
  }

  .modes a.on {
    background: #6f3b24;
    color: #f8f1e7;
  }

  /* На телефоне строка двери переносится: режимы встают отдельной строкой во
     всю ширину после разрыва, рядом с кошельком им места нет. */
  @media (max-width: 40rem) {
    .modes {
      order: 2;
      flex-basis: 100%;
      margin-left: 0;
    }

    .modes a {
      flex: 1;
      text-align: center;
    }
  }
</style>
