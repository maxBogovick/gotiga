<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { t , brandName } from '$lib/i18n';
  import { api } from '$lib/api';
  import { VISUAL_CATEGORIES, iconLabel, getIconById, generatePersonalPool } from '$lib/data/visualIcons';
  import { lang } from '$lib/i18n';
  import AuthFrame from '$lib/components/auth/AuthFrame.svelte';

  // steps: 0=validating, 1-4=icon selection, 5=success, -1=error
  let step        = $state(0);
  let error       = $state('');
  let loading     = $state(false);
  let userEmail   = $state('');
  let token       = $state('');
  let selections  = $state<string[]>(['', '', '', '']);

  // Reset regenerates the personal subset, so the new selections are always
  // replayable at login. Generated once and sent with the reset.
  const pool = generatePersonalPool();

  function currentStepIndex() { return step - 1; }   // 0-based index into categories
  function currentCategory()  { return VISUAL_CATEGORIES[currentStepIndex()]; }
  function currentIcons() {
    const cat = currentCategory();
    return pool[currentStepIndex()]
      .map(id => getIconById(cat.id, id))
      .filter((i): i is NonNullable<typeof i> => !!i);
  }

  onMount(async () => {
    token = $page.url.searchParams.get('token') ?? '';
    if (!token) { error = $t('setPasswordInvalid'); step = -1; return; }
    try {
      const user = await api.validateResetToken(token);
      userEmail = user.email;
      step = 1;
    } catch {
      error = $t('setPasswordInvalid');
      step = -1;
    }
  });

  function back() {
    if (step > 1) step--;
  }

  async function advance() {
    error = '';
    if (step >= 1 && step <= 4) {
      const idx = currentStepIndex();
      if (!selections[idx]) { error = $t('authErrorSelection'); return; }
      if (step < 4) { step++; return; }
      await submit();
    }
  }

  async function submit() {
    loading = true;
    try {
      await api.applyPasswordReset(token, selections as [string, string, string, string], pool);
      step = 5;
    } catch {
      error = $t('authErrorServer');
    } finally {
      loading = false;
    }
  }
</script>

<svelte:head>
  <title>{$t('setPasswordTitle')} — {$brandName}</title>
</svelte:head>

<AuthFrame tilt={0.5}>
  {#if step === 0}
    <p class="auth-hint">…</p>

  {:else if step === -1}
    <p class="auth-error">{error}</p>

  {:else if step >= 1 && step <= 4}
    <div class="auth-progress">
      {#each Array(4) as _, i}
        <div class="auth-pip" class:active={step >= i + 1} class:done={step > i + 1}></div>
      {/each}
    </div>
    <h1 class="auth-title">{$t('setPasswordTitle')}</h1>
    <p class="auth-hint">{$t('setPasswordHint')} — {$t('authStep')} {step} {$t('authOf')} 4</p>
    <p class="auth-email-hint">{userEmail}</p>

    {@const cat = currentCategory()}
    <p class="auth-hint auth-category-hint">{$t(`authCategory${cat.id.charAt(0).toUpperCase()}${cat.id.slice(1)}` as Parameters<typeof $t>[0])}</p>
    <p class="auth-choose">{$t('authChooseOne')}</p>
    <div class="auth-grid">
      {#each currentIcons() as icon}
        <button
          class="auth-icon-btn"
          class:selected={selections[currentStepIndex()] === icon.id}
          onclick={() => { selections[currentStepIndex()] = icon.id; }}
          title={icon.labelRu}
          aria-label={icon.labelRu}
        >
          {@html icon.svg}
          <span class="auth-icon-label">{iconLabel(icon, $lang)}</span>
        </button>
      {/each}
    </div>

    {#if error}
      <p class="auth-error">{error}</p>
    {/if}

    <div class="auth-nav">
      {#if step > 1}
        <button class="auth-btn-ghost" onclick={back}>{$t('authBack')}</button>
      {/if}
      <button class="auth-btn-primary" onclick={advance} disabled={loading}>
        {loading ? '…' : step === 4 ? $t('setPasswordSubmit') : $t('authNext')}
      </button>
    </div>

  {:else if step === 5}
    <div class="success-wrap">
      <div class="wax-seal">✦</div>
      <h1 class="auth-title">{$t('setPasswordSuccess')}</h1>
      <button class="auth-btn-primary" onclick={() => goto('/login')}>
        {$t('setPasswordGoLogin')}
      </button>
    </div>
  {/if}
</AuthFrame>

<style>
  .auth-email-hint {
    font-size: .8rem;
    color: #6f3b24;
    opacity: .7;
    margin: -.25rem 0 .5rem;
    text-align: center;
  }
  .auth-category-hint {
    font-weight: 600;
    color: #34251c;
  }
  .success-wrap {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1.25rem;
    text-align: center;
  }
  .wax-seal {
    font-size: 2.5rem;
    color: #c65f3c;
    line-height: 1;
  }
</style>
