<script lang="ts">
  /**
   * Имя и двери: чем открывается это имя, где оно сейчас открыто, как привязать
   * расписку гостя и как уйти.
   *
   * Всё это стояло в шапке профиля тремя одинаковыми мелкими ссылками
   * («Код · Telegram · Двери»), рядом с которыми четвёртой такой же стоял
   * «Выйти» — действие в ряду переключателей. Каждый переключатель раздвигал
   * страницу сверху, отодвигая то, ради чего в профиль заходят. Здесь у них
   * свой адрес, и ничего открывать не нужно: разделы просто напечатаны.
   */
  import { goto } from '$app/navigation';
  import { t, lang, brandName } from '$lib/i18n';
  import { api } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import { userDesk } from '$lib/stores/user-desk.svelte';
  import TelegramDoor from '$lib/components/auth/TelegramDoor.svelte';
  import { claimKindLabel, formatDate } from '$lib/profile-labels';
  import type { OwnSessionDto, LinkClaimResponse, LinkClaimKind } from '$lib/types/api';

  // ── Двери ──
  let doors = $state<OwnSessionDto[]>([]);
  let busy = $state(false);
  let said = $state('');
  let doorsError = $state('');

  let canAskForSigns = $derived(!!authStore.user?.emailConfirmed || !!authStore.user?.telegramLinked);
  let canUnlink = $derived(!!authStore.user?.emailConfirmed && !!authStore.user?.hasSigns);

  $effect(() => {
    if (!authStore.token) return;
    api.userSessions(authStore.token).then((s) => { doors = s; }).catch(() => { doors = []; });
  });

  async function withBusy(run: () => Promise<string>) {
    if (busy) return;
    busy = true;
    doorsError = '';
    said = '';
    try {
      said = await run();
    } catch {
      doorsError = $t('profileActionError');
    } finally {
      busy = false;
    }
  }

  const resendConfirm = () => withBusy(async () => {
    await api.resendConfirmEmail(authStore.token!);
    return $t('profileEmailResent');
  });

  const askForSigns = () => withBusy(async () => {
    await api.askForSignsLetter(authStore.token!);
    return $t('profileSignsSent');
  });

  const closeOthers = () => withBusy(async () => {
    await api.userCloseOtherSessions(authStore.token!);
    doors = await api.userSessions(authStore.token!);
    return $t('profileSessionsClosed');
  });

  // ── Telegram ──
  let tgBusy = $state(false);
  let tgDone = $state('');
  let tgError = $state('');

  async function telegramLinked() {
    const token = authStore.token;
    if (!token) return;
    tgError = '';
    try {
      authStore.setSession(token, await api.userMe(token));
    } catch {
      tgError = $t('profileActionError');
    }
  }

  async function unlinkTelegram() {
    const token = authStore.token;
    if (!token || tgBusy) return;
    tgBusy = true;
    tgError = '';
    tgDone = '';
    try {
      authStore.setSession(token, await api.telegramUnlink(token));
      tgDone = $t('profileTelegramUnlinked');
    } catch {
      tgError = $t('profileActionError');
    } finally {
      tgBusy = false;
    }
  }

  // ── Расписка по коду ──
  let code = $state('');
  let claiming = $state(false);
  let claimResult = $state<LinkClaimResponse | null>(null);
  let claimError = $state('');

  function claimKindToFilter(kind: LinkClaimKind): string {
    return kind === 'booking' ? 'booking'
      : kind === 'notify' ? 'order'
      : kind === 'commission' ? 'commission'
      : 'waitlist';
  }

  async function submitClaim() {
    const value = code.trim();
    if (!value || claiming) return;
    claiming = true;
    claimError = '';
    claimResult = null;
    try {
      const res = await api.linkClaimByToken(authStore.token!, value);
      claimResult = res;
      if (res.result === 'linked') {
        code = '';
        await userDesk.load({ force: true });
        // Уводим ровно к тому виду дел, который только что привязали: адрес —
        // единственный способ сказать другой странице «открой вот это».
        if (res.kind) goto(`/profile/dealings?kind=${claimKindToFilter(res.kind)}`);
      }
    } catch {
      claimError = $t('profileActionError');
    } finally {
      claiming = false;
    }
  }

  // ── Уход ──
  async function logout() {
    const token = authStore.token;
    if (token) {
      try { await api.userLogout(token); } catch { /* ok */ }
    }
    userDesk.reset();
    authStore.logout();
    goto('/');
  }

  let confirmDelete = $state(false);
  let deleting = $state(false);
  let deleteError = $state('');

  async function deleteAccount() {
    if (deleting) return;
    deleting = true;
    deleteError = '';
    try {
      await api.deleteAccount(authStore.token!);
      userDesk.reset();
      authStore.purgeAllLocalData();
      // Жёсткий переход: сторы в памяти (отложенное, расписки) сбрасываются
      // вместе со страницей, а не только их хранилище.
      window.location.href = '/';
    } catch {
      deleteError = $t('profileActionError');
      deleting = false;
    }
  }
</script>

<svelte:head>
  <title>{$t('profileTabAccount')} — {$brandName}</title>
</svelte:head>

<!-- ── Двери ───────────────────────────────────────────────────────────── -->
<section class="block" aria-labelledby="doors-h">
  <h2 id="doors-h" class="pf-h2">{$t('profileDoorsToggle')}</h2>
  <p class="pf-note">{$t('profileDoorsHint')}</p>

  <div class="door">
    <span class="door-name">{$t('profileDoorEmail')}</span>
    <div class="door-state">
      {#if !authStore.user?.email}
        <span class="pf-quiet">{$t('profileNoEmail')}</span>
      {:else if authStore.user?.emailConfirmed}
        <span class="ok">{authStore.user.email}</span>
      {:else}
        <span class="warn">{authStore.user.email} — {$t('profileEmailUnconfirmed')}</span>
        <button class="pf-btn" onclick={resendConfirm} disabled={busy}>
          {busy ? '…' : $t('profileEmailResend')}
        </button>
      {/if}
    </div>
  </div>

  <div class="door">
    <span class="door-name">{$t('profileDoorSigns')}</span>
    <div class="door-state">
      {#if authStore.user?.hasSigns}
        <span class="ok">{$t('profileSignsYes')}</span>
      {:else if canAskForSigns}
        <span class="pf-quiet">{$t('profileSignsNo')}</span>
        <button class="pf-btn" onclick={askForSigns} disabled={busy}>
          {busy ? '…' : $t('profileSignsAsk')}
        </button>
      {:else}
        <span class="warn">{$t('profileSignsNeedChannel')}</span>
      {/if}
    </div>
  </div>

  <div class="door">
    <span class="door-name">Telegram</span>
    <div class="door-state">
      {#if authStore.user?.telegramLinked}
        <span class="ok">
          {authStore.user?.telegramUsername ? `@${authStore.user.telegramUsername}` : $t('profileTelegramYours')}
        </span>
        {#if canUnlink}
          <button class="pf-btn" onclick={unlinkTelegram} disabled={tgBusy}>
            {tgBusy ? '…' : $t('profileTelegramUnlink')}
          </button>
        {:else}
          <span class="pf-quiet">{$t('profileTelegramOnlyDoor')}</span>
        {/if}
      {:else}
        <span class="pf-quiet">{$t('profileTelegramNone')}</span>
        <TelegramDoor mode="link" sessionToken={authStore.token} ondone={() => { void telegramLinked(); }} />
      {/if}
    </div>
  </div>

  {#if tgError}<p class="pf-error" role="alert">{tgError}</p>{:else if tgDone}<p class="pf-said" role="status">{tgDone}</p>{/if}

  <div class="door door--wide">
    <span class="door-name">{$t('profileSessionsTitle')}</span>
    <div class="door-state">
      {#if doors.length === 0}
        <span class="pf-quiet">{$t('profileSessionsNone')}</span>
      {:else}
        <ul class="sessions">
          {#each doors as door (door.id)}
            <li class:now={door.current}>
              <span class="s-where">{door.browser ?? $t('profileSessionUnknown')}{door.place ? ` · ${door.place}` : ''}</span>
              <span class="s-when">
                {formatDate($lang, door.createdAt)}{door.current ? ` · ${$t('profileSessionCurrent')}` : ''}
              </span>
            </li>
          {/each}
        </ul>
        {#if doors.length > 1}
          <button class="pf-btn" onclick={closeOthers} disabled={busy}>
            {busy ? '…' : $t('profileSessionsClose')}
          </button>
        {/if}
      {/if}
    </div>
  </div>

  {#if doorsError}<p class="pf-error" role="alert">{doorsError}</p>{:else if said}<p class="pf-said" role="status">{said}</p>{/if}
</section>

<!-- ── Расписка по коду ────────────────────────────────────────────────── -->
<section class="block" aria-labelledby="claim-h">
  <h2 id="claim-h" class="pf-h2">{$t('profileLinkClaimTitle')}</h2>
  <p class="pf-note">{$t('profileLinkClaimHint')}</p>
  <form class="claim" onsubmit={(e) => { e.preventDefault(); submitClaim(); }}>
    <input
      class="claim-input"
      bind:value={code}
      placeholder={$t('profileLinkClaimPlaceholder')}
      autocomplete="off"
      autocapitalize="off"
      spellcheck="false"
      aria-label={$t('profileLinkClaimTitle')}
    />
    <button class="pf-btn" type="submit" disabled={claiming || !code.trim()}>
      {claiming ? $t('profileLinkClaimLinking') : $t('profileLinkClaimButton')}
    </button>
  </form>
  {#if claimError}
    <p class="pf-error" role="alert">{claimError}</p>
  {:else if claimResult}
    {#if claimResult.result === 'linked'}
      <p class="pf-said" role="status">
        {$t('profileLinkClaimLinked')}
        {#if claimResult.kind}{claimKindLabel($t, claimResult.kind)}{/if}
        {#if claimResult.name}— {claimResult.name}{/if}
      </p>
    {:else if claimResult.result === 'email_mismatch'}
      <p class="pf-error" role="alert">{$t('profileLinkClaimMismatch')}</p>
    {:else if claimResult.result === 'already_linked'}
      <p class="pf-quiet" role="status">{$t('profileLinkClaimAlready')}</p>
    {:else}
      <p class="pf-error" role="alert">{$t('profileLinkClaimNotFound')}</p>
    {/if}
  {/if}
</section>

<!-- ── Уход ────────────────────────────────────────────────────────────── -->
<section class="block leave" aria-labelledby="leave-h">
  <h2 id="leave-h" class="pf-h2">{$t('profileLogout')}</h2>
  <div class="leave-row">
    <button class="pf-btn" onclick={logout}>{$t('profileLogout')}</button>
    {#if !confirmDelete}
      <button class="pf-link pf-link--danger" onclick={() => (confirmDelete = true)}>
        {$t('profileDeleteAccount')}
      </button>
    {/if}
  </div>

  {#if confirmDelete}
    <div class="danger">
      <p class="pf-note">{$t('profileDeleteWarning')}</p>
      <div class="leave-row">
        <button class="pf-btn danger-yes" onclick={deleteAccount} disabled={deleting}>
          {deleting ? $t('profileDeleting') : $t('profileDeleteConfirm')}
        </button>
        <button class="pf-link" onclick={() => (confirmDelete = false)}>{$t('profileDeleteCancel')}</button>
      </div>
      {#if deleteError}<p class="pf-error" role="alert">{deleteError}</p>{/if}
    </div>
  {/if}
</section>

<style>
  .block {
    padding-bottom: 1.8rem;
    margin-bottom: 1.8rem;
    border-bottom: 1px solid color-mix(in srgb, var(--color-ink-primary) 10%, transparent);
  }
  .block:last-child { border-bottom: none; margin-bottom: 0; }
  .block .pf-note { margin-top: 0.4rem; max-width: 46rem; }

  .door {
    display: grid;
    grid-template-columns: 8.5rem 1fr;
    align-items: baseline;
    gap: 0.5rem 1rem;
    padding: 0.85rem 0;
    border-top: 1px solid color-mix(in srgb, var(--color-ink-primary) 7%, transparent);
  }
  .door:first-of-type { margin-top: 1rem; }

  .door-name {
    font-family: var(--font-body);
    font-size: 0.74rem;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    color: var(--color-ink-tertiary);
  }

  .door-state {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.7rem;
    font-family: var(--font-body);
    font-size: 0.88rem;
    color: var(--color-ink-secondary);
    overflow-wrap: anywhere;
  }

  .ok { color: var(--color-sage-ink); }
  .warn { color: var(--color-ember-deep); }

  .sessions {
    list-style: none;
    margin: 0;
    padding: 0;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .sessions li {
    display: flex;
    flex-wrap: wrap;
    gap: 0.2rem 0.7rem;
    align-items: baseline;
  }
  .sessions li.now .s-where { color: var(--color-ink-primary); }

  .s-where { font-size: 0.86rem; color: var(--color-ink-secondary); }
  .s-when { font-size: 0.74rem; color: var(--color-ink-tertiary); }

  .claim {
    display: flex;
    flex-wrap: wrap;
    gap: 0.6rem;
    margin-top: 0.9rem;
  }

  .claim-input {
    flex: 1 1 16rem;
    font-family: var(--font-mono);
    font-size: 0.86rem;
    color: var(--color-ink-primary);
    background: var(--color-canvas-raised);
    border: 1px solid var(--color-border-default);
    border-radius: 2px;
    padding: 0.45rem 0.65rem;
    outline: none;
  }
  .claim-input:focus { border-color: var(--color-ember); }

  .leave-row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 1.2rem;
    margin-top: 1rem;
  }

  .danger {
    margin-top: 1rem;
    padding: 0.9rem 1rem;
    background: var(--color-ember-subtle);
    border: 1px solid var(--color-border-ember);
  }
  .danger .leave-row { margin-top: 0.7rem; }
  .danger-yes {
    color: var(--color-canvas-raised);
    background: var(--color-ember-deep);
    border-color: var(--color-ember-deep);
  }
  .danger-yes:hover:not(:disabled) {
    background: var(--color-ember-ink);
    border-color: var(--color-ember-ink);
  }
</style>
