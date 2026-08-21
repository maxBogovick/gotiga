<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { t , brandName } from '$lib/i18n';
  import { api } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import { savedFigurines } from '$lib/stores/saved-figurines.svelte';
  import { isValidEmail } from '$lib/validation';
  import { getIconById, iconLabel, type IconCategory } from '$lib/data/visualIcons';
  import { lang } from '$lib/i18n';
  import AuthFrame from '$lib/components/auth/AuthFrame.svelte';
  import type { ChallengeStepDto, ContactSettings } from '$lib/types/api';
  import { onMount } from 'svelte';

  const TOTAL_STEPS = 6;
  const CATEGORY_IDS: IconCategory[] = ['animals', 'dishes', 'seasons', 'symbols'];

  let step = $state(1);
  let email = $state('');
  let error = $state('');
  let loading = $state(false);
  let forgotOpen = $state(false);
  let forgotEmail = $state('');
  let forgotLoading = $state(false);
  let forgotSent = $state(false);
  let contacts = $state<ContactSettings>({ email: null, telegram: null, phone: null });
  let pendingWishlistImport = $state<{ localIds: string[]; serverIds: string[]; redirectTo: string } | null>(null);
  let importLoading = $state(false);

  onMount(async () => {
    try { contacts = await api.getContactSettings(); } catch { /* non-critical */ }
  });

  function toggleForgot() {
    forgotOpen = !forgotOpen;
    if (forgotOpen && !forgotEmail) forgotEmail = email;
  }

  async function requestReset() {
    if (!isValidEmail(forgotEmail)) { error = $t('authErrorEmail'); return; }
    error = '';
    forgotLoading = true;
    try {
      await api.requestPasswordReset(forgotEmail.trim().toLowerCase());
    } catch { /* generic — never reveal whether the account exists */ }
    finally {
      // Always show the same confirmation regardless of outcome (anti-enumeration).
      forgotSent = true;
      forgotLoading = false;
    }
  }

  let challengeId = $state('');
  let challengeSteps = $state<ChallengeStepDto[]>([]);
  let selectedTokens = $state<string[]>(['', '', '', '']);

  function categoryStepIndex() { return step - 2; }
  function currentChallengeStep(): ChallengeStepDto | null {
    return challengeSteps[categoryStepIndex()] ?? null;
  }

  async function advance() {
    error = '';
    if (step === 1) {
      if (!isValidEmail(email)) { error = $t('authErrorEmail'); return; }
      await fetchChallenge();
      return;
    }
    if (step >= 2 && step <= 5) {
      const idx = categoryStepIndex();
      if (!selectedTokens[idx]) { error = $t('authErrorSelection'); return; }
      if (step < 5) { step++; return; }
      await verify();
    }
  }

  function back() {
    error = '';
    if (step === 2) {
      // Returning to email step — discard challenge so a fresh one is fetched
      // if the user changes their email and advances again.
      challengeId = '';
      challengeSteps = [];
      selectedTokens = ['', '', '', ''];
    }
    if (step > 1) step--;
  }

  async function fetchChallenge() {
    loading = true;
    try {
      const res = await api.userLoginChallenge(email.trim().toLowerCase());
      challengeId = res.challengeId;
      challengeSteps = res.steps;
      step = 2;
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : '';
      if (msg.includes('401')) {
        error = $t('authErrorWrong');
      } else if (msg.includes('400') && msg.toLowerCase().includes('attempt')) {
        error = $t('authErrorLockout');
      } else {
        error = $t('authErrorServer');
      }
    } finally {
      loading = false;
    }
  }

  async function verify() {
    loading = true;
    try {
      const res = await api.userLoginVerify(
        challengeId,
        selectedTokens as [string, string, string, string]
      );
      authStore.setSession(res.sessionToken, res.user);

      // Link any existing cancel tokens from localStorage.
      // Each gotiga_claims_* entry is a ClaimData[] where each item has `.token`.
      try {
        if (typeof localStorage !== 'undefined') {
          const claimsKeys = Object.keys(localStorage).filter(k => k.startsWith('gotiga_claims_'));
          const tokens: string[] = [];
          for (const key of claimsKeys) {
            const items: Array<{ token?: string }> = JSON.parse(localStorage.getItem(key) ?? '[]');
            if (Array.isArray(items)) {
              for (const item of items) {
                if (item?.token) tokens.push(item.token);
              }
            }
          }
          if (tokens.length > 0) await api.userLinkBookings(res.sessionToken, tokens);
        }
      } catch { /* non-critical */ }

      const redirectTo = page.url.searchParams.get('from') ?? '/';
      const localIds = savedFigurines.localIds();
      const serverIds = await api.getWishlist(res.sessionToken).catch(() => null);
      if (serverIds && localIds.some((id) => !serverIds.includes(id))) {
        pendingWishlistImport = { localIds, serverIds, redirectTo };
        step = 6;
        return;
      }

      await savedFigurines.syncWithServer({ importLocal: false });
      goto(redirectTo);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : '';
      if (msg.includes('401')) {
        error = $t('authErrorWrong');
      } else if (msg.includes('400') && msg.toLowerCase().includes('attempt')) {
        error = $t('authErrorLockout');
      } else {
        error = $t('authErrorServer');
      }
    } finally {
      loading = false;
    }
  }

  async function finishWishlistImport(shouldImport: boolean) {
    if (!pendingWishlistImport || importLoading) return;
    importLoading = true;
    error = '';
    try {
      if (shouldImport) {
        await savedFigurines.importLocalToServer(pendingWishlistImport.localIds);
      } else {
        savedFigurines.replaceLocal(pendingWishlistImport.serverIds);
      }
      goto(pendingWishlistImport.redirectTo);
    } catch {
      error = $t('wishlistImportError');
    } finally {
      importLoading = false;
    }
  }
</script>

<svelte:head>
  <title>{$t('authLogin')} — {$brandName}</title>
</svelte:head>

<AuthFrame tilt={0.3}>
  {#if step < 6}
    <div class="auth-progress">
      {#each Array(TOTAL_STEPS - 1) as _, i}
        <div class="auth-pip" class:active={step >= i + 1} class:done={step > i + 1}></div>
      {/each}
    </div>
    <h1 class="auth-title">{$t('authLogin')}</h1>
  {/if}

  {#if step === 1}
    <p class="auth-hint">{$t('authStep')} 1 {$t('authOf')} 5</p>
    <div class="auth-fields">
      <label class="auth-field">
        <span>{$t('authEmail')}</span>
        <input type="email" bind:value={email} placeholder={$t('authEmailPlaceholder')}
               id="auth-email" name="email" autocomplete="email" onkeydown={(e) => e.key === 'Enter' && advance()} />
      </label>
    </div>
  {/if}

  {#if step >= 2 && step <= 5}
    {@const catStep = currentChallengeStep()}
    {@const catIdx = categoryStepIndex()}
    {#if catStep}
      <p class="auth-hint">{$t('authStep')} {step} {$t('authOf')} 5 — {$t(`authCategory${CATEGORY_IDS[catIdx].charAt(0).toUpperCase()}${CATEGORY_IDS[catIdx].slice(1)}` as any)}</p>
      <p class="auth-choose">{$t('authChooseOne')}</p>
      <div class="auth-grid">
        {#each catStep.icons as item}
          {@const icon = getIconById(CATEGORY_IDS[catIdx], item.iconId)}
          {#if icon}
            <button
              class="auth-icon-btn"
              class:selected={selectedTokens[catIdx] === item.token}
              onclick={() => { selectedTokens[catIdx] = item.token; }}
              title={icon.labelRu}
              aria-label={icon.labelRu}
            >
              {@html icon.svg}
              <span class="auth-icon-label">{iconLabel(icon, $lang)}</span>
            </button>
          {/if}
        {/each}
      </div>
    {/if}
  {/if}

  {#if error}
    <p class="auth-error">{error}</p>
  {/if}

  {#if step === 6 && pendingWishlistImport}
    <h1 class="auth-title">{$t('wishlistImportTitle')}</h1>
    <p class="auth-hint">
      {$t('wishlistImportBody').replace('{count}', String(pendingWishlistImport.localIds.filter((id) => !pendingWishlistImport?.serverIds.includes(id)).length))}
    </p>
    <div class="auth-nav">
      <button class="auth-btn-primary" onclick={() => finishWishlistImport(true)} disabled={importLoading}>
        {importLoading ? '…' : $t('wishlistImportConfirm')}
      </button>
      <button class="auth-link" onclick={() => finishWishlistImport(false)} disabled={importLoading}>
        {$t('wishlistImportSkip')}
      </button>
    </div>
  {/if}

  {#if step < 6}
    <div class="auth-nav">
      {#if step > 1}
        <button class="auth-btn-ghost" onclick={back}>{$t('authBack')}</button>
      {/if}
      <button class="auth-btn-primary" onclick={advance} disabled={loading}>
        {loading ? '…' : step === 5 ? $t('authSubmit') : $t('authNext')}
      </button>
    </div>
    <p class="auth-switch">
      {$t('authNoAccount')}
      <button class="auth-link" onclick={() => goto('/register')}>{$t('authRegister')}</button>
    </p>
    <div class="auth-forgot">
      <button class="auth-forgot-toggle" onclick={toggleForgot}>{$t('authForgotLink')}</button>
      {#if forgotOpen}
        {#if forgotSent}
          <span class="auth-forgot-note">{$t('authForgotSent')}</span>
        {:else}
          <span class="auth-forgot-note">{$t('authForgotPrompt')}</span>
          <div class="auth-forgot-form">
            <input type="email" bind:value={forgotEmail} placeholder={$t('authEmailPlaceholder')}
                   id="auth-forgot-email" name="email" autocomplete="email" onkeydown={(e) => e.key === 'Enter' && requestReset()} />
            <button class="auth-btn-primary" onclick={requestReset} disabled={forgotLoading}>
              {forgotLoading ? '…' : $t('authForgotSend')}
            </button>
          </div>
        {/if}
        <span class="auth-forgot-note">{$t('authForgotNote')}</span>
        {#if contacts.email || contacts.telegram || contacts.phone}
          <span class="auth-forgot-contacts">
            {#if contacts.email}
              <a href="mailto:{contacts.email}" class="auth-forgot-link">
                <svg width="13" height="13" viewBox="0 0 13 13" fill="none" aria-hidden="true"><rect x="1" y="2.5" width="11" height="8" rx="1" stroke="currentColor" stroke-width="1"/><path d="M1 3.5L6.5 7.5L12 3.5" stroke="currentColor" stroke-width="1"/></svg>
                {contacts.email}
              </a>
            {/if}
            {#if contacts.telegram}
              <a href={contacts.telegram.startsWith('http') ? contacts.telegram : `https://t.me/${contacts.telegram.replace(/^@/, '')}`} target="_blank" rel="noopener" class="auth-forgot-link">
                <svg width="13" height="13" viewBox="0 0 13 13" fill="none" aria-hidden="true"><circle cx="6.5" cy="6.5" r="5.5" stroke="currentColor" stroke-width="1"/><path d="M3.5 6.5L5.5 8.5L9.5 4.5" stroke="currentColor" stroke-width="1" stroke-linecap="round"/></svg>
                {contacts.telegram.replace(/^https?:\/\/t\.me\//, '@').replace(/^@+/, '@')}
              </a>
            {/if}
            {#if contacts.phone}
              <a href="tel:{contacts.phone.replace(/\s/g, '')}" class="auth-forgot-link">
                <svg width="13" height="13" viewBox="0 0 13 13" fill="none" aria-hidden="true"><path d="M3 2C3 2 2 3 2 4.5C2 8.09 4.91 11 8.5 11C10 11 11 10 11 10L9 8L7.5 9C6.5 8.5 4.5 6.5 4 5.5L5 4L3 2Z" stroke="currentColor" stroke-width="1" stroke-linejoin="round"/></svg>
                {contacts.phone}
              </a>
            {/if}
          </span>
        {/if}
      {/if}
    </div>
  {/if}
</AuthFrame>

<style>
  .auth-forgot {
    margin-top: 10px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
  }

  .auth-forgot-toggle {
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 13px;
    font-style: italic;
    color: rgba(95, 70, 54, 0.55);
    text-decoration: underline;
    text-decoration-style: dotted;
    text-underline-offset: 3px;
    transition: color 0.2s;
  }
  .auth-forgot-toggle:hover { color: #6f3b24; }

  .auth-forgot-note {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 12px;
    font-style: italic;
    color: rgba(95, 70, 54, 0.45);
    text-align: center;
    line-height: 1.45;
  }

  .auth-forgot-form {
    display: flex;
    gap: 6px;
    align-items: center;
    width: 100%;
    max-width: 320px;
    margin-top: 2px;
  }
  .auth-forgot-form input {
    flex: 1;
    min-width: 0;
    padding: 6px 9px;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 14px;
    color: #34251c;
    background: rgba(255, 255, 255, 0.5);
    border: 1px solid #d8c6b1;
    border-radius: 3px;
  }
  .auth-forgot-form input:focus {
    outline: none;
    border-color: #c65f3c;
  }
  .auth-forgot-form .auth-btn-primary {
    flex: 0 0 auto;
    padding: 6px 12px;
    font-size: 13px;
    white-space: nowrap;
  }

  .auth-forgot-contacts {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    margin-top: 4px;
  }

  .auth-forgot-link {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 13px;
    color: #6f3b24;
    text-decoration: none;
    opacity: 0.75;
    transition: opacity 0.2s;
  }
  .auth-forgot-link:hover { opacity: 1; }
</style>
