<script lang="ts">
    import { onMount } from 'svelte';
    import { api } from '$lib/api';
    import type { ServerRelease } from '$lib/types/api';
    import { fade } from 'svelte/transition';

    let releases = $state<ServerRelease[]>([]);
    let isLoading = $state(false);
    let message = $state('');
    let isExporting = $state(false);

    async function loadReleases() {
        isLoading = true;
        try {
            releases = await api.getServerReleases();
        } catch (e) {
            console.error(e);
            showMessage('Ошибка загрузки списка релизов', 'error');
        } finally {
            isLoading = false;
        }
    }

    async function handleExport() {
        if (!confirm('Создать новый релиз на основе локальных данных? Это зафиксирует текущее состояние как новую версию на сервере.')) return;
        
        isExporting = true;
        try {
            await api.exportRelease();
            showMessage('Релиз успешно создан', 'success');
            await loadReleases();
        } catch (e) {
            showMessage('Ошибка экспорта: ' + e, 'error');
        } finally {
            isExporting = false;
        }
    }

    async function handleActivate(release: ServerRelease) {
        if (release.is_active) return;
        if (!confirm(`Активировать версию ${release.version} (${release.description || 'Без описания'})? Пользователи начнут получать этот контент.`)) return;

        try {
            await api.activateServerRelease(release.id);
            showMessage(`Версия ${release.version} активирована`, 'success');
            await loadReleases(); 
        } catch (e) {
            showMessage('Ошибка активации: ' + e, 'error');
        }
    }

    async function handlePull() {
        if (!confirm('Загрузить активную версию с сервера? Локальные изменения могут быть перезаписаны.')) return;
        try {
            const res = await api.pullUpdates();
            showMessage('Синхронизация завершена: ' + res, 'success');
        } catch (e) {
            showMessage('Ошибка синхронизации: ' + e, 'error');
        }
    }

    function formatDate(dateStr: string) {
        try {
            return new Date(dateStr).toLocaleString();
        } catch {
            return dateStr;
        }
    }

    function showMessage(text: string, type: 'success' | 'error' = 'success') {
        message = text;
        setTimeout(() => message = '', 4000);
    }

    onMount(loadReleases);
</script>

<div class="h-full flex flex-col">
    <div class="flex justify-between items-center mb-6">
        <div>
            <h2 class="text-xl font-gothic text-[#d4c5b0]">Хроники Эфира (Релизы)</h2>
            <p class="text-[10px] text-[#8a7f70] uppercase tracking-wider mt-1">Управление версиями на сервере</p>
        </div>
        <div class="flex gap-4 items-center">
            {#if message}
                <span class="text-xs transition-opacity duration-500 text-[#d4c5b0]" in:fade>{message}</span>
            {/if}
            <button onclick={handlePull} class="btn-gothic border-blue-900/40 text-blue-400">
                📥 Скачать активную
            </button>
            <button onclick={handleExport} class="btn-gothic border-amber-900/40 text-amber-600" disabled={isExporting}>
                {isExporting ? 'Создание...' : '📦 Создать релиз'}
            </button>
        </div>
    </div>

    <div class="flex-1 bg-[#141210]/50 border border-[#d4c5b0]/10 overflow-hidden flex flex-col">
        <div class="grid grid-cols-12 bg-[#0a0806] border-b border-[#d4c5b0]/20 p-3 text-[10px] uppercase tracking-widest text-[#8a7f70] font-bold">
            <div class="col-span-1">Ver.</div>
            <div class="col-span-1">Status</div>
            <div class="col-span-3">Описание / ID</div>
            <div class="col-span-3">Дата создания</div>
            <div class="col-span-4 text-right">Действия</div>
        </div>

        <div class="overflow-y-auto flex-1 p-2 space-y-1">
            {#if isLoading}
                <div class="p-4 text-center text-[#8a7f70] text-xs animate-pulse">Загрузка данных из эфира...</div>
            {:else if releases.length === 0}
                <div class="p-10 text-center text-[#8a7f70] text-xs opacity-50">
                    Нет доступных релизов на сервере.<br>
                    Создайте первый релиз, нажав кнопку выше.
                </div>
            {:else}
                {#each releases as release}
                    <div class="grid grid-cols-12 items-center p-3 border border-[#d4c5b0]/5 hover:bg-[#d4c5b0]/5 transition-colors group {release.is_active ? 'bg-[#d4c5b0]/5' : ''}">
                        <div class="col-span-1 text-[#d4c5b0] font-bold">{release.version}</div>
                        <div class="col-span-1">
                            {#if release.isActive}
                                <span class="text-[9px] bg-green-900/30 text-green-400 border border-green-900/50 px-1 py-0.5 rounded">ACTIVE</span>
                            {/if}
                        </div>
                        <div class="col-span-3 text-xs text-[#d4c5b0] truncate pr-2" title={release.id}>
                            <span class="block">{release.description || 'Без описания'}</span>
                            <span class="text-[9px] text-[#8a7f70] font-mono">{release.id.slice(0, 8)}...</span>
                        </div>
                        <div class="col-span-3 text-xs text-[#8a7f70]">{formatDate(release.createdAt)}</div>
                        <div class="col-span-4 flex justify-end gap-2 opacity-60 group-hover:opacity-100 transition-opacity">
                            {#if !release.isActive}
                                <button 
                                    onclick={() => handleActivate(release)}
                                    class="btn-xs border border-[#d4c5b0]/20 hover:bg-[#d4c5b0]/10 text-[#d4c5b0]"
                                    title="Сделать эту версию активной"
                                >
                                    Активировать
                                </button>
                            {/if}
                        </div>
                    </div>
                {/each}
            {/if}
        </div>
    </div>
    
    <div class="mt-4 p-4 border border-[#d4c5b0]/10 bg-[#141210]/30 text-xs text-[#8a7f70]">
        <h4 class="font-bold mb-2 uppercase tracking-widest text-[#d4c5b0]">Справка Смотрителя</h4>
        <ul class="list-disc pl-4 space-y-1">
            <li><strong class="text-[#d4c5b0]">Создать релиз:</strong> Текущее состояние локальной базы (фигурки, зоны, тексты) отправляется на сервер как новая версия.</li>
            <li><strong class="text-[#d4c5b0]">Активировать:</strong> Выбранная версия становится текущей. Все пользователи при синхронизации получат именно её.</li>
            <li><strong class="text-[#d4c5b0]">Скачать активную:</strong> Заменяет вашу локальную базу данных на ту версию, которая сейчас активна на сервере.</li>
        </ul>
    </div>
</div>

<style>
    .btn-gothic {
        padding: 0.5rem 1rem;
        border: 1px solid rgba(212, 197, 176, 0.3);
        font-size: 11px;
        text-transform: uppercase;
        cursor: pointer;
        transition: all 0.2s;
        background: transparent;
    }
    
    .btn-gothic:hover {
        background-color: rgba(212, 197, 176, 0.1);
    }

    .btn-gothic:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }

    .btn-xs {
        padding: 0.25rem 0.75rem;
        font-size: 10px;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        cursor: pointer;
        transition: all 0.2s;
    }
</style>
