<script lang="ts">
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import { api } from '$lib/api';
  import { t, lang } from '$lib/i18n';
  import { authStore } from '$lib/stores/auth.svelte';
  import { visitorBook } from '$lib/stores/visitor-book.svelte';
  import { visitorToken } from '$lib/visitorToken';
  import { knownReaderEmail } from '$lib/readerEmail';
  import { CHANNEL_URL } from '$lib/channel';
  import type { GazetteNeighbor, TaleDoors } from '$lib/types/api';
  import LetterSlip from './LetterSlip.svelte';
  import TalePoll from './TalePoll.svelte';
  import TaleSeal from './TaleSeal.svelte';

  /**
   * После последней строки: «прочитано», «хотите продолжение?», «прислать
   * следующую?» и голос «о ком записать следующую».
   *
   * Стоит сразу под откликом, а не внизу страницы: человек, который только
   * что дочитал, — единственный, про кого точно известно, что он провёл с
   * текстом несколько минут, и спрашивать о следующей нужно его и сейчас.
   *
   * Письмо обещается только тогда, когда дому есть чем его отправить
   * (`doors.letters`), а канал — только названный по имени.
   */
  let {
    taleId,
    wants = $bindable(false),
    letter = $bindable(false),
    telegram = $bindable(false),
    sequel = null,
    read = false,
    fresh = false,
  }: {
    taleId: string;
    /** Просил ли этот читатель продолжение. */
    wants?: boolean;
    /**
     * Ждёт ли его просьба письма. Приходит со страницы вместе с просьбой:
     * заведённое здесь, оно на каждом новом заходе было бы «нет», и читатель,
     * уже оставивший адрес, видел бы приглашение оставить его снова.
     */
    letter?: boolean;
    /** Ждёт ли его просьба записки в Telegram — тоже приходит со страницы. */
    telegram?: boolean;
    /** Вышедшее продолжение. Есть — просить нечего, ведём туда. */
    sequel?: GazetteNeighbor | null;
    /** Байка дочитана — на листе стоит печать. */
    read?: boolean;
    /** Дочитана только что — печать ставится на глазах. */
    fresh?: boolean;
  } = $props();

  let doors = $state<TaleDoors>({ letters: false, telegram: null, telegramNotes: false });
  let sequelBusy = $state(false);
  let sequelError = $state('');
  let sequelSlip = $state(false);

  let bookBusy = $state(false);
  let bookError = $state('');
  let bookJustSigned = $state(false);

  // Имя канала назвал сервер — берём его; не назвал, стоит тот, что известен
  // сайту: у канала дома адрес один, и дверь не должна исчезать из-за настройки.
  let channel = $derived(doors.telegram ?? CHANNEL_URL);
  let booked = $derived(visitorBook.signed && !!visitorBook.email.trim());
  // Записку пишет бот, с которым читатель начинал разговор, когда входил:
  // предложить её можно только вошедшему через Telegram.
  let canTelegram = $derived(doors.telegramNotes && !!authStore.user?.telegramLinked);
  let accountEmail = $derived(authStore.user?.email?.trim() ?? '');
  let known = $derived(knownReaderEmail());

  onMount(() => {
    visitorBook.load();
    void api.getTaleDoors().then((d) => (doors = d));
  });

  /** «Хочу продолжение» — назначается; повтор снимает просьбу. */
  async function toggleWish() {
    if (sequelBusy) return;
    const next = !wants;
    wants = next;
    sequelBusy = true;
    sequelError = '';
    try {
      const out = await api.setTaleSequelWish(
        taleId,
        { visitorToken: visitorToken(), want: next, lang: $lang },
        authStore.token,
      );
      wants = out.wants;
      letter = out.letter;
      telegram = out.telegram;
      if (!out.wants) sequelSlip = false;
    } catch {
      wants = !next;
      sequelError = $t('talesSequelError');
    } finally {
      sequelBusy = false;
    }
  }

  async function askSequelLetter(email: string, ageConfirmed: boolean) {
    if (sequelBusy) return;
    if (!email) {
      sequelError = $t('talesLetterNeedEmail');
      return;
    }
    if (!ageConfirmed) {
      sequelError = $t('formAgeConfirmRequired');
      return;
    }
    sequelBusy = true;
    sequelError = '';
    try {
      const out = await api.setTaleSequelWish(
        taleId,
        { visitorToken: visitorToken(), want: true, email, lang: $lang, ageConfirmed: true },
        authStore.token,
      );
      wants = out.wants;
      letter = out.letter;
      sequelSlip = false;
    } catch {
      sequelError = $t('talesLetterError');
    } finally {
      sequelBusy = false;
    }
  }

  /** Записка в Telegram, когда продолжение выйдет: одно нажатие, адрес не нужен. */
  async function askSequelTelegram() {
    if (sequelBusy) return;
    sequelBusy = true;
    sequelError = '';
    try {
      const out = await api.setTaleSequelWish(
        taleId,
        { visitorToken: visitorToken(), want: true, telegram: true, lang: $lang },
        authStore.token,
      );
      wants = out.wants;
      telegram = out.telegram;
    } catch {
      sequelError = $t('talesSequelError');
    } finally {
      sequelBusy = false;
    }
  }

  /**
   * «Прислать следующую» — это книга дома, вписанная с другой стороны:
   * один список, одна дверь из него (`/unsubscribe/…`). Второй список рядом с
   * книгой разошёлся бы с ней на первой же отписке.
   */
  async function signForNext(email: string, ageConfirmed: boolean) {
    if (bookBusy) return;
    if (!email) {
      bookError = $t('talesLetterNeedEmail');
      return;
    }
    if (!ageConfirmed) {
      bookError = $t('formAgeConfirmRequired');
      return;
    }
    bookBusy = true;
    bookError = '';
    try {
      const res = await api.subscribe({
        email,
        name: authStore.user?.displayName ?? null,
        source: 'tale',
        lang: $lang,
        ageConfirmed: true,
      });
      visitorBook.sign(res.unsubscribeToken, email, authStore.user?.displayName ?? '');
      bookJustSigned = true;
    } catch {
      bookError = $t('talesLetterError');
    } finally {
      bookBusy = false;
    }
  }
</script>

<section class="afterword" aria-label={$t('talesAfterwordLabel')}>
  {#if read}
    <p class="read" in:fade={{ duration: fresh ? 0 : 400 }}>
      <TaleSeal size={30} pressed={fresh} />
      <span>{$t('talesReadSeal')}</span>
    </p>
  {/if}

  <!-- Продолжение: вышло — ведём туда; нет — спрашиваем, ждут ли. -->
  <div class="part">
    {#if sequel}
      <p class="kicker">{$t('talesSequelOutKicker')}</p>
      <a class="sequel" href="/tales/{sequel.slug}?src=tale_sequel">
        {$lang === 'ru' ? sequel.titleRu || sequel.titleEn : sequel.titleEn || sequel.titleRu} →
      </a>
    {:else}
      <p class="kicker">{$t('talesSequelKicker')}</p>
      <div class="line">
        <button
          type="button"
          class="want"
          class:want--on={wants}
          aria-pressed={wants}
          onclick={toggleWish}
          disabled={sequelBusy}
        >
          <span class="want-mark" aria-hidden="true">{wants ? '✓' : '+'}</span>
          {wants ? $t('talesSequelWanted') : $t('talesSequelWant')}
        </button>
        {#if wants}<span class="note" in:fade={{ duration: 300 }}>{$t('talesSequelNoted')}</span>{/if}
      </div>
      {#if wants && doors.letters}
        {#if letter}
          <p class="kept">{$t('talesSequelLetterKept')}</p>
        {:else if known}
          <button type="button" class="quiet" onclick={() => askSequelLetter(known, true)} disabled={sequelBusy}>
            {$t('talesSequelLetterTo').replace('{email}', known)}
          </button>
        {:else if sequelSlip}
          <LetterSlip
            id="tale-sequel"
            submitLabel={$t('talesLetterSend')}
            busy={sequelBusy}
            error={sequelError}
            onsubmit={askSequelLetter}
          />
        {:else}
          <button type="button" class="quiet" onclick={() => (sequelSlip = true)}>
            {$t('talesSequelLetterAsk')}
          </button>
        {/if}
      {/if}
      {#if wants && canTelegram}
        {#if telegram}
          <p class="kept">{$t('talesSequelTelegramKept')}</p>
        {:else}
          <button type="button" class="quiet quiet--line" onclick={askSequelTelegram} disabled={sequelBusy}>
            {$t('talesSequelTelegramAsk')}
          </button>
        {/if}
      {/if}
      {#if sequelError && !sequelSlip}<p class="err" role="alert">{sequelError}</p>{/if}
    {/if}
  </div>

  <!-- Следующая небылица: письмом (книга дома) или в канале. -->
  {#if doors.letters || channel}
    <div class="part">
      <p class="kicker">{$t('talesNextKicker')}</p>
      <p class="rule">{$t('talesNextRule')}</p>
      {#if doors.letters}
        {#if booked}
          <p class="kept">
            {(bookJustSigned ? $t('talesNextSigned') : $t('talesNextKept')).replace('{email}', visitorBook.email)}
          </p>
        {:else if accountEmail}
          <button type="button" class="quiet" onclick={() => signForNext(accountEmail, true)} disabled={bookBusy}>
            {$t('talesNextTo').replace('{email}', accountEmail)}
          </button>
          {#if bookError}<p class="err" role="alert">{bookError}</p>{/if}
        {:else}
          <LetterSlip
            id="tale-next"
            submitLabel={$t('talesLetterSend')}
            busy={bookBusy}
            error={bookError}
            onsubmit={signForNext}
          />
        {/if}
      {/if}
      {#if channel}
        <a class="channel" href={channel} target="_blank" rel="noopener noreferrer">
          {doors.letters ? $t('talesNextTelegramToo') : $t('talesNextTelegram')} →
        </a>
      {/if}
    </div>
  {/if}

  <div class="part part--poll">
    <TalePoll place="tale" letters={doors.letters} />
  </div>
</section>

<style>
  /* Та же левая граница, что у прозы и отклика: нить слева отмеряет всю
     страницу (`--spine`, `--spine-gap` объявлены на `.page`). */
  .afterword {
    margin: clamp(34px, 5vw, 56px) 0 0 calc(var(--spine, 0px) + var(--spine-gap, 0px));
    max-width: 62ch;
  }

  .read {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 0 0 clamp(26px, 4vw, 40px);
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: var(--deep, #6f3b24);
  }

  .part {
    padding: 22px 0 0;
    margin: 0 0 clamp(26px, 4vw, 38px);
    border-top: 1px solid rgba(52, 37, 28, 0.12);
  }
  .part:empty { display: none; }
  /* Голосования может не быть вовсе — тогда и черты над ним не надо. */
  .part--poll:has(:global(.poll)) { padding-top: 26px; }
  .part--poll:not(:has(:global(.poll))) { border-top: none; padding: 0; margin: 0; }

  .kicker {
    margin: 0 0 10px;
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: var(--copper, #c65f3c);
  }

  .rule {
    margin: 0 0 4px;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 17px;
    font-style: italic;
    line-height: 1.5;
    color: var(--muted, #5f4636);
  }

  .line {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px 16px;
  }

  /* Кнопка того же рода, что голос под байкой: одна семья жестов. */
  .want {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 8px 15px;
    background: transparent;
    border: 1px solid rgba(52, 37, 28, 0.22);
    color: var(--ink, #34251c);
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    cursor: pointer;
    transition: color 0.2s, border-color 0.2s, background 0.2s;
  }
  .want:hover { border-color: rgba(52, 37, 28, 0.45); }
  .want:active { transform: translateY(1px); }
  .want-mark { font-size: 12px; line-height: 1; }
  .want.want--on,
  .want.want--on:hover {
    background: var(--copper, #c65f3c);
    border-color: var(--copper, #c65f3c);
    color: #f8f1e7;
  }

  .note,
  .kept {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 17px;
    font-style: italic;
    color: var(--muted, #5f4636);
  }
  .kept { margin: 12px 0 0; color: var(--deep, #6f3b24); }

  .sequel {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: clamp(22px, 2.6vw, 30px);
    line-height: 1.15;
    color: var(--ink, #34251c);
    text-decoration: none;
    transition: color 0.25s;
  }
  .sequel:hover { color: var(--copper, #c65f3c); }

  .quiet {
    margin-top: 12px;
    padding: 0 0 2px;
    background: none;
    border: none;
    border-bottom: 1px solid rgba(198, 95, 60, 0.4);
    color: var(--deep, #6f3b24);
    font: inherit;
    font-size: 14px;
    text-align: left;
    cursor: pointer;
    transition: color 0.2s, border-color 0.2s;
  }
  .quiet:hover { color: var(--copper, #c65f3c); border-color: var(--copper, #c65f3c); }
  /* Вторая просьба под первой — своей строкой, а не впритык к ней. */
  .quiet--line { display: table; }
  .quiet:disabled { opacity: 0.55; cursor: default; }

  .channel {
    display: inline-block;
    margin-top: 14px;
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--copper, #c65f3c);
    text-decoration: none;
    transition: color 0.25s;
  }
  .channel:hover { color: var(--deep, #6f3b24); }

  .err {
    margin: 10px 0 0;
    font-size: 12px;
    color: #8a2a2a;
  }

  @media (prefers-reduced-motion: reduce) {
    .want { transition: none; }
    .want:active { transform: none; }
  }
</style>
