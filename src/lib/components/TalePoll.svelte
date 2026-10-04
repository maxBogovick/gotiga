<script lang="ts">
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import { api } from '$lib/api';
  import { t, lang } from '$lib/i18n';
  import { authStore } from '$lib/stores/auth.svelte';
  import { visitorBook } from '$lib/stores/visitor-book.svelte';
  import { visitorToken } from '$lib/visitorToken';
  import { knownReaderEmail } from '$lib/readerEmail';
  import type { TalePoll } from '$lib/types/api';
  import AppImage from './AppImage.svelte';
  import LetterSlip from './LetterSlip.svelte';

  /**
   * «О ком записать следующую небылицу?» — голосование, которое назначил дом.
   *
   * Чисел здесь нет, и это решение, а не недосмотр: голосов мало, а при малых
   * числах счёт говорит о случае, а не о выборе, и первый же голос тянул бы
   * за собой остальных. Любой исход годится — кандидатов выбрал дом, — поэтому
   * и трёх голосов достаточно. Счёт видит только стол рассказов.
   *
   * Грузится само, после монтирования: полка и байка пререндерятся, и
   * голосование, запечённое в сборку, стояло бы там и после закрытия.
   */
  let {
    place,
    letters = true,
  }: {
    /** Где стоит: под байкой или на полке. Нужен для имён полей. */
    place: 'tale' | 'shelf';
    /** Есть ли у дома чем написать. Нет — просьбы о письме не печатаются. */
    letters?: boolean;
  } = $props();

  let poll = $state<TalePoll | null>(null);
  let busy = $state(false);
  let error = $state('');
  let slipOpen = $state(false);

  let known = $derived(knownReaderEmail());
  let winner = $derived(poll?.winner ?? null);

  onMount(() => {
    visitorBook.load();
    void api.getTalePoll(visitorToken()).then((found) => (poll = found));
  });

  /**
   * Выбор отвечает пальцу сразу, а сервер потом называет настоящее. Не
   * записалось — возвращается прежний выбор: отмеченная работа, за которую
   * голоса нет, хуже, чем неотмеченная.
   */
  async function choose(figurineId: string) {
    if (!poll || busy || poll.state !== 'open') return;
    const was = poll.myChoice;
    poll.myChoice = figurineId;
    busy = true;
    error = '';
    try {
      const out = await api.voteTalePoll(
        poll.id,
        { visitorToken: visitorToken(), figurineId, lang: $lang },
        authStore.token,
      );
      poll.myChoice = out.myChoice;
      poll.myLetter = out.myLetter;
    } catch {
      poll.myChoice = was;
      error = $t('talesPollError');
    } finally {
      busy = false;
    }
  }

  async function askLetter(email: string, ageConfirmed: boolean) {
    if (!poll?.myChoice || busy) return;
    if (!email) {
      error = $t('talesLetterNeedEmail');
      return;
    }
    if (!ageConfirmed) {
      error = $t('formAgeConfirmRequired');
      return;
    }
    busy = true;
    error = '';
    try {
      const out = await api.voteTalePoll(
        poll.id,
        {
          visitorToken: visitorToken(),
          figurineId: poll.myChoice,
          email,
          lang: $lang,
          ageConfirmed: true,
        },
        authStore.token,
      );
      poll.myLetter = out.myLetter;
      slipOpen = false;
    } catch {
      error = $t('talesLetterError');
    } finally {
      busy = false;
    }
  }
</script>

{#snippet letterAsk()}
  {#if letters && poll}
    {#if poll.myLetter}
      <p class="kept">{$t('talesPollLetterKept')}</p>
    {:else if known}
      <button type="button" class="quiet" onclick={() => askLetter(known, true)} disabled={busy}>
        {$t('talesPollLetterTo').replace('{email}', known)}
      </button>
    {:else if slipOpen}
      <LetterSlip
        id="tale-poll-{place}"
        submitLabel={$t('talesLetterSend')}
        {busy}
        {error}
        onsubmit={askLetter}
      />
    {:else}
      <button type="button" class="quiet" onclick={() => (slipOpen = true)}>
        {$t('talesPollLetterAsk')}
      </button>
    {/if}
  {/if}
{/snippet}

{#if poll && (poll.state === 'open' || winner)}
  <section class="poll" class:on-shelf={place === 'shelf'} aria-labelledby="tale-poll-title-{place}" in:fade={{ duration: 500 }}>
    <p class="kicker">{$t('talesPollKicker')}</p>

    {#if poll.state === 'open'}
      <h2 class="question" id="tale-poll-title-{place}">{$t('talesPollQuestion')}</h2>
      <p class="rule">{$t('talesPollRule')}</p>
      <ul class="choices" class:choices--four={poll.candidates.length === 4} style="--n: {poll.candidates.length}">
        {#each poll.candidates as work (work.figurineId)}
          {@const mine = poll.myChoice === work.figurineId}
          <li>
            <button
              type="button"
              class="choice"
              class:mine
              aria-pressed={mine}
              onclick={() => choose(work.figurineId)}
              disabled={busy}
            >
              <span class="arch">
                {#if work.imageUrl}
                  <AppImage src={work.imageUrl} alt="" class="arch-img" sizes="160px" />
                {/if}
              </span>
              <span class="name">{work.name}</span>
              <span class="mark" aria-hidden={!mine}>{mine ? $t('talesPollMine') : ''}</span>
            </button>
          </li>
        {/each}
      </ul>
      {#if poll.myChoice}
        <p class="thanks">{$t('talesPollThanks')}</p>
        {@render letterAsk()}
      {/if}
    {:else if winner}
      <div class="chosen" id="tale-poll-title-{place}">
        <span class="arch arch--small">
          {#if winner.imageUrl}
            <AppImage src={winner.imageUrl} alt="" class="arch-img" sizes="72px" />
          {/if}
        </span>
        <p class="chosen-line">{$t('talesPollChosen').replace('{name}', winner.name)}</p>
      </div>
      {#if poll.myChoice}
        {@render letterAsk()}
      {/if}
    {/if}

    {#if error && !slipOpen}<p class="err" role="alert">{error}</p>{/if}
  </section>
{/if}

<style>
  .poll {
    max-width: 62ch;
  }
  .poll.on-shelf {
    max-width: none;
    margin: clamp(40px, 6vw, 64px) 0 0;
    padding-top: clamp(28px, 4vw, 44px);
    border-top: 1px solid var(--line, #d8c6b1);
  }

  .kicker {
    margin: 0 0 10px;
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: var(--copper, #c65f3c);
  }

  .question {
    margin: 0 0 6px;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(24px, 2.8vw, 32px);
    font-weight: 400;
    line-height: 1.1;
    color: var(--ink, #34251c);
  }

  .rule {
    margin: 0 0 18px;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 17px;
    font-style: italic;
    line-height: 1.45;
    color: var(--muted, #5f4636);
  }

  .choices {
    display: grid;
    grid-template-columns: repeat(var(--n, 3), minmax(0, 1fr));
    gap: clamp(12px, 2vw, 22px);
    max-width: calc(var(--n, 3) * 170px);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .choice {
    display: grid;
    gap: 8px;
    width: 100%;
    padding: 0;
    background: none;
    border: none;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .choice:disabled { cursor: default; }
  .choice:focus-visible { outline: 2px solid rgba(198, 95, 60, 0.6); outline-offset: 5px; }

  /* Та же стрельчатая арка, что на полке: выбирают из тех же дверей. */
  .arch {
    display: block;
    aspect-ratio: 3 / 4;
    overflow: hidden;
    background: #1a120e;
    clip-path: url(#tale-arch);
    transition: transform 0.35s cubic-bezier(0.2, 0.8, 0.3, 1);
  }
  .arch :global(.app-image-wrap),
  .arch :global(img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .arch :global(img) {
    filter: sepia(0.3) contrast(0.95) brightness(0.9);
    transition: filter 0.4s ease;
  }
  .choice:hover .arch { transform: translateY(-4px); }
  .choice:hover .arch :global(img),
  .choice.mine .arch :global(img) { filter: none; }

  .name {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 16px;
    line-height: 1.2;
    color: var(--ink, #34251c);
    padding-top: 7px;
    border-top: 2px solid var(--brown, #6f3b24);
  }
  .choice.mine .name { border-top-color: var(--copper, #c65f3c); }

  .mark {
    min-height: 1em;
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--copper, #c65f3c);
  }

  .thanks,
  .kept,
  .chosen-line {
    margin: 16px 0 0;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 17px;
    font-style: italic;
    color: var(--muted, #5f4636);
  }
  .kept { color: var(--deep, #6f3b24); }

  .chosen {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .chosen .chosen-line { margin: 0; font-size: 19px; color: var(--ink, #34251c); }
  .arch--small { width: 56px; flex-shrink: 0; }

  .quiet {
    margin-top: 10px;
    padding: 0 0 2px;
    background: none;
    border: none;
    border-bottom: 1px solid rgba(198, 95, 60, 0.4);
    color: var(--deep, #6f3b24);
    font: inherit;
    font-size: 14px;
    cursor: pointer;
    transition: color 0.2s, border-color 0.2s;
  }
  .quiet:hover { color: var(--copper, #c65f3c); border-color: var(--copper, #c65f3c); }
  .quiet:disabled { opacity: 0.55; cursor: default; }

  .err {
    margin: 10px 0 0;
    font-size: 12px;
    color: #8a2a2a;
  }

  @media (max-width: 480px) {
    .name { font-size: 14px; }
    /* Четыре лица в ряд на телефоне — это полоска по семьдесят точек, а не
       лица. Два на два. */
    .choices--four {
      grid-template-columns: repeat(2, minmax(0, 1fr));
      max-width: 300px;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .arch { transition: none; }
    .choice:hover .arch { transform: none; }
  }
</style>
