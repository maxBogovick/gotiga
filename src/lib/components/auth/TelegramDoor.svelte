<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { t } from '$lib/i18n';
  import { api } from '$lib/api';
  import type { TelegramCodeStatus, UserDto } from '$lib/types/api';

  interface Props {
    /** `link` — привязка к уже вошедшему; тогда сверху приходит его сессия. */
    mode?: 'login' | 'link';
    sessionToken?: string | null;
    ondone?: (sessionToken: string, user: UserDto | null) => void;
  }
  let { mode = 'login', sessionToken = null, ondone }: Props = $props();

  // Опрос раз в две с половиной секунды: за пять минут жизни слова это сто
  // двадцать вопросов — заметно ниже предела, и человек не ждёт лишнего.
  const EVERY = 2500;
  // Печать «вы вошли» перед уходом со страницы. Столько её видно.
  const SEAL_HELD = 1300;
  // Ожидание переживает уход в Telegram. Ключ в sessionStorage, а не в
  // localStorage: ждёт ОДНА вкладка, та самая, которой достанется сессия.
  const KEPT = 'gotiga_tg_door';

  // Дверь не рисуется, пока сервер не сказал, что она есть: кнопка, ведущая в
  // никуда, хуже отсутствующей.
  let enabled = $state(false);
  // Стенд на своей машине: Telegram сюда не достучится никогда — webhook
  // принадлежит рабочему серверу. Пока страница об этом не знала, она честно
  // вела в Telegram, где бот отвечал «слово остыло», — то есть показывала
  // тупик и молчала о нём.
  let local = $state(false);
  let playing = $state(false);
  // Не `state`: в компоненте это имя забирает себе `$state`, и компонент молча
  // уезжает в legacy-режим (CLAUDE.md § 14.1).
  let stage = $state<'idle' | 'waiting' | 'asked' | 'done' | 'refused' | 'cold' | 'failed'>('idle');
  let word = $state('');
  let link = $state('');
  let busy = $state(false);
  let enteredAs = $state('');
  // Сколько слову осталось жить, в секундах. Считается из `expiresAt`, который
  // сервер присылает с самого начала и который прежде выбрасывался.
  let left = $state(0);
  let timer: ReturnType<typeof setInterval> | null = null;
  let clock: ReturnType<typeof setInterval> | null = null;
  let seal: ReturnType<typeof setTimeout> | null = null;
  let code = '';
  let dies = 0;

  let clockFace = $derived(
    `${Math.floor(Math.max(left, 0) / 60)}:${String(Math.max(left, 0) % 60).padStart(2, '0')}`
  );

  onMount(async () => {
    try {
      const conf = await api.telegramLoginConfig();
      enabled = conf.enabled;
      local = conf.local;
    } catch {
      enabled = false; // сервер старой сборки — двери просто нет
    }
    document.addEventListener('visibilitychange', wake);
    if (enabled) resume();
  });

  onDestroy(() => {
    stopPolling();
    if (seal) clearTimeout(seal);
    if (typeof document !== 'undefined') document.removeEventListener('visibilitychange', wake);
  });

  /**
   * Возврат из Telegram на телефоне — это чаще всего не «вкладка снова видна»,
   * а перезагрузка страницы: система выгружает вкладку, пока человек в другом
   * приложении. Прежде вместе с ней пропадало слово, и вернувшийся видел
   * кнопку «Войти через Telegram» — хотя «Это я» он уже нажал и сессия лежала
   * на сервере неотданной. Поэтому слово хранится, а не живёт в памяти.
   */
  function resume() {
    let kept: { code?: string; word?: string; link?: string; dies?: number; mode?: string } | null = null;
    try {
      kept = JSON.parse(sessionStorage.getItem(KEPT) ?? 'null');
    } catch {
      kept = null;
    }
    if (!kept?.code || !kept.word || !kept.link || !kept.dies) return;
    // Чужой обряд: слово, взятое для входа, ничего не значит для привязки.
    if ((kept.mode ?? 'login') !== mode) return forget();
    if (kept.dies <= Date.now()) return forget();
    code = kept.code;
    word = kept.word;
    link = kept.link;
    dies = kept.dies;
    stage = 'waiting';
    startPolling();
    void ask(); // не ждать такта: сессия могла быть подтверждена минуту назад
  }

  function keep() {
    try {
      sessionStorage.setItem(KEPT, JSON.stringify({ code, word, link, dies, mode }));
    } catch { /* приватное окно — тогда просто не переживём перезагрузку */ }
  }

  function forget() {
    try {
      sessionStorage.removeItem(KEPT);
    } catch { /* нечего забывать */ }
  }

  /** Вернулись из Telegram — спрашиваем сразу, не дожидаясь такта: в фоновой
      вкладке браузер растягивает таймеры до минуты. */
  function wake() {
    if (document.visibilityState === 'visible' && timer) void ask();
  }

  function startPolling() {
    stopPolling();
    tick();
    timer = setInterval(() => void ask(), EVERY);
    clock = setInterval(tick, 1000);
  }

  function stopPolling() {
    if (timer) clearInterval(timer);
    if (clock) clearInterval(clock);
    timer = null;
    clock = null;
  }

  /** Часы слова. Остывшее слово гасится здесь же: ждать ответа сервера, чтобы
      сказать то, что уже видно по часам, незачем. */
  function tick() {
    left = Math.max(0, Math.round((dies - Date.now()) / 1000));
    if (left === 0) {
      stopPolling();
      forget();
      stage = 'cold';
    }
  }

  async function begin() {
    if (busy) return;
    busy = true;
    try {
      const res = await api.telegramLoginCode(sessionToken);
      code = res.code;
      word = res.word;
      link = res.link;
      dies = Date.parse(res.expiresAt);
      stage = 'waiting';
      keep();
      startPolling();
    } catch {
      stage = 'failed';
    } finally {
      busy = false;
    }
  }

  async function ask() {
    let res: TelegramCodeStatus;
    try {
      res = await api.telegramLoginStatus(code);
    } catch {
      return; // одна сорвавшаяся попытка — не повод гасить слово
    }
    if (res.state === 'ready' && res.sessionToken) {
      stopPolling();
      forget();
      // Привязка из профиля о себе рассказывает сама, у неё своя строка под
      // кнопкой. Вход — уходит со страницы, и уходить молча ему нельзя:
      // человек возвращался из Telegram на главную и не знал, вошёл ли он.
      if (mode === 'link') {
        ondone?.(res.sessionToken, res.user);
        return;
      }
      enteredAs = res.user?.displayName ?? '';
      stage = 'done';
      const token = res.sessionToken;
      const user = res.user;
      seal = setTimeout(() => ondone?.(token, user), SEAL_HELD);
      return;
    }
    if (res.state === 'refused') {
      stopPolling();
      forget();
      stage = 'refused';
      return;
    }
    if (res.state === 'expired') {
      stopPolling();
      forget();
      stage = 'cold';
      return;
    }
    // waiting → asked: записка дошла. Остальные состояния уже разобраны выше,
    // и молча превращать неизвестное в «ждём» нельзя — слово тогда висело бы
    // вечно.
    if (res.state === 'waiting' || res.state === 'asked') stage = res.state;
  }

  /**
   * Доиграть обряд на стенде. Сервер играет бота тем же путём, каким приходит
   * Telegram; страница после этого ничего не спрашивает — обычный опрос сам
   * заберёт сессию следующим тактом, как если бы человек нажал «Это я» в
   * Telegram.
   */
  async function playHere(yes: boolean) {
    if (playing || !code) return;
    playing = true;
    try {
      await api.telegramPlayLocally(code, yes);
      await ask();
    } catch {
      /* не вышло — слово живо, часы идут, кнопка на месте */
    } finally {
      playing = false;
    }
  }

  function again() {
    stopPolling();
    forget();
    stage = 'idle';
    word = '';
    link = '';
    code = '';
    dies = 0;
    left = 0;
  }
</script>

{#if enabled}
  <div class="door">
    {#if stage === 'idle' || stage === 'failed'}
      {#if mode === 'login'}
        <div class="rule"><span>{$t('authTelegramOr')}</span></div>
      {/if}
      <button class="door-btn" onclick={begin} disabled={busy}>
        <svg width="15" height="15" viewBox="0 0 15 15" fill="none" aria-hidden="true">
          <path d="M1.5 7L13.5 2L11 13L7.5 9.5L5.5 11.5V8.5L11 4L5 8L1.5 7Z"
                stroke="currentColor" stroke-width="1" stroke-linejoin="round" />
        </svg>
        {busy ? '…' : mode === 'link' ? $t('authTelegramLink') : $t('authTelegramButton')}
      </button>
      {#if stage === 'failed'}
        <p class="door-note door-note--warn">{$t('authTelegramFailed')}</p>
      {/if}
    {:else if stage === 'waiting' || stage === 'asked'}
      <p class="door-label">{$t('authTelegramWord')}</p>
      <p class="door-word">{word}</p>
      {#if local}
        <!-- Настоящая ссылка на бота здесь не показывается вовсе: она ведёт
             туда, где это слово неизвестно. Показывать её значит предлагать
             тупик. -->
        <button class="door-btn door-btn--go" onclick={() => playHere(true)} disabled={playing}>
          {playing ? '…' : $t('authTelegramPlayHere')}
        </button>
        <p class="door-note door-stand">{$t('authTelegramStandNote')}</p>
        <button class="door-again" onclick={() => playHere(false)} disabled={playing}>
          {$t('authTelegramPlayRefuse')}
        </button>
      {:else}
        <a class="door-btn door-btn--go" href={link} target="_blank" rel="noopener">
          {$t('authTelegramOpen')}
        </a>

        <!-- Два шага вместо абзаца: первый гаснет сам, когда бот получил слово
             (`asked`). Сервер это знал и прежде, а страница молчала — отсюда
             «получилось у меня или нет». -->
        <ol class="door-steps">
          <li class:passed={stage === 'asked'}>
            <span class="door-mark" aria-hidden="true">{stage === 'asked' ? '✓' : '1'}</span>
            <span>{$t('authTelegramStep1')}</span>
          </li>
          <li class:now={stage === 'asked'}>
            <span class="door-mark" aria-hidden="true">2</span>
            <span>{$t('authTelegramStep2')}</span>
          </li>
        </ol>
      {/if}

      <p class="door-live" role="status">
        <span class="door-pulse" aria-hidden="true"></span>
        <!-- «Ждём Telegram» на стенде — неправда: Telegram сюда не придёт.
             Часы слова остаются: срок у него тот же. -->
        {#if !local}{$t('authTelegramWaiting')} {/if}{$t('authTelegramLives')} {clockFace}
      </p>

      {#if !local}
        <p class="door-note door-note--warn">{$t('authTelegramWarn')}</p>
      {/if}
      <button class="door-again" onclick={again}>{$t('authTelegramAgain')}</button>
    {:else if stage === 'done'}
      <!-- Печать держится секунду: переход на главную без единого слова о том,
           что вход состоялся, читался как «меня выкинуло». -->
      <div class="door-seal" role="status">
        <span class="door-seal-mark" aria-hidden="true">✓</span>
        <span class="door-seal-word">{$t('authTelegramDone')}</span>
        {#if enteredAs}<span class="door-seal-name">{enteredAs}</span>{/if}
      </div>
    {:else}
      <p class="door-note door-note--warn">
        {stage === 'refused' ? $t('authTelegramRefused') : $t('authTelegramCold')}
      </p>
      <button class="door-btn" onclick={begin} disabled={busy}>
        {busy ? '…' : $t('authTelegramAgain')}
      </button>
    {/if}
  </div>
{/if}

<style>
  /* Подпись стенда. Приглушена нарочно: это не часть двери, а сообщение о том,
     где мы находимся. */
  .door-stand {
    max-width: 22rem;
    text-align: center;
  }

  .door {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    margin-top: 14px;
  }

  /* «или» на линии: две двери равны, и ни одна не объявлена главной. */
  .rule {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    margin: 2px 0 6px;
    color: rgba(95, 70, 54, 0.4);
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 12px;
    font-style: italic;
  }
  .rule::before,
  .rule::after {
    content: '';
    flex: 1;
    height: 1px;
    background: #d8c6b1;
  }

  /* Своя кнопка, а не телеграмовская: чужой синий прямоугольник на пергаменте
     — чужой предмет, и перекрасить его нельзя, он в чужой рамке. */
  .door-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    padding: 9px 18px;
    font-family: Georgia, serif;
    font-size: 14px;
    letter-spacing: 0.02em;
    color: #6f3b24;
    background: rgba(255, 255, 255, 0.45);
    border: 1px solid #d8c6b1;
    border-radius: 3px;
    cursor: pointer;
    text-decoration: none;
    transition: border-color 0.2s, color 0.2s, background 0.2s;
  }
  .door-btn:hover:not(:disabled) {
    border-color: #c65f3c;
    color: #c65f3c;
  }
  .door-btn:disabled { opacity: 0.6; cursor: default; }
  .door-btn--go {
    background: #6f3b24;
    border-color: #6f3b24;
    color: #f8f1e7;
  }
  /* Та же форма селектора, что у общего наведения: иначе общее правило
     (оно на один класс «весомее») перебивает цвет надписи собственным, и
     подпись пропадает под курсором, ровно там, где на неё смотрят. */
  .door-btn--go:hover:not(:disabled) {
    background: #c65f3c;
    border-color: #c65f3c;
    color: #f8f1e7;
  }

  .door-label {
    margin: 4px 0 0;
    font-family: 'Instrument Sans', sans-serif;
    font-size: 0.7rem;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: #9a7c5c;
  }

  /* Слово сверяют взглядом, поэтому оно крупное, с разрядкой и без соседей. */
  .door-word {
    margin: 0;
    font-family: Georgia, serif;
    font-size: 1.9rem;
    letter-spacing: 0.16em;
    color: #34251c;
    padding: 6px 16px;
    border: 1px solid #d8c6b1;
    border-radius: 3px;
    background: rgba(255, 255, 255, 0.5);
  }

  .door-note {
    margin: 0;
    max-width: 320px;
    text-align: center;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 13px;
    font-style: italic;
    line-height: 1.45;
    color: rgba(95, 70, 54, 0.6);
  }
  .door-note--warn { color: #c65f3c; }

  /* Два шага дороги: что уже случилось и что осталось сделать. */
  .door-steps {
    list-style: none;
    margin: 2px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 5px;
    max-width: 320px;
    width: 100%;
  }
  .door-steps li {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 13px;
    line-height: 1.4;
    color: rgba(95, 70, 54, 0.6);
    transition: color 0.3s;
  }
  .door-steps li.passed { color: rgba(95, 70, 54, 0.4); }
  .door-steps li.now { color: #6f3b24; }
  .door-mark {
    flex: 0 0 auto;
    width: 17px;
    height: 17px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid #d8c6b1;
    border-radius: 50%;
    font-family: 'Instrument Sans', sans-serif;
    font-size: 9px;
    color: #9a7c5c;
    margin-top: 1px;
  }
  .door-steps li.passed .door-mark {
    border-color: #6f3b24;
    background: #6f3b24;
    color: #f8f1e7;
  }
  .door-steps li.now .door-mark { border-color: #c65f3c; color: #c65f3c; }

  /* Признак того, что окно живое, и сколько слову осталось: неподвижная серая
     страница выглядит одинаково и когда она ждёт, и когда всё сломалось. */
  .door-live {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 2px 0 0;
    font-family: 'Instrument Sans', sans-serif;
    font-size: 0.66rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: #9a7c5c;
  }
  .door-pulse {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #c65f3c;
    animation: door-breathe 1.8s ease-in-out infinite;
  }
  @keyframes door-breathe {
    0%, 100% { opacity: 0.25; }
    50%      { opacity: 1; }
  }
  @media (prefers-reduced-motion: reduce) {
    .door-pulse { animation: none; opacity: 0.8; }
  }

  /* Печать: тот же язык, что у сургуча в модальных окнах. */
  .door-seal {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 12px 20px;
    border: 1px solid #c0a384;
    border-radius: 3px;
    background: #f4ead8;
  }
  .door-seal-mark {
    width: 26px;
    height: 26px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    background: #6f3b24;
    color: #f8f1e7;
    font-size: 13px;
  }
  .door-seal-word {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 15px;
    letter-spacing: 0.06em;
    color: #34251c;
  }
  .door-seal-name {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 13px;
    font-style: italic;
    color: rgba(95, 70, 54, 0.7);
  }

  .door-again {
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 12px;
    font-style: italic;
    color: rgba(95, 70, 54, 0.5);
    text-decoration: underline;
    text-decoration-style: dotted;
    text-underline-offset: 3px;
  }
  .door-again:hover { color: #6f3b24; }
</style>
