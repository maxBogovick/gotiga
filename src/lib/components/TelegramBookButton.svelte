<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { t, lang } from '$lib/i18n';
  import { authStore } from '$lib/stores/auth.svelte';
  import type { TelegramSubscribeResponse } from '$lib/types/api';

  /**
   * «Подписаться через Telegram» — второй канал книги дома рядом с почтой.
   *
   * Это не способ подтвердить адрес: нажатие «Запустить» в боте доказывает
   * Telegram, а не ящик, поэтому и подписка отдельная — вести приходят в бота.
   * Печатается только когда дом умеет говорить с ботом входа.
   */
  let {
    source,
    age = $bindable(false),
    showAge = true,
    onrequested,
  }: {
    source: string;
    /** Подтверждён ли возраст. Форма, у которой есть своя галочка, привязывает её. */
    age?: boolean;
    /** Своя галочка возраста нужна, когда рядом формы с ней нет. */
    showAge?: boolean;
    onrequested: (res: TelegramSubscribeResponse) => void;
  } = $props();

  let enabled = $state(false);
  let busy = $state(false);
  let error = $state('');

  onMount(async () => {
    try {
      enabled = (await api.telegramLoginConfig()).enabled;
    } catch {
      enabled = false;
    }
  });

  async function go() {
    if (busy) return;
    error = '';
    if (!age) {
      error = $t('formAgeConfirmRequired');
      return;
    }
    busy = true;
    try {
      onrequested(await api.subscribeTelegram({ lang: $lang, source, ageConfirmed: true }, authStore.token));
    } catch {
      error = $t('bookTelegramError');
    } finally {
      busy = false;
    }
  }
</script>

{#if enabled}
  <div class="tg">
    <p class="or">{$t('bookOr')}</p>
    {#if showAge}
      <label class="age">
        <input type="checkbox" name="tg-age-confirm" bind:checked={age} disabled={busy} />
        {$t('formAgeConfirm')}
      </label>
    {/if}
    <button type="button" class="tg-btn" onclick={go} disabled={busy}>
      {busy ? '…' : $t('bookTelegramButton')}
    </button>
    {#if error}<p class="err" role="alert">{error}</p>{/if}
  </div>
{/if}

<style>
  .tg {
    display: grid;
    gap: 8px;
    justify-items: start;
    margin-top: 14px;
  }
  .or {
    margin: 0;
    font-size: 11px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--ink-soft, #5a4638);
  }
  .age {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    font-size: 12.5px;
    color: var(--ink-soft, #5a4638);
  }
  .tg-btn {
    padding: 0.55rem 0.9rem;
    background: transparent;
    border: 1px solid rgba(52, 37, 28, 0.35);
    color: var(--ink, #34251c);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
    transition: border-color 0.2s, color 0.2s;
  }
  .tg-btn:hover:not(:disabled) {
    border-color: var(--color-ember, #c65f3c);
    color: var(--color-ember-deep, #6f3b24);
  }
  .tg-btn:disabled {
    opacity: 0.55;
    cursor: default;
  }
  .err {
    margin: 0;
    font-size: 12.5px;
    color: var(--color-ember-deep, #6f3b24);
  }
</style>
