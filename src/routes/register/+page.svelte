<script lang="ts">
  import { goto } from '$app/navigation';
  import { t , brandName } from '$lib/i18n';
  import { api } from '$lib/api';
  import { isValidEmail } from '$lib/validation';
  import { VISUAL_CATEGORIES, iconLabel } from '$lib/data/visualIcons';
  import { lang } from '$lib/i18n';
  import AuthFrame from '$lib/components/auth/AuthFrame.svelte';
  import ReminderCard from '$lib/components/auth/ReminderCard.svelte';

  const TOTAL_STEPS = 6; // 1 (email+name) + 4 (categories) + 1 (success)

  let step = $state(1);
  let email = $state('');
  let displayName = $state('');
  let error = $state('');
  let loading = $state(false);

  let selections = $state<string[]>(['', '', '', '']);
  let registeredUser = $state<{ id: string; email: string; displayName: string } | null>(null);
  let finalSelections = $state<string[]>([]);

  function categoryStepIndex() { return step - 2; }
  function currentCategory()   { return VISUAL_CATEGORIES[categoryStepIndex()]; }

  function validateStep1(): string {
    if (!isValidEmail(email)) return $t('authErrorEmail');
    if (!displayName.trim())  return $t('authErrorName');
    return '';
  }

  async function advance() {
    error = '';
    if (step === 1) {
      const err = validateStep1();
      if (err) { error = err; return; }
      step = 2;
      return;
    }
    if (step >= 2 && step <= 5) {
      const idx = categoryStepIndex();
      if (!selections[idx]) { error = $t('authErrorSelection'); return; }
      if (step < 5) { step++; return; }
      await submit();
    }
  }

  function back() {
    error = '';
    if (step > 1) step--;
  }

  async function submit() {
    loading = true;
    try {
      const result = await api.userRegister(
        email.trim().toLowerCase(),
        displayName.trim(),
        selections as [string, string, string, string]
      );
      registeredUser = result.user;
      finalSelections = [...selections]; // selections stay local — never returned by server

      if (typeof localStorage !== 'undefined') {
        localStorage.setItem('gotiga_visual_reminder', JSON.stringify(finalSelections));
      }
      step = 6;
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : '';
      if (msg.includes('409') || msg.toLowerCase().includes('conflict')) {
        error = $t('authErrorConflict');
      } else {
        error = $t('authErrorServer');
      }
    } finally {
      loading = false;
    }
  }
</script>

<svelte:head>
  <title>{$t('authRegister')} — {$brandName}</title>
</svelte:head>

<AuthFrame tilt={-0.3}>
  {#if step < 6}
    <div class="auth-progress">
      {#each Array(TOTAL_STEPS - 1) as _, i}
        <div class="auth-pip" class:active={step >= i + 1} class:done={step > i + 1}></div>
      {/each}
    </div>
    <h1 class="auth-title">{$t('authRegister')}</h1>
  {/if}

  {#if step === 1}
    <p class="auth-hint">{$t('authStep')} 1 {$t('authOf')} 5</p>
    <div class="auth-fields">
      <label class="auth-field">
        <span>{$t('authEmail')}</span>
        <input type="email" bind:value={email} placeholder={$t('authEmailPlaceholder')}
               autocomplete="email" onkeydown={(e) => e.key === 'Enter' && advance()} />
      </label>
      <label class="auth-field">
        <span>{$t('authDisplayName')}</span>
        <input type="text" bind:value={displayName} placeholder={$t('authDisplayNameHint')}
               autocomplete="name" onkeydown={(e) => e.key === 'Enter' && advance()} />
      </label>
    </div>
  {/if}

  {#if step >= 2 && step <= 5}
    {@const cat = currentCategory()}
    <p class="auth-hint">{$t('authStep')} {step} {$t('authOf')} 5 — {$t(`authCategory${cat.id.charAt(0).toUpperCase()}${cat.id.slice(1)}` as any)}</p>
    <p class="auth-choose">{$t('authChooseOne')}</p>
    <div class="auth-grid">
      {#each cat.icons as icon}
        <button
          class="auth-icon-btn"
          class:selected={selections[categoryStepIndex()] === icon.id}
          onclick={() => { selections[categoryStepIndex()] = icon.id; }}
          title={icon.labelRu}
          aria-label={icon.labelRu}
        >
          {@html icon.svg}
          <span class="auth-icon-label">{iconLabel(icon, $lang)}</span>
        </button>
      {/each}
    </div>
  {/if}

  {#if step === 6 && registeredUser}
    <ReminderCard {finalSelections} userName={registeredUser.displayName} onContinue={() => goto('/')} />
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
        {loading ? '…' : step === 5 ? $t('authCreateAccount') : $t('authNext')}
      </button>
    </div>
    <p class="auth-switch">
      {$t('authHaveAccount')}
      <button class="auth-link" onclick={() => goto('/login')}>{$t('authLogin')}</button>
    </p>
  {/if}
</AuthFrame>
