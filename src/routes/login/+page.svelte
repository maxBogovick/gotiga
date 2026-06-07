<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { t } from '$lib/i18n';
  import { api } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import { getIconById, iconLabel, type IconCategory } from '$lib/data/visualIcons';
  import { lang } from '$lib/i18n';
  import AuthFrame from '$lib/components/auth/AuthFrame.svelte';
  import type { ChallengeStepDto } from '$lib/types/api';

  const TOTAL_STEPS = 6;
  const CATEGORY_IDS: IconCategory[] = ['animals', 'dishes', 'seasons', 'colors'];

  let step = $state(1);
  let email = $state('');
  let error = $state('');
  let loading = $state(false);

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
      if (!email.includes('@')) { error = $t('authErrorEmail'); return; }
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

      const redirectTo = $page.url.searchParams.get('from') ?? '/';
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
</script>

<svelte:head>
  <title>{$t('authLogin')} — Gotiga</title>
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
               autocomplete="email" onkeydown={(e) => e.key === 'Enter' && advance()} />
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
  {/if}
</AuthFrame>
