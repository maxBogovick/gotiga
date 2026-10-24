<script lang="ts">
  // Разговор в листе поиска: последние три вопроса, как в переписке — старый
  // сверху, новый снизу. Загружается при первом открытии листа вместе с кодом
  // ответов (`site-ask.svelte.ts`), а не на каждой странице: нужен он только тому,
  // кто открыл поиск.
  //
  // Ответ, показанный на лету (`turn.live`), виден, только если он есть: пока
  // сайт не нашёл ответа, о вопросе молчим, иначе под каждой промежуточной
  // фразой стояло бы «не нашлось». Поиск по теме (`search`) тоже не ответ:
  // «нашёл 15 работ» повторяло бы список, стоящий ниже, и числа в них расходились.
  //
  // Кнопки-вопросы и выбор стоят только у последнего ответа: у прежних они уже
  // сыграли, а три списка подряд, в которых повторяются одни и те же вопросы,
  // не говорят, куда нажимать.
  import { tick } from 'svelte';
  import { t } from '$lib/i18n';
  import { siteAsk, prefersReducedMotion } from '$lib/stores/site-ask.svelte';
  import SiteAskAnswer from '$lib/components/SiteAskAnswer.svelte';

  let { onfollow, compact = false }: { onfollow: () => void; compact?: boolean } = $props();

  const shown = $derived(
    siteAsk.turns
      .filter((x) => !x.live || (!!x.reply && !x.reply.missed && !x.failed && x.reply.intent !== 'search'))
      .slice(-3),
  );
  const lastId = $derived(shown.at(-1)?.id ?? null);

  let box = $state<HTMLDivElement | null>(null);

  /** Показать последний ответ: ровно настолько прокрутить вниз, чтобы был виден его
   *  конец, но не дальше, чем начало его вопроса, — читают сверху, и печатаемый
   *  текст растёт вниз, не унося вопрос за край. */
  function follow(smooth = false) {
    const last = box?.querySelector<HTMLElement>('.turn:last-of-type');
    if (!box || !last) return;
    const top = last.offsetTop - box.offsetTop - 12;
    const bottom = last.offsetTop - box.offsetTop + last.offsetHeight - box.clientHeight + 12;
    const to = Math.max(0, Math.min(top, Math.max(box.scrollTop, bottom)));
    if (Math.abs(to - box.scrollTop) < 1) return;
    box.scrollTo({ top: to, behavior: smooth && !prefersReducedMotion() ? 'smooth' : 'auto' });
  }

  // Новый вопрос, ответ на него и ссылки под ним, которые появляются после
  // печати, — каждое меняет высоту, и конец разговора обязан остаться на виду.
  $effect(() => {
    const last = shown.at(-1);
    if (!last) return;
    void [last.id, !!last.reply, last.typed];
    void tick().then(() => follow(true));
  });
</script>

{#if shown.length}
  <div class="talk" class:compact bind:this={box}>
    {#each shown as turn (turn.id)}
      <section class="turn">
        {#if !turn.live}<p class="q">{turn.question}</p>{/if}
        <div class="a"><SiteAskAnswer {turn} current={turn.id === lastId} ontick={() => follow()} {onfollow} /></div>
      </section>
    {/each}
    <div class="foot">
      <p class="note">{$t('askMissNote')}</p>
      <button type="button" class="clear" onclick={() => siteAsk.clear()}>{$t('askClear')}</button>
    </div>
  </div>
{/if}

<style>
  .talk {
    --ak-paper: #fbf7ef;
    --ak-line: var(--color-border-soft, #d8c6b1);
    --ak-rule: #b9a184;
    --ak-ink: var(--color-ink-primary, #34251c);
    --ak-ink-2: var(--color-ink-secondary, #5a3420);
    --ak-ink-3: var(--color-ink-tertiary, #7a5035);
    --ak-ember: var(--color-ember, #b44e24);
    --ak-ember-deep: var(--color-ember-deep, #9a4120);

    flex: none;
    max-height: min(46vh, 380px);
    overflow-y: auto;
    overscroll-behavior: contain;
    position: relative;
    padding: 16px 26px 8px;
    border-bottom: 1px solid var(--ak-line);
    color: var(--ak-ink);
  }
  /* Под ответом идут результаты поиска: им нужно место. */
  .talk.compact {
    max-height: min(34vh, 280px);
  }
  .turn {
    margin-bottom: 18px;
  }
  .q {
    margin: 0 0 8px;
    font-family: var(--font-display, Fraunces, Georgia, serif);
    font-style: italic;
    font-size: 15px;
    color: var(--ak-ink-3);
  }
  .q::before {
    content: '— ';
  }
  .a {
    padding-left: 14px;
    border-left: 2px solid var(--ak-line);
  }
  .foot {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 6px;
  }
  .clear {
    flex: none;
    padding: 4px 0;
    border: 0;
    background: none;
    cursor: pointer;
    font-family: var(--font-ui, Inter, system-ui, sans-serif);
    font-size: 11.5px;
    color: var(--ak-ember-deep);
    text-decoration: underline;
    text-underline-offset: 3px;
  }
  .clear:hover,
  .clear:focus-visible {
    color: var(--ak-ember);
  }
  .clear:focus-visible {
    outline: 2px solid rgba(180, 78, 36, 0.6);
    outline-offset: 2px;
  }
  .note {
    margin: 0;
    font-family: var(--font-ui, Inter, system-ui, sans-serif);
    font-size: 11.5px;
    color: var(--ak-ink-3);
  }
</style>
