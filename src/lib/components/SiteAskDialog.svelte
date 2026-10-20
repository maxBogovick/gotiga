<script lang="ts">
  // Окно «Спросить сайт». Нейросети за ним нет: вопрос разбирается по словам и
  // отвечается из данных сайта (`site-ask.ts`). Об этом сказано в самом окне.
  //
  // Родной `<dialog>` с `showModal()`: он встаёт над всей страницей и не зависит
  // от шапки, которая размывает фон и держит свой `position: fixed`.
  import { tick } from 'svelte';
  import { afterNavigate } from '$app/navigation';
  import { page } from '$app/state';
  import { hasPageContext } from '$lib/site-ask-page';
  import { t } from '$lib/i18n';
  import { siteAsk, QUESTION_MAX } from '$lib/stores/site-ask.svelte';
  import { siteAskUi } from '$lib/stores/site-ask-ui.svelte';
  import SiteAskAnswer from '$lib/components/SiteAskAnswer.svelte';

  let dialog = $state<HTMLDialogElement | null>(null);
  let input = $state<HTMLInputElement | null>(null);
  let log = $state<HTMLElement | null>(null);
  let text = $state('');

  const onPage = $derived(hasPageContext({ path: page.url.pathname, card: page.url.searchParams.get('card') }));
  const chips = $derived([
    ...(onPage ? [$t('askChipPage')] : []),
    ...(['askChipSummary', 'askChipCount', 'askChipNew', 'askChipTale', 'askChipStart', 'askChipPlay', 'askChipOrder', 'askChipWitch'] as const).map(
      (key) => $t(key),
    ),
  ]);

  $effect(() => {
    if (!dialog) return;
    if (siteAskUi.open && !dialog.open) {
      dialog.showModal();
      void tick().then(() => input?.focus());
    } else if (!siteAskUi.open && dialog.open) {
      dialog.close();
    }
  });

  afterNavigate(() => siteAskUi.hide());

  // Ответ растёт вниз: держим конец на виду, пока человек сам не прокрутил вверх.
  function stick(slack = 90) {
    if (!log) return;
    if (log.scrollHeight - log.scrollTop - log.clientHeight < slack) log.scrollTop = log.scrollHeight;
  }
  $effect(() => {
    void siteAsk.turns.length;
    void tick().then(() => {
      if (log) log.scrollTop = log.scrollHeight;
    });
  });

  // Дописанный ответ достраивается ссылками: показать и их, если читали с конца.
  $effect(() => {
    void siteAsk.turns.at(-1)?.typed;
    void tick().then(() => stick(260));
  });

  function submit(e: SubmitEvent) {
    e.preventDefault();
    const q = text;
    text = '';
    void siteAsk.ask(q, 'input');
  }

  function pickChip(q: string) {
    void siteAsk.ask(q, 'chip');
    input?.focus();
  }

  // Нажатие по фону (по самому `<dialog>`, а не по его содержимому) закрывает окно.
  function onBackdrop(e: MouseEvent) {
    if (e.target === dialog) siteAskUi.hide();
  }
</script>

<dialog
  bind:this={dialog}
  class="ask"
  aria-labelledby="ask-title"
  onclose={() => {
    siteAsk.cancelAction();
    siteAskUi.hide();
  }}
  onclick={onBackdrop}
>
  <div class="sheet">
    <header class="head">
      <div>
        <h2 id="ask-title">{$t('askTitle')}</h2>
        <p class="note">{$t('askNote')} {$t('askMissNote')}</p>
      </div>
      <button type="button" class="close" aria-label={$t('askClose')} onclick={() => siteAskUi.hide()}>×</button>
    </header>

    <div class="log" bind:this={log}>
      {#if siteAsk.turns.length === 0}
        <p class="empty">{$t('askEmpty')}</p>
      {/if}
      {#each siteAsk.turns as turn (turn.id)}
        <section class="turn">
          <p class="q">{turn.question}</p>
          <div class="a"><SiteAskAnswer {turn} ontick={() => stick()} /></div>
        </section>
      {/each}
    </div>

    <div class="foot">
      <div class="chips" role="group" aria-label={$t('askChipsLabel')}>
        {#each chips as c (c)}
          <button type="button" class="chip" onclick={() => pickChip(c)}>{c}</button>
        {/each}
      </div>
      <form class="form" onsubmit={submit}>
        <input
          bind:this={input}
          bind:value={text}
          type="text"
          class="input"
          maxlength={QUESTION_MAX}
          placeholder={$t('askPlaceholder')}
          aria-label={$t('askPlaceholder')}
          autocomplete="off"
          enterkeyhint="send"
        />
        <button type="submit" class="send" disabled={!text.trim()}>{$t('askSend')}</button>
      </form>
    </div>
  </div>
</dialog>

<style>
  .ask {
    --ak-paper: #fbf7ef;
    --ak-line: var(--color-border-soft, #d8c6b1);
    --ak-rule: #b9a184;
    --ak-ink: var(--color-ink-primary, #34251c);
    --ak-ink-2: var(--color-ink-secondary, #5a3420);
    --ak-ink-3: var(--color-ink-tertiary, #7a5035);
    --ak-ember: var(--color-ember, #b44e24);
    --ak-ember-deep: var(--color-ember-deep, #9a4120);

    padding: 0;
    border: 0;
    background: none;
    color: var(--ak-ink);
    width: min(660px, calc(100vw - 24px));
    max-width: none;
    max-height: none;
    overflow: visible;
    margin: 8vh auto auto;
  }
  .ask::backdrop {
    background: rgba(52, 37, 28, 0.38);
    backdrop-filter: blur(2px);
  }
  .ask[open] {
    animation: unfold 0.32s cubic-bezier(0.2, 0.7, 0.2, 1) both;
  }
  .ask[open]::backdrop {
    animation: dim 0.3s ease both;
  }
  @keyframes unfold {
    from {
      opacity: 0;
      transform: translateY(-8px) scale(0.985);
    }
  }
  @keyframes dim {
    from {
      opacity: 0;
    }
  }

  /* Рамка с поворотом на 1° и двойная граница — как у остальных окон сайта. */
  .sheet {
    position: relative;
    display: flex;
    flex-direction: column;
    max-height: min(680px, 84vh);
    background: var(--ak-paper);
    box-shadow:
      0 0 0 1px var(--ak-rule),
      0 0 0 5px var(--ak-paper),
      0 0 0 6px var(--ak-line),
      0 30px 60px -24px rgba(60, 30, 15, 0.5);
  }
  .sheet::before {
    content: '';
    position: absolute;
    inset: -9px;
    border: 1px solid var(--ak-line);
    transform: rotate(1deg);
    pointer-events: none;
  }

  .head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 16px;
    padding: 22px 26px 14px;
    border-bottom: 1px solid var(--ak-line);
  }
  h2 {
    margin: 0;
    font-family: var(--font-display, Fraunces, Georgia, serif);
    font-style: italic;
    font-weight: 500;
    font-size: 26px;
    line-height: 1.15;
  }
  .note {
    margin: 6px 0 0;
    font-family: var(--font-ui, Inter, system-ui, sans-serif);
    font-size: 12.5px;
    line-height: 1.45;
    color: var(--ak-ink-3);
    max-width: 46ch;
  }
  .close {
    flex: none;
    width: 34px;
    height: 34px;
    border: 0;
    border-radius: 50%;
    background: none;
    color: var(--ak-ink-2);
    font-size: 24px;
    line-height: 1;
    cursor: pointer;
  }
  .close:hover {
    background: color-mix(in srgb, var(--ak-ember) 10%, transparent);
    color: var(--ak-ember-deep);
  }

  .log {
    flex: 1;
    min-height: 140px;
    overflow-y: auto;
    padding: 18px 26px 6px;
    overscroll-behavior: contain;
  }
  .empty {
    margin: 4px 0 18px;
    font-family: var(--font-display, Fraunces, Georgia, serif);
    font-style: italic;
    font-size: 17px;
    color: var(--ak-ink-3);
  }
  .turn {
    margin-bottom: 22px;
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
    padding: 10px 26px 18px;
    border-top: 1px solid var(--ak-line);
  }
  .chips {
    display: flex;
    gap: 6px;
    margin-bottom: 10px;
    overflow-x: auto;
    padding-bottom: 4px;
    scrollbar-width: thin;
  }
  .chip {
    flex: none;
    padding: 5px 12px;
    border: 1px solid var(--ak-line);
    border-radius: 14px;
    background: none;
    color: var(--ak-ink-2);
    font-family: var(--font-ui, Inter, system-ui, sans-serif);
    font-size: 12.5px;
    cursor: pointer;
    transition: color 0.2s ease, border-color 0.2s ease, background 0.2s ease;
  }
  .chip:hover {
    color: var(--ak-ember-deep);
    border-color: color-mix(in srgb, var(--ak-ember) 45%, transparent);
    background: color-mix(in srgb, var(--ak-ember) 7%, transparent);
  }
  .form {
    display: flex;
    gap: 10px;
    align-items: stretch;
  }
  .input {
    flex: 1;
    min-width: 0;
    height: 42px;
    padding: 0 14px;
    border: 1px solid var(--ak-rule);
    border-radius: 0;
    background: #fff9;
    color: var(--ak-ink);
    font-family: var(--font-display, Fraunces, Georgia, serif);
    font-style: italic;
    font-size: 17px;
    caret-color: var(--ak-ember);
  }
  .input:focus {
    outline: none;
    border-color: var(--ak-ember);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--ak-ember) 20%, transparent);
  }
  .send {
    flex: none;
    padding: 0 20px;
    border: 0;
    background: var(--ak-ember-deep);
    color: #fbf7ef;
    font-family: var(--font-ui, Inter, system-ui, sans-serif);
    font-size: 14px;
    cursor: pointer;
    transition: background 0.2s ease, opacity 0.2s ease;
  }
  .send:hover:not(:disabled) {
    background: var(--ak-ember);
  }
  .send:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .send:focus-visible,
  .chip:focus-visible,
  .close:focus-visible {
    outline: 2px solid rgba(180, 78, 36, 0.6);
    outline-offset: 2px;
  }

  @media (max-width: 720px) {
    .ask {
      width: 100vw;
      margin: auto 0 0;
    }
    .sheet {
      max-height: 90dvh;
      padding-bottom: env(safe-area-inset-bottom, 0px);
      box-shadow: 0 -20px 40px -20px rgba(60, 30, 15, 0.5);
    }
    .sheet::before {
      display: none;
    }
    .head,
    .log,
    .foot {
      padding-left: 18px;
      padding-right: 18px;
    }
    .input {
      font-size: 16px;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .ask[open],
    .ask[open]::backdrop {
      animation: none;
    }
  }
</style>
