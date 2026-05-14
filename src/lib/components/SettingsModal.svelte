<script lang="ts">
  import { fade, scale } from 'svelte/transition';
  import { api, type AppSettings } from '$lib/api';

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
</script>

{#if isOpen}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4">
    <div class="absolute inset-0 bg-black/80 backdrop-blur-sm" transition:fade onclick={onClose}></div>
    
    <div 
      class="relative bg-[#0c0a08] border border-[#d4c5b0]/30 w-full max-w-md p-8 shadow-[0_0_50px_rgba(0,0,0,0.8)] font-cinzel text-[#d4c5b0]"
      transition:scale={{ start: 0.95 }}
    >
        <div class="absolute top-0 left-0 w-4 h-4 border-t border-l border-[#d4c5b0]/50"></div>
        <div class="absolute top-0 right-0 w-4 h-4 border-t border-r border-[#d4c5b0]/50"></div>
        <div class="absolute bottom-0 left-0 w-4 h-4 border-b border-l border-[#d4c5b0]/50"></div>
        <div class="absolute bottom-0 right-0 w-4 h-4 border-b border-r border-[#d4c5b0]/50"></div>

        <h2 class="text-2xl font-bold mb-2 text-center text-[#e6decb] tracking-wider uppercase">Настройки Связи</h2>
        <div class="w-full h-px bg-gradient-to-r from-transparent via-[#d4c5b0]/30 to-transparent mb-8"></div>

        <div class="space-y-6">
            <label class="block">
                <span class="text-xs uppercase tracking-widest text-[#8a7f70] block mb-2">Адрес Сервера</span>
                <input 
                    bind:value={settings.serverUrl}
                    type="text" 
                    placeholder="https://api.gotiga.com"
                    class="w-full bg-[#141210] border border-[#d4c5b0]/20 p-3 text-sm focus:border-[#d4c5b0]/60 outline-none transition-colors"
                />
            </label>

            <label class="block">
                <span class="text-xs uppercase tracking-widest text-[#8a7f70] block mb-2">Ключ Доступа (API Key)</span>
                <input
                    bind:value={settings.apiKey}
                    type="password"
                    placeholder="••••••••••••••••"
                    class="w-full bg-[#141210] border border-[#d4c5b0]/20 p-3 text-sm focus:border-[#d4c5b0]/60 outline-none transition-colors"
                />
            </label>

            <label class="block">
                <span class="text-xs uppercase tracking-widest text-[#8a7f70] block mb-2">Email для заявок</span>
                <input
                    bind:value={contactEmail}
                    type="email"
                    placeholder="info@gotiga.art"
                    class="w-full bg-[#141210] border border-[#d4c5b0]/20 p-3 text-sm focus:border-[#d4c5b0]/60 outline-none transition-colors"
                />
            </label>

            <div class="flex items-center gap-3">
                <button
                    onclick={testConnection}
                    disabled={isTesting}
                    class="flex-1 py-2 border text-xs uppercase tracking-widest transition-all
                        {testStatus === 'ok' ? 'border-green-700/50 text-green-400 bg-green-900/10' :
                         testStatus === 'fail' ? 'border-red-700/50 text-red-400 bg-red-900/10' :
                         'border-[#d4c5b0]/20 text-[#8a7f70] hover:text-[#d4c5b0] hover:bg-[#d4c5b0]/5'}"
                >
                    {#if isTesting}Проверка...
                    {:else if testStatus === 'ok'}✓ Связь установлена
                    {:else if testStatus === 'fail'}✗ Нет ответа
                    {:else}Проверить соединение{/if}
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
                class="flex-1 py-3 border border-[#d4c5b0]/20 text-[#8a7f70] hover:text-[#d4c5b0] hover:bg-[#d4c5b0]/5 transition-all text-xs uppercase tracking-widest"
            >
                Отмена
            </button>
            <button 
                onclick={handleSave}
                disabled={isSaving}
                class="flex-1 py-3 bg-[#d4c5b0]/10 border border-[#d4c5b0]/40 hover:bg-[#d4c5b0]/20 text-[#d4c5b0] transition-all text-xs uppercase tracking-widest"
            >
                {isSaving ? 'Сохранение...' : 'Применить'}
            </button>
        </div>
    </div>
  </div>
{/if}
