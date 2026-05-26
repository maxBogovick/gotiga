<script lang="ts">
  import { fade, scale } from 'svelte/transition';
  import { api, type AppSettings } from '$lib/api';
  import { t } from '$lib/i18n';

  // Runes for props
  let { isOpen, onClose } = $props<{ isOpen: boolean; onClose: () => void }>();

  // Internal state
  let settings = $state<AppSettings>({ serverUrl: 'http://localhost:3000', apiKey: '' });
  let contactEmail = $state('');
  let isSaving = $state(false);
  let isTesting = $state(false);
  let error = $state<string | null>(null);
  let testStatus = $state<'idle' | 'ok' | 'fail'>('idle');

  // Load settings when opened
  $effect(() => {
    if (isOpen) {
        loadSettings();
    }
  });

  async function loadSettings() {
    try {
        settings = await api.getSettings();
        contactEmail = localStorage.getItem('gotiga_contact_email') ?? '';
        testStatus = 'idle';
    } catch (e) {
        console.error("Failed to load settings", e);
    }
  }

  async function testConnection() {
    isTesting = true;
    testStatus = 'idle';
    try {
        const base = settings.serverUrl ? `${settings.serverUrl}/api/v1` : '/api/v1';
        const res = await fetch(`${base}/health`, { signal: AbortSignal.timeout(5000) });
        testStatus = res.ok ? 'ok' : 'fail';
    } catch {
        testStatus = 'fail';
    } finally {
        isTesting = false;
    }
  }

  async function handleSave() {
    isSaving = true;
    error = null;
    try {
      // Remove trailing slash if present
      if (settings.serverUrl.endsWith('/')) {
        settings.serverUrl = settings.serverUrl.slice(0, -1);
      }
      await api.saveSettings(settings);
      if (contactEmail) localStorage.setItem('gotiga_contact_email', contactEmail);
      else localStorage.removeItem('gotiga_contact_email');
      onClose();
    } catch (e) {
      error = String(e);
    } finally {
      isSaving = false;
    }
  }

  function handleBackdropKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onClose();
    }
  }
</script>

{#if isOpen}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4">
    <div
      class="absolute inset-0 bg-[#6f3b24]/35 backdrop-blur-sm"
      transition:fade
      onclick={onClose}
      onkeydown={handleBackdropKeydown}
      role="button"
      tabindex="0"
      aria-label={$t('settingsCancel')}
    ></div>
    
    <div 
      class="relative bg-[#fff9f0] border border-[#34251c]/30 w-full max-w-md p-8 shadow-[0_0_50px_rgba(111,59,36,0.18)] font-cinzel text-[#34251c]"
      transition:scale={{ start: 0.95 }}
    >
        <div class="absolute top-0 left-0 w-4 h-4 border-t border-l border-[#34251c]/50"></div>
        <div class="absolute top-0 right-0 w-4 h-4 border-t border-r border-[#34251c]/50"></div>
        <div class="absolute bottom-0 left-0 w-4 h-4 border-b border-l border-[#34251c]/50"></div>
        <div class="absolute bottom-0 right-0 w-4 h-4 border-b border-r border-[#34251c]/50"></div>

        <h2 class="text-2xl font-bold mb-2 text-center text-[#6f3b24] tracking-wider uppercase">{$t('settingsTitle')}</h2>
        <div class="w-full h-px bg-gradient-to-r from-transparent via-[#34251c]/30 to-transparent mb-8"></div>

        <div class="space-y-6">
            <label class="block">
                <span class="text-xs uppercase tracking-wide text-[#5f4636] block mb-2">{$t('settingsServerUrl')}</span>
                <input
                    bind:value={settings.serverUrl}
                    type="text"
                    placeholder="https://api.gotiga.com"
                    class="w-full bg-[#fff9f0] border border-[#34251c]/20 p-3 text-sm focus:border-[#34251c]/60 outline-none transition-colors"
                />
            </label>

            <label class="block">
                <span class="text-xs uppercase tracking-wide text-[#5f4636] block mb-2">{$t('settingsApiKey')}</span>
                <input
                    bind:value={settings.apiKey}
                    type="password"
                    placeholder="••••••••••••••••"
                    class="w-full bg-[#fff9f0] border border-[#34251c]/20 p-3 text-sm focus:border-[#34251c]/60 outline-none transition-colors"
                />
            </label>

            <label class="block">
                <span class="text-xs uppercase tracking-wide text-[#5f4636] block mb-2">{$t('settingsContactEmail')}</span>
                <input
                    bind:value={contactEmail}
                    type="email"
                    placeholder="info@gotiga.art"
                    class="w-full bg-[#fff9f0] border border-[#34251c]/20 p-3 text-sm focus:border-[#34251c]/60 outline-none transition-colors"
                />
            </label>

            <div class="flex items-center gap-3">
                <button
                    onclick={testConnection}
                    disabled={isTesting}
                    class="flex-1 py-2 border text-xs uppercase tracking-wide transition-all
                        {testStatus === 'ok' ? 'border-green-700/50 text-green-400 bg-green-900/10' :
                         testStatus === 'fail' ? 'border-red-700/50 text-red-400 bg-red-900/10' :
                         'border-[#34251c]/20 text-[#5f4636] hover:text-[#34251c] hover:bg-[#34251c]/5'}"
                >
                    {#if isTesting}{$t('settingsTesting')}
                    {:else if testStatus === 'ok'}{$t('settingsTestOk')}
                    {:else if testStatus === 'fail'}{$t('settingsTestFail')}
                    {:else}{$t('settingsTest')}{/if}
                </button>
            </div>
        </div>

        {#if error}
            <div class="mt-4 p-3 bg-red-950/30 border border-red-900/50 text-red-300 text-xs">
                {error}
            </div>
        {/if}

        <div class="mt-8 flex gap-4">
            <button
                onclick={onClose}
                class="flex-1 py-3 border border-[#34251c]/20 text-[#5f4636] hover:text-[#34251c] hover:bg-[#34251c]/5 transition-all text-xs uppercase tracking-wide"
            >
                {$t('settingsCancel')}
            </button>
            <button
                onclick={handleSave}
                disabled={isSaving}
                class="flex-1 py-3 bg-[#34251c]/10 border border-[#34251c]/40 hover:bg-[#34251c]/20 text-[#34251c] transition-all text-xs uppercase tracking-wide"
            >
                {isSaving ? '…' : $t('settingsApply')}
            </button>
        </div>
    </div>
  </div>
{/if}
