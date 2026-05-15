<script lang="ts">
    import { onMount } from 'svelte';
    import { api, isTauri } from '$lib/api';
    import type { CabinetZone } from '$lib/types/api';
    import { fade } from 'svelte/transition';
    import { t } from '$lib/i18n';

    let zones = $state<CabinetZone[]>([]);
    let selectedZone = $state<CabinetZone | null>(null);
    let message = $state('');
    let bgImage = $state('/images/cabinet-room.jpg');

    const zoneTypes = [
        { value: 'showcase', label: 'Showcase', defaultRoute: '/figurines' },
        { value: 'desk',     label: 'Desk',     defaultRoute: '/workshop' },
        { value: 'shelf',    label: 'Shelf',    defaultRoute: '/figurines' },
        { value: 'note',     label: 'Note',     defaultRoute: '/author' },
    ];

    // === Drag/resize state ===
    let canvasEl = $state<HTMLDivElement | null>(null);
    type DragMode = 'move' | 'resize-tl' | 'resize-tr' | 'resize-bl' | 'resize-br' | null;
    let dragMode = $state<DragMode>(null);
    let dragStart = $state({ x: 0, y: 0, zoneX: 0, zoneY: 0, zoneW: 0, zoneH: 0 });

    function getCanvasRelative(clientX: number, clientY: number) {
        if (!canvasEl) return { x: 0, y: 0 };
        const rect = canvasEl.getBoundingClientRect();
        return {
            x: ((clientX - rect.left) / rect.width) * 100,
            y: ((clientY - rect.top) / rect.height) * 100,
        };
    }

    function onCanvasMouseMove(e: MouseEvent) {
        if (!dragMode || !selectedZone) return;
        const pos = getCanvasRelative(e.clientX, e.clientY);
        const dx = pos.x - dragStart.x;
        const dy = pos.y - dragStart.y;

        if (dragMode === 'move') {
            selectedZone.x = Math.max(0, Math.min(100 - selectedZone.width, dragStart.zoneX + dx));
            selectedZone.y = Math.max(0, Math.min(100 - selectedZone.height, dragStart.zoneY + dy));
        } else {
            const minSize = 3;
            if (dragMode === 'resize-br') {
                selectedZone.width  = Math.max(minSize, dragStart.zoneW + dx);
                selectedZone.height = Math.max(minSize, dragStart.zoneH + dy);
            } else if (dragMode === 'resize-bl') {
                const newW = Math.max(minSize, dragStart.zoneW - dx);
                selectedZone.x = dragStart.zoneX + (dragStart.zoneW - newW);
                selectedZone.width  = newW;
                selectedZone.height = Math.max(minSize, dragStart.zoneH + dy);
            } else if (dragMode === 'resize-tr') {
                selectedZone.width  = Math.max(minSize, dragStart.zoneW + dx);
                const newH = Math.max(minSize, dragStart.zoneH - dy);
                selectedZone.y = dragStart.zoneY + (dragStart.zoneH - newH);
                selectedZone.height = newH;
            } else if (dragMode === 'resize-tl') {
                const newW = Math.max(minSize, dragStart.zoneW - dx);
                const newH = Math.max(minSize, dragStart.zoneH - dy);
                selectedZone.x = dragStart.zoneX + (dragStart.zoneW - newW);
                selectedZone.y = dragStart.zoneY + (dragStart.zoneH - newH);
                selectedZone.width  = newW;
                selectedZone.height = newH;
            }
        }
    }

    function onCanvasMouseUp() {
        dragMode = null;
    }

    function startDrag(e: MouseEvent, zone: CabinetZone, mode: DragMode) {
        e.stopPropagation();
        e.preventDefault();
        selectZone(zone);
        const pos = getCanvasRelative(e.clientX, e.clientY);
        dragStart = { x: pos.x, y: pos.y, zoneX: zone.x, zoneY: zone.y, zoneW: zone.width, zoneH: zone.height };
        dragMode = mode;
    }

    async function loadData() {
        const [loadedZones, loadedBg] = await Promise.all([
            api.getCabinetZones(),
            api.getMainBackground()
        ]);
        zones = loadedZones;
        if (loadedBg) bgImage = loadedBg;
    }

    async function changeBackground() {
        try {
            let fileOrPath: string | File;
            if (isTauri) {
                const { open } = await import('@tauri-apps/plugin-dialog');
                const selected = await open({ multiple: false, filters: [{ name: 'Images', extensions: ['jpg', 'png', 'webp'] }] });
                if (!selected || typeof selected !== 'string') return;
                fileOrPath = selected;
            } else {
                fileOrPath = await new Promise<File>((resolve, reject) => {
                    const input = document.createElement('input');
                    input.type = 'file';
                    input.accept = 'image/jpeg,image/png,image/webp';
                    input.onchange = () => {
                        const file = input.files?.[0];
                        if (file) resolve(file); else reject(new Error('no file'));
                    };
                    input.click();
                });
            }
            const url = await api.setMainBackground(fileOrPath as string);
            bgImage = url;
            showMsg($t('adminZoneBgUpdated'));
        } catch (e) {
            if (String(e) !== 'Error: no file') alert(String(e));
        }
    }

    function selectZone(zone: CabinetZone) {
        selectedZone = { ...zone };
    }

    function createNew() {
        selectedZone = {
            id: crypto.randomUUID(),
            zoneType: 'showcase',
            x: 20, y: 20, width: 20, height: 20,
            targetRoute: '/figurines'
        };
    }

    async function save() {
        if (!selectedZone) return;
        try {
            await api.saveCabinetZone(selectedZone);
            await loadData();
            // Re-select the saved zone from fresh list
            const fresh = zones.find(z => z.id === selectedZone?.id);
            if (fresh) selectedZone = { ...fresh };
            showMsg($t('adminZoneSaved'));
        } catch (e) { alert(e); }
    }

    async function remove() {
        if (!selectedZone || !confirm($t('adminZoneDeleteConfirm'))) return;
        try {
            await api.deleteCabinetZone(selectedZone.id);
            selectedZone = null;
            await loadData();
        } catch (e) { alert(e); }
    }

    function showMsg(text: string) {
        message = text;
        setTimeout(() => message = '', 2000);
    }

    onMount(loadData);
</script>

<svelte:window
    onmousemove={onCanvasMouseMove}
    onmouseup={onCanvasMouseUp}
/>

<div class="h-full flex flex-col">
    <div class="flex justify-between items-center mb-4 shrink-0">
        <h2 class="text-xl font-gothic text-[#d4c5b0]">{$t('adminZoneHeading')}</h2>
        <div class="flex gap-3 items-center">
            {#if message}
                <span class="text-green-400 text-xs" in:fade>{message}</span>
            {/if}
            <button onclick={changeBackground} class="btn-gothic border-amber-900/40 text-amber-600">{$t('adminZoneChangeBg')}</button>
            <button onclick={createNew} class="btn-gothic">{$t('adminZoneNew')}</button>
        </div>
    </div>

    <div class="flex-1 grid grid-cols-1 lg:grid-cols-3 gap-5 min-h-0">

        <!-- Visual canvas -->
        <div
            bind:this={canvasEl}
            class="lg:col-span-2 bg-black/50 relative border border-[#d4c5b0]/20 overflow-hidden select-none"
            style="cursor: {dragMode === 'move' ? 'grabbing' : 'default'}"
        >
            <img src={bgImage} alt="Cabinet" class="w-full h-full object-contain pointer-events-none opacity-50" />

            <div class="absolute inset-0">
                <!-- Existing zones -->
                {#each zones as zone (zone.id)}
                    {@const isSelected = selectedZone?.id === zone.id}
                    <div
                        role="button"
                        tabindex="0"
                        onmousedown={(e) => { if (e.button === 0) startDrag(e, zone, 'move'); }}
                        onkeydown={(e) => e.key === 'Enter' && selectZone(zone)}
                        class="absolute flex items-center justify-center group"
                        style="
                            left: {zone.x}%;
                            top: {zone.y}%;
                            width: {zone.width}%;
                            height: {zone.height}%;
                            border: 2px solid {isSelected ? '#f59e0b' : 'rgba(212,197,176,0.3)'};
                            z-index: {isSelected ? 10 : 1};
                            cursor: {isSelected ? 'grab' : 'pointer'};
                            background: {isSelected ? 'rgba(245,158,11,0.08)' : 'transparent'};
                        "
                    >
                        <span class="text-[10px] bg-black/70 px-1 text-[#d4c5b0] opacity-60 group-hover:opacity-100 pointer-events-none select-none">
                            {zone.zoneType}
                        </span>

                        <!-- Resize handles (only for selected) -->
                        {#if isSelected}
                            {#each [['tl','top-0 left-0','-translate-x-1/2 -translate-y-1/2','nw-resize'],
                                    ['tr','top-0 right-0','translate-x-1/2 -translate-y-1/2','ne-resize'],
                                    ['bl','bottom-0 left-0','-translate-x-1/2 translate-y-1/2','sw-resize'],
                                    ['br','bottom-0 right-0','translate-x-1/2 translate-y-1/2','se-resize']] as [corner, pos, tr, cur]}
                                <div
                                    role="button"
                                    tabindex="-1"
                                    class="absolute {pos} w-3 h-3 bg-amber-500 border-2 border-[#0a0806] {tr}"
                                    style="cursor: {cur}; z-index: 20;"
                                    onmousedown={(e) => { if (e.button === 0) startDrag(e, zone, `resize-${corner}` as DragMode); }}
                                ></div>
                            {/each}
                        {/if}
                    </div>
                {/each}

                <!-- Ghost: new unsaved zone -->
                {#if selectedZone && !zones.find(z => z.id === selectedZone?.id)}
                    <div
                        role="button"
                        tabindex="-1"
                        class="absolute border-2 border-dashed border-amber-500/70 bg-amber-500/10"
                        style="left:{selectedZone.x}%;top:{selectedZone.y}%;width:{selectedZone.width}%;height:{selectedZone.height}%;cursor:grab;z-index:10;"
                        onmousedown={(e) => { if (e.button === 0) startDrag(e, selectedZone!, 'move'); }}
                    >
                        <span class="text-[10px] bg-black/70 px-1 text-amber-300 absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 whitespace-nowrap">{$t('adminZoneNewLabel')}</span>
                        <!-- Resize corner for ghost -->
                        <div
                            role="button" tabindex="-1"
                            class="absolute bottom-0 right-0 w-3 h-3 bg-amber-500 border-2 border-[#0a0806] translate-x-1/2 translate-y-1/2"
                            style="cursor:se-resize;z-index:20;"
                            onmousedown={(e) => { if (e.button === 0) startDrag(e, selectedZone!, 'resize-br'); }}
                        ></div>
                    </div>
                {/if}

                <!-- Live preview while dragging (update selected display) -->
                {#if selectedZone && dragMode && zones.find(z => z.id === selectedZone?.id)}
                    <div
                        class="absolute pointer-events-none border-2 border-amber-400 bg-amber-500/15"
                        style="left:{selectedZone.x}%;top:{selectedZone.y}%;width:{selectedZone.width}%;height:{selectedZone.height}%;z-index:15;"
                    ></div>
                {/if}
            </div>

            <div class="absolute bottom-2 left-2 text-[9px] text-[#8a7f70] bg-black/60 px-2 py-1 pointer-events-none">
                {$t('adminZoneDragHint')}
            </div>
        </div>

        <!-- Inspector -->
        <div class="bg-[#141210]/50 p-5 border border-[#d4c5b0]/10 overflow-y-auto">
            {#if selectedZone}
                <div class="space-y-5">
                    <h3 class="font-bold text-[#d4c5b0] border-b border-[#d4c5b0]/20 pb-2 text-sm uppercase tracking-widest">{$t('adminZoneParams')}</h3>

                    <label class="block">
                        <span class="label">{$t('adminZoneType')}</span>
                        <select bind:value={selectedZone.zoneType} class="input-gothic">
                            {#each zoneTypes as zt}
                                <option value={zt.value}>{zt.label}</option>
                            {/each}
                        </select>
                    </label>

                    <label class="block">
                        <span class="label">{$t('adminZoneRoute')}</span>
                        <input bind:value={selectedZone.targetRoute} class="input-gothic" placeholder="/figurines" />
                    </label>

                    <div class="grid grid-cols-2 gap-3">
                        <label class="block">
                            <span class="label">X (%)</span>
                            <input type="number" bind:value={selectedZone.x} class="input-gothic" min="0" max="100" step="0.5" />
                        </label>
                        <label class="block">
                            <span class="label">Y (%)</span>
                            <input type="number" bind:value={selectedZone.y} class="input-gothic" min="0" max="100" step="0.5" />
                        </label>
                        <label class="block">
                            <span class="label">{$t('adminZoneWidth')}</span>
                            <input type="number" bind:value={selectedZone.width} class="input-gothic" min="1" max="100" step="0.5" />
                        </label>
                        <label class="block">
                            <span class="label">{$t('adminZoneHeight')}</span>
                            <input type="number" bind:value={selectedZone.height} class="input-gothic" min="1" max="100" step="0.5" />
                        </label>
                    </div>

                    <!-- Live coordinate readout -->
                    <div class="text-[9px] text-[#8a7f70] bg-[#0a0806] p-2 border border-[#d4c5b0]/10 font-mono">
                        x:{selectedZone.x.toFixed(1)}% y:{selectedZone.y.toFixed(1)}% w:{selectedZone.width.toFixed(1)}% h:{selectedZone.height.toFixed(1)}%
                    </div>

                    <div class="pt-4 flex gap-2">
                        <button onclick={remove} class="btn-gothic border-red-900/40 text-red-500 flex-1">{$t('adminDelete')}</button>
                        <button onclick={save} class="btn-gothic bg-[#d4c5b0]/10 text-[#d4c5b0] flex-1">{$t('adminSave')}</button>
                    </div>
                </div>
            {:else}
                <div class="text-[#8a7f70] text-center mt-10 opacity-40 text-sm">
                    {$t('adminZoneSelectPrompt')}
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
        margin-bottom: 0.4rem;
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
        font-family: inherit;
    }

    .input-gothic:focus { border-color: rgba(212, 197, 176, 0.5); }

    .btn-gothic {
        padding: 0.45rem 1rem;
        border: 1px solid rgba(212, 197, 176, 0.3);
        font-size: 11px;
        text-transform: uppercase;
        cursor: pointer;
        transition: all 0.2s;
        background: transparent;
        color: #d4c5b0;
        font-family: inherit;
    }

    .btn-gothic:hover { background-color: rgba(212, 197, 176, 0.08); }
</style>
