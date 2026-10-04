<script lang="ts">
  /**
   * Одна ветка переписки.
   *
   * У неё есть адрес — и это главное, ради чего маршруты и заведены: прежде
   * ветка открывалась переменной внутри страницы, поэтому ни автор из письма,
   * ни строка дела не могли сказать «смотрите вот здесь».
   */
  import { tick } from 'svelte';
  import { page } from '$app/state';
  import { t, lang, brandName } from '$lib/i18n';
  import { api, resolveMediaUrl } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import MessageAttachments from '$lib/components/MessageAttachments.svelte';
  import { userDesk } from '$lib/stores/user-desk.svelte';
  import { formatDate } from '$lib/profile-labels';
  import type { ThreadDetailDto, AttachmentInput } from '$lib/types/api';

  let id = $derived(page.params.id ?? '');

  let detail = $state<ThreadDetailDto | null>(null);
  let loading = $state(true);
  let loadError = $state('');
  let box = $state<HTMLElement | null>(null);

  let body = $state('');
  let sending = $state(false);
  let sent = $state(false);
  let sendError = $state('');
  let attachments = $state<AttachmentInput[]>([]);
  let uploading = $state(false);

  // Ветка перечитывается при смене адреса: из карточки дела сюда приходят
  // прямой ссылкой, и вторая такая ссылка не должна показать первую ветку.
  $effect(() => {
    const threadId = id;
    if (!threadId || !authStore.token) return;
    let alive = true;
    loading = true;
    loadError = '';
    detail = null;
    body = '';
    attachments = [];
    api.getThread(authStore.token, threadId)
      .then(async (d) => {
        if (!alive) return;
        detail = d;
        userDesk.markThreadRead(threadId);
        await tick();
        toBottom();
      })
      .catch(() => { if (alive) loadError = $t('profileActionError'); })
      .finally(() => { if (alive) loading = false; });
    return () => { alive = false; };
  });

  function toBottom() {
    if (box) box.scrollTop = box.scrollHeight;
  }

  async function pickFiles(e: Event) {
    const input = e.target as HTMLInputElement;
    if (!input.files || !authStore.token) return;
    for (const file of Array.from(input.files)) {
      if (attachments.length >= 5) break;
      if (file.size > 8 * 1024 * 1024) continue;
      uploading = true;
      try {
        attachments = [...attachments, await api.uploadUserMedia(authStore.token, file)];
      } catch { /* ignore */ }
      finally { uploading = false; }
    }
    input.value = '';
  }

  async function send() {
    if ((!body.trim() && attachments.length === 0) || sending || !detail) return;
    sending = true;
    sendError = '';
    try {
      const msg = await api.replyToThread(authStore.token!, detail.thread.id, body.trim(), attachments);
      detail = { ...detail, messages: [...detail.messages, msg] };
      body = '';
      attachments = [];
      sent = true;
      setTimeout(() => { sent = false; }, 2000);
      await tick();
      toBottom();
    } catch {
      sendError = $t('profileActionError');
    } finally {
      sending = false;
    }
  }
</script>

<svelte:head>
  <title>{detail?.thread.subject || $t('profileMessages')} — {$brandName}</title>
</svelte:head>

<a class="pf-crumb" href="/profile/messages">← {$t('profileMessages')}</a>

{#if loading}
  <p class="pf-empty">…</p>
{:else if loadError || !detail}
  <p class="pf-error" role="alert">{loadError || $t('profileActionError')}</p>
{:else}
  <header class="head">
    <h1 class="pf-h2">{detail.thread.subject}</h1>
    {#if detail.thread.status === 'resolved'}
      <span class="pf-quiet">{$t('profileMessagesResolved')}</span>
    {/if}
  </header>

  <div class="messages" bind:this={box}>
    {#each detail.messages as msg (msg.id)}
      <article class="msg" class:mine={!msg.fromAdmin}>
        <div class="bubble" class:bubble--mine={!msg.fromAdmin}>
          <p class="who">{msg.fromAdmin ? $t('profileMessagesFromAdmin') : $t('profileMessagesFromYou')}</p>
          {#if msg.body}<p class="text">{msg.body}</p>{/if}
          {#if msg.attachments && msg.attachments.length > 0}
            <MessageAttachments attachments={msg.attachments} />
          {/if}
        </div>
        <p class="when">{formatDate($lang, msg.createdAt)}</p>
      </article>
    {/each}
  </div>

  {#if detail.thread.status !== 'resolved'}
    <div class="reply">
      <textarea
        class="reply-area"
        bind:value={body}
        rows="3"
        placeholder={$t('profileMessageWriteBody')}
        aria-label={$t('profileMessageWriteBody')}
        onkeydown={(e) => { if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) send(); }}
      ></textarea>

      {#if attachments.length > 0}
        <div class="atts">
          {#each attachments as att, i (att.url)}
            <div class="att">
              <img src={resolveMediaUrl(att.thumbUrl ?? att.url)} alt="" />
              <button
                type="button"
                onclick={() => (attachments = attachments.filter((_, idx) => idx !== i))}
                aria-label={$t('profileWishRemove')}
              >×</button>
            </div>
          {/each}
        </div>
      {/if}

      <div class="reply-foot">
        <span class="pf-quiet">Ctrl+Enter</span>
        <label class="attach" title={$t('profileAttachImage')}>
          <input type="file" accept="image/*" multiple hidden onchange={pickFiles} />
          {#if uploading}
            <span aria-hidden="true">…</span>
          {:else}
            <svg width="15" height="15" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.1" aria-hidden="true">
              <path d="M9.6 4.2 5.1 8.7a1.6 1.6 0 0 0 2.2 2.2l4.8-4.8a3 3 0 0 0-4.2-4.2L2.9 6.9a4.4 4.4 0 0 0 6.2 6.2" stroke-linecap="round"/>
            </svg>
          {/if}
          <span class="pf-sr">{$t('profileAttachImage')}</span>
        </label>
        <button class="pf-btn" onclick={send} disabled={sending || (!body.trim() && attachments.length === 0)}>
          {sending ? $t('profileMessagesReplying') : sent ? $t('profileMessageWriteSent') : $t('profileMessagesReply')}
        </button>
      </div>
      {#if sendError}<p class="pf-error" role="alert">{sendError}</p>{/if}
    </div>
  {:else}
    <p class="pf-quiet closed">{$t('profileMessagesResolved')}</p>
  {/if}
{/if}

<style>
  .head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.8rem;
    padding-bottom: 0.9rem;
    border-bottom: 1px solid color-mix(in srgb, var(--color-ink-primary) 12%, transparent);
  }

  .messages {
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
    max-height: 56vh;
    overflow-y: auto;
    padding: 1.1rem 0.1rem;
  }

  .msg { display: flex; flex-direction: column; align-items: flex-start; gap: 0.2rem; }
  .msg.mine { align-items: flex-end; }

  .bubble {
    max-width: 82%;
    padding: 0.6rem 0.8rem;
    background: var(--color-canvas-sunken);
    border: 1px solid var(--color-border-subtle);
  }
  .bubble--mine {
    background: var(--color-canvas-raised);
    border-color: var(--color-border-ember);
  }

  .who {
    font-family: var(--font-body);
    font-size: 0.7rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary);
    margin: 0 0 0.3rem;
  }

  .text {
    font-family: var(--font-body);
    font-size: 0.9rem;
    line-height: 1.55;
    color: var(--color-ink-primary);
    margin: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .when {
    font-family: var(--font-body);
    font-size: 0.72rem;
    color: var(--color-ink-tertiary);
    margin: 0;
  }

  .reply {
    border-top: 1px solid color-mix(in srgb, var(--color-ink-primary) 12%, transparent);
    padding-top: 0.9rem;
  }

  .reply-area {
    width: 100%;
    font-family: var(--font-body);
    font-size: 0.9rem;
    line-height: 1.5;
    color: var(--color-ink-primary);
    background: var(--color-canvas-raised);
    border: 1px solid var(--color-border-default);
    border-radius: 2px;
    padding: 0.6rem 0.7rem;
    resize: vertical;
    outline: none;
  }
  .reply-area:focus { border-color: var(--color-ember); }

  .reply-foot {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    margin-top: 0.6rem;
  }
  .reply-foot .pf-btn { margin-left: auto; }

  .attach {
    display: flex;
    align-items: center;
    cursor: pointer;
    font-size: 1rem;
    line-height: 1;
    color: var(--color-ink-tertiary);
    transition: color 0.15s;
  }
  .attach:hover { color: var(--color-ember); }

  .atts { display: flex; flex-wrap: wrap; gap: 0.4rem; margin-top: 0.5rem; }
  .att { position: relative; }
  .att img {
    width: 56px;
    height: 56px;
    object-fit: cover;
    border: 1px solid var(--color-border-subtle);
    display: block;
  }
  .att button {
    position: absolute;
    top: -6px;
    right: -6px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 1px solid var(--color-border-default);
    background: var(--color-canvas-raised);
    color: var(--color-ink-secondary);
    font-size: 0.7rem;
    line-height: 1;
    cursor: pointer;
  }

  .closed { display: block; padding-top: 0.9rem; }
</style>
