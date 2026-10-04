<script lang="ts">
  /**
   * Шапка листа профиля: кто вошёл. Только личность — ни навигации, ни выхода.
   *
   * Прежде здесь же стоял ряд из четырёх одинаковых мелких ссылок
   * («Код · Telegram · Двери · Выйти»), в котором выход ничем не отличался от
   * переключателя, а каждый переключатель раздвигал страницу сверху. Разделы
   * уехали в рейку под шапкой, двери — на свой маршрут, выход — туда же и в
   * меню шапки сайта.
   */
  import { t, lang } from '$lib/i18n';
  import { api, resolveMediaUrl } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';

  let editing = $state(false);
  let draft = $state('');
  let saving = $state(false);
  let saved = $state(false);
  let nameError = $state('');

  let avatarInput = $state<HTMLInputElement | null>(null);
  let uploading = $state(false);
  let avatarError = $state('');

  let avatarUrl = $derived(resolveMediaUrl(authStore.user?.avatarUrl));
  let initial = $derived((authStore.user?.displayName ?? '?').trim().charAt(0).toUpperCase() || '?');

  function focusOnMount(node: HTMLElement) {
    node.focus();
  }

  function startEdit() {
    draft = authStore.user?.displayName ?? '';
    editing = true;
    saved = false;
    nameError = '';
  }

  async function save() {
    if (!draft.trim() || saving) return;
    saving = true;
    nameError = '';
    try {
      authStore.user = await api.updateProfile(authStore.token!, draft.trim());
      editing = false;
      saved = true;
      setTimeout(() => { saved = false; }, 2000);
    } catch {
      nameError = $t('profileActionError');
    } finally {
      saving = false;
    }
  }

  async function onAvatarPicked(e: Event) {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    uploading = true;
    avatarError = '';
    try {
      authStore.user = await api.uploadAvatar(authStore.token!, file);
    } catch {
      avatarError = $t('profileAvatarError');
    } finally {
      uploading = false;
      if (avatarInput) avatarInput.value = '';
    }
  }

  function formatDate(iso: string): string {
    return new Date(iso).toLocaleDateString($lang, { day: 'numeric', month: 'long', year: 'numeric' });
  }
</script>

<header class="ms">
  <div class="ms-photo">
    <span class="ms-frame">
      {#if avatarUrl}
        <img src={avatarUrl} alt="" class="ms-img" />
      {:else}
        <span class="ms-initial" aria-hidden="true">{initial}</span>
      {/if}
    </span>
    <!-- Кнопка видна всегда. Прежде подпись «загрузить фото» проявлялась только
         по наведению, то есть на телефоне не проявлялась вовсе. -->
    <button
      type="button"
      class="ms-photo-btn"
      onclick={() => avatarInput?.click()}
      disabled={uploading}
      aria-label={$t('profileUploadPhoto')}
      title={uploading ? $t('profileUploadingPhoto') : $t('profileUploadPhoto')}
    >
      {#if uploading}
        <span class="ms-photo-wait" aria-hidden="true">…</span>
      {:else}
        <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.2" aria-hidden="true">
          <path d="M1.5 3.8h2l.9-1.3h3.2l.9 1.3h2v5.7h-9z" stroke-linejoin="round"/>
          <circle cx="6" cy="6.4" r="1.7"/>
        </svg>
      {/if}
    </button>
    <input
      bind:this={avatarInput}
      type="file"
      accept="image/jpeg,image/png,image/webp"
      class="pf-sr"
      onchange={onAvatarPicked}
    />
  </div>

  <div class="ms-id">
    {#if editing}
      <div class="ms-edit">
        <input
          class="ms-input"
          bind:value={draft}
          onkeydown={(e) => { if (e.key === 'Enter') save(); if (e.key === 'Escape') editing = false; }}
          use:focusOnMount
          aria-label={$t('profileEditName')}
        />
        <button class="pf-btn" onclick={save} disabled={saving}>
          {saving ? '…' : $t('profileSaveName')}
        </button>
        <button class="ms-cancel" onclick={() => (editing = false)} aria-label={$t('profileDeleteCancel')}>✕</button>
      </div>
    {:else}
      <div class="ms-name-row">
        <h1 class="ms-name">{authStore.user?.displayName ?? ''}</h1>
        <button class="ms-pencil" onclick={startEdit} aria-label={$t('profileEditName')} title={$t('profileEditName')}>
          <svg width="13" height="13" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.3" aria-hidden="true">
            <path d="M8.5 1.5L10.5 3.5L4 10H2V8L8.5 1.5Z"/>
          </svg>
        </button>
        {#if saved}<span class="ms-saved">{$t('profileNameSaved')}</span>{/if}
      </div>
    {/if}

    <p class="ms-sub">
      <span>{authStore.handle}</span>
      {#if authStore.user?.createdAt}
        <span class="ms-dot" aria-hidden="true">·</span>
        <span>{$t('profileMemberSince')} {formatDate(authStore.user.createdAt)}</span>
      {/if}
    </p>

    {#if nameError}<p class="pf-error" role="alert">{nameError}</p>{/if}
    {#if avatarError}<p class="pf-error" role="alert">{avatarError}</p>{/if}
  </div>
</header>

<style>
  .ms {
    display: flex;
    align-items: center;
    gap: 1rem;
    min-width: 0;
  }

  .ms-photo { position: relative; flex-shrink: 0; }

  .ms-frame {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 58px;
    height: 58px;
    overflow: hidden;
    border: 1px solid var(--color-border-default);
    background: var(--color-canvas-sunken);
  }

  .ms-img { width: 100%; height: 100%; object-fit: cover; display: block; }

  .ms-initial {
    font-family: var(--font-display);
    font-size: 1.5rem;
    line-height: 1;
    color: var(--color-ink-tertiary);
  }

  .ms-photo-btn {
    position: absolute;
    right: -7px;
    bottom: -7px;
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: 1px solid var(--color-border-default);
    border-radius: 50%;
    background: var(--color-canvas-raised);
    color: var(--color-ink-secondary);
    cursor: pointer;
    transition: color 0.15s, border-color 0.15s;
  }
  .ms-photo-btn:hover:not(:disabled) {
    color: var(--color-ember);
    border-color: var(--color-border-ember);
  }
  .ms-photo-btn:disabled { cursor: default; }
  .ms-photo-wait { font-size: 0.8rem; line-height: 1; }

  .ms-id { min-width: 0; }

  .ms-name-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex-wrap: wrap;
  }

  .ms-name {
    font-family: var(--font-display);
    font-size: 1.55rem;
    font-weight: 400;
    line-height: 1.15;
    margin: 0;
    color: var(--color-ink-primary);
  }

  /* 24×24 — палец попадает; цвет ink-tertiary, 6.44:1, выше порога 3:1 для
     нетекстовых элементов управления. Прежний #b5a090 давал 2.23:1. */
  .ms-pencil {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    background: none;
    border: none;
    color: var(--color-ink-tertiary);
    cursor: pointer;
    transition: color 0.15s;
  }
  .ms-pencil:hover { color: var(--color-ember); }

  .ms-saved {
    font-family: var(--font-body);
    font-size: 0.72rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--color-sage-ink);
  }

  .ms-edit {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .ms-input {
    font-family: var(--font-display);
    font-size: 1.3rem;
    background: transparent;
    border: none;
    border-bottom: 1.5px solid var(--color-ember);
    color: var(--color-ink-primary);
    padding: 2px 0;
    outline: none;
    width: 220px;
    max-width: 60vw;
  }

  .ms-cancel {
    background: none;
    border: none;
    color: var(--color-ink-tertiary);
    cursor: pointer;
    font-size: 0.9rem;
    line-height: 1;
    padding: 4px 6px;
  }
  .ms-cancel:hover { color: var(--color-ink-primary); }

  .ms-sub {
    font-family: var(--font-body);
    font-size: 0.8rem;
    line-height: 1.45;
    color: var(--color-ink-tertiary);
    margin: 0.25rem 0 0;
    overflow-wrap: anywhere;
  }
  .ms-dot { color: var(--color-ink-disabled); padding: 0 0.15em; }
</style>
