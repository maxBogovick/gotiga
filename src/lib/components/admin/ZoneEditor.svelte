<script lang="ts">
    import { onMount } from 'svelte';
    import { api, isTauri } from '$lib/api';
    import type { CabinetZone } from '$lib/types/api';
    import { fade } from 'svelte/transition';

    let zones = $state<CabinetZone[]>([]);
    let selectedZone = $state<CabinetZone | null>(null);
    let message = $state('');
    let bgImage = $state('/images/cabinet-room.jpg');

    const zoneTypes = [
        { value: 'showcase', label: 'Витрина', defaultRoute: '/figurines' },
        { value: 'desk', label: 'Стол', defaultRoute: '/workshop' },
        { value: 'shelf', label: 'Полка', defaultRoute: '/figurines' },
        { value: 'note', label: 'Записка', defaultRoute: '/author' },
    ];

    function resolveUrl(path: string | null) {
        return path ?? '';
    }

    async function loadData() {
        const [loadedZones, loadedBg] = await Promise.all([
            api.getCabinetZones(),
            api.getMainBackground()
        ]);
        zones = loadedZones;
        if (loadedBg) {
            bgImage = resolveUrl(loadedBg);
        }
    }

    async function changeBackground() {
        try {
            let fileOrPath: string | File;
            if (isTauri) {
                const { open } = await import('@tauri-apps/plugin-dialog');
                const selected = await open({
                    multiple: false,
                    filters: [{ name: 'Images', extensions: ['jpg', 'png', 'webp'] }]
                });
                if (!selected || typeof selected !== 'string') return;
                fileOrPath = selected;
            } else {
                fileOrPath = await new Promise<File>((resolve, reject) => {
                    const input = document.createElement('input');
                    input.type = 'file';
                    input.accept = 'image/jpeg,image/png,image/webp';
                    input.onchange = () => {
                        const file = input.files?.[0];
                        if (file) resolve(file); else reject(new Error('Файл не выбран'));
                    };
                    input.click();
                });
            }
            const url = await api.setMainBackground(fileOrPath as string);
            bgImage = resolveUrl(url);
            message = 'Фон обновлен';
            setTimeout(() => message = '', 2000);
        } catch (e) {
            if (String(e) !== 'Error: Файл не выбран') alert('Ошибка загрузки фона: ' + e);
        }
    }

    function selectZone(zone: CabinetZone) {
        // Clone to avoid immediate mutation
        selectedZone = { ...zone };
    }

    function createNew() {
        selectedZone = {
            id: crypto.randomUUID(),
            zoneType: 'showcase',
            x: 10,
            y: 10,
            width: 20,
            height: 20,
            targetRoute: '/figurines'
        };
    }

    async function save() {
        if (!selectedZone) return;
        try {
            await api.saveCabinetZone(selectedZone);
            await loadData(); // Reload zones
            message = 'Зона сохранена';
            setTimeout(() => message = '', 2000);
        } catch (e) {
            alert(e);
        }
    }

    async function remove() {
        if (!selectedZone || !confirm('Удалить зону?')) return;
        try {
            await api.deleteCabinetZone(selectedZone.id);
            selectedZone = null;
            await loadData();
        } catch (e) {
            alert(e);
        }
    }

    onMount(loadData);
</script>

<div class="h-full flex flex-col">
    <div class="flex justify-between items-center mb-6">
        <h2 class="text-xl font-gothic text-[#d4c5b0]">Картография (Зоны)</h2>
        <div class="flex gap-4 items-center">
            {#if message}
                <span class="text-green-500 text-xs" in:fade>{message}</span>
            {/if}
            <button onclick={changeBackground} class="btn-gothic border-amber-900/40 text-amber-600">🖼 Изменить фон</button>
            <button onclick={createNew} class="btn-gothic">✚ Новая зона</button>
        </div>
    </div>

    <div class="flex-1 grid grid-cols-1 lg:grid-cols-3 gap-6 min-h-0">
        <!-- Visual Editor -->
        <div class="lg:col-span-2 bg-black/50 relative border border-[#d4c5b0]/20 overflow-hidden select-none">
            <img 
                src={bgImage} 
                alt="Cabinet" 
                class="w-full h-full object-contain pointer-events-none opacity-50" 
            />
            
            <!-- Overlay Zones -->
            <div class="absolute inset-0">
                {#each zones as zone}
                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                    <div 
                        role="button"
                        tabindex="0"
                        onclick={() => selectZone(zone)}
                        class="absolute border-2 cursor-pointer transition-all hover:bg-[#d4c5b0]/10 flex items-center justify-center group"
                        style="
                            left: {zone.x}%; 
                            top: {zone.y}%; 
                            width: {zone.width}%; 
                            height: {zone.height}%;
                            border-color: {selectedZone?.id === zone.id ? '#f59e0b' : 'rgba(212, 197, 176, 0.3)'};
                            z-index: {selectedZone?.id === zone.id ? 10 : 1};
                        "
                    >
                        <span class="text-[10px] bg-black/70 px-1 text-[#d4c5b0] opacity-50 group-hover:opacity-100">
                            {zone.zoneType}
                        </span>
                    </div>
                {/each}

                <!-- Active Edit Preview (Ghost) -->
                {#if selectedZone && !zones.find(z => z.id === selectedZone?.id)}
                     <div 
                        class="absolute border-2 border-amber-500/50 bg-amber-500/10"
                        style="
                            left: {selectedZone.x}%; 
                            top: {selectedZone.y}%; 
                            width: {selectedZone.width}%; 
                            height: {selectedZone.height}%;
                        "
                    ></div>
                {/if}
            </div>
        </div>

        <!-- Inspector -->
        <div class="bg-[#141210]/50 p-6 border border-[#d4c5b0]/10 overflow-y-auto">
            {#if selectedZone}
                <div class="space-y-6">
                    <h3 class="font-bold text-[#d4c5b0] border-b border-[#d4c5b0]/20 pb-2">Параметры</h3>
                    
                    <label class="block">
                        <span class="label">Тип зоны</span>
                        <select bind:value={selectedZone.zoneType} class="input-gothic">
                            {#each zoneTypes as t}
                                <option value={t.value}>{t.label}</option>
                            {/each}
                        </select>
                    </label>

                    <label class="block">
                        <span class="label">Целевой маршрут</span>
                        <input bind:value={selectedZone.targetRoute} class="input-gothic" />
                    </label>

                    <div class="grid grid-cols-2 gap-4">
                        <label class="block">
                            <span class="label">X (%)</span>
                            <input type="number" bind:value={selectedZone.x} class="input-gothic" min="0" max="100" />
                        </label>
                        <label class="block">
                            <span class="label">Y (%)</span>
                            <input type="number" bind:value={selectedZone.y} class="input-gothic" min="0" max="100" />
                        </label>
                        <label class="block">
                            <span class="label">Ширина (%)</span>
                            <input type="number" bind:value={selectedZone.width} class="input-gothic" min="0" max="100" />
                        </label>
                        <label class="block">
                            <span class="label">Высота (%)</span>
                            <input type="number" bind:value={selectedZone.height} class="input-gothic" min="0" max="100" />
                        </label>
                    </div>

                    <div class="pt-6 flex gap-2">
                        <button onclick={remove} class="btn-gothic border-red-900/40 text-red-500 flex-1">Удалить</button>
                        <button onclick={save} class="btn-gothic bg-[#d4c5b0]/10 text-[#d4c5b0] flex-1">Сохранить</button>
                    </div>
                </div>
            {:else}
                <div class="text-[#8a7f70] text-center mt-10 opacity-50">
                    Выберите зону или создайте новую
                </div>
            {/if}
        </div>
    </div>
</div>

<style>
    .label {
        font-size: 10px;
        text-transform: uppercase;
        letter-spacing: 0.1em;
        color: #8a7f70;
        margin-bottom: 0.5rem;
        display: block;
        font-weight: 700;
    }

    .input-gothic {
        width: 100%;
        background-color: #0a0806;
        border: 1px solid rgba(212, 197, 176, 0.2);
        padding: 0.5rem;
        font-size: 0.875rem;
        color: #d4c5b0;
        outline: none;
    }

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
</style>
