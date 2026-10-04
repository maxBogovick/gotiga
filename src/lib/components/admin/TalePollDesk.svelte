<script lang="ts">
  // Стол голосования «о ком записать следующую небылицу».
  //
  // Кандидатов назначает хозяин, поэтому любой исход годится, а счёт здесь —
  // для стола, не для читателя: читатель чисел не видит. Подсказка кандидатов
  // берётся из замера посещений — работы, на которые уже чаще всего смотрят и
  // у которых байки ещё нет: байку о них увидит больше людей, потому что дверь
  // в неё стоит на странице работы.
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { t } from '$lib/i18n';
  import AppImage from '$lib/components/AppImage.svelte';
  import type { AdminTalePoll, FigurineListItem, GazetteLeaf } from '$lib/types/api';

  let {
    figurines,
    tales,
    onWrite,
    onBack,
  }: {
    figurines: FigurineListItem[];
    tales: GazetteLeaf[];
    /** Записать байку о выбранной работе — стол открывается с ней. */
    onWrite: (figurineId: string) => void;
    onBack: () => void;
  } = $props();

  const MIN = 2;
  const MAX = 4;

  let polls = $state<AdminTalePoll[]>([]);
  let loading = $state(true);
  let busy = $state(false);
  let message = $state('');
  let draft = $state<string[]>([]);
  let query = $state('');
  let pickerOpen = $state(false);
  /** Посетители по работам за последний месяц — для подсказки кандидатов. */
  let seen = $state<Record<string, number>>({});

  let openPoll = $derived(polls.find((p) => p.state === 'open') ?? null);
  let waiting = $derived(
    polls.find((p) => p.state === 'closed' && p.winnerFigurineId && !p.fulfilled) ?? null,
  );
  let earlier = $derived(polls.filter((p) => p !== openPoll && p !== waiting));
  let withTale = $derived(new Set(tales.map((tale) => tale.figurineId).filter(Boolean) as string[]));
  let byId = $derived(new Map(figurines.map((f) => [f.id, f])));

  let suggestions = $derived(
    figurines
      .filter((f) => !withTale.has(f.id) && !draft.includes(f.id) && (seen[f.id] ?? 0) > 0)
      .sort((a, b) => (seen[b.id] ?? 0) - (seen[a.id] ?? 0))
      .slice(0, 6),
  );
  let matches = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const list = figurines.filter((f) => !draft.includes(f.id));
    return (q ? list.filter((f) => f.name.toLowerCase().includes(q)) : list).slice(0, 12);
  });

  function flash(text: string, ms = 5000) {
    message = text;
    setTimeout(() => {
      if (message === text) message = '';
    }, ms);
  }

  async function load() {
    try {
      polls = await api.adminTalePolls();
    } catch (e) {
      flash(String(e));
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void load();
    // Подсказка — не содержимое стола: не пришла, и стол работает без неё.
    void api
      .listFigurineAnalytics()
      .then((page) => {
        seen = Object.fromEntries(page.items.map((row) => [row.figurineId, row.uniqueVisitors]));
      })
      .catch(() => (seen = {}));
  });

  function add(id: string) {
    if (draft.includes(id) || draft.length >= MAX) return;
    draft = [...draft, id];
    query = '';
    pickerOpen = false;
  }

  function drop(id: string) {
    draft = draft.filter((x) => x !== id);
  }

  async function openNew() {
    if (draft.length < MIN) {
      flash($t('adminTalesPollNeed'));
      return;
    }
    busy = true;
    try {
      await api.adminOpenTalePoll(draft);
      draft = [];
      await load();
      flash($t('adminTalesPollOpened'));
    } catch (e) {
      flash(String(e));
    } finally {
      busy = false;
    }
  }

  async function closeOn(poll: AdminTalePoll, figurineId: string, name: string) {
    if (!confirm($t('adminTalesPollCloseAsk').replace('{name}', name))) return;
    busy = true;
    try {
      await api.adminCloseTalePoll(poll.id, figurineId);
      await load();
    } catch (e) {
      flash(String(e));
    } finally {
      busy = false;
    }
  }

  async function remove(poll: AdminTalePoll) {
    if (!confirm($t('adminTalesPollDeleteAsk'))) return;
    busy = true;
    try {
      await api.adminDeleteTalePoll(poll.id);
      await load();
    } catch (e) {
      flash(String(e));
    } finally {
      busy = false;
    }
  }

  function when(iso: string | null): string {
    if (!iso) return '';
    return new Date(iso).toLocaleDateString(undefined, { day: 'numeric', month: 'short' });
  }

  function nameOf(poll: AdminTalePoll, id: string | null): string {
    if (!id) return '';
    return poll.candidates.find((c) => c.figurineId === id)?.name ?? byId.get(id)?.name ?? '';
  }
</script>

<div class="flex-1 overflow-y-auto bg-[#f8f1e7]">
  <div class="mx-auto max-w-[820px] px-6 py-8 space-y-10 text-[#34251c]">
    <div class="flex items-center gap-4">
      <button onclick={onBack} class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] hover:text-[#34251c]">
        {$t('adminTalesPollBack')}
      </button>
      {#if message}<span class="text-xs text-[#6f3b24]">{message}</span>{/if}
    </div>

    <header>
      <p class="text-[9px] uppercase tracking-[0.2em] text-[#c65f3c] mb-2">{$t('adminTalesPollDesk')}</p>
      <h2 class="text-3xl leading-tight" style="font-family: 'Cormorant Garamond', Georgia, serif;">
        {$t('adminTalesPollTitle')}
      </h2>
    </header>

    {#if loading}
      <p class="text-xs text-[#5f4636]">…</p>
    {:else}
      {#if openPoll}
        <section class="space-y-4">
          <p class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">
            {$t('adminTalesPollOpenNow')} · {when(openPoll.openedAt)}
            {#if openPoll.letters > 0}· {$t('adminTalesPollLetters')}: {openPoll.letters}{/if}
          </p>
          <ul class="grid grid-cols-2 sm:grid-cols-4 gap-4">
            {#each openPoll.candidates as c (c.figurineId)}
              <li class="space-y-2">
                <div class="aspect-[3/4] overflow-hidden bg-[#1a120e] border border-[#d8c6b1]">
                  {#if c.imageUrl}<AppImage src={c.imageUrl} alt="" class="w-full h-full object-cover" sizes="180px" />{/if}
                </div>
                <p class="text-sm leading-snug" style="font-family: 'Cormorant Garamond', Georgia, serif;">{c.name}</p>
                <!-- Число после слова, а не перед ним: «голосов: 1» склонять не
                     нужно, а «1 голосов» режет глаз. -->
                <p class="text-[11px] text-[#5f4636]">
                  {$t('adminTalesPollVotes')}: <b class="text-[#34251c]">{c.votes}</b>
                  {#if c.letters > 0}· {$t('adminTalesPollLetters')}: {c.letters}{/if}
                </p>
                <button
                  onclick={() => closeOn(openPoll!, c.figurineId, c.name)}
                  disabled={busy}
                  class="w-full py-1.5 text-[9px] uppercase tracking-[0.16em] border border-[#c65f3c]/45 text-[#c65f3c] hover:bg-[#c65f3c]/5 disabled:opacity-40"
                >{$t('adminTalesPollCloseOn')}</button>
              </li>
            {/each}
          </ul>
          <button
            onclick={() => remove(openPoll!)}
            disabled={busy}
            class="text-[10px] uppercase tracking-[0.14em] text-[#8a6a55] hover:text-red-700"
          >{$t('adminTalesPollDelete')}</button>
        </section>
      {/if}

      {#if waiting}
        {@const winner = waiting.candidates.find((c) => c.figurineId === waiting.winnerFigurineId)}
        <section class="flex items-center gap-4 p-4 border border-[#c65f3c]/35 bg-[#c65f3c]/5">
          {#if winner?.imageUrl}
            <div class="w-14 aspect-[3/4] overflow-hidden bg-[#1a120e] flex-shrink-0">
              <AppImage src={winner.imageUrl} alt="" class="w-full h-full object-cover" sizes="56px" />
            </div>
          {/if}
          <p class="flex-1 text-sm">
            {$t('adminTalesPollChosen')
              .replace('{name}', nameOf(waiting, waiting.winnerFigurineId))
              .replace('{n}', String(waiting.letters))}
          </p>
          <button
            onclick={() => onWrite(waiting.winnerFigurineId!)}
            class="px-3 py-2 text-[10px] uppercase tracking-[0.16em] border border-[#c65f3c]/50 text-[#c65f3c] hover:bg-[#c65f3c]/10"
          >{$t('adminTalesPollWrite')}</button>
        </section>
      {/if}

      {#if !openPoll}
        <section class="space-y-4">
          <p class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminTalesPollNew')}</p>
          <p class="text-sm text-[#5f4636] max-w-[60ch]">{$t('adminTalesPollHint')}</p>

          {#if suggestions.length > 0}
            <div class="space-y-2">
              <p class="text-[10px] uppercase tracking-[0.14em] text-[#8a6a55]">{$t('adminTalesPollSuggest')}</p>
              <div class="flex flex-wrap gap-2">
                {#each suggestions as f (f.id)}
                  <button
                    onclick={() => add(f.id)}
                    disabled={draft.length >= MAX}
                    class="flex items-center gap-2 pr-3 border border-[#34251c]/15 hover:border-[#c65f3c]/50 text-xs disabled:opacity-40"
                  >
                    {#if f.faceImageUrl}
                      <img src={f.faceImageUrl} alt="" class="w-8 h-10 object-cover" />
                    {/if}
                    <span>{f.name}</span>
                    <span class="text-[#8a6a55]">· {seen[f.id]}</span>
                  </button>
                {/each}
              </div>
            </div>
          {/if}

          <div class="relative max-w-[360px]">
            <input
              bind:value={query}
              onfocus={() => (pickerOpen = true)}
              onblur={() => setTimeout(() => (pickerOpen = false), 150)}
              placeholder={$t('adminTalesPollSearch')}
              disabled={draft.length >= MAX}
              class="w-full px-2 py-1.5 text-xs bg-transparent border border-[#34251c]/15 focus:border-[#34251c]/35 outline-none"
            />
            {#if pickerOpen && matches.length}
              <ul class="absolute top-full left-0 right-0 mt-1 max-h-60 overflow-y-auto bg-[#f8f1e7] border border-[#34251c]/15 shadow-lg z-20">
                {#each matches as f (f.id)}
                  <li>
                    <button
                      onmousedown={(e) => e.preventDefault()}
                      onclick={() => add(f.id)}
                      class="w-full text-left px-2 py-1.5 hover:bg-[#34251c]/5 text-xs flex gap-2"
                    >
                      <span class="truncate">{f.name}</span>
                      {#if withTale.has(f.id)}<span class="ml-auto text-[#8a6a55]">{$t('adminTalesPollHasTale')}</span>{/if}
                    </button>
                  </li>
                {/each}
              </ul>
            {/if}
          </div>

          {#if draft.length}
            <ul class="flex flex-wrap gap-3">
              {#each draft as id (id)}
                {@const f = byId.get(id)}
                <li class="w-28 space-y-1.5">
                  <div class="aspect-[3/4] overflow-hidden bg-[#1a120e] border border-[#d8c6b1]">
                    {#if f?.faceImageUrl}<img src={f.faceImageUrl} alt="" class="w-full h-full object-cover" />{/if}
                  </div>
                  <p class="text-xs leading-snug truncate">{f?.name ?? id}</p>
                  <button onclick={() => drop(id)} class="text-[10px] text-[#8a6a55] hover:text-[#34251c]">
                    {$t('adminTalesPollRemove')}
                  </button>
                </li>
              {/each}
            </ul>
          {/if}

          <button
            onclick={openNew}
            disabled={busy || draft.length < MIN}
            class="px-3 py-2 text-[10px] uppercase tracking-[0.18em] border border-[#c65f3c]/50 text-[#c65f3c] hover:bg-[#c65f3c]/5 disabled:opacity-40"
          >{$t('adminTalesPollOpen')}</button>
        </section>
      {/if}

      {#if earlier.length}
        <section class="space-y-2">
          <p class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">{$t('adminTalesPollHistory')}</p>
          <ul class="divide-y divide-[#34251c]/10 text-xs">
            {#each earlier as poll (poll.id)}
              <li class="py-2 flex flex-wrap items-baseline gap-x-3 gap-y-1">
                <span class="text-[#8a6a55]">{when(poll.openedAt)}</span>
                <span>{nameOf(poll, poll.winnerFigurineId) || '—'}</span>
                {#if poll.fulfilled}
                  <span class="text-[#5f4636]">{$t('adminTalesPollFulfilled')} «{poll.fulfilled.titleRu || poll.fulfilled.titleEn}»</span>
                {/if}
                <span class="text-[#8a6a55]">
                  {$t('adminTalesPollVotes')}: {poll.candidates.reduce((n, c) => n + c.votes, 0)}
                </span>
                <button onclick={() => remove(poll)} disabled={busy} class="ml-auto text-[#8a6a55] hover:text-red-700">
                  {$t('adminTalesDelete')}
                </button>
              </li>
            {/each}
          </ul>
        </section>
      {/if}
    {/if}
  </div>
</div>
