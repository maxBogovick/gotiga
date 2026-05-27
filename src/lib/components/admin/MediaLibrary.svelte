<script lang="ts">
    import { onMount } from 'svelte';
    import { api, isTauri } from '$lib/api';
    import type { MediaCleanupReport, MediaFile, MediaInventory } from '$lib/types/api';

    let {
        onEditFigurine = (_id: string) => {},
    } = $props<{
        onEditFigurine?: (id: string) => void;
    }>();

    let inventory = $state<MediaInventory | null>(null);
    let cleanupReport = $state<MediaCleanupReport | null>(null);
    let selectedPath = $state<string | null>(null);
    let filter = $state<'all' | 'used' | 'orphan' | 'missing'>('all');
    let query = $state('');
    let loading = $state(false);
    let message = $state('');
    let error = $state('');

    let selectedFile = $derived(
        inventory?.files.find(file => file.path === selectedPath) ?? null
    );

    let filteredFiles = $derived(
        inventory?.files.filter(file => {
            if (filter === 'used' && file.usages.length === 0) return false;
            if (filter === 'orphan' && file.usages.length > 0) return false;
            if (filter === 'missing' && file.exists) return false;
            if (!query.trim()) return true;
            const q = query.toLowerCase();
            return file.path.toLowerCase().includes(q)
                || file.mediaType.toLowerCase().includes(q)
                || file.usages.some(usage => usage.label.toLowerCase().includes(q));
        }) ?? []
    );

    function formatSize(bytes: number): string {
        if (bytes < 1024) return `${bytes} B`;
        if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
        return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
    }

    function showMessage(text: string) {
        message = text;
        error = '';
        setTimeout(() => message = '', 3500);
    }

    function showError(text: string) {
        error = text;
        message = '';
    }

    function canOpenFigurine(entityType: string): boolean {
        return entityType === 'figurineImage'
            || entityType === 'processStep'
            || entityType === 'figurine';
    }

    async function loadInventory() {
        loading = true;
        error = '';
        try {
            inventory = await api.getMediaInventory();
            if (selectedPath && !inventory.files.some(file => file.path === selectedPath)) {
                selectedPath = null;
            }
        } catch (e) {
            showError(e instanceof Error ? e.message : String(e));
        } finally {
            loading = false;
        }
    }

    async function buildCleanupReport() {
        loading = true;
        try {
            cleanupReport = await api.getUnusedMediaReport();
            showMessage(`Found ${cleanupReport.files.length} unused file(s), ${formatSize(cleanupReport.totalSizeBytes)}`);
        } catch (e) {
            showError(e instanceof Error ? e.message : String(e));
        } finally {
            loading = false;
        }
    }

    async function cleanupUnused() {
        if (!cleanupReport || cleanupReport.files.length === 0) return;
        if (!confirm(`Delete ${cleanupReport.files.length} unused file(s)? Used files are protected.`)) return;
        loading = true;
        try {
            const removed = await api.cleanupReportedUnusedMedia();
            cleanupReport = null;
            await loadInventory();
            showMessage(`Removed ${removed.length} unused file(s)`);
        } catch (e) {
            showError(e instanceof Error ? e.message : String(e));
        } finally {
            loading = false;
        }
    }

    async function replaceSelected() {
        if (!selectedFile) return;
        if (selectedFile.usages.length === 0) {
            showError('This file is not used anywhere. Delete it via cleanup instead.');
            return;
        }
        try {
            const replacement = isTauri
                ? await pickReplacementInTauri(selectedFile)
                : await pickReplacementInWeb(selectedFile);
            if (!replacement) return;
            loading = true;
            const result = await api.replaceMediaEverywhere(selectedFile.path, replacement);
            selectedPath = result.newPath;
            cleanupReport = null;
            await loadInventory();
            showMessage(`Replaced ${result.updatedReferences} reference(s) with ${result.newPath}`);
        } catch (e) {
            showError(e instanceof Error ? e.message : String(e));
        } finally {
            loading = false;
        }
    }

    async function pickReplacementInTauri(file: MediaFile): Promise<string | null> {
        const { open } = await import('@tauri-apps/plugin-dialog');
        const filters = file.mediaType === 'image'
            ? [{ name: 'Images', extensions: ['jpg', 'jpeg', 'png', 'webp'] }]
            : file.mediaType === 'video'
                ? [{ name: 'Videos', extensions: ['mp4', 'webm', 'mov'] }]
                : file.mediaType === 'audio'
                    ? [{ name: 'Audio', extensions: ['mp3', 'wav', 'ogg', 'm4a'] }]
                    : [];
        const replacement = await open({ multiple: false, filters });
        return typeof replacement === 'string' ? replacement : null;
    }

    function pickReplacementInWeb(file: MediaFile): Promise<File | null> {
        return new Promise(resolve => {
            const input = document.createElement('input');
            input.type = 'file';
            if (file.mediaType === 'image') input.accept = 'image/jpeg,image/png,image/webp';
            else if (file.mediaType === 'video') input.accept = 'video/mp4,video/webm,video/quicktime';
            else if (file.mediaType === 'audio') input.accept = 'audio/mpeg,audio/wav,audio/ogg,audio/mp4';
            input.onchange = () => resolve(input.files?.[0] ?? null);
            input.click();
        });
    }

    onMount(loadInventory);
</script>

<div class="h-full flex flex-col gap-4">
    <div class="flex items-center justify-between gap-4 shrink-0">
        <div>
            <h2 class="text-xl font-gothic">Media Library</h2>
            <p class="text-[10px] uppercase tracking-wide text-[#5f4636] mt-1">
                {inventory ? `${inventory.files.length} files / ${inventory.usedCount} used / ${inventory.orphanCount} orphan / ${formatSize(inventory.totalSizeBytes)}` : 'Scanning media'}
            </p>
        </div>
        <div class="flex items-center gap-2">
            <button onclick={loadInventory} disabled={loading} class="btn-gothic text-[10px] disabled:opacity-70">Refresh</button>
            <button onclick={buildCleanupReport} disabled={loading} class="btn-gothic text-[10px] disabled:opacity-70">Preview cleanup</button>
            <button onclick={cleanupUnused} disabled={loading || !cleanupReport || cleanupReport.files.length === 0} class="btn-gothic text-[10px] border-red-900/40 text-red-700 disabled:opacity-70">Delete unused</button>
        </div>
    </div>

    {#if message}
        <div class="border border-emerald-700/30 bg-emerald-50 px-4 py-2 text-xs text-emerald-800">{message}</div>
    {/if}
    {#if error}
        <div class="border border-red-700/30 bg-red-50 px-4 py-2 text-xs text-red-800">{error}</div>
    {/if}

    <div class="flex gap-2 shrink-0">
        <input bind:value={query} class="input-gothic text-xs max-w-sm" placeholder="Search path or usage" />
        {#each ['all', 'used', 'orphan', 'missing'] as item}
            <button
                onclick={() => filter = item as typeof filter}
                class="px-3 py-2 border text-[10px] uppercase tracking-wide transition-colors
                    {filter === item ? 'border-[#34251c]/50 bg-[#c65f3c]/12 text-[#34251c]' : 'border-[#34251c]/15 text-[#5f4636] hover:text-[#34251c]'}"
            >{item}</button>
        {/each}
    </div>

    {#if cleanupReport}
        <div class="border border-[#34251c]/15 px-4 py-3 shrink-0">
            <div class="flex items-center justify-between gap-3">
                <div>
                    <div class="text-xs uppercase tracking-wide text-[#34251c]">Cleanup report before deletion</div>
                    <div class="text-[10px] text-[#5f4636] mt-1">{cleanupReport.files.length} orphan file(s), {formatSize(cleanupReport.totalSizeBytes)}</div>
                </div>
                <button onclick={() => cleanupReport = null} class="text-[10px] uppercase text-[#5f4636] hover:text-[#34251c]">Dismiss</button>
            </div>
            {#if cleanupReport.files.length > 0}
                <div class="mt-3 max-h-24 overflow-y-auto text-[10px] text-[#5f4636] font-mono space-y-1">
                    {#each cleanupReport.files as file}
                        <div>{file.path} ({formatSize(file.sizeBytes)})</div>
                    {/each}
                </div>
            {/if}
        </div>
    {/if}

    <div class="grid grid-cols-[minmax(0,1fr)_320px] gap-4 min-h-0 flex-1">
        <div class="border border-[#34251c]/10 overflow-hidden min-h-0">
            <div class="grid grid-cols-[76px_minmax(0,1fr)_90px_90px_88px] gap-3 px-3 py-2 border-b border-[#34251c]/10 text-[9px] uppercase tracking-wide text-[#5f4636]">
                <span>Preview</span>
                <span>Path</span>
                <span>Variant</span>
                <span>Size</span>
                <span>Usage</span>
            </div>
            <div class="h-full overflow-y-auto pb-10">
                {#each filteredFiles as file (file.path)}
                    <button
                        onclick={() => selectedPath = file.path}
                        class="grid grid-cols-[76px_minmax(0,1fr)_90px_90px_88px] gap-3 items-center w-full text-left px-3 py-2 border-b border-[#34251c]/8 transition-colors
                            {selectedPath === file.path ? 'bg-[#34251c]/10' : 'hover:bg-[#34251c]/5'}"
                    >
                        <div class="w-14 h-14 border border-[#34251c]/10 bg-[#f8f1e7] overflow-hidden flex items-center justify-center">
                            {#if file.mediaType === 'image' && file.exists}
                                <img src={file.url} alt="" class="w-full h-full object-cover" />
                            {:else}
                                <span class="text-[9px] uppercase text-[#5f4636]">{file.mediaType}</span>
                            {/if}
                        </div>
                        <div class="min-w-0">
                            <div class="text-xs text-[#34251c] font-mono truncate">{file.path}</div>
                            {#if !file.exists}
                                <div class="text-[10px] text-red-700 uppercase mt-1">Missing on disk</div>
                            {/if}
                        </div>
                        <div class="text-[10px] text-[#5f4636] uppercase">{file.variant ?? file.mediaType}</div>
                        <div class="text-[10px] text-[#5f4636]">{formatSize(file.sizeBytes)}</div>
                        <div class="text-[10px] {file.usages.length ? 'text-emerald-700' : 'text-amber-700'}">
                            {file.usages.length ? `${file.usages.length} used` : 'orphan'}
                        </div>
                    </button>
                {/each}
                {#if filteredFiles.length === 0}
                    <div class="p-8 text-center text-xs text-[#5f4636]">No media files match this filter.</div>
                {/if}
            </div>
        </div>

        <aside class="border border-[#34251c]/10 p-4 min-h-0 overflow-y-auto">
            {#if selectedFile}
                <div class="space-y-4">
                    <div>
                        <div class="text-[10px] uppercase tracking-wide text-[#5f4636] mb-2">Selected file</div>
                        <div class="text-xs font-mono break-all text-[#34251c]">{selectedFile.path}</div>
                    </div>

                    {#if selectedFile.mediaType === 'image' && selectedFile.exists}
                        <img src={selectedFile.url} alt="" class="w-full aspect-square object-cover border border-[#34251c]/15" />
                    {/if}

                    <div class="grid grid-cols-2 gap-2 text-[10px] text-[#5f4636]">
                        <div>Type</div><div class="text-[#34251c]">{selectedFile.mediaType}</div>
                        <div>Variant</div><div class="text-[#34251c]">{selectedFile.variant ?? '-'}</div>
                        <div>Size</div><div class="text-[#34251c]">{formatSize(selectedFile.sizeBytes)}</div>
                        <div>Status</div><div class={selectedFile.exists ? 'text-emerald-700' : 'text-red-700'}>{selectedFile.exists ? 'Exists' : 'Missing'}</div>
                    </div>

                    <div>
                        <div class="text-[10px] uppercase tracking-wide text-[#5f4636] mb-2">Used in</div>
                        {#if selectedFile.usages.length}
                            <div class="space-y-2">
                                {#each selectedFile.usages as usage}
                                    <button
                                        type="button"
                                        onclick={() => canOpenFigurine(usage.entityType) && onEditFigurine(usage.entityId)}
                                        disabled={!canOpenFigurine(usage.entityType)}
                                        class="w-full text-left border border-[#34251c]/10 p-2 transition-colors
                                            {canOpenFigurine(usage.entityType) ? 'hover:border-[#34251c]/35 hover:bg-[#34251c]/5 cursor-pointer' : 'cursor-default'}"
                                        title={canOpenFigurine(usage.entityType) ? 'Open card editor' : ''}
                                    >
                                        <div class="text-xs text-[#34251c]">{usage.label}</div>
                                        <div class="text-[10px] text-[#5f4636] mt-1">{usage.entityType} / {usage.field}</div>
                                    </button>
                                {/each}
                            </div>
                        {:else}
                            <div class="text-xs text-amber-700">Not used anywhere. Cleanup can delete it.</div>
                        {/if}
                    </div>

                    <button
                        onclick={replaceSelected}
                        disabled={loading || selectedFile.usages.length === 0 || !selectedFile.exists}
                        class="btn-gothic w-full text-xs disabled:opacity-70"
                    >Replace file everywhere</button>

                    {#if selectedFile.usages.length > 0}
                        <div class="text-[10px] leading-relaxed text-[#5f4636]">
                            Used files are protected from cleanup. Remove references in the editor or use replacement instead.
                        </div>
                    {/if}
                </div>
            {:else}
                <div class="h-full flex items-center justify-center text-xs text-[#5f4636] text-center">
                    Select a file to inspect usages and replacement options.
                </div>
            {/if}
        </aside>
    </div>
</div>
