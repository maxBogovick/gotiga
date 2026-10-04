<script lang="ts">
  /**
   * Лист профиля: кто вошёл, рейка разделов и место под раздел.
   *
   * Данные набирает он один и кладёт их в `userDesk`. Прежде профиль был одной
   * страницей на 3388 строк с переменной `view`, и это значило: «назад»
   * уводило с профиля, перезагрузка возвращала в хаб, на переписку нельзя было
   * дать ссылку. Теперь разделы — маршруты, а общая загрузка — одна.
   *
   * Аналитике (§ 17) правка ничего не стоит: `PATH_GROUP_SQL` сворачивает
   * `LIKE '/profile%'` в одну комнату, поэтому подмаршруты не заводят новых
   * строк в отчёте и не требуют правки сервера.
   */
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { api } from '$lib/api';
  import { authStore } from '$lib/stores/auth.svelte';
  import { userDesk } from '$lib/stores/user-desk.svelte';
  import ProfileMasthead from '$lib/components/profile/ProfileMasthead.svelte';
  import ProfileTabs from '$lib/components/profile/ProfileTabs.svelte';

  let { children } = $props();

  let ready = $state(false);

  function toLogin() {
    goto(`/login?from=${encodeURIComponent(page.url.pathname)}`);
  }

  onMount(async () => {
    if (!authStore.isLoggedIn && !authStore.token) {
      toLogin();
      return;
    }
    if (!authStore.isLoggedIn && authStore.token) {
      try {
        authStore.user = await api.userMe(authStore.token);
      } catch {
        authStore.clearSession();
        toLogin();
        return;
      }
    }
    // Прошение, поданное гостем до входа, привязывается к имени при первом же
    // заходе в профиль.
    if (authStore.token && typeof localStorage !== 'undefined') {
      const pending = localStorage.getItem('gotiga_pending_claim');
      if (pending) {
        try { await api.claimCommission(authStore.token, pending); } catch { /* ignore */ }
        localStorage.removeItem('gotiga_pending_claim');
      }
    }
    ready = true;
    await userDesk.load();
  });

  // Сессия истекла на загрузке стола: стор про адреса не знает и только
  // поднимает признак, уводит отсюда лист.
  $effect(() => {
    if (userDesk.expired) {
      userDesk.reset();
      toLogin();
    }
  });
</script>

<div class="pf-page">
  <div class="pf-body">
    {#if ready}
      <ProfileMasthead />
      <ProfileTabs />
      <div class="section">
        {@render children()}
      </div>
    {:else}
      <p class="pf-empty">…</p>
    {/if}
  </div>
</div>

<style>
  .section { padding-top: 1.6rem; }
</style>
