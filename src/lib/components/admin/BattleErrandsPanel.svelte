<script lang="ts">
  // Стол поручений.
  //
  // Новое ПОРУЧЕНИЕ из существующего условия заводится здесь, без кода. Новое
  // УСЛОВИЕ — код: то, что нельзя померить по строкам базы, нельзя и заплатить
  // автоматически (`BATTLE-ERRANDS.md` §3). Обещание «любой квест из админки»
  // соврало бы в тот же день, когда хранителю захочется поручение «посмотреть
  // работу внимательно».
  //
  // Стол показывает ДВА числа, которых нет у гостя, и оба — про последствия
  // правки: сколько поручение уже выдало и заперт ли его slug. Без первого
  // перекос обнаруживается по жалобе, а не по отчёту; без второго хранитель
  // однажды переименует поручение и заплатит за него второй раз всем, кто его
  // уже прошёл.
  import { onMount } from 'svelte';
  import { t, lang } from '$lib/i18n';
  import { api } from '$lib/api';
  import BattleErrandList from '$lib/components/BattleErrandList.svelte';
  import type {
    AdminBattleErrand,
    BattleErrand,
    SaveBattleErrandRequest,
  } from '$lib/types/api';

  const RULES = [
    'works_seen',
    'works_liked',
    'tales_read',
    'comments_left',
    'cards_owned',
    'card_level',
    'deck_laid',
    'matches_finished',
    'matches_won',
    'challenges_won',
    'dust_spent',
    'bookings_done',
    'orders_made',
    'visits',
  ] as const;

  /** Условия о состоянии: у снимка нет времени, и повторять его нельзя. */
  const STATEFUL = new Set(['cards_owned', 'card_level', 'deck_laid']);

  const PERIODS = ['once', 'daily', 'weekly', 'window'] as const;

  let rows = $state<AdminBattleErrand[]>([]);
  let draft = $state<SaveBattleErrandRequest | null>(null);
  let editingId = $state<string | null>(null);
  let busy = $state(false);
  let note = $state<string | null>(null);
  let loaded = $state(false);

  let dragFrom = $state<number | null>(null);
  let dragOver = $state<number | null>(null);
  let orderDirty = $state(false);

  let held = $derived(rows.find((r) => r.id === editingId) ?? null);
  let statefulPicked = $derived(!!draft && STATEFUL.has(draft.rule));
  let repeatingPublished = $derived(
    rows.filter((r) => r.status === 'published' && r.period !== 'once' && r.id !== editingId)
      .length,
  );

  function say(text: string, ms = 3000) {
    note = text;
    setTimeout(() => {
      if (note === text) note = null;
    }, ms);
  }

  async function load() {
    try {
      rows = await api.adminListBattleErrands();
      orderDirty = false;
    } catch (e) {
      say(String(e), 6000);
    } finally {
      loaded = true;
    }
  }

  onMount(load);

  function blank(): SaveBattleErrandRequest {
    return {
      id: null,
      slug: '',
      titleEn: '',
      titleRu: '',
      noteEn: '',
      noteRu: '',
      rule: 'works_seen',
      threshold: 1,
      currency: 'dust',
      amount: 5,
      // Умолчание `once` — не вкус, а ограничение: повторяющееся поручение даёт
      // повод прийти РАДИ пыли, и его надо выбирать руками.
      period: 'once',
      startsAt: null,
      endsAt: null,
      status: 'draft',
      byHand: false,
      sortOrder: null,
    };
  }

  function open(row: AdminBattleErrand | null) {
    editingId = row?.id ?? null;
    draft = row
      ? {
          id: row.id,
          slug: row.slug,
          titleEn: row.titleEn,
          titleRu: row.titleRu,
          noteEn: row.noteEn ?? '',
          noteRu: row.noteRu ?? '',
          rule: row.rule,
          threshold: row.threshold,
          currency: row.currency,
          amount: row.amount,
          period: row.period,
          startsAt: row.startsAt,
          endsAt: row.endsAt,
          status: row.status,
          byHand: row.byHand,
          sortOrder: row.sortOrder,
        }
      : blank();
  }

  /** Условие о состоянии не может повторяться — стол не даёт даже выбрать. */
  function onRule(rule: string) {
    if (!draft) return;
    draft.rule = rule;
    if (STATEFUL.has(rule)) draft.period = 'once';
  }

  async function save() {
    if (!draft || busy) return;
    busy = true;
    try {
      const saved = await api.adminSaveBattleErrand(draft);
      await load();
      open(rows.find((r) => r.id === saved.id) ?? null);
      say($t('adminErrandsSaved'));
    } catch (e) {
      const word = String(e).match(/errand:(\w+)/)?.[1];
      say(word ? $t(`adminErrandFault_${word}` as never) || word : String(e), 6000);
    } finally {
      busy = false;
    }
  }

  async function remove() {
    if (!editingId || busy) return;
    if (!confirm($t('adminErrandsDeleteAsk'))) return;
    busy = true;
    try {
      await api.adminDeleteBattleErrand(editingId);
      draft = null;
      editingId = null;
      await load();
      say($t('adminErrandsDeleted'));
    } catch (e) {
      say(String(e), 6000);
    } finally {
      busy = false;
    }
  }

  function onDrop(to: number) {
    if (dragFrom === null || dragFrom === to) return;
    const next = [...rows];
    const [moved] = next.splice(dragFrom, 1);
    next.splice(to, 0, moved);
    rows = next;
    orderDirty = true;
  }

  async function saveOrder() {
    busy = true;
    try {
      await api.adminReorderBattleErrands(rows.map((r) => r.id));
      orderDirty = false;
      say($t('adminErrandsOrderSaved'));
    } catch (e) {
      say(String(e), 6000);
    } finally {
      busy = false;
    }
  }

  const titleOf = (r: AdminBattleErrand) => ($lang === 'ru' ? r.titleRu : r.titleEn) || r.slug;

  /**
   * Предпросмотр — глазами гостя, у которого ещё ничего не сделано.
   *
   * Тем же компонентом, что и полка: второй отрисовщик разошёлся бы с первым, и
   * стол начал бы врать ровно про то, что на нём настраивают.
   */
  let preview = $derived<BattleErrand[]>(
    rows
      .filter((r) => r.status === 'published')
      .map((r) => ({
        id: r.id,
        slug: r.slug,
        titleEn: r.titleEn,
        titleRu: r.titleRu,
        noteEn: r.noteEn,
        noteRu: r.noteRu,
        rule: r.rule,
        threshold: r.threshold,
        currency: r.currency,
        amount: r.amount,
        period: r.period,
        have: 0,
        done: false,
        byHand: r.byHand,
      })),
  );

  const field =
    'w-full px-2 py-1.5 text-sm bg-transparent border border-[#34251c]/15 outline-none focus:border-[#34251c]/35';
  const label = 'block mb-1 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]';
</script>

<div class="flex-1 flex min-h-0">
  <!-- ── Лист ──────────────────────────────────────────────────────────── -->
  <aside class="w-80 flex-shrink-0 flex flex-col border-r border-[#34251c]/10">
    <div class="p-3 border-b border-[#34251c]/10 flex gap-2">
      <button
        onclick={() => open(null)}
        class="flex-1 px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/20 hover:bg-[#34251c]/5"
        >{$t('adminErrandsNew')}</button
      >
      {#if orderDirty}
        <button
          onclick={saveOrder}
          disabled={busy}
          class="px-3 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#c65f3c]/50 text-[#c65f3c] hover:bg-[#c65f3c]/10 disabled:opacity-40"
          >{$t('adminErrandsOrderSave')}</button
        >
      {/if}
    </div>
    <p class="px-3 py-2 text-[9px] uppercase tracking-[0.16em] text-[#8a6a55]">
      {$t('adminErrandsDragHint')}
    </p>

    <ul class="flex-1 overflow-y-auto pb-4">
      {#each rows as row, i (row.id)}
        <li
          draggable="true"
          ondragstart={() => (dragFrom = i)}
          ondragover={(e) => {
            e.preventDefault();
            dragOver = i;
          }}
          ondragleave={() => {
            if (dragOver === i) dragOver = null;
          }}
          ondrop={(e) => {
            e.preventDefault();
            onDrop(i);
          }}
          ondragend={() => {
            dragFrom = null;
            dragOver = null;
          }}
          class="border-b border-[#34251c]/5 {dragOver === i ? 'bg-[#c65f3c]/10' : ''}"
        >
          <button
            onclick={() => open(row)}
            class="w-full px-3 py-2 text-left hover:bg-[#34251c]/5 {editingId === row.id
              ? 'bg-[#34251c]/8'
              : ''}"
          >
            <span class="flex items-baseline gap-2">
              <span
                class="w-1.5 h-1.5 rounded-full flex-shrink-0 {row.status === 'published'
                  ? 'bg-[#c65f3c]'
                  : 'bg-[#34251c]/20'}"
              ></span>
              <span class="flex-1 text-sm truncate">{titleOf(row)}</span>
              <span class="text-[10px] text-[#8a6a55] flex-shrink-0"
                >{row.amount}
                {row.currency === 'dust' ? $t('battleGreetDust') : $t('battleGreetFeed')}</span
              >
            </span>
            <span class="mt-0.5 flex items-baseline gap-2 text-[10px] text-[#8a6a55]">
              <span class="flex-1 truncate">
                {row.rule}{row.threshold > 1 ? ` ≥ ${row.threshold}` : ''}
                {#if row.byHand}
                  · {$t('adminErrandByHandShort')}
                {:else if row.period !== 'once'}
                  · {$t(`adminErrandPeriod_${row.period}` as never)}
                {/if}
              </span>
              <!-- Сколько уже выдано. Без этого перекос находят по жалобе. -->
              {#if row.paidGuests > 0}
                <span class="flex-shrink-0 text-[#6f3b24]"
                  >{$t('adminErrandsPaidShort')} {row.paidGuests} · {row.paidCoins}</span
                >
              {/if}
            </span>
          </button>
        </li>
      {/each}
      {#if loaded && !rows.length}
        <li class="px-3 py-4 text-xs italic text-[#8a6a55]">{$t('adminErrandsEmpty')}</li>
      {/if}
    </ul>
  </aside>

  <!-- ── Правка ────────────────────────────────────────────────────────── -->
  <div class="flex-1 min-w-0 overflow-y-auto">
    {#if note}
      <p class="px-5 py-2 text-xs text-[#c65f3c] border-b border-[#34251c]/10">{note}</p>
    {/if}

    {#if !draft}
      <div class="p-5 max-w-[62ch]">
        <details class="mb-4">
          <summary
            class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
            >{$t('adminBattlesHintOpen')}</summary
          >
          <p class="mt-2 text-[11px] leading-relaxed italic text-[#8a6a55]">
            {$t('adminErrandsIntro')}
          </p>
        </details>
        <!-- Предпросмотр стоит здесь, а не под формой: хранитель приходит на
             стол посмотреть, что видит гость, чаще, чем что-то править. -->
        <p class="mb-2 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55]">
          {$t('adminErrandsPreview')}
        </p>
        {#if preview.length}
          <BattleErrandList errands={preview} preview={true} />
        {:else}
          <p class="text-xs italic text-[#8a6a55]">{$t('adminErrandsPreviewEmpty')}</p>
        {/if}
      </div>
    {:else}
      <div class="p-5 max-w-[70ch] space-y-5">
        <!-- Правка суммы прошлое не переписывает: книга только дописывается.
             Сказано словами, а не оставлено догадываться. -->
        <details>
          <summary
            class="text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] cursor-pointer"
            >{$t('adminBattlesHintOpen')}</summary
          >
          <p class="mt-2 text-[11px] leading-relaxed italic text-[#8a6a55]">
            {$t('adminErrandsLedgerNote')}
          </p>
        </details>

        <div class="grid grid-cols-2 gap-3">
          <label class="block">
            <span class={label}>{$t('adminErrandTitleRu')}</span>
            <input bind:value={draft.titleRu} class={field} />
          </label>
          <label class="block">
            <span class={label}>{$t('adminErrandTitleEn')}</span>
            <input bind:value={draft.titleEn} class={field} />
          </label>
          <label class="block">
            <span class={label}>{$t('adminErrandNoteRu')}</span>
            <input bind:value={draft.noteRu} class={field} />
          </label>
          <label class="block">
            <span class={label}>{$t('adminErrandNoteEn')}</span>
            <input bind:value={draft.noteEn} class={field} />
          </label>
        </div>

        <label class="block">
          <span class={label}>{$t('adminErrandSlug')}</span>
          <input bind:value={draft.slug} disabled={held?.slugLocked} class="{field} disabled:opacity-50" />
          <span class="block mt-1 text-[10px] text-[#8a6a55]">
            {held?.slugLocked ? $t('adminErrandSlugLocked') : $t('adminErrandSlugHint')}
          </span>
        </label>

        <div class="grid grid-cols-2 gap-3">
          <label class="block">
            <span class={label}>{$t('adminErrandRule')}</span>
            <select value={draft.rule} onchange={(e) => onRule(e.currentTarget.value)} class={field}>
              {#each RULES as r (r)}
                <option value={r}>{$t(`adminErrandRule_${r}` as never)}</option>
              {/each}
            </select>
          </label>
          <label class="block">
            <span class={label}>{$t('adminErrandThreshold')}</span>
            <input type="number" min="1" bind:value={draft.threshold} class={field} />
          </label>
          <label class="block">
            <span class={label}>{$t('adminErrandCurrency')}</span>
            <select bind:value={draft.currency} class={field}>
              <option value="dust">{$t('battleGreetDust')}</option>
              <option value="feed">{$t('battleGreetFeed')}</option>
            </select>
          </label>
          <label class="block">
            <span class={label}>{$t('adminErrandAmount')}</span>
            <input type="number" min="1" bind:value={draft.amount} class={field} />
          </label>
        </div>

        <!-- Дело. Стоит рядом с наградой, а не среди периодов: это свойство
             ВЫПЛАТЫ, и от него зависит, показывать ли период вообще. -->
        <label class="flex items-start gap-2 cursor-pointer">
          <input type="checkbox" bind:checked={draft.byHand} class="mt-0.5" />
          <span>
            <span class="block text-sm">{$t('adminErrandByHand')}</span>
            <span class="block text-[10px] leading-relaxed text-[#8a6a55]"
              >{$t('adminErrandByHandHint')}</span
            >
          </span>
        </label>

        <div class:hidden={draft.byHand}>
          <label class="block">
            <span class={label}>{$t('adminErrandPeriod')}</span>
            <select bind:value={draft.period} disabled={statefulPicked} class="{field} disabled:opacity-50">
              {#each PERIODS as p (p)}
                <option value={p}>{$t(`adminErrandPeriod_${p}` as never)}</option>
              {/each}
            </select>
          </label>
          <!-- Условие о состоянии не знает окна по существу: повторяющееся
               поручение с ним платило бы каждый период за однажды сделанное. -->
          {#if statefulPicked}
            <p class="mt-1 text-[10px] leading-relaxed text-[#8a6a55]">
              {$t('adminErrandStatefulNote')}
            </p>
          {:else if draft.period !== 'once'}
            <p class="mt-1 text-[10px] leading-relaxed text-[#c65f3c]">
              {$t('adminErrandRepeatingNote')}
              {repeatingPublished} / 3
            </p>
          {/if}
        </div>

        {#if draft.period === 'window'}
          <div class="grid grid-cols-2 gap-3">
            <label class="block">
              <span class={label}>{$t('adminErrandFrom')}</span>
              <input
                type="datetime-local"
                value={draft.startsAt ? draft.startsAt.slice(0, 16) : ''}
                oninput={(e) =>
                  (draft!.startsAt = e.currentTarget.value
                    ? new Date(e.currentTarget.value).toISOString()
                    : null)}
                class={field}
              />
            </label>
            <label class="block">
              <span class={label}>{$t('adminErrandUntil')}</span>
              <input
                type="datetime-local"
                value={draft.endsAt ? draft.endsAt.slice(0, 16) : ''}
                oninput={(e) =>
                  (draft!.endsAt = e.currentTarget.value
                    ? new Date(e.currentTarget.value).toISOString()
                    : null)}
                class={field}
              />
            </label>
          </div>
        {/if}

        <label class="block w-56">
          <span class={label}>{$t('adminErrandStatus')}</span>
          <select bind:value={draft.status} class={field}>
            <option value="draft">{$t('adminErrandStatusDraft')}</option>
            <option value="published">{$t('adminErrandStatusPublished')}</option>
          </select>
        </label>

        {#if held && held.paidGuests > 0}
          <p class="text-[11px] text-[#6f3b24]">
            {$t('adminErrandsPaid')}
            {held.paidGuests} · {held.paidCoins}
          </p>
        {/if}

        <div class="flex items-center gap-3 pt-2 border-t border-[#34251c]/10">
          <button
            onclick={save}
            disabled={busy}
            class="px-4 py-1.5 text-[10px] uppercase tracking-[0.16em] border border-[#34251c]/25 hover:bg-[#34251c]/5 disabled:opacity-40"
            >{$t('adminErrandsSave')}</button
          >
          <button
            onclick={() => {
              draft = null;
              editingId = null;
            }}
            class="px-4 py-1.5 text-[10px] uppercase tracking-[0.16em] text-[#8a6a55] hover:text-[#34251c]"
            >{$t('adminErrandsCancel')}</button
          >
          {#if editingId}
            <button
              onclick={remove}
              disabled={busy}
              class="ml-auto px-4 py-1.5 text-[10px] uppercase tracking-[0.16em] text-[#c65f3c] hover:bg-[#c65f3c]/10 disabled:opacity-40"
              >{$t('adminErrandsDelete')}</button
            >
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>
