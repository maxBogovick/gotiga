<script lang="ts">
  /**
   * Вкладка «Каналы»: Telegram-каналы по языкам и черновик поста о релизе.
   *
   * Новые работы, байки, карты и этюды в каналы уходят сами. Пост о релизе —
   * нет: после выкладки сервер заводит черновик из `release-notes.md`, а
   * отправляет его человек. Слева направо: какие каналы заведены, пункты
   * релиза по языкам, как пост выглядит в выбранном канале, и кнопки отправки.
   *
   * Предпросмотр рисует сервер (тот же текст уходит в канал), поэтому он
   * показывает сохранённые пункты, а не набранные: пока есть несохранённая
   * правка, отправить нельзя.
   */
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { t } from '$lib/i18n';
  import type { AdminChannelOverview, ReleaseLangNotes } from '$lib/types/api';

  let overview = $state<AdminChannelOverview | null>(null);
  let loading = $state(true);
  let busy = $state(false);
  let message = $state<{ kind: 'ok' | 'error'; text: string } | null>(null);
  let cur = $state('en');
  let edit = $state<Record<string, { new: string; better: string }>>({});
  let savedKey = $state('');

  const release = $derived(overview?.release ?? null);
  const open = $derived(release?.status === 'draft');
  /** Языки вкладок: каналы, затем языки, на которых уже есть пункты. */
  const tabs = $derived.by(() => {
    const out: string[] = [];
    for (const c of overview?.channels ?? []) out.push(c.lang);
    for (const l of Object.keys(release?.notes ?? {})) if (!out.includes(l)) out.push(l);
    if (!out.includes('en')) out.unshift('en');
    return out;
  });
  const channel = $derived(release?.channels.find((c) => c.lang === cur) ?? null);
  const dirty = $derived(JSON.stringify(fromEdit()) !== savedKey);
  const unsent = $derived((release?.channels ?? []).filter((c) => !c.postedAt && c.caption));

  const lines = (s: string) => s.split('\n').map((x) => x.trim()).filter(Boolean);

  function fromEdit(): Record<string, ReleaseLangNotes> {
    const out: Record<string, ReleaseLangNotes> = {};
    for (const [l, v] of Object.entries(edit)) {
      const n = { new: lines(v.new), better: lines(v.better) };
      if (n.new.length || n.better.length) out[l] = n;
    }
    return out;
  }

  function apply(o: AdminChannelOverview) {
    overview = o;
    const next: Record<string, { new: string; better: string }> = {};
    for (const [l, n] of Object.entries(o.release?.notes ?? {})) {
      next[l] = { new: n.new.join('\n'), better: n.better.join('\n') };
    }
    edit = next;
    savedKey = JSON.stringify(fromEdit());
    if (!tabs.includes(cur)) cur = tabs[0] ?? 'en';
  }

  function slot(l: string) {
    if (!edit[l]) edit[l] = { new: '', better: '' };
    return edit[l];
  }

  function reason(e: unknown): string {
    const raw = e instanceof Error ? e.message : String(e);
    try {
      const parsed = JSON.parse(raw.slice(raw.indexOf('{')));
      return parsed.error ?? parsed.message ?? raw;
    } catch {
      return raw;
    }
  }

  async function run(job: () => Promise<AdminChannelOverview>, done?: string) {
    busy = true;
    message = null;
    try {
      apply(await job());
      if (done) message = { kind: 'ok', text: done };
    } catch (e) {
      message = { kind: 'error', text: reason(e) };
    }
    busy = false;
  }

  onMount(async () => {
    try {
      apply(await api.adminChannelOverview());
    } catch (e) {
      message = { kind: 'error', text: reason(e) };
    }
    loading = false;
  });

  const save = () => release && run(() => api.adminSaveReleaseNotes(release.id, fromEdit()), $t('adminChannelSaved'));

  function send(langs: string[]) {
    if (!release || !confirm($t('adminChannelSendConfirm'))) return;
    void run(() => api.adminSendRelease(release.id, langs));
  }

  function dismiss() {
    if (!release || !confirm($t('adminChannelDismissConfirm'))) return;
    void run(() => api.adminDismissRelease(release.id));
  }

  const when = (iso: string) => new Date(iso).toLocaleString();
  const area = 'w-full bg-[#fff9f0] border border-[#34251c]/20 px-2 py-1.5 text-sm text-[#34251c] outline-none focus:border-[#34251c]/50';
  const label = 'text-[10px] uppercase tracking-[0.08em] text-[#5f4636] block mb-1';
</script>

<div class="h-full overflow-y-auto p-6 text-[#34251c]">
  <div class="max-w-5xl mx-auto space-y-8">
    <header>
      <h2 class="font-cinzel text-lg">{$t('adminTabChannel')}</h2>
      <p class="mt-1 text-xs text-[#5f4636] leading-relaxed max-w-2xl">{$t('adminChannelIntro')}</p>
    </header>

    {#if loading}
      <p class="text-xs text-[#8a7060]">…</p>
    {:else if overview}
      {#if !overview.channels.length}
        <p class="border border-[#c65f3c]/50 bg-[#fff9f0] px-3 py-2 text-sm">{$t('adminChannelNone')}</p>
      {/if}
      {#if !overview.botReady}
        <p class="border border-[#c65f3c]/50 bg-[#fff9f0] px-3 py-2 text-sm">{$t('adminChannelNoBot')}</p>
      {/if}
      {#if overview.local}
        <p class="border border-[#34251c]/20 bg-[#fff9f0] px-3 py-2 text-sm text-[#5f4636]">{$t('adminChannelLocal')}</p>
      {/if}

      {#if overview.channels.length}
        <ul class="flex flex-wrap gap-3">
          {#each overview.channels as c (c.lang)}
            <li class="border border-[#34251c]/20 bg-[#fff9f0] px-3 py-2 text-sm min-w-[11rem]">
              <span class="font-semibold uppercase tracking-wide">{c.lang}</span>
              <span class="ml-2 text-[#5f4636]">{c.chat}</span>
              <span class="block text-[11px] text-[#5f4636] mt-1">
                {$t('adminChannelWaiting')}: {c.waiting}
                {#if c.failed}
                  · <span class="text-[#8f2d12] font-semibold">{$t('adminChannelFailed')}: {c.failed}</span>
                {/if}
              </span>
            </li>
          {/each}
        </ul>
      {/if}

      <section class="space-y-5">
        <h3 class="font-cinzel text-base">{$t('adminChannelRelease')}</h3>

        {#if !release}
          <p class="text-sm text-[#5f4636]">{$t('adminChannelNoRelease')}</p>
        {:else}
          <div class="flex flex-wrap items-baseline gap-x-4 gap-y-1 text-sm">
            <span class="font-semibold">{release.id}</span>
            <span class="text-[11px] uppercase tracking-wide border border-[#34251c]/30 px-1.5 py-0.5 {open ? '' : 'text-[#5f4636]'}">
              {$t(`adminChannelStatus_${release.status}`)}
            </span>
            <span class="text-[#5f4636]">{when(release.createdAt)}</span>
            {#if release.since}
              <span class="text-[#5f4636]">{$t('adminChannelNewOnSite')}: {release.newsCount}</span>
            {/if}
          </div>

          <div class="flex flex-wrap gap-1 border-b border-[#34251c]/15">
            {#each tabs as l (l)}
              <button
                onclick={() => (cur = l)}
                class="px-3 py-1.5 text-xs uppercase tracking-wide border border-b-0 -mb-px {cur === l
                  ? 'bg-[#fff9f0] border-[#34251c]/30'
                  : 'border-transparent text-[#5f4636] hover:bg-[#fff9f0]/60'}"
              >
                {l}{#if release.notes[l]} ·{/if}
              </button>
            {/each}
          </div>

          <div class="grid gap-6 lg:grid-cols-2">
            <div class="space-y-3">
              <label class="block">
                <span class={label}>{$t('adminChannelNotesNew')}</span>
                <textarea
                  class={area}
                  rows="5"
                  disabled={!open}
                  value={edit[cur]?.new ?? ''}
                  oninput={(e) => (slot(cur).new = e.currentTarget.value)}
                ></textarea>
              </label>
              <label class="block">
                <span class={label}>{$t('adminChannelNotesBetter')}</span>
                <textarea
                  class={area}
                  rows="4"
                  disabled={!open}
                  value={edit[cur]?.better ?? ''}
                  oninput={(e) => (slot(cur).better = e.currentTarget.value)}
                ></textarea>
              </label>
              <p class="text-[11px] text-[#8a7060] leading-relaxed">{$t('adminChannelNotesHint')}</p>
              {#if open}
                <div class="flex flex-wrap items-center gap-3">
                  <button class="btn-gothic text-[10px]" disabled={busy || !dirty} onclick={save}>
                    {$t('adminChannelSave')}
                  </button>
                  {#if dirty}<span class="text-[11px] text-[#8f2d12]">{$t('adminChannelDirty')}</span>{/if}
                </div>
              {/if}
            </div>

            <div class="space-y-3 min-w-0">
              <span class={label}>{$t('adminChannelPreview')}</span>
              {#if channel}
                <div class="border border-[#34251c]/20 bg-[#fffdf8] p-4 text-sm leading-relaxed shadow-sm">
                  {#if channel.caption}
                    <div class="whitespace-pre-line break-words preview">{@html channel.caption}</div>
                  {:else}
                    <p class="text-[#5f4636]">{$t('adminChannelNothing')}</p>
                  {/if}
                </div>
                {#if channel.writtenIn}
                  <p class="text-[11px] text-[#5f4636]">{$t('adminChannelWrittenIn')}: {channel.writtenIn.toUpperCase()}</p>
                {/if}
                <p class="text-[11px] text-[#8a7060] break-all">{$t('adminChannelButtonTo')}: {channel.link}</p>

                {#if channel.postedAt}
                  <p class="text-sm text-[#2f5d2a]">{$t('adminChannelOut')}: {when(channel.postedAt)}</p>
                {:else if channel.lastError}
                  <p class="text-sm text-[#8f2d12]">
                    {$t('adminChannelError')}: {channel.lastError} ({channel.attempts})
                  </p>
                {/if}
                {#if open && !channel.postedAt && channel.caption}
                  <button
                    class="btn-gothic text-[10px]"
                    disabled={busy || dirty || !overview.botReady}
                    onclick={() => send([cur])}
                  >
                    {channel.lastError ? $t('adminChannelRetry') : $t('adminChannelSend')} · {cur.toUpperCase()}
                  </button>
                {/if}
              {:else}
                <p class="text-sm text-[#5f4636]">—</p>
              {/if}
            </div>
          </div>

          {#if open}
            <div class="flex flex-wrap items-center gap-4 border-t border-[#34251c]/15 pt-4">
              <button
                class="btn-gothic text-[11px]"
                disabled={busy || dirty || !unsent.length || !overview.botReady}
                onclick={() => send([])}
              >
                {$t('adminChannelSendAll')}
              </button>
              <button
                class="text-xs text-[#5f4636] underline underline-offset-2 hover:text-[#34251c]"
                disabled={busy}
                onclick={dismiss}
              >
                {$t('adminChannelDismiss')}
              </button>
            </div>
          {/if}
        {/if}

        {#if message}
          <p
            class="text-sm {message.kind === 'ok' ? 'text-[#2f5d2a]' : 'text-[#8f2d12]'}"
            role={message.kind === 'error' ? 'alert' : 'status'}
          >
            {message.text}
          </p>
        {/if}
      </section>
    {/if}
  </div>
</div>

<style>
  .preview :global(a) {
    color: #8f2d12;
    text-decoration: underline;
    text-underline-offset: 2px;
  }
</style>
