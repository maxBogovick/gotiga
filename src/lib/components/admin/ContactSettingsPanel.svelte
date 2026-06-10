<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { t } from '$lib/i18n';
  import type { ContactSettings } from '$lib/types/api';

  let contact = $state<ContactSettings>({ email: null, telegram: null, phone: null });
  let loading = $state(true);
  let saving = $state(false);
  let status = $state<'idle' | 'saved' | 'error'>('idle');

  onMount(async () => {
    try {
      contact = await api.getContactSettings();
    } catch { /* ok if empty */ }
    loading = false;
  });

  async function handleSave(e: SubmitEvent) {
    e.preventDefault();
    saving = true;
    status = 'idle';
    try {
      await api.saveContactSettings({
        email:    contact.email    || null,
        telegram: contact.telegram || null,
        phone:    contact.phone    || null,
      });
      status = 'saved';
      setTimeout(() => { status = 'idle'; }, 2500);
    } catch {
      status = 'error';
    } finally {
      saving = false;
    }
  }
</script>

<div class="p-6 max-w-lg">
  <h2 class="text-sm uppercase tracking-[0.1em] text-[#6f3b24] mb-1">{$t('adminContactTitle')}</h2>
  <p class="text-xs text-[#8a7060] mb-6">{$t('adminContactHint')}</p>

  {#if loading}
    <div class="text-xs text-[#8a7060]">…</div>
  {:else}
    <form onsubmit={handleSave} class="space-y-4">
      <label class="block">
        <span class="text-[10px] uppercase tracking-[0.08em] text-[#5f4636] block mb-1">{$t('adminContactEmail')}</span>
        <input
          type="email"
          bind:value={contact.email}
          placeholder="curator@example.com"
          class="w-full bg-[#fff9f0] border border-[#34251c]/20 px-3 py-2 text-sm text-[#34251c] outline-none focus:border-[#34251c]/50 transition-colors"
        />
      </label>

      <label class="block">
        <span class="text-[10px] uppercase tracking-[0.08em] text-[#5f4636] block mb-1">{$t('adminContactTelegram')}</span>
        <input
          type="text"
          bind:value={contact.telegram}
          placeholder="https://t.me/username"
          class="w-full bg-[#fff9f0] border border-[#34251c]/20 px-3 py-2 text-sm text-[#34251c] outline-none focus:border-[#34251c]/50 transition-colors"
        />
      </label>

      <label class="block">
        <span class="text-[10px] uppercase tracking-[0.08em] text-[#5f4636] block mb-1">{$t('adminContactPhone')}</span>
        <input
          type="tel"
          bind:value={contact.phone}
          placeholder="+7 900 000 00 00"
          class="w-full bg-[#fff9f0] border border-[#34251c]/20 px-3 py-2 text-sm text-[#34251c] outline-none focus:border-[#34251c]/50 transition-colors"
        />
      </label>

      <div class="flex items-center gap-4 pt-2">
        <button
          type="submit"
          disabled={saving}
          class="btn-gothic text-[10px] px-4 py-2 disabled:opacity-50"
        >
          {saving ? '…' : $t('adminContactSave')}
        </button>
        {#if status === 'saved'}
          <span class="text-xs text-green-700">{$t('adminContactSaved')}</span>
        {:else if status === 'error'}
          <span class="text-xs text-red-700">{$t('adminContactError')}</span>
        {/if}
      </div>
    </form>
  {/if}
</div>
