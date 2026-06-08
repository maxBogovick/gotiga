<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { t } from '$lib/i18n';
  import { authStore } from '$lib/stores/auth.svelte';
  import type { CommentDto } from '$lib/types/api';

  let { figurineId }: { figurineId: string } = $props();

  let comments = $state<CommentDto[]>([]);
  let loading = $state(true);
  let newestFirst = $state(false);

  // Form state
  let authorName = $state('');
  let authorEmail = $state('');
  let body = $state('');
  let submitting = $state(false);
  let sent = $state(false);
  let error = $state('');

  async function loadComments() {
    loading = true;
    comments = await api.getComments(figurineId, newestFirst);
    loading = false;
  }

  onMount(loadComments);

  function formatDate(iso: string): string {
    return new Date(iso).toLocaleDateString(undefined, { year: 'numeric', month: 'long', day: 'numeric' });
  }

  async function handleSubmit(e: SubmitEvent) {
    e.preventDefault();
    error = '';
    const bodyTrimmed = body.trim();
    if (!bodyTrimmed) return;
    if (!authStore.isLoggedIn && !authorName.trim()) {
      error = $t('commentsNameLabel');
      return;
    }
    submitting = true;
    try {
      await api.submitComment(
        figurineId,
        {
          authorName: authStore.isLoggedIn ? undefined : authorName.trim() || undefined,
          authorEmail: authorEmail.trim() || undefined,
          body: bodyTrimmed,
        },
        authStore.token,
      );
      sent = true;
      body = '';
      authorName = '';
      authorEmail = '';
    } catch (err: unknown) {
      error = err instanceof Error ? err.message : 'Error';
    } finally {
      submitting = false;
    }
  }
</script>

<section class="comments-section">
  <div class="comments-top">
    <h2 class="comments-heading">{$t('commentsTitle')}</h2>
    {#if comments.length > 1 || (!loading && comments.length > 0)}
      <button class="sort-btn" onclick={() => { newestFirst = !newestFirst; loadComments(); }}>
        {newestFirst ? $t('commentsSortNewest') : $t('commentsSortOldest')}
      </button>
    {/if}
  </div>

  <!-- Comment list -->
  {#if loading}
    <div class="comments-loading">
      {#each Array(2) as _}
        <div class="comment-skeleton">
          <div class="skel skel-name"></div>
          <div class="skel skel-body"></div>
        </div>
      {/each}
    </div>
  {:else if comments.length === 0}
    <p class="comments-empty">{$t('commentsEmpty')}</p>
  {:else}
    <ol class="comments-list">
      {#each comments as c (c.id)}
        <li class="comment-item">
          <div class="comment-header">
            <span class="comment-author">{c.authorName}</span>
            <time class="comment-date" datetime={c.createdAt}>{formatDate(c.createdAt)}</time>
          </div>
          <p class="comment-body">{c.body}</p>
          {#if c.adminReply}
            <div class="comment-reply">
              <span class="comment-reply-label">{$t('commentsReplyLabel')}</span>
              <p class="comment-reply-text">{c.adminReply}</p>
            </div>
          {/if}
        </li>
      {/each}
    </ol>
  {/if}

  <!-- Submit form -->
  <div class="comment-form-wrap">
    <h3 class="comment-form-title">{$t('commentsFormTitle')}</h3>

    {#if sent}
      <p class="comment-sent">{$t('commentsSent')}</p>
      <button class="comment-again" onclick={() => sent = false}>↩</button>
    {:else}
      <form class="comment-form" onsubmit={handleSubmit} novalidate>
        {#if !authStore.isLoggedIn}
          <div class="form-row form-row--half">
            <div class="form-field">
              <label class="form-label" for="comment-name">{$t('commentsNameLabel')} *</label>
              <input
                id="comment-name"
                class="form-input"
                type="text"
                placeholder={$t('commentsNamePlaceholder')}
                bind:value={authorName}
                maxlength="100"
                required
              />
            </div>
            <div class="form-field">
              <label class="form-label" for="comment-email">{$t('commentsEmailLabel')}</label>
              <input
                id="comment-email"
                class="form-input"
                type="email"
                placeholder={$t('commentsEmailPlaceholder')}
                bind:value={authorEmail}
              />
            </div>
          </div>
          <p class="comment-login-hint">
            <a href="/login" class="comment-login-link">{$t('commentsLoginHint')}</a>
          </p>
        {:else}
          <p class="comment-authed-name">{authStore.user?.displayName}</p>
        {/if}

        <div class="form-field">
          <label class="form-label" for="comment-body">{$t('commentsBodyLabel')}</label>
          <textarea
            id="comment-body"
            class="form-textarea"
            placeholder={$t('commentsBodyPlaceholder')}
            bind:value={body}
            maxlength="1000"
            rows="4"
            required
          ></textarea>
          <span class="form-hint">{body.length}/1000</span>
        </div>

        {#if error}
          <p class="form-error">{error}</p>
        {/if}

        <button class="comment-submit" type="submit" disabled={submitting || !body.trim()}>
          {submitting ? $t('commentsSubmitting') : $t('commentsSubmit')}
        </button>
      </form>
    {/if}
  </div>
</section>

<style>
  .comments-section {
    position: relative;
    z-index: 1;
    border-top: 1px solid #d8c6b1;
    margin-top: 4rem;
    padding-top: 3rem;
  }

  .comments-top {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 2rem;
  }

  .sort-btn {
    background: none;
    border: none;
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: #8a7060;
    cursor: pointer;
    padding: 0;
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  .sort-btn:hover { color: #34251c; }

  .comments-heading {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1.25rem;
    font-weight: 500;
    color: #34251c;
    letter-spacing: 0.02em;
    margin: 0;
  }

  .comments-empty {
    font-size: 0.85rem;
    color: #8a7060;
    font-style: italic;
    margin-bottom: 2rem;
  }

  .comments-loading {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    margin-bottom: 2rem;
  }

  .comment-skeleton {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .skel {
    background: #34251c10;
    border-radius: 3px;
    animation: pulse 1.6s ease-in-out infinite;
  }

  .skel-name  { height: 10px; width: 120px; }
  .skel-body  { height: 10px; width: 80%; }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50%       { opacity: 0.4; }
  }

  .comments-list {
    list-style: none;
    padding: 0;
    margin: 0 0 3rem;
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .comment-item {
    padding: 1.5rem 0;
    border-bottom: 1px solid #d8c6b120;
  }

  .comment-item:last-child {
    border-bottom: none;
  }

  .comment-header {
    display: flex;
    align-items: baseline;
    gap: 1rem;
    margin-bottom: 0.5rem;
  }

  .comment-author {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 0.9rem;
    font-weight: 500;
    color: #34251c;
  }

  .comment-date {
    font-size: 0.75rem;
    color: #8a7060;
    font-style: italic;
  }

  .comment-body {
    font-size: 0.875rem;
    line-height: 1.7;
    color: #4a3327;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .comment-reply {
    margin-top: 0.75rem;
    padding: 0.75rem 1rem;
    border-left: 2px solid #c65f3c;
    background: #c65f3c08;
  }

  .comment-reply-label {
    display: block;
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: #c65f3c;
    margin-bottom: 0.3rem;
  }

  .comment-reply-text {
    font-size: 0.875rem;
    line-height: 1.6;
    color: #34251c;
    font-style: italic;
    white-space: pre-wrap;
    word-break: break-word;
  }

  /* ── Form ─────────────────────────────────── */

  .comment-form-wrap {
    background: #ede4d6;
    border: 1px solid #c9b49d;
    padding: 2rem;
  }

  .comment-form-title {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1rem;
    font-weight: 500;
    color: #34251c;
    margin-bottom: 1.5rem;
  }

  .comment-form {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .form-row {
    display: grid;
    gap: 1rem;
  }

  .form-row--half {
    grid-template-columns: 1fr 1fr;
  }

  @media (max-width: 640px) {
    .form-row--half { grid-template-columns: 1fr; }
  }

  .form-field {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .form-label {
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: #6f3b24;
  }

  .form-input,
  .form-textarea {
    background: #fdfaf5;
    border: 1.5px solid #b09880;
    padding: 0.6rem 0.75rem;
    font-size: 0.875rem;
    color: #34251c;
    font-family: inherit;
    transition: border-color 0.2s;
    outline: none;
    resize: vertical;
    width: 100%;
  }

  .form-input:focus,
  .form-textarea:focus {
    border-color: #c65f3c;
    background: #fff9f2;
  }

  .form-hint {
    font-size: 0.7rem;
    color: #8a7060;
    text-align: right;
  }

  .form-error {
    font-size: 0.8rem;
    color: #c65f3c;
  }

  .comment-login-hint {
    font-size: 0.75rem;
    color: #8a7060;
    font-style: italic;
  }

  .comment-login-link {
    color: #6f3b24;
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  .comment-authed-name {
    font-size: 0.85rem;
    color: #34251c;
    font-style: italic;
    padding: 0.4rem 0;
  }

  .comment-submit {
    align-self: flex-start;
    background: #34251c;
    color: #f8f1e7;
    border: none;
    padding: 0.65rem 1.5rem;
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    cursor: pointer;
    transition: background 0.2s, opacity 0.2s;
  }

  .comment-submit:hover:not(:disabled) {
    background: #6f3b24;
  }

  .comment-submit:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .comment-sent {
    font-size: 0.875rem;
    color: #4a3327;
    font-style: italic;
    margin-bottom: 0.75rem;
  }

  .comment-again {
    background: none;
    border: none;
    color: #8a7060;
    font-size: 0.8rem;
    cursor: pointer;
    text-decoration: underline;
    padding: 0;
  }
</style>
