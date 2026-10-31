<script lang="ts">
  // Уведомления о новом: включение и выбор тем. Живёт в подвале и в листе «Ещё».
  // Без разрешения браузер ничего не показывает, поэтому «Включить» — это нажатие,
  // а не фон: разрешение просится только по нему.
  import { t } from '$lib/i18n';
  import { onMount } from 'svelte';
  import { pwa, PUSH_TOPICS } from '$lib/stores/pwa.svelte';
  import type { PushTopic } from '$lib/types/api';

  const LABEL: Record<PushTopic, 'pwaTopicWorks' | 'pwaTopicTales' | 'pwaTopicGazette' | 'pwaTopicGame'> = {
    works: 'pwaTopicWorks',
    tales: 'pwaTopicTales',
    gazette: 'pwaTopicGazette',
    game: 'pwaTopicGame',
  };

  let { id = 'push' }: { id?: string } = $props();

  // Показываем, когда сервер ответил, что уведомления на нём настроены, и они возможны:
  // браузер умеет — либо это iPhone, где они появятся после установки. Пока ответа нет,
  // панели нет вовсе: мигающая панель, которая потом исчезает, хуже запоздавшей.
  let shown = $derived(
    pwa.pushConfigured === true && (pwa.pushSupported || pwa.pushNeedsInstall),
  );

  // Сервер спрашивается, когда панель вот-вот попадёт на экран (подвал внизу страницы), а не
  // на каждой загрузке: большинство посетителей до подвала не дойдёт. В листе «Ещё» панель
  // создаётся при его открытии — и спрашивается сразу.
  let anchor: HTMLElement | undefined = $state();
  onMount(() => {
    if (!anchor || typeof IntersectionObserver === 'undefined') {
      void pwa.loadPush();
      return;
    }
    const watcher = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) {
          void pwa.loadPush();
          watcher.disconnect();
        }
      },
      { rootMargin: '400px 0px' },
    );
    watcher.observe(anchor);
    return () => watcher.disconnect();
  });

  function toggle(topic: PushTopic) {
    const next = pwa.pushTopics.includes(topic)
      ? pwa.pushTopics.filter((x) => x !== topic)
      : PUSH_TOPICS.filter((x) => x === topic || pwa.pushTopics.includes(x));
    void pwa.setPushTopics(next);
  }

  /** Последнюю отмеченную тему не снять: подписка без тем ничего бы не присылала. */
  const isLast = (topic: PushTopic) => pwa.pushTopics.length === 1 && pwa.pushTopics[0] === topic;
</script>

<span class="anchor" bind:this={anchor}></span>
{#if shown}
  <section class="push" aria-labelledby="{id}-title">
    <h3 id="{id}-title" class="title">{$t('pwaNotifTitle')}</h3>

    {#if pwa.pushNeedsInstall}
      <p class="note">{$t('pwaNotifNeedsInstall')}</p>
    {:else}
      <p class="note">{$t('pwaNotifLead')}</p>

      {#if pwa.pushPermission === 'denied'}
        <p class="note note--warn">{$t('pwaNotifDenied')}</p>
      {:else}
        <ul class="topics">
          {#each PUSH_TOPICS as topic (topic)}
            <li>
              <label class="topic">
                <input
                  type="checkbox"
                  checked={pwa.pushTopics.includes(topic)}
                  disabled={pwa.pushBusy || isLast(topic)}
                  onchange={() => toggle(topic)}
                />
                <span>{$t(LABEL[topic])}</span>
              </label>
            </li>
          {/each}
        </ul>

        {#if pwa.pushSubscribed}
          <p class="state" role="status">{$t('pwaNotifOnNow')}</p>
          <button type="button" class="btn btn--quiet" disabled={pwa.pushBusy} onclick={() => pwa.unsubscribePush()}>
            {$t('pwaNotifOff')}
          </button>
        {:else}
          <button
            type="button"
            class="btn"
            disabled={pwa.pushBusy || pwa.pushTopics.length === 0}
            onclick={() => pwa.subscribePush()}
          >
            {$t('pwaNotifOn')}
          </button>
        {/if}

        {#if pwa.pushFailed}
          <p class="note note--warn" role="alert">{$t('pwaNotifFailed')}</p>
        {/if}
      {/if}
    {/if}
  </section>
{/if}

<style>
  .anchor {
    display: block;
    height: 0;
  }
  .push {
    font-family: var(--font-body, system-ui, sans-serif);
    color: inherit;
  }
  .title {
    margin: 0 0 6px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: inherit;
    opacity: 0.8;
  }
  .note {
    margin: 0 0 10px;
    font-size: 13px;
    line-height: 1.45;
    opacity: 0.85;
  }
  .note--warn {
    opacity: 1;
  }
  .topics {
    margin: 0 0 12px;
    padding: 0;
    list-style: none;
  }
  .topic {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 36px;
    font-size: 14px;
    cursor: pointer;
  }
  .topic input {
    width: 18px;
    height: 18px;
    accent-color: #a8482a;
    flex: none;
  }
  .state {
    margin: 0 0 8px;
    font-size: 13px;
  }
  .btn {
    min-height: 44px;
    padding: 0 18px;
    border: 1px solid #a8482a;
    border-radius: 999px;
    background: #a8482a;
    color: #fff9f0;
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    cursor: pointer;
  }
  .btn:disabled {
    opacity: 0.55;
    cursor: default;
  }
  .btn--quiet {
    background: transparent;
    color: inherit;
    border-color: currentColor;
  }
  .btn:focus-visible,
  .topic input:focus-visible {
    outline: 2px solid #a8482a;
    outline-offset: 2px;
  }
</style>
