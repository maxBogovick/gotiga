<script lang="ts">
  /**
   * Вкладка «Ответы сайта»: каталог, из которого окно «Спросить сайт» отвечает
   * по смыслу вопроса. Слева записи, справа правка выбранной; ниже — проверка
   * вопроса и вопросы, на которые сайт не ответил (из замера), с кнопкой
   * «Создать запись».
   *
   * Что запись включена, решает галочка: выключенная лежит черновиком и в
   * подборе не участвует. Проверку полноты (3 фразы, вопрос и ответ на en и ru)
   * делает сервер и говорит словами. Остальные четыре языка не обязательны, но
   * без них читатель получит текст по цепочке откатов, поэтому у включённой
   * записи список показывает, сколько языков из шести заполнено.
   *
   * Список — по темам, с поиском по ключу, вопросу и фразам; «Копия» заводит
   * новую запись из выбранной, а пустой ключ при сохранении берётся из
   * английского вопроса.
   */
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { t, lang } from '$lib/i18n';
  import type { AskAnswerEntry, AskHandler, AskLangTexts, AskRouteReply, InteractionRow } from '$lib/types/api';

  const LANGS = ['en', 'ru', 'uk', 'es', 'de', 'it'] as const;
  const HANDLERS: AskHandler[] = ['count', 'new', 'read', 'start'];
  const TOPICS = ['works', 'order', 'rights', 'site', 'stories', 'play', 'studio'] as const;
  const PAGES = ['work', 'tale', 'play', 'gazette'] as const;
  /** Адреса, которые чаще всего ставят под ответ: подсказка в поле, ввод свободный. */
  const SITE_PAGES = [
    '/acquire', '/commission', '/figurines', '/author', '/workshop', '/tales', '/gazette', '/battles', '/battles/etude',
    '/studio', '/rights', '/privacy', '/terms', '/bookings', '/login', '/register', '/profile', '/profile/wishlist',
    '/profile/dealings',
  ];
  const TOPIC_KEY = { works: 'askTopicWorks', order: 'askTopicOrder', rights: 'askTopicRights', site: 'askTopicSite', stories: 'askTopicStories', play: 'askTopicPlay', studio: 'askTopicStudio' } as const;

  const blank = (): AskLangTexts => ({ en: '', ru: '', uk: '', es: '', de: '', it: '' });
  const fresh = (phrase = '', topic = 'site', sortOrder = 500): AskAnswerEntry => ({
    id: null,
    key: '',
    handler: null,
    enabled: false,
    sortOrder,
    topic,
    pages: [],
    phrases: phrase ? [phrase] : [],
    question: blank(),
    answer: blank(),
    links: [],
  });

  let entries = $state<AskAnswerEntry[]>([]);
  let loading = $state(true);
  let draft = $state<AskAnswerEntry | null>(null);
  let phrasesText = $state('');
  let saving = $state(false);
  let message = $state<{ kind: 'ok' | 'error'; text: string } | null>(null);
  let onlyDrafts = $state(false);
  let query = $state('');
  let topicFilter = $state<'all' | (typeof TOPICS)[number]>('all');

  let tryText = $state('');
  let tryResult = $state<AskRouteReply | null>(null);
  let tryFailed = $state(false);
  let trying = $state(false);

  let rows = $state<InteractionRow[]>([]);
  const misses = $derived(
    rows
      .filter((r) => r.name === 'site_ask_miss')
      .sort((a, b) => b.events - a.events)
      .slice(0, 20),
  );
  const hits = $derived.by(() => {
    const out = new Map<string, number>();
    for (const r of rows) {
      if (r.name !== 'site_ask_route' || !r.detail.startsWith('answer:')) continue;
      const key = r.detail.slice('answer:'.length);
      out.set(key, (out.get(key) ?? 0) + r.events);
    }
    return out;
  });

  const needle = $derived(query.trim().toLowerCase());
  const matches = (e: AskAnswerEntry) =>
    !needle || [e.key, ...Object.values(e.question), ...e.phrases].some((text) => text.toLowerCase().includes(needle));
  const shown = $derived(
    entries.filter((e) => (!onlyDrafts || !e.enabled) && (topicFilter === 'all' || e.topic === topicFilter) && matches(e)),
  );
  /** Записи по темам, в порядке каталога; тема без записей не показывается. */
  const groups = $derived(
    TOPICS.map((topic) => ({ topic, items: shown.filter((e) => e.topic === topic) })).filter((g) => g.items.length),
  );

  /** Языки, на которых у записи чего-то не хватает: вопроса, ответа или подписи ссылки. */
  function missing(e: AskAnswerEntry): string[] {
    return LANGS.filter(
      (l) => !e.question[l].trim() || (!e.handler && !e.answer[l].trim()) || e.links.some((k) => !k.label[l].trim()),
    );
  }
  const draftMissing = $derived(draft && draft.enabled ? missing(draft) : []);

  const nextOrder = () => Math.min(10000, Math.max(0, ...entries.map((e) => e.sortOrder)) + 10);

  /** Ключ из английского вопроса: латиница, цифры и «_», до сорока знаков. */
  function keyFrom(text: string): string {
    const slug = text
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '_')
      .replace(/^_+|_+$/g, '')
      .slice(0, 40)
      .replace(/_+$/g, '');
    return /^[a-z]/.test(slug) ? slug : slug ? `q_${slug}`.slice(0, 40) : '';
  }

  async function load() {
    loading = true;
    try {
      entries = await api.adminListAskAnswers();
    } catch (e) {
      message = { kind: 'error', text: reason(e) };
    }
    loading = false;
  }

  onMount(() => {
    void load();
    // Замер нужен для подсказок, а не для работы вкладки: нет его — вкладка цела.
    api.getBehaviourReport().then((r) => (rows = r.interactions ?? [])).catch(() => {});
  });

  function reason(e: unknown): string {
    const raw = e instanceof Error ? e.message : String(e);
    try {
      const parsed = JSON.parse(raw.slice(raw.indexOf('{')));
      return parsed.error ?? parsed.message ?? raw;
    } catch {
      return raw;
    }
  }

  function pick(e: AskAnswerEntry) {
    draft = JSON.parse(JSON.stringify(e)) as AskAnswerEntry;
    phrasesText = draft.phrases.join('\n');
    message = null;
  }

  function create(phrase = '') {
    draft = fresh(phrase, topicFilter === 'all' ? 'site' : topicFilter, nextOrder());
    phrasesText = phrase;
    message = null;
  }

  /** Копия выбранной записи: новая, выключенная, без ключа. Правят тексты, а не набирают заново. */
  function duplicate() {
    if (!draft) return;
    const copy = JSON.parse(JSON.stringify(draft)) as AskAnswerEntry;
    copy.id = null;
    copy.key = '';
    copy.enabled = false;
    copy.sortOrder = nextOrder();
    draft = copy;
    message = null;
  }

  async function save() {
    if (!draft) return;
    saving = true;
    message = null;
    try {
      if (!draft.key.trim()) draft.key = keyFrom(draft.question.en);
      const saved = await api.adminSaveAskAnswer({
        ...draft,
        phrases: phrasesText.split('\n'),
        handler: draft.handler || null,
      });
      await load();
      pick(entries.find((e) => e.id === saved.id) ?? saved);
      message = { kind: 'ok', text: $t('adminAskSaved') };
    } catch (e) {
      message = { kind: 'error', text: reason(e) };
    }
    saving = false;
  }

  async function remove() {
    if (!draft?.id || !confirm($t('adminAskDeleteConfirm'))) return;
    try {
      await api.adminDeleteAskAnswer(draft.id);
      draft = null;
      await load();
    } catch (e) {
      message = { kind: 'error', text: reason(e) };
    }
  }

  function addLink() {
    if (draft && draft.links.length < 4) draft.links.push({ href: '', label: blank() });
  }

  async function tryIt(e: SubmitEvent) {
    e.preventDefault();
    if (!tryText.trim()) return;
    trying = true;
    tryFailed = false;
    tryResult = await api.askRoute(tryText, $lang);
    tryFailed = tryResult === null;
    trying = false;
  }

  const field = 'w-full bg-[#fff9f0] border border-[#34251c]/20 px-2 py-1.5 text-sm text-[#34251c] outline-none focus:border-[#34251c]/50';
  const label = 'text-[10px] uppercase tracking-[0.08em] text-[#5f4636] block mb-1';
</script>

<datalist id="ask-site-pages">
  {#each SITE_PAGES as href (href)}<option value={href}></option>{/each}
</datalist>

<div class="h-full overflow-y-auto p-6 text-[#34251c]">
  <div class="max-w-6xl mx-auto grid gap-6 lg:grid-cols-[18rem_1fr]">
    <aside class="space-y-3">
      <div>
        <h2 class="font-cinzel text-lg">{$t('adminTabAsk')}</h2>
        <p class="mt-1 text-xs text-[#5f4636] leading-relaxed">{$t('adminAskIntro')}</p>
      </div>
      <div class="flex items-center justify-between gap-2">
        <button onclick={() => create()} class="btn-gothic text-[10px]">{$t('adminAskNew')}</button>
        <label class="flex items-center gap-1.5 text-[11px] text-[#5f4636]">
          <input type="checkbox" bind:checked={onlyDrafts} />{$t('adminAskOnlyDrafts')}
        </label>
      </div>
      <input class={field} type="search" bind:value={query} placeholder={$t('adminAskSearch')} aria-label={$t('adminAskSearch')} />
      <select class={field} bind:value={topicFilter} aria-label={$t('adminAskTopic')}>
        <option value="all">{$t('adminAskAllTopics')}</option>
        {#each TOPICS as topic (topic)}<option value={topic}>{$t(TOPIC_KEY[topic])}</option>{/each}
      </select>
      {#if loading}
        <p class="text-xs text-[#8a7060]">…</p>
      {:else}
        <div class="border-t border-[#34251c]/10">
          {#each groups as g (g.topic)}
            <h3 class="pt-3 pb-1 px-1 text-[10px] uppercase tracking-[0.1em] text-[#5f4636]">{$t(TOPIC_KEY[g.topic])} <span class="text-[#8a7060]">{g.items.length}</span></h3>
            <ul>
              {#each g.items as e (e.id)}
                {@const gaps = e.enabled ? missing(e) : []}
                <li>
                  <button
                    onclick={() => pick(e)}
                    class="w-full text-left py-2 px-1 border-b border-[#34251c]/10 hover:bg-[#fff9f0] flex items-center gap-2 {draft?.id === e.id ? 'bg-[#fff9f0]' : ''}"
                  >
                    <span
                      class="w-2 h-2 rounded-full shrink-0 {e.enabled ? 'bg-green-700' : 'bg-[#34251c]/25'}"
                      title={e.enabled ? $t('adminAskOn') : $t('adminAskDraft')}
                    ></span>
                    <span class="min-w-0 flex-1">
                      <span class="block text-sm truncate">{e.question.ru || e.question.en || e.key}</span>
                      <span class="block text-[11px] text-[#8a7060] truncate">{e.key}{e.handler ? ` · ${$t(`adminAskHandler_${e.handler}`)}` : ''}{e.pages.length ? ` · ${e.pages.map((pg) => $t(`adminAskPage_${pg as (typeof PAGES)[number]}`)).join(', ')}` : ''}</span>
                    </span>
                    {#if gaps.length}
                      <span class="text-[10px] text-amber-800 border border-amber-800/40 px-1" title={$t('adminAskMissing').replace('{langs}', gaps.join(', '))}>{LANGS.length - gaps.length}/{LANGS.length}</span>
                    {/if}
                    {#if hits.get(e.key)}<span class="text-[11px] text-[#5f4636]" title={$t('adminAskHits')}>{hits.get(e.key)}</span>{/if}
                  </button>
                </li>
              {/each}
            </ul>
          {/each}
          {#if !groups.length}<p class="pt-3 text-xs text-[#8a7060]">{$t('adminAskNothing')}</p>{/if}
        </div>
      {/if}
    </aside>

    <section class="space-y-8 min-w-0">
      {#if draft}
        <form
          onsubmit={(e) => {
            e.preventDefault();
            void save();
          }}
          class="space-y-4"
        >
          <div class="grid gap-4 sm:grid-cols-[1fr_12rem_6rem]">
            <label class="block">
              <span class={label}>{$t('adminAskKey')}</span>
              <input class={field} bind:value={draft.key} placeholder="lead_time" spellcheck="false" />
              {#if !draft.id}<span class="text-[11px] text-[#8a7060]">{$t('adminAskKeyHint')}</span>{/if}
            </label>
            <label class="block">
              <span class={label}>{$t('adminAskKind')}</span>
              <select class={field} bind:value={draft.handler}>
                <option value={null}>{$t('adminAskKindText')}</option>
                {#each HANDLERS as h (h)}<option value={h}>{$t(`adminAskHandler_${h}`)}</option>{/each}
              </select>
            </label>
            <label class="block">
              <span class={label}>{$t('adminAskOrder')}</span>
              <input class={field} type="number" min="0" max="10000" bind:value={draft.sortOrder} />
            </label>
          </div>
          <div class="grid gap-4 sm:grid-cols-[14rem_1fr]">
            <label class="block">
              <span class={label}>{$t('adminAskTopic')}</span>
              <select class={field} bind:value={draft.topic}>
                {#each TOPICS as topic (topic)}<option value={topic}>{$t(TOPIC_KEY[topic])}</option>{/each}
              </select>
            </label>
            <fieldset class="block">
              <legend class={label}>{$t('adminAskPages')}</legend>
              <div class="flex flex-wrap gap-x-5 gap-y-1 pt-1.5 text-sm">
                {#each PAGES as pg (pg)}
                  <label class="flex items-center gap-2">
                    <input type="checkbox" checked={draft.pages.includes(pg)} onchange={(e) => {
                      if (!draft) return;
                      draft.pages = e.currentTarget.checked ? [...draft.pages, pg] : draft.pages.filter((x) => x !== pg);
                    }} />{$t(`adminAskPage_${pg}`)}
                  </label>
                {/each}
              </div>
            </fieldset>
          </div>
          <label class="flex items-center gap-2 text-sm">
            <input type="checkbox" bind:checked={draft.enabled} />{$t('adminAskEnabled')}
          </label>
          {#if draft.enabled}
            <p class="text-[11px] {draftMissing.length ? 'text-amber-800' : 'text-green-700'}">
              {draftMissing.length ? $t('adminAskMissing').replace('{langs}', draftMissing.join(', ')) : $t('adminAskComplete')}
            </p>
          {/if}

          <label class="block">
            <span class={label}>{$t('adminAskPhrases')}</span>
            <textarea class="{field} font-mono text-[13px]" rows="8" bind:value={phrasesText}></textarea>
            <span class="text-[11px] text-[#8a7060]">{$t('adminAskPhrasesHint')}</span>
          </label>

          <fieldset class="space-y-2">
            <legend class={label}>{$t('adminAskQuestion')}</legend>
            {#each LANGS as l (l)}
              <div class="flex items-center gap-2">
                <span class="w-8 text-[11px] uppercase text-[#5f4636]">{l}{l === 'en' || l === 'ru' ? ' *' : ''}</span>
                <input class={field} bind:value={draft.question[l]} />
              </div>
            {/each}
          </fieldset>

          {#if !draft.handler}
            <fieldset class="space-y-2">
              <legend class={label}>{$t('adminAskAnswer')}</legend>
              {#each LANGS as l (l)}
                <div class="flex items-start gap-2">
                  <span class="w-8 pt-1.5 text-[11px] uppercase text-[#5f4636]">{l}{l === 'en' || l === 'ru' ? ' *' : ''}</span>
                  <textarea class={field} rows={l === 'en' || l === 'ru' ? 4 : 2} bind:value={draft.answer[l]}></textarea>
                </div>
              {/each}
              <p class="text-[11px] text-[#8a7060]">{$t('adminAskAnswerHint')}</p>
            </fieldset>

            <fieldset class="space-y-2">
              <legend class={label}>{$t('adminAskLinks')}</legend>
              {#each draft.links as link, i (i)}
                <div class="grid gap-2 sm:grid-cols-[1fr_auto] items-start border border-[#34251c]/10 p-2">
                  <div class="space-y-2">
                    <input class={field} placeholder="/acquire" bind:value={link.href} aria-label={$t('adminAskLinkHref')} list="ask-site-pages" spellcheck="false" />
                    <div class="grid gap-2 sm:grid-cols-3">
                      {#each LANGS as l (l)}
                        <input class={field} placeholder="{l}{l === 'en' || l === 'ru' ? ' *' : ''}" bind:value={link.label[l]} aria-label="{$t('adminAskLinkLabel')} {l}" />
                      {/each}
                    </div>
                  </div>
                  <button type="button" class="text-xs text-red-800 px-2 pt-1.5" onclick={() => draft?.links.splice(i, 1)} aria-label={$t('adminAskLinkRemove')}>×</button>
                </div>
              {/each}
              {#if draft.links.length < 4}
                <button type="button" class="btn-gothic text-[10px]" onclick={addLink}>{$t('adminAskLinkAdd')}</button>
              {/if}
            </fieldset>
          {:else}
            <p class="text-xs text-[#5f4636]">{$t('adminAskDataHint')}</p>
          {/if}

          <div class="flex flex-wrap items-center gap-4 pt-2">
            <button type="submit" disabled={saving} class="btn-gothic text-[10px] px-4 py-2 disabled:opacity-50">
              {saving ? '…' : $t('adminAskSave')}
            </button>
            {#if draft.id}
              <button type="button" onclick={duplicate} class="btn-gothic text-[10px]">{$t('adminAskDuplicate')}</button>
              <button type="button" onclick={remove} class="text-xs text-red-800 underline">{$t('adminAskDelete')}</button>
            {/if}
            {#if message}
              <span class="text-xs {message.kind === 'ok' ? 'text-green-700' : 'text-red-700'}" role="status">{message.text}</span>
            {/if}
          </div>
        </form>
      {:else}
        <p class="text-sm text-[#5f4636]">{$t('adminAskPick')}</p>
      {/if}

      <div class="border-t border-[#34251c]/10 pt-6 space-y-3">
        <h3 class={label}>{$t('adminAskTry')}</h3>
        <form onsubmit={tryIt} class="flex gap-2">
          <input class={field} bind:value={tryText} maxlength="200" placeholder={$t('adminAskTryPlaceholder')} />
          <button class="btn-gothic text-[10px] shrink-0 disabled:opacity-50" disabled={trying}>{$t('adminAskTryGo')}</button>
        </form>
        {#if tryFailed}
          <p class="text-xs text-red-700">{$t('adminAskTryFailed')}</p>
        {:else if tryResult}
          <div class="text-sm space-y-1">
            <p><b>{$t(`adminAskTryResult_${tryResult.decision}`)}</b>{tryResult.answer ? ` — ${tryResult.answer.key}` : ''}</p>
            {#if tryResult.answer?.text}<p class="text-[#5f4636]">{tryResult.answer.text}</p>{/if}
            {#if tryResult.candidates.length}
              <ul class="list-disc pl-5 text-[#5f4636]">
                {#each tryResult.candidates as c (c.key)}<li>{c.question} <span class="text-[11px] text-[#8a7060]">({c.key})</span></li>{/each}
              </ul>
            {/if}
          </div>
        {/if}
      </div>

      <div class="border-t border-[#34251c]/10 pt-6 space-y-3">
        <h3 class={label}>{$t('adminAskMisses')}</h3>
        <p class="text-[11px] text-[#8a7060]">{$t('adminAskMissesHint')}</p>
        {#if misses.length}
          <ul>
            {#each misses as m (m.detail)}
              <li class="flex items-center gap-3 py-1.5 border-b border-[#34251c]/10">
                <span class="min-w-0 flex-1 text-sm">{m.detail}</span>
                <span class="text-[11px] text-[#5f4636]">{m.events}</span>
                <button class="btn-gothic text-[10px] shrink-0" onclick={() => create(m.detail)}>{$t('adminAskMissNew')}</button>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="text-xs text-[#8a7060]">{$t('adminAskMissesNone')}</p>
        {/if}
      </div>
    </section>
  </div>
</div>
