<script lang="ts">
  // «Послушать»: небылицу читает голос самого браузера (`speechSynthesis`).
  // Ничего не грузится с сервера и никуда не уходит: звук собирает устройство
  // читателя. Кнопки нет, если у браузера нет голоса на языке страницы.
  //
  // Читается кусками до ~220 знаков: в Chrome длинная реплика обрывается
  // примерно на пятнадцатой секунде, а короткие доходят до конца.
  // Озвучка живёт отдельно от страницы: при переходе на другую страницу и
  // закрытии вкладки она обязана замолчать, иначе небылица дочитывалась бы
  // поверх следующей.
  import { onMount, untrack } from 'svelte';
  import { beforeNavigate } from '$app/navigation';
  import { t, lang } from '$lib/i18n';
  import { interact } from '$lib/analytics';

  let { title, spoken }: { title: string; spoken: string[] } = $props();

  const CHUNK_MAX = 220;
  const RATES = [0.9, 1, 1.15] as const;
  const BEST_WORDS = /(enhanced|premium|natural|neural|online|siri)/i;
  // Шуточные голоса macOS (и «Eddy», «Flo» на каждом языке): сказку ими не читают.
  const NOVELTY = /(grandma|grandpa|eddy|flo|reed|rocko|sandy|shelley|albert|bad news|bahh|bells|boing|bubbles|cellos|good news|jester|organ|superstar|trinoids|whisper|wobble|zarvox|fred|junior|kathy|ralph)/i;

  let supported = $state(false);
  let voice = $state<SpeechSynthesisVoice | null>(null);
  let state = $state<'idle' | 'playing' | 'paused'>('idle');
  let at = $state(0);
  let rateAt = $state(1);
  let chunks = $state<string[]>([]);
  // Каждая остановка меняет метку: запоздавший `onend` старого куска ничего не запускает.
  let token = 0;

  function split(text: string): string[] {
    if ([...text].length <= CHUNK_MAX) return [text];
    const out: string[] = [];
    let held = '';
    for (const s of text.split(/(?<=[.!?…])\s+/)) {
      if (held && [...held].length + 1 + [...s].length > CHUNK_MAX) {
        out.push(held);
        held = s;
      } else {
        held = held ? `${held} ${s}` : s;
      }
    }
    if (held) out.push(held);
    return out;
  }

  function pickVoice() {
    const all = window.speechSynthesis.getVoices();
    const own = all.filter((v) => v.lang.toLowerCase().replace('_', '-').startsWith($lang) && !NOVELTY.test(v.name));
    voice =
      own.find((v) => BEST_WORDS.test(v.name)) ??
      own.find((v) => v.default) ??
      own[0] ??
      null;
  }

  onMount(() => {
    if (!('speechSynthesis' in window) || typeof SpeechSynthesisUtterance === 'undefined') return;
    supported = true;
    pickVoice();
    window.speechSynthesis.addEventListener?.('voiceschanged', pickVoice);
    const hush = () => stop(false);
    window.addEventListener('pagehide', hush);
    return () => {
      window.speechSynthesis.removeEventListener?.('voiceschanged', pickVoice);
      window.removeEventListener('pagehide', hush);
      stop(false);
    };
  });

  // Язык сменили или открыли другую небылицу — голос и текст уже не те.
  $effect(() => {
    void $lang;
    void title;
    if (supported) {
      // Без `untrack` эффект следил бы и за `state`, и нажатие «Послушать» тут же его останавливало.
      untrack(() => {
        stop(false);
        pickVoice();
      });
    }
  });

  beforeNavigate(() => stop(false));

  function speakFrom(i: number, mine: number) {
    if (mine !== token) return;
    if (i >= chunks.length) {
      state = 'idle';
      at = 0;
      interact('tale_listen', 'done');
      return;
    }
    const u = new SpeechSynthesisUtterance(chunks[i]);
    if (voice) {
      u.voice = voice;
      u.lang = voice.lang;
    }
    u.rate = RATES[rateAt];
    u.onend = () => {
      if (mine !== token) return;
      at = i + 1;
      speakFrom(i + 1, mine);
    };
    u.onerror = (e) => {
      // `interrupted` и `canceled` — это наша же остановка; остальное — голос не смог.
      if (mine === token && e.error !== 'interrupted' && e.error !== 'canceled') stop(false);
    };
    at = i;
    window.speechSynthesis.speak(u);
  }

  function start() {
    if (!voice) return;
    stop(false);
    chunks = spoken.flatMap(split);
    if (!chunks.length) return;
    state = 'playing';
    token += 1;
    interact('tale_listen', 'start');
    speakFrom(0, token);
  }

  function stop(manual: boolean) {
    token += 1;
    if (typeof window !== 'undefined' && 'speechSynthesis' in window) window.speechSynthesis.cancel();
    if (state !== 'idle' && manual) interact('tale_listen', 'stop');
    state = 'idle';
    at = 0;
  }

  function toggle() {
    if (state === 'idle') start();
    else if (state === 'playing') {
      window.speechSynthesis.pause();
      state = 'paused';
    } else {
      window.speechSynthesis.resume();
      state = 'playing';
    }
  }

  function faster() {
    rateAt = (rateAt + 1) % RATES.length;
  }

  let progress = $derived(chunks.length && state !== 'idle' ? Math.min(100, Math.round((at / chunks.length) * 100)) : 0);
</script>

{#if supported && voice}
  <div class="listen" role="group" aria-label={$t('talesListen')}>
    <button type="button" class="play" onclick={toggle} aria-pressed={state === 'playing'}>
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" aria-hidden="true">
        {#if state === 'playing'}
          <path d="M8 5v14M16 5v14" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" />
        {:else}
          <path d="M7 4.5v15l13-7.5z" fill="currentColor" />
        {/if}
      </svg>
      <span>
        {#if state === 'idle'}{$t('talesListen')}{:else if state === 'playing'}{$t('talesListenPause')}{:else}{$t('talesListenResume')}{/if}
      </span>
    </button>
    {#if state !== 'idle'}
      <button type="button" class="aux" onclick={() => stop(true)}>{$t('talesListenStop')}</button>
      <button type="button" class="aux rate" onclick={faster} aria-label={$t('talesListenSpeed')} title={$t('talesListenSpeed')}>
        {RATES[rateAt]}×
      </button>
      <span class="bar" aria-hidden="true"><i style:width="{progress}%"></i></span>
    {/if}
  </div>
{/if}

<style>
  .listen {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    margin: 0 0 18px;
  }
  .play,
  .aux {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 34px;
    padding: 0 14px;
    border: 1px solid var(--color-border-soft, #d8c6b1);
    border-radius: 17px;
    background: none;
    color: var(--color-ink-secondary, #5a3420);
    font-family: var(--font-display, Fraunces, Georgia, serif);
    font-style: italic;
    font-size: 14px;
    cursor: pointer;
    transition: color 0.2s ease, background 0.2s ease, border-color 0.2s ease;
  }
  .aux {
    padding: 0 12px;
    font-family: var(--font-ui, Inter, system-ui, sans-serif);
    font-style: normal;
    font-size: 12.5px;
  }
  .play:hover,
  .aux:hover,
  .play[aria-pressed='true'] {
    color: var(--color-ember-deep, #9a4120);
    border-color: color-mix(in srgb, var(--color-ember, #b44e24) 40%, transparent);
    background: color-mix(in srgb, var(--color-ember, #b44e24) 8%, transparent);
  }
  .play:focus-visible,
  .aux:focus-visible {
    outline: 2px solid rgba(180, 78, 36, 0.6);
    outline-offset: 3px;
  }
  .rate {
    min-width: 52px;
    justify-content: center;
    font-variant-numeric: tabular-nums;
  }
  .bar {
    flex: 1 1 80px;
    min-width: 80px;
    max-width: 220px;
    height: 2px;
    background: var(--color-border-soft, #d8c6b1);
  }
  .bar i {
    display: block;
    height: 100%;
    background: var(--color-ember, #b44e24);
    transition: width 0.4s linear;
  }
  @media (prefers-reduced-motion: reduce) {
    .bar i {
      transition: none;
    }
  }
</style>
