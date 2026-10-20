<script lang="ts">
  // Один ответ: пауза с точками, затем текст по словам, затем ссылки.
  // Печатание — только вид: для чтеца с экрана полный текст стоит рядом
  // целиком и появляется один раз, а печатаемый скрыт (`aria-hidden`).
  import { t } from '$lib/i18n';
  import { resolveMediaUrl } from '$lib/api';
  import { interact } from '$lib/analytics';
  import { siteAsk, prefersReducedMotion, type AskTurn } from '$lib/stores/site-ask.svelte';
  import { siteAskUi } from '$lib/stores/site-ask-ui.svelte';

  let { turn, ontick }: { turn: AskTurn; ontick: () => void } = $props();

  const WORD_MS = 34;

  const paragraphs = $derived(turn.reply?.paragraphs ?? []);
  const words = $derived(paragraphs.map((p) => p.split(' ')));
  const total = $derived(words.reduce((n, w) => n + w.length, 0));

  let shown = $state(0);

  $effect(() => {
    if (!turn.reply) return;
    if (turn.typed || prefersReducedMotion()) {
      shown = total;
      siteAsk.markTyped(turn.id);
      return;
    }
    shown = 0;
    const every = setInterval(() => {
      shown += 1;
      ontick();
      if (shown >= total) {
        clearInterval(every);
        siteAsk.markTyped(turn.id);
      }
    }, WORD_MS);
    return () => clearInterval(every);
  });

  // Дописали по сигналу снаружи (новый вопрос): показать целиком.
  $effect(() => {
    if (turn.typed) shown = total;
  });

  const visible = $derived.by(() => {
    let left = shown;
    return words.map((w) => {
      const take = Math.max(0, Math.min(w.length, left));
      left -= take;
      return w.slice(0, take).join(' ');
    });
  });

  const writing = $derived(!turn.typed && !!turn.reply);
  /** Абзац, в конце которого стоит черта печатающего пера. */
  const lastShown = $derived(visible.reduce((at, p, i) => (p ? i : at), -1));

  function followed(kind: string) {
    interact('site_ask_link', kind);
    siteAskUi.hide();
  }
</script>

{#if turn.failed}
  <p class="line">{$t('askFailed')}</p>
{:else if !turn.reply}
  <p class="thinking" role="status">
    <span class="dots" aria-hidden="true"><i></i><i></i><i></i></span>
    {$t('askThinking')}
  </p>
{:else}
  <div class="sr">
    {#each paragraphs as p}<p>{p}</p>{/each}
  </div>
  <div class="text" aria-hidden="true">
    {#each visible as p, i}
      {#if p}
        <p class="line">{p}{#if writing && i === lastShown}<span class="caret"></span>{/if}</p>
      {/if}
    {/each}
  </div>
  {#if turn.typed && turn.reply.links.length}
    <ul class="links">
      {#each turn.reply.links as link, i (link.href + i)}
        <li style:--i={i}>
          {#if link.kind === 'question'}
            <button type="button" class="ask-again" onclick={() => siteAsk.ask(link.label, 'chip')}>
              <span class="name">{link.label}</span>
              <span class="arrow" aria-hidden="true">?</span>
            </button>
          {:else}
          <a href={link.href} onclick={() => followed(link.kind)}>
            {#if link.image}
              <span class="thumb"><img src={resolveMediaUrl(link.image)} alt="" loading="lazy" decoding="async" /></span>
            {/if}
            <span class="name">{link.label}</span>
            {#if link.hint}<span class="hint">{link.hint}</span>{/if}
            <span class="arrow" aria-hidden="true">→</span>
          </a>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
{/if}

<style>
  .line {
    margin: 0 0 0.6em;
    font-family: var(--font-body, Georgia, serif);
    font-size: 16px;
    line-height: 1.55;
    color: var(--ak-ink);
  }
  .text :global(p:last-child) {
    margin-bottom: 0;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  .caret {
    display: inline-block;
    width: 2px;
    height: 1em;
    margin-left: 2px;
    vertical-align: -0.12em;
    background: var(--ak-ember);
    animation: blink 0.9s steps(1) infinite;
  }
  @keyframes blink {
    50% {
      opacity: 0;
    }
  }

  .thinking {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0;
    font-family: var(--font-display, Fraunces, Georgia, serif);
    font-style: italic;
    font-size: 15px;
    color: var(--ak-ink-3);
  }
  .dots {
    display: inline-flex;
    gap: 4px;
  }
  .dots i {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--ak-ember);
    opacity: 0.35;
    animation: pulse 1.1s ease-in-out infinite;
  }
  .dots i:nth-child(2) {
    animation-delay: 0.15s;
  }
  .dots i:nth-child(3) {
    animation-delay: 0.3s;
  }
  @keyframes pulse {
    40% {
      opacity: 1;
      transform: translateY(-2px);
    }
  }

  .links {
    list-style: none;
    margin: 14px 0 0;
    padding: 0;
    display: grid;
    gap: 2px;
    border-top: 1px solid var(--ak-line);
  }
  .links li {
    animation: rise 0.35s ease both;
    animation-delay: calc(var(--i) * 45ms);
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
  }
  .links a,
  .links .ask-again {
    width: 100%;
    border-top: 0;
    border-left: 0;
    border-right: 0;
    background: none;
    text-align: left;
    cursor: pointer;
    font: inherit;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 4px;
    border-bottom: 1px solid var(--ak-line);
    color: var(--ak-ink);
    text-decoration: none;
    transition: background 0.2s ease, color 0.2s ease;
  }
  .links a:hover,
  .links a:focus-visible,
  .links .ask-again:hover,
  .links .ask-again:focus-visible {
    background: color-mix(in srgb, var(--ak-ember) 7%, transparent);
    color: var(--ak-ember-deep);
  }
  .links a:focus-visible,
  .links .ask-again:focus-visible {
    outline: 2px solid rgba(180, 78, 36, 0.6);
    outline-offset: -2px;
  }
  .thumb {
    flex: none;
    width: 30px;
    height: 38px;
    overflow: hidden;
    border-radius: 15px 15px 2px 2px;
    background: var(--ak-line);
  }
  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .name {
    flex: 1;
    min-width: 0;
    font-family: var(--font-display, Fraunces, Georgia, serif);
    font-size: 16px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hint {
    flex: none;
    font-family: var(--font-ui, Inter, system-ui, sans-serif);
    font-size: 12px;
    color: var(--ak-ink-3);
  }
  .arrow {
    flex: none;
    color: var(--ak-ember-deep);
  }

  @media (prefers-reduced-motion: reduce) {
    .caret,
    .dots i,
    .links li {
      animation: none;
    }
    .dots i {
      opacity: 0.7;
    }
  }
</style>
