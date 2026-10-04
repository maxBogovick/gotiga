<script lang="ts">
  /** Переписка с мастерской: список веток. Ветка открывается своим адресом. */
  import { page } from '$app/state';
  import { replaceState } from '$app/navigation';
  import { t, lang, brandName } from '$lib/i18n';
  import { api } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import { userDesk } from '$lib/stores/user-desk.svelte';
  import { formatDate } from '$lib/profile-labels';

  type Filter = 'all' | 'booking' | 'order' | 'waitlist' | 'general';
  const FILTERS: Filter[] = ['all', 'booking', 'order', 'waitlist', 'general'];

  let filter = $derived<Filter>(
    (FILTERS as string[]).includes(page.url.searchParams.get('kind') ?? '')
      ? (page.url.searchParams.get('kind') as Filter)
      : 'all'
  );

  let threads = $derived(
    filter === 'all' ? userDesk.threads : userDesk.threads.filter((th) => th.category === filter)
  );

  function filterLabel(f: Filter): string {
    return f === 'all' ? $t('profileMessagesAll')
      : f === 'booking' ? $t('profileMessagesBooking')
      : f === 'order' ? $t('profileMessagesOrder')
      : f === 'waitlist' ? $t('profileMessagesWaitlist')
      : $t('profileMessagesGeneral');
  }

  function choose(f: Filter) {
    const url = new URL(page.url);
    if (f === 'all') url.searchParams.delete('kind');
    else url.searchParams.set('kind', f);
    replaceState(url, page.state);
  }

  // ── Новое письмо ──
  let composing = $state(false);
  let subject = $state('');
  let body = $state('');
  let sending = $state(false);
  let sent = $state(false);
  let composeError = $state('');

  async function send() {
    if (!subject.trim() || !body.trim() || sending) return;
    sending = true;
    composeError = '';
    try {
      const detail = await api.createThread(authStore.token!, subject.trim(), body.trim(), 'general');
      userDesk.addThread(detail.thread);
      subject = '';
      body = '';
      sent = true;
      setTimeout(() => { sent = false; composing = false; }, 1600);
    } catch {
      composeError = $t('profileActionError');
    } finally {
      sending = false;
    }
  }
</script>

<svelte:head>
  <title>{$t('profileMessages')} — {$brandName}</title>
</svelte:head>

<div class="top">
  <nav class="filters" aria-label={$t('profileMessages')}>
    {#each FILTERS as f (f)}
      <button
        type="button"
        class="filter"
        class:on={filter === f}
        aria-pressed={filter === f}
        onclick={() => choose(f)}
      >{filterLabel(f)}</button>
    {/each}
  </nav>
  <button class="pf-btn" onclick={() => (composing = !composing)} aria-expanded={composing}>
    {$t('profileMessagesCompose')}
  </button>
</div>

{#if composing}
  <div class="compose">
    <input class="field" bind:value={subject} placeholder={$t('profileMessageWriteSubject')} aria-label={$t('profileMessageWriteSubject')} />
    <textarea class="field area" bind:value={body} rows="3" placeholder={$t('profileMessageWriteBody')} aria-label={$t('profileMessageWriteBody')}></textarea>
    <button class="pf-btn" onclick={send} disabled={sending || !subject.trim() || !body.trim()}>
      {sending ? $t('profileMessagesSending') : sent ? $t('profileMessagesSent') : $t('profileMessagesSend')}
    </button>
    {#if composeError}<p class="pf-error" role="alert">{composeError}</p>{/if}
  </div>
{/if}

{#if userDesk.loading && !userDesk.loaded}
  <p class="pf-empty">…</p>
{:else if threads.length === 0}
  <p class="pf-empty">{$t('profileMessagesEmpty')}</p>
{:else}
  <ul class="pf-rows">
    {#each threads as thread (thread.id)}
      <li class="pf-row">
        <a class="row" class:unread={thread.unread > 0} href="/profile/messages/{thread.id}">
          <span class="row-head">
            <span class="row-subject">{thread.subject}</span>
            <span class="row-end">
              {#if thread.unread > 0}<span class="pf-count pf-count--new">{thread.unread}</span>{/if}
              {#if thread.status === 'resolved'}<span class="pf-quiet">{$t('profileMessagesResolved')}</span>{/if}
            </span>
          </span>
          {#if thread.preview}<span class="row-preview">{thread.preview}</span>{/if}
          <span class="row-date">{formatDate($lang, thread.lastMessageAt)}</span>
        </a>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 0.8rem;
    margin-bottom: 1.2rem;
  }

  .filters { display: flex; flex-wrap: wrap; gap: 0.35rem; }

  .filter {
    font-family: var(--font-body);
    font-size: 0.78rem;
    color: var(--color-ink-tertiary);
    background: transparent;
    border: 1px solid transparent;
    border-radius: 2px;
    padding: 0.3rem 0.6rem;
    cursor: pointer;
    transition: color 0.15s, border-color 0.15s, background 0.15s;
  }
  .filter:hover { color: var(--color-ink-primary); }
  .filter.on {
    color: var(--color-ember-ink);
    border-color: var(--color-border-ember);
    background: var(--color-ember-subtle);
  }

  .compose {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.6rem;
    padding: 1rem;
    margin-bottom: 1.4rem;
    background: var(--color-canvas-raised);
    border: 1px solid var(--color-border-subtle);
  }

  .field {
    width: 100%;
    font-family: var(--font-body);
    font-size: 0.88rem;
    color: var(--color-ink-primary);
    background: var(--color-canvas-base);
    border: 1px solid var(--color-border-default);
    border-radius: 2px;
    padding: 0.5rem 0.65rem;
    outline: none;
  }
  .field:focus { border-color: var(--color-ember); }
  .area { resize: vertical; line-height: 1.5; }

  .row {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.85rem 0.2rem;
    text-decoration: none;
    transition: background 0.15s;
  }
  .row:hover { background: var(--color-canvas-raised); }
  .row.unread { border-left: 2px solid var(--color-ember); padding-left: 0.7rem; }

  .row-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.8rem;
  }

  .row-subject {
    font-family: var(--font-serif);
    font-size: 1.02rem;
    color: var(--color-ink-primary);
    overflow-wrap: anywhere;
  }

  .row-end { display: flex; align-items: center; gap: 0.5rem; white-space: nowrap; }

  .row-preview {
    font-family: var(--font-body);
    font-size: 0.82rem;
    line-height: 1.5;
    color: var(--color-ink-secondary);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .row-date {
    font-family: var(--font-body);
    font-size: 0.74rem;
    color: var(--color-ink-tertiary);
  }
</style>
