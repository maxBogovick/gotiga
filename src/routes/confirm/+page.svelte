<script lang="ts">
  /**
   * Дверь, открытая изнутри почтового ящика.
   *
   * Сюда ведёт единственная ссылка из письма. Она же и подтверждает адрес, и
   * впускает: спрашивать знаки ещё раз сразу после того, как человек их
   * выбрал, значило бы спросить дважды одно и то же.
   *
   * Страница не рисует ничего, кроме ожидания и отказа: удача уводит в дом.
   */
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { t, brandName } from '$lib/i18n';
  import { api } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import { savedFigurines } from '$lib/stores/saved-figurines.svelte';
  import AuthFrame from '$lib/components/auth/AuthFrame.svelte';

  let stage = $state<'opening' | 'cold'>('opening');

  onMount(async () => {
    const token = $page.url.searchParams.get('token') ?? '';
    if (!token) {
      stage = 'cold';
      return;
    }
    try {
      const res = await api.confirmEmail(token);
      authStore.setSession(res.sessionToken, res.user);
      // То же «потом», что и у остальных дверей: список желаний гостя
      // сверяется с именем. Расписки здесь не привязываются — у только что
      // заведённого имени их ещё нет.
      await savedFigurines.syncWithServer({ importLocal: true }).catch(() => {});
      goto('/');
    } catch {
      stage = 'cold';
    }
  });
</script>

<svelte:head>
  <title>{$t('confirmTitle')} — {$brandName}</title>
  <meta name="robots" content="noindex" />
</svelte:head>

<AuthFrame tilt={0.4}>
  {#if stage === 'opening'}
    <h1 class="auth-title">{$t('confirmTitle')}</h1>
    <p class="auth-hint">{$t('confirmOpening')}</p>
  {:else}
    <h1 class="auth-title">{$t('confirmColdTitle')}</h1>
    <p class="auth-hint">{$t('confirmColdText')}</p>
    <div class="auth-nav">
      <button class="auth-btn-ghost" onclick={() => goto('/register')}>{$t('authRegister')}</button>
      <button class="auth-btn-primary" onclick={() => goto('/login')}>{$t('authLogin')}</button>
    </div>
  {/if}
</AuthFrame>
