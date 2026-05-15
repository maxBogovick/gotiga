<script lang="ts">
    import { onMount } from 'svelte';
    import { api, isTauri } from '$lib/api';
    import type { AuthorText, WorkshopItem } from '$lib/types/api';
    import { fade } from 'svelte/transition';
    import { t } from '$lib/i18n';

    // Props
    let { category } = $props<{ category: 'author' | 'workshop' }>();

    let items = $state<(AuthorText | WorkshopItem)[]>([]);
    let selectedItem = $state<AuthorText | WorkshopItem | null>(null);
    let message = $state('');

    async function loadItems() {
        if (category === 'author') {
            items = await api.getAuthorTexts();
        } else {
            items = await api.getWorkshopContent();
        }
    }

    function createNew() {
        selectedItem = {
            id: crypto.randomUUID(),
            content: '',
            caption: '',
            imageUrl: ''
        } as WorkshopItem; // Cast for simplicity, AuthorText is subset
    }

    function edit(item: AuthorText | WorkshopItem) {
        // Clone
        selectedItem = JSON.parse(JSON.stringify(item));
    }

    async function handlePickImage() {
        if (!selectedItem) return;
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
                        if (file) resolve(file); else reject(new Error('no file'));
                    };
                    input.click();
                });
            }
            const url = await api.importMedia(fileOrPath, 'images');
            (selectedItem as WorkshopItem).imageUrl = url;
        } catch (e) {
            if (String(e) !== 'Error: no file') alert(String(e));
        }
    }

    async function save() {
        if (!selectedItem) return;
        try {
            await api.saveText(selectedItem, category);
            await loadItems();
            selectedItem = null;
            message = $t('adminTextSaved');
            setTimeout(() => message = '', 2000);
        } catch (e) {
            alert(e);
        }
    }

    async function remove(id: string) {
        if (!confirm($t('adminTextDeleteConfirm'))) return;
        try {
            await api.deleteText(id);
            if (selectedItem?.id === id) selectedItem = null;
            await loadItems();
        } catch (e) {
            alert(e);
        }
    }

    function resolveUrl(url: string | null) {
        return url || '';
    }

    $effect(() => {
        // Reload when category changes
        loadItems();
        selectedItem = null;
    });

    onMount(loadItems);
</script>

<div class="h-full flex gap-6">
    <!-- List -->
    <div class="w-1/3 border-r border-[#d4c5b0]/10 pr-6 flex flex-col">
        <div class="flex justify-between items-center mb-4">
            <h2 class="text-xl font-gothic text-[#d4c5b0]">
                {category === 'author' ? $t('adminTextAuthorCategory') : $t('adminTextWorkshopCategory')}
            </h2>
            <button onclick={createNew} class="btn-gothic text-xs">{$t('adminTextAdd')}</button>
        </div>

        <div class="flex-1 overflow-y-auto space-y-2">
            {#each items as item}
                <div class="p-3 border border-[#d4c5b0]/10 hover:border-[#d4c5b0]/40 bg-[#141210]/30 group relative transition-colors">
                    <button onclick={() => edit(item)} class="w-full text-left pr-6">
                        <div class="text-sm text-[#d4c5b0] line-clamp-2 italic">
                            "{item.content || '...'}"
                        </div>
                        {#if (item as WorkshopItem).caption}
                            <div class="text-[10px] text-[#8a7f70] mt-1">
                                {(item as WorkshopItem).caption}
                            </div>
                        {/if}
                    </button>
                    <button 
                        onclick={() => remove(item.id)}
                        class="absolute top-2 right-2 text-[#8a7f70] hover:text-red-500 opacity-0 group-hover:opacity-100 transition-opacity"
                    >✕</button>
                </div>
            {/each}
        </div>
    </div>

    <!-- Editor -->
    <div class="flex-1">
        {#if selectedItem}
            <div class="bg-[#141210]/50 p-6 border border-[#d4c5b0]/10 h-full flex flex-col" in:fade>
                <h3 class="font-bold text-[#d4c5b0] border-b border-[#d4c5b0]/20 pb-4 mb-6">{$t('adminTextEditing')}</h3>

                <div class="space-y-6 flex-1">
                    <label class="block">
                        <span class="label">{$t('adminTextContent')}</span>
                        <textarea bind:value={selectedItem.content} class="input-gothic h-32 italic"></textarea>
                    </label>

                    {#if category === 'workshop'}
                        <label class="block">
                            <span class="label">{$t('adminTextCaption')}</span>
                            <input bind:value={(selectedItem as WorkshopItem).caption} class="input-gothic" />
                        </label>

                        <div>
                            <span class="label block mb-2">{$t('adminTextImage')}</span>
                            <div class="flex gap-4 items-start">
                                <div class="w-32 h-32 bg-black border border-[#d4c5b0]/20 flex items-center justify-center overflow-hidden">
                                    {#if (selectedItem as WorkshopItem).imageUrl}
                                        <img src={resolveUrl((selectedItem as WorkshopItem).imageUrl)} alt="" class="w-full h-full object-cover" />
                                    {:else}
                                        <span class="text-[#8a7f70] text-xs">{$t('adminTextNoPhoto')}</span>
                                    {/if}
                                </div>
                                <div class="space-y-2">
                                    <button onclick={handlePickImage} class="btn-gothic w-full">{$t('adminTextUploadPhoto')}</button>
                                    {#if (selectedItem as WorkshopItem).imageUrl}
                                        <button
                                            onclick={() => (selectedItem as WorkshopItem).imageUrl = null}
                                            class="text-xs text-red-500 hover:underline block text-center"
                                        >
                                            {$t('adminTextRemovePhoto')}
                                        </button>
                                    {/if}
                                </div>
                            </div>
                        </div>
                    {/if}
                </div>

                <div class="pt-6 flex justify-end gap-4">
                    <span class="text-green-500 text-xs self-center transition-opacity duration-500" class:opacity-0={!message}>{message}</span>
                    <button onclick={save} class="btn-gothic bg-[#d4c5b0]/10 text-[#d4c5b0] min-w-[120px]">{$t('adminTextSave')}</button>
                </div>
            </div>
        {:else}
            <div class="h-full flex items-center justify-center text-[#8a7f70] opacity-30">
                {$t('adminTextSelectPrompt')}
            </div>
        {/if}
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
        padding: 0.75rem;
        font-size: 0.875rem;
        color: #d4c5b0;
        outline: none;
        transition: all 0.2s;
    }
    
    .input-gothic:focus {
        border-color: rgba(212, 197, 176, 0.6);
    }

    .btn-gothic {
        padding: 0.5rem 1rem;
        border: 1px solid rgba(212, 197, 176, 0.3);
        font-size: 11px;
        text-transform: uppercase;
        cursor: pointer;
        transition: all 0.2s;
    }
    
    .btn-gothic:hover {
        background-color: rgba(212, 197, 176, 0.1);
    }
</style>
