<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { t } from '$lib/i18n';
  import type { SmtpSettings } from '$lib/types/api';

  let smtp = $state<SmtpSettings>({ host: null, port: null, user: null, pass: null, from: null });
  let loading = $state(true);
  let saving = $state(false);
  let status = $state<'idle' | 'saved' | 'error'>('idle');

  onMount(async () => {
    try {
      smtp = await api.getSmtpSettings();
    } catch { /* ok if empty */ }
    loading = false;
  });

  async function handleSave(e: SubmitEvent) {
    e.preventDefault();
    saving = true;
    status = 'idle';
    try {
      await api.saveSmtpSettings({
        host: smtp.host || null,
        port: smtp.port ? Number(smtp.port) : null,
        user: smtp.user || null,
        pass: smtp.pass || null,
        from: smtp.from || null,
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
  <h2 class="text-sm uppercase tracking-[0.1em] text-[#6f3b24] mb-1">{$t('adminSmtpTitle')}</h2>
  <p class="text-xs text-[#8a7060] mb-6">{$t('adminSmtpHint')}</p>

  {#if loading}
    <div class="text-xs text-[#8a7060]">…</div>
  {:else}
    <form onsubmit={handleSave} class="space-y-4">
      <div class="grid grid-cols-2 gap-4">
        <label class="block">
          <span class="text-[10px] uppercase tracking-[0.08em] text-[#5f4636] block mb-1">{$t('adminSmtpHost')}</span>
          <input
            type="text"
            bind:value={smtp.host}
            placeholder="smtp.example.com"
            class="w-full bg-[#fff9f0] border border-[#34251c]/20 px-3 py-2 text-sm text-[#34251c] outline-none focus:border-[#34251c]/50 transition-colors"
          />
        </label>
        <label class="block">
          <span class="text-[10px] uppercase tracking-[0.08em] text-[#5f4636] block mb-1">{$t('adminSmtpPort')}</span>
          <input
            type="number"
            bind:value={smtp.port}
            placeholder="587"
            min="1"
            max="65535"
            class="w-full bg-[#fff9f0] border border-[#34251c]/20 px-3 py-2 text-sm text-[#34251c] outline-none focus:border-[#34251c]/50 transition-colors"
          />
        </label>
      </div>

      <div class="grid grid-cols-2 gap-4">
        <label class="block">
          <span class="text-[10px] uppercase tracking-[0.08em] text-[#5f4636] block mb-1">{$t('adminSmtpUser')}</span>
          <input
            type="text"
            bind:value={smtp.user}
            autocomplete="off"
            class="w-full bg-[#fff9f0] border border-[#34251c]/20 px-3 py-2 text-sm text-[#34251c] outline-none focus:border-[#34251c]/50 transition-colors"
          />
        </label>
        <label class="block">
          <span class="text-[10px] uppercase tracking-[0.08em] text-[#5f4636] block mb-1">{$t('adminSmtpPass')}</span>
          <input
            type="password"
            bind:value={smtp.pass}
            autocomplete="new-password"
            class="w-full bg-[#fff9f0] border border-[#34251c]/20 px-3 py-2 text-sm text-[#34251c] outline-none focus:border-[#34251c]/50 transition-colors"
          />
        </label>
      </div>

      <label class="block">
        <span class="text-[10px] uppercase tracking-[0.08em] text-[#5f4636] block mb-1">{$t('adminSmtpFrom')}</span>
        <input
          type="email"
          bind:value={smtp.from}
          placeholder="noreply@example.com"
          class="w-full bg-[#fff9f0] border border-[#34251c]/20 px-3 py-2 text-sm text-[#34251c] outline-none focus:border-[#34251c]/50 transition-colors"
        />
      </label>

      <div class="flex items-center gap-4 pt-2">
        <button
          type="submit"
          disabled={saving}
          class="btn-gothic text-[10px] px-4 py-2 disabled:opacity-50"
        >
          {saving ? '…' : $t('adminSmtpSave')}
        </button>
        {#if status === 'saved'}
          <span class="text-xs text-green-700">{$t('adminSmtpSaved')}</span>
        {:else if status === 'error'}
          <span class="text-xs text-red-700">{$t('adminSmtpError')}</span>
        {/if}
      </div>
    </form>
  {/if}
</div>
