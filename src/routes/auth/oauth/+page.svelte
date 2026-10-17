<script lang="ts">
  /**
   * Возвращение от Google или Apple.
   *
   * Сюда сервер отправляет браузер после callback. В `#`-части адреса лежат
   * `state` и одноразовый код провайдера (или `error`). Сам по себе код ничего
   * не открывает: сервер обменяет его, только если вместе с ним придёт
   * привязка, а она лежит в `sessionStorage` вкладки, которая просила вход.
   * Поэтому ссылка, присланная посторонним, или вход, продолженный в другой
   * вкладке, кончаются здесь словами «остыл», а не чужой сессией; а тот, кто
   * сам получил привязку и прислал жертве ссылку, не видит кода.
   *
   * Код из адреса убирается сразу, до первого запроса: он не должен остаться в
   * истории браузера.
   *
   * После первого входа через провайдера страница спрашивает имя: имя видно
   * другим людям, и брать его из почты или чужого профиля нельзя.
   */
  import { goto } from '$app/navigation';
  import { onMount, onDestroy } from 'svelte';
  import { t, brandName } from '$lib/i18n';
  import { api } from '$lib/api';
  import { interact } from '$lib/analytics';
  import { authStore } from '$lib/stores/auth.svelte';
  import { savedFigurines } from '$lib/stores/saved-figurines.svelte';
  import { forgetOauth, keptOauth, type KeptOauth } from '$lib/oauth';
  import AuthFrame from '$lib/components/auth/AuthFrame.svelte';

  // Печать «вы вошли» перед уходом: переход без единого слова о том, что вход
  // состоялся, читался как «меня выкинуло» (так же устроена дверь Telegram).
  const SEAL_HELD = 1200;

  let stage = $state<'opening' | 'naming' | 'done' | 'linked' | 'failed' | 'cold'>('opening');
  let failure = $state('failed');
  let name = $state('');
  let kept: KeptOauth | null = null;
  let linking = $state(false);
  let nameInput = $state('');
  let nameSaving = $state(false);
  let nameError = $state(false);
  let enteredFrom = '/';
  let enteredToken = '';
  let seal: ReturnType<typeof setTimeout> | null = null;

  onDestroy(() => {
    if (seal) clearTimeout(seal);
  });

  /** Расписки гостя из localStorage привязываются к имени — как и у других дверей. */
  async function linkGuestClaims(sessionToken: string) {
    try {
      const tokens: string[] = [];
      for (const key of Object.keys(localStorage).filter((k) => k.startsWith('gotiga_claims_'))) {
        const items: Array<{ token?: string }> = JSON.parse(localStorage.getItem(key) ?? '[]');
        if (Array.isArray(items)) for (const item of items) if (item?.token) tokens.push(item.token);
      }
      if (tokens.length > 0) await api.userLinkBookings(sessionToken, tokens);
    } catch { /* non-critical */ }
  }

  /** Провайдер без имени получает временное — по своему названию; его не подставляем. */
  const PLACEHOLDER_NAMES = ['google', 'apple'];

  onMount(async () => {
    const returned = new URLSearchParams(window.location.hash.replace(/^#/, ''));
    const state = returned.get('state') ?? '';
    const code = returned.get('code') ?? '';
    const refusal = returned.get('error');
    const appleUser = returned.get('user');
    // Код не должен остаться в адресной строке и в истории.
    if (window.location.hash) history.replaceState(null, '', window.location.pathname);

    kept = keptOauth();
    if (!state || !kept || kept.state !== state || kept.dies < Date.now()) {
      forgetOauth();
      stage = 'cold';
      return;
    }
    const { binder, provider, mode, from } = kept;
    linking = mode === 'link';

    // Человек отказался у провайдера (или провайдер вернул не то): сервер уже
    // закрыл попытку, обменивать нечего.
    if (refusal || !code) {
      forgetOauth();
      failure = refusal ?? 'denied';
      interact('auth_oauth', `${provider}:${mode}:${failure}`);
      stage = 'failed';
      return;
    }

    let res;
    try {
      res = await api.oauthRedeem(state, binder, code, appleUser);
    } catch {
      // Попытка сервером, возможно, уже взята; код убран из адреса, так что
      // перезагрузка не повторит обмен — человек начинает вход заново.
      forgetOauth();
      failure = 'failed';
      stage = 'failed';
      return;
    }
    forgetOauth();

    if (res.state === 'expired') {
      stage = 'cold';
      return;
    }
    if (res.state === 'failed' || !res.user) {
      failure = res.failure ?? 'failed';
      interact('auth_oauth', `${provider}:${mode}:${failure}`);
      stage = 'failed';
      return;
    }

    name = res.user.displayName;
    // Привязка из профиля: сессия у человека своя, новой не заводили.
    if (mode === 'link') {
      if (authStore.token) authStore.setSession(authStore.token, res.user);
      interact('auth_oauth', `${provider}:link:done`);
      stage = 'linked';
      seal = setTimeout(() => goto(from), SEAL_HELD);
      return;
    }

    if (!res.sessionToken) {
      stage = 'failed';
      return;
    }
    authStore.setSession(res.sessionToken, res.user);
    interact('auth_done', `${res.isNew ? 'register' : 'login'}_${provider}`);
    await linkGuestClaims(res.sessionToken);
    await savedFigurines.syncWithServer({ importLocal: true }).catch(() => {});

    // Первый вход: имя видят другие люди, поэтому его называет сам человек.
    // Имя, которое дал провайдер, подставлено; временное («Google») — нет.
    if (res.isNew) {
      enteredFrom = from;
      enteredToken = res.sessionToken;
      nameInput = PLACEHOLDER_NAMES.includes(name.trim().toLowerCase()) ? '' : name;
      stage = 'naming';
      return;
    }
    stage = 'done';
    seal = setTimeout(() => goto(from), SEAL_HELD);
  });

  async function saveName(event: SubmitEvent) {
    event.preventDefault();
    const chosen = nameInput.trim();
    if (!chosen || nameSaving) return;
    nameSaving = true;
    nameError = false;
    try {
      if (chosen !== name) {
        const user = await api.updateProfile(enteredToken, chosen);
        authStore.setSession(enteredToken, user);
        name = user.displayName;
      }
      stage = 'done';
      seal = setTimeout(() => goto(enteredFrom), SEAL_HELD);
    } catch {
      nameError = true;
    } finally {
      nameSaving = false;
    }
  }

  const failureText = $derived(
    failure === 'denied'
      ? $t('oauthDenied')
      : failure === 'age'
        ? $t('oauthAge')
      : failure === 'conflict'
        ? $t('oauthConflict')
        : failure === 'blocked'
          ? $t('oauthBlocked')
          : $t('oauthFailed')
  );
  // При отказе по возрасту человека ведут на регистрацию: там галочка.
  const back = $derived(linking ? '/profile/account' : failure === 'age' ? '/register' : '/login');
</script>

<svelte:head>
  <title>{$t('oauthTitle')} — {$brandName}</title>
  <meta name="robots" content="noindex" />
</svelte:head>

<AuthFrame tilt={0.4}>
  {#if stage === 'opening'}
    <h1 class="auth-title">{$t('oauthTitle')}</h1>
    <p class="auth-hint" role="status">{$t('oauthOpening')}</p>
  {:else if stage === 'naming'}
    <h1 class="auth-title">{$t('oauthNameTitle')}</h1>
    <p class="note">{$t('oauthNameLead')}</p>
    <form onsubmit={saveName}>
      <div class="auth-fields">
        <label class="auth-field">
          <span>{$t('authDisplayName')}</span>
          <input type="text" bind:value={nameInput} maxlength="80" placeholder={$t('authDisplayNameHint')}
                 id="auth-display-name" name="name" autocomplete="name" />
        </label>
      </div>
      {#if nameError}<p class="note" role="alert">{$t('oauthNameFailed')}</p>{/if}
      <div class="auth-nav">
        <button class="auth-btn-primary" type="submit" disabled={!nameInput.trim() || nameSaving}>
          {$t('oauthNameSave')}
        </button>
      </div>
    </form>
  {:else if stage === 'done' || stage === 'linked'}
    <div class="seal" role="status">
      <span class="seal-mark" aria-hidden="true">✓</span>
      <span class="seal-word">{stage === 'linked' ? $t('oauthLinked') : $t('authTelegramDone')}</span>
      {#if name}<span class="seal-name">{name}</span>{/if}
    </div>
  {:else if stage === 'failed'}
    <h1 class="auth-title">{$t('oauthTitle')}</h1>
    <p class="note" role="alert">{failureText}</p>
    <div class="auth-nav">
      <button class="auth-btn-primary" onclick={() => goto(back)}>
        {back === '/login' ? $t('authLogin') : back === '/register' ? $t('authRegister') : $t('oauthBackProfile')}
      </button>
    </div>
  {:else}
    <h1 class="auth-title">{$t('oauthColdTitle')}</h1>
    <p class="note">{$t('oauthCold')}</p>
    <div class="auth-nav">
      <button class="auth-btn-primary" onclick={() => goto('/login')}>{$t('authLogin')}</button>
    </div>
  {/if}
</AuthFrame>

<style>
  .note {
    margin: 0 0 1.2rem;
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 1rem;
    font-style: italic;
    line-height: 1.5;
    color: #6f3b24;
  }

  /* Печать: тот же язык, что у сургуча в модальных окнах и у двери Telegram. */
  .seal {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 12px 20px;
    border: 1px solid #c0a384;
    border-radius: 3px;
    background: #f4ead8;
  }
  .seal-mark {
    width: 26px;
    height: 26px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    background: #6f3b24;
    color: #f8f1e7;
    font-size: 13px;
  }
  .seal-word {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 15px;
    letter-spacing: 0.06em;
    color: #34251c;
  }
  .seal-name {
    font-family: 'Cormorant Garamond', Georgia, serif;
    font-size: 13px;
    font-style: italic;
    color: rgba(95, 70, 54, 0.7);
  }
</style>
