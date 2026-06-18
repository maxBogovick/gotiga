<script lang="ts">
  import { goto } from '$app/navigation';
  import { t , brandName } from '$lib/i18n';
  import { api } from '$lib/api';
  import { isValidEmail } from '$lib/validation';
  import { VISUAL_CATEGORIES, iconLabel, getIconById, generatePersonalPool } from '$lib/data/visualIcons';
  import { lang } from '$lib/i18n';
  import { downloadKeyCard } from '$lib/utils/keyCard';
  import AuthFrame from '$lib/components/auth/AuthFrame.svelte';
  import ReminderCard from '$lib/components/auth/ReminderCard.svelte';

  // Steps: 1 = email+name · 2–5 = pick a sign per category · 6 = seal in memory · 7 = success
  const SELECT_STEPS = 5; // numbered, pip-tracked data-entry steps (1 + 4 categories)

  let step = $state(1);
  let email = $state('');
  let displayName = $state('');
  let error = $state('');
  let loading = $state(false);
  let downloading = $state(false);
  let memorized = $state(false);

  // Personal subset shown to this user (one list of icon_ids per category).
  // Generated once on load and sent with the registration so the same grid is
  // replayed at login. Picked from each category's master pool.
  const pool = generatePersonalPool();

  let selections = $state<string[]>(['', '', '', '']);
  let registeredUser = $state<{ id: string; email: string; displayName: string } | null>(null);
  let finalSelections = $state<string[]>([]);

  function categoryStepIndex() { return step - 2; }
  function currentCategory()   { return VISUAL_CATEGORIES[categoryStepIndex()]; }
  // Icons to show this step: the user's personal subset, resolved to full defs.
  function currentIcons() {
    const cat = currentCategory();
    return pool[categoryStepIndex()]
      .map(id => getIconById(cat.id, id))
      .filter((i): i is NonNullable<typeof i> => !!i);
  }
  // The four key slots (resolved icon or null) — drives the running strip and the seal.
  function keySlots() {
    return VISUAL_CATEGORIES.map((cat, i) => ({
      index: i,
      icon: selections[i] ? getIconById(cat.id, selections[i]) : null,
    }));
  }

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
      step++; // step 5 → 6 (the sealing step), not submit yet
      return;
    }
    if (step === 6) {
      if (!memorized) { error = $t('authSealUnchecked'); return; }
      await submit();
    }
  }

  function back() {
    error = '';
    if (step > 1) step--;
  }

  async function downloadKey() {
    downloading = true;
    try {
      await downloadKeyCard(selections, displayName.trim() || $brandName, $lang);
    } finally {
      downloading = false;
    }
  }

  async function submit() {
    loading = true;
    try {
      const result = await api.userRegister(
        email.trim().toLowerCase(),
        displayName.trim(),
        selections as [string, string, string, string],
        pool
      );
      registeredUser = result.user;
      finalSelections = [...selections]; // selections stay local — never returned by server

      if (typeof localStorage !== 'undefined') {
        localStorage.setItem('gotiga_visual_reminder', JSON.stringify(finalSelections));
      }
      step = 7;
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
  {#if step <= SELECT_STEPS}
    <div class="auth-progress">
      {#each Array(SELECT_STEPS) as _, i}
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

    <div class="memorize-banner">
      <span class="memorize-seal" aria-hidden="true">⚠</span>
      <span class="memorize-text">{$t('authMemorizeWarning')}</span>
    </div>

    <p class="auth-choose">{$t('authChooseOne')}</p>
    <div class="auth-grid">
      {#each currentIcons() as icon}
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

    <!-- Running "your key so far" — reinforces memory as it builds -->
    <div class="key-strip" aria-label={$t('authKeyProgress')}>
      <span class="key-strip-label">{$t('authKeyProgress')}</span>
      <div class="key-strip-slots">
        {#each keySlots() as slot}
          <div class="key-slot" class:filled={!!slot.icon} class:current={slot.index === categoryStepIndex()}>
            {#if slot.icon}
              {@html slot.icon.svg}
            {:else}
              <span class="key-slot-dot">·</span>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {/if}

  {#if step === 6}
    <h1 class="auth-title seal-title">{$t('authSealTitle')}</h1>
    <p class="seal-lead">{$t('authSealLead')}</p>

    <div class="seal-card">
      <p class="seal-card-name">{displayName.trim() || $brandName}</p>
      <div class="seal-icons">
        {#each keySlots() as slot}
          {#if slot.icon}
            <div class="seal-icon">
              {@html slot.icon.svg}
              <span>{iconLabel(slot.icon, $lang)}</span>
            </div>
          {/if}
        {/each}
      </div>
      <p class="seal-card-brand">{$brandName.toUpperCase()}</p>
    </div>

    <p class="seal-text">{$t('authSealText')}</p>

    <p class="seal-download-hint">{$t('authSealDownloadHint')}</p>
    <button class="seal-download" onclick={downloadKey} disabled={downloading}>
      {downloading ? '…' : `↓ ${$t('authDownloadCard')}`}
    </button>

    <label class="seal-check" class:armed={memorized}>
      <input type="checkbox" bind:checked={memorized} />
      <span>{$t('authMemorizedCheck')}</span>
    </label>
  {/if}

  {#if step === 7 && registeredUser}
    <ReminderCard {finalSelections} userName={registeredUser.displayName} onContinue={() => goto('/')} />
  {/if}

  {#if error}
    <p class="auth-error">{error}</p>
  {/if}

  {#if step <= 6}
    <div class="auth-nav">
      {#if step > 1}
        <button class="auth-btn-ghost" onclick={back}>{$t('authBack')}</button>
      {/if}
      <button class="auth-btn-primary" onclick={advance} disabled={loading || (step === 6 && !memorized)}>
        {loading ? '…' : step === 6 ? $t('authCreateAccount') : $t('authNext')}
      </button>
    </div>
    <p class="auth-switch">
      {$t('authHaveAccount')}
      <button class="auth-link" onclick={() => goto('/login')}>{$t('authLogin')}</button>
    </p>
  {/if}
</AuthFrame>

<style>
  /* Caution strip on every pick step — the four signs ARE the password */
  .memorize-banner {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    background: #f3e3d1;
    border: 1px solid #d8a06f;
    border-left: 3px solid #c65f3c;
    padding: 0.55rem 0.7rem;
    margin: 0 0 0.85rem;
  }
  .memorize-seal {
    color: #c65f3c;
    font-size: 0.95rem;
    line-height: 1.3;
    flex: 0 0 auto;
  }
  .memorize-text {
    font-family: Inter, sans-serif;
    font-size: 0.72rem;
    line-height: 1.35;
    color: #6f3b24;
    letter-spacing: 0.01em;
  }

  /* Running key */
  .key-strip {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.6rem;
    margin-top: 1rem;
  }
  .key-strip-label {
    font-family: Inter, sans-serif;
    font-size: 0.6rem;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    color: #9a7c5c;
  }
  .key-strip-slots { display: flex; gap: 0.4rem; }
  .key-slot {
    width: 30px;
    height: 30px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid #d8c6b1;
    border-radius: 3px;
    color: #6f3b24;
    background: #fdf8f2;
  }
  .key-slot.filled { border-color: #c0a384; }
  .key-slot.current { border-color: #c65f3c; box-shadow: 0 0 0 1px #c65f3c33; }
  .key-slot :global(svg) { width: 20px; height: 20px; }
  .key-slot-dot { color: #cbb79c; font-weight: 700; }

  /* Sealing step */
  .seal-title { margin-bottom: 0.25rem; }
  .seal-lead {
    text-align: center;
    font-style: italic;
    color: #6f3b24;
    font-size: 0.9rem;
    margin: 0 0 1rem;
  }
  .seal-card {
    background: #fdf8f2;
    border: 1px solid #d8c6b1;
    outline: 3px solid #f4ead8;
    outline-offset: -5px;
    padding: 1.1rem 1rem;
    text-align: center;
    margin-bottom: 1rem;
  }
  .seal-card-name {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1rem;
    color: #34251c;
    margin: 0 0 0.9rem;
    letter-spacing: 0.08em;
  }
  .seal-icons {
    display: flex;
    justify-content: center;
    gap: 1.1rem;
  }
  .seal-icon {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.3rem;
    color: #6f3b24;
  }
  .seal-icon :global(svg) { width: 40px; height: 40px; }
  .seal-icon span {
    font-size: 0.6rem;
    font-family: Inter, sans-serif;
    color: #9a7c5c;
    letter-spacing: 0.04em;
  }
  .seal-card-brand {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 0.65rem;
    letter-spacing: 0.3em;
    color: #d8c6b1;
    margin: 0.9rem 0 0;
  }
  .seal-text {
    font-family: Inter, sans-serif;
    font-size: 0.8rem;
    line-height: 1.5;
    color: #6f3b24;
    text-align: center;
    margin: 0 0 1.1rem;
  }
  .seal-download-hint {
    font-family: Inter, sans-serif;
    font-size: 0.72rem;
    color: #9a7c5c;
    text-align: center;
    margin: 0 0 0.5rem;
  }
  .seal-download {
    display: block;
    width: 100%;
    background: transparent;
    border: 1px solid #c65f3c;
    color: #6f3b24;
    padding: 0.6rem 0.9rem;
    font-size: 0.85rem;
    font-family: Inter, sans-serif;
    cursor: pointer;
    letter-spacing: 0.04em;
    transition: background 0.2s, border-color 0.2s;
    margin-bottom: 1.1rem;
  }
  .seal-download:hover:not(:disabled) { background: #f3e3d1; }
  .seal-download:disabled { opacity: 0.5; cursor: not-allowed; }

  .seal-check {
    display: flex;
    align-items: flex-start;
    gap: 0.55rem;
    cursor: pointer;
    padding: 0.6rem 0.7rem;
    border: 1px dashed #c0a384;
    background: #faf3e9;
    transition: border-color 0.2s, background 0.2s;
  }
  .seal-check.armed { border-color: #6f3b24; border-style: solid; background: #f4ead8; }
  .seal-check input { margin-top: 0.15rem; accent-color: #6f3b24; flex: 0 0 auto; }
  .seal-check span {
    font-family: Inter, sans-serif;
    font-size: 0.8rem;
    line-height: 1.4;
    color: #34251c;
  }
</style>
