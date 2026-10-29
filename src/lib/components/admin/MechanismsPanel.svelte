<script lang="ts">
  /**
   * Вкладка «Механизмы»: всё, что на сайте работает само, чем пользуетесь вы, какие скрипты лежат
   * на компьютере и что просить у Claude. Описания — в `$lib/admin/mechanisms.ts`; здесь они
   * сведены с журналом фоновых проходов (`GET /admin/jobs`), чтобы у каждого прохода было видно,
   * когда он шёл в последний раз и чем кончился.
   *
   * Тексты вкладки русские: она читается одним человеком, как и «Behaviour» читается по-английски.
   */
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import type { JobStatus } from '$lib/types/api';
  import {
    AUTO,
    HAND_TOOLS,
    SCRIPTS,
    SKILLS,
    DESCRIBED_JOBS,
    type AutoMechanism,
  } from '$lib/admin/mechanisms';

  let {
    tabs,
    onOpenTab,
  }: {
    /** Вкладки админки с их названиями на языке админки: по ним строятся кнопки «Открыть». */
    tabs: { id: string; label: string }[];
    onOpenTab: (id: string) => void;
  } = $props();

  let jobs = $state<JobStatus[]>([]);
  let error = $state('');
  let loaded = $state(false);
  let now = $state(Date.now());

  const tabLabel = (id: string) => tabs.find((t) => t.id === id)?.label ?? id;
  let byKey = $derived(new Map(jobs.map((j) => [j.job, j])));

  async function load() {
    try {
      jobs = await api.getJobs();
      error = '';
    } catch (e) {
      error = String((e as Error)?.message ?? e);
    } finally {
      loaded = true;
      now = Date.now();
    }
  }

  onMount(() => {
    void load();
    // Пока вкладка открыта, статус обновляется сам; раз в минуту достаточно: самый частый
    // проход идёт раз в пять минут.
    const timer = setInterval(() => void load(), 60_000);
    return () => clearInterval(timer);
  });

  type Mood = 'ok' | 'late' | 'failed' | 'never';
  const RANK: Record<Mood, number> = { ok: 0, never: 1, late: 2, failed: 3 };

  function moodOf(key: string): Mood {
    const j = byKey.get(key);
    if (!j) return 'never';
    return j.state;
  }
  function entryMood(a: AutoMechanism): Mood {
    // Механизм без фоновых проходов в журнал не попадает: ему нечего показывать, кроме описания.
    if (a.jobs.length === 0) return 'ok';
    let worst: Mood = 'ok';
    for (const k of a.jobs) {
      const m = moodOf(k);
      if (RANK[m] > RANK[worst]) worst = m;
    }
    // «Ещё не шёл» у всех проходов сразу — это одно состояние, а у части — просто тихая часть.
    if (a.jobs.every((k) => moodOf(k) === 'never')) return 'never';
    return worst;
  }

  const MOOD_TEXT: Record<Mood, string> = {
    ok: 'идёт',
    late: 'давно не шёл',
    failed: 'последний запуск не удался',
    never: 'с запуска сервера не шёл',
  };

  function ago(iso: string | null | undefined): string {
    if (!iso) return '—';
    const s = Math.max(0, Math.round((now - Date.parse(iso)) / 1000));
    if (s < 60) return 'только что';
    const m = Math.round(s / 60);
    if (m < 60) return `${m} мин назад`;
    const h = Math.round(m / 60);
    if (h < 48) return `${h} ч назад`;
    return `${Math.round(h / 24)} дн назад`;
  }

  let attention = $derived.by(() => {
    let failed = 0;
    let late = 0;
    for (const a of AUTO) {
      const m = entryMood(a);
      if (m === 'failed') failed++;
      else if (m === 'late') late++;
    }
    return { failed, late };
  });

  /** Проходы, о которых знает сервер, а описания здесь нет. */
  let undescribed = $derived(jobs.filter((j) => !DESCRIBED_JOBS.has(j.job)));
</script>

<div class="mech">
  <header class="intro">
    <h2>Механизмы</h2>
    <p>
      Всё, что на сайте работает само, чем вы пользуетесь руками, какие скрипты лежат на компьютере
      и что можно попросить у Claude. Описания лежат в
      <code>src/lib/admin/mechanisms.ts</code>: новый механизм вписывается туда, и он появляется здесь.
    </p>
  </header>

  {#if error}
    <p class="note note--bad">Журнал проходов не прочитан: {error}</p>
  {/if}

  {#if loaded && !error && (attention.failed || attention.late)}
    <p class="note note--bad">
      {#if attention.failed}Сбой в последнем запуске: {attention.failed}.{/if}
      {#if attention.late}Давно не шли: {attention.late}.{/if}
      Подробности ниже, причина — в журнале сервера.
    </p>
  {/if}

  <section>
    <h3>Работает само</h3>
    <ul class="list">
      {#each AUTO as a (a.id)}
        {@const mood = entryMood(a)}
        <li class="row">
          <span class="dot dot--{mood}" title={MOOD_TEXT[mood]} aria-label={MOOD_TEXT[mood]}></span>
          <div class="body">
            <div class="title">
              <strong>{a.title}</strong>
              <span class="when">{a.schedule}</span>
            </div>
            <p class="what">{a.what}</p>

            {#if a.jobs.length}
              <ul class="runs">
                {#each a.jobs as key (key)}
                  {@const j = byKey.get(key)}
                  <li class="run run--{moodOf(key)}">
                    {#if !j}
                      <span class="key">{key}</span>
                      <span class="muted">с запуска сервера не шёл</span>
                    {:else}
                      <span class="key">{key}</span>
                      <span>
                        {#if j.state === 'failed'}
                          сбой {ago(j.finishedAt)}
                          {#if j.inARow > 1}<span class="muted">· подряд: {j.inARow}</span>{/if}
                        {:else if j.state === 'late'}
                          последний раз {ago(j.startedAt)}, а должен каждые {Math.round(j.everySecs / 60)} мин
                        {:else}
                          шёл {ago(j.startedAt)}
                        {/if}
                      </span>
                      {#if j.note}
                        <span class="muted">· делал: {j.note}{#if j.didAt} ({ago(j.didAt)}){/if}</span>
                      {/if}
                      {#if j.error}
                        <span class="bad">· {j.error}</span>
                      {/if}
                    {/if}
                  </li>
                {/each}
              </ul>
            {/if}
          </div>
          {#if a.tab}
            <button type="button" class="open" onclick={() => onOpenTab(a.tab!)}>{tabLabel(a.tab)} →</button>
          {/if}
        </li>
      {/each}
    </ul>

    {#if undescribed.length}
      <div class="orphans">
        <h4>Проходы без описания</h4>
        <p class="muted">
          Сервер пишет о них в журнал, а в <code>mechanisms.ts</code> их нет. Впишите, чтобы они не
          потерялись.
        </p>
        <ul class="runs">
          {#each undescribed as j (j.job)}
            <li class="run run--{j.state}">
              <span class="key">{j.job}</span>
              <span>{j.state === 'failed' ? 'сбой' : 'шёл'} {ago(j.finishedAt)}</span>
              {#if j.note}<span class="muted">· делал: {j.note}</span>{/if}
              {#if j.error}<span class="bad">· {j.error}</span>{/if}
            </li>
          {/each}
        </ul>
      </div>
    {/if}
  </section>

  <section>
    <h3>Запускаете вы</h3>
    <ul class="list">
      {#each HAND_TOOLS as t (t.id)}
        <li class="row row--plain">
          <div class="body">
            <div class="title"><strong>{t.title}</strong></div>
            <p class="what">{t.what}</p>
            {#if t.where}<p class="where">Где: {t.where}</p>{/if}
          </div>
          {#if t.tab}
            <button type="button" class="open" onclick={() => onOpenTab(t.tab!)}>{tabLabel(t.tab)} →</button>
          {/if}
        </li>
      {/each}
    </ul>
  </section>

  <section>
    <h3>Скрипты на компьютере</h3>
    <ul class="list">
      {#each SCRIPTS as s (s.title)}
        <li class="row row--plain">
          <div class="body">
            <div class="title"><strong>{s.title}</strong></div>
            <p class="what">{s.what}</p>
            <p class="cmd"><code>{s.run}</code></p>
          </div>
        </li>
      {/each}
    </ul>
  </section>

  <section>
    <h3>Что просить у Claude</h3>
    <ul class="list">
      {#each SKILLS as k (k.skill)}
        <li class="row row--plain">
          <div class="body">
            <div class="title">
              <strong>{k.title}</strong>
              <span class="when"><code>{k.skill}</code></span>
            </div>
            <p class="what">{k.say}</p>
          </div>
        </li>
      {/each}
    </ul>
  </section>
</div>

<style>
  .mech {
    max-width: 62rem;
    padding: 1.2rem 1.4rem 2.4rem;
    color: #34251c;
    display: grid;
    gap: 1.6rem;
  }
  h2 { margin: 0 0 0.4rem; font-family: Georgia, 'Fraunces', serif; font-size: 1.15rem; color: #6f3b24; }
  h3 {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    margin: 0 0 0.7rem;
    color: #6f3b24;
    font-family: Georgia, 'Fraunces', serif;
    font-size: 0.78rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  h3::after { content: ''; flex: 1; height: 1px; background: #d8c6b1; }
  h4 { margin: 0 0 0.3rem; font-size: 0.78rem; color: #6f3b24; }
  p { margin: 0; }
  code { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 0.72rem; }
  .intro p { color: #5f4636; font-size: 0.78rem; line-height: 1.55; }
  .muted { color: #8a6f5c; }
  .bad { color: #9a2d1c; }

  .note { padding: 0.6rem 0.8rem; border: 1px solid #d8c6b1; background: #fff; font-size: 0.78rem; }
  .note--bad { border-color: #c65f3c; color: #9a2d1c; }

  .list { list-style: none; margin: 0; padding: 0; display: grid; gap: 0.5rem; }
  .row {
    display: grid;
    grid-template-columns: 0.8rem 1fr auto;
    gap: 0.8rem;
    align-items: start;
    padding: 0.7rem 0.85rem;
    border: 1px solid #d8c6b1;
    background: #fff;
  }
  .row--plain { grid-template-columns: 1fr auto; }
  .title { display: flex; flex-wrap: wrap; align-items: baseline; gap: 0.2rem 0.8rem; }
  .title strong { font-size: 0.86rem; }
  .when { color: #8a6f5c; font-size: 0.7rem; }
  .what { margin-top: 0.25rem; color: #5f4636; font-size: 0.76rem; line-height: 1.5; }
  .where, .cmd { margin-top: 0.25rem; font-size: 0.72rem; color: #5f4636; }
  .cmd code { background: #f8f1e7; padding: 0.1rem 0.35rem; word-break: break-all; }

  .dot { width: 0.7rem; height: 0.7rem; margin-top: 0.3rem; border-radius: 50%; border: 2px solid; background: transparent; }
  /* Состояние не только цветом: разная заливка и обводка. */
  .dot--ok { border-color: #5a7a4a; background: #5a7a4a; }
  .dot--never { border-color: #8a6f5c; border-style: dotted; }
  .dot--late { border-color: #b8860b; background: linear-gradient(90deg, #b8860b 50%, transparent 50%); }
  .dot--failed { border-color: #9a2d1c; background: #9a2d1c; border-radius: 2px; }

  .runs { list-style: none; margin: 0.45rem 0 0; padding: 0; display: grid; gap: 0.15rem; font-size: 0.72rem; }
  .run { display: flex; flex-wrap: wrap; gap: 0.1rem 0.5rem; }
  .run .key { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; color: #6f3b24; }
  .run--failed .key { color: #9a2d1c; }

  .open {
    padding: 0.35rem 0.7rem;
    border: 1px solid #d8c6b1;
    background: #fff9f0;
    color: #6f3b24;
    font: inherit;
    font-size: 0.72rem;
    cursor: pointer;
    white-space: nowrap;
  }
  .open:hover { border-color: #6f3b24; }
  .orphans { margin-top: 0.8rem; padding: 0.7rem 0.85rem; border: 1px dashed #c65f3c; background: #fff; }

  @media (max-width: 640px) {
    .row, .row--plain { grid-template-columns: 1fr; }
    .dot { display: none; }
  }
</style>
