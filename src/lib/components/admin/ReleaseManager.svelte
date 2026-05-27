<script lang="ts">
    import { onMount } from 'svelte';
    import { api, isTauri } from '$lib/api';
    import type { ServerRelease } from '$lib/types/api';
    import { fade } from 'svelte/transition';
    import { t } from '$lib/i18n';

    let releases = $state<ServerRelease[]>([]);
    let isLoading = $state(false);
    let message = $state('');
    let messageType = $state<'success' | 'error'>('success');
    let isExporting = $state(false);

    async function loadReleases() {
        isLoading = true;
        try {
            releases = await api.getServerReleases();
        } catch (e) {
            console.error(e);
            showMessage('Error loading releases: ' + e, 'error');
        } finally {
            isLoading = false;
        }
    }

    async function handleExport() {
        if (!confirm('Create a new release from local data? This will snapshot the current state as a new version on the server.')) return;

        isExporting = true;
        try {
            await api.exportRelease();
            showMessage('Release created', 'success');
            await loadReleases();
        } catch (e) {
            showMessage('Export error: ' + e, 'error');
        } finally {
            isExporting = false;
        }
    }

    async function handleActivate(release: ServerRelease) {
        if (release.isActive) return;
        if (!confirm(`Activate version ${release.version} (${release.description || 'No description'})? Users will start receiving this content.`)) return;

        try {
            await api.activateServerRelease(release.id);
            showMessage(`Version ${release.version} activated`, 'success');
            await loadReleases();
        } catch (e) {
            showMessage('Activation error: ' + e, 'error');
        }
    }

    async function handlePull() {
        if (!confirm('Download the active version from server? Local changes may be overwritten.')) return;
        try {
            const res = await api.pullUpdates();
            showMessage('Sync complete: ' + res, 'success');
        } catch (e) {
            showMessage('Sync error: ' + e, 'error');
        }
    }

    async function handleCleanupMedia() {
        if (!confirm('Remove local media files that are no longer referenced by the database?')) return;
        try {
            const removed = await api.cleanupUnusedMedia();
            showMessage(`Removed ${removed.length} unused media file(s)`, 'success');
        } catch (e) {
            showMessage('Cleanup error: ' + e, 'error');
        }
    }

    async function handleUpload(e: Event) {
        const file = (e.target as HTMLInputElement).files?.[0];
        if (!file) return;
        isExporting = true;
        try {
            const form = new FormData();
            form.append('file', file);
            const { serverUrl, apiKey } = (await api.getSettings());
            const base = serverUrl ? `${serverUrl}/api/v1` : '/api/v1';
            const headers: Record<string, string> = apiKey ? { Authorization: `Bearer ${apiKey}` } : {};
            const res = await fetch(`${base}/admin/releases`, { method: 'POST', headers, body: form });
            if (!res.ok) throw new Error(`${res.status}: ${await res.text().catch(() => '')}`);
            showMessage('Release uploaded and activated', 'success');
            await loadReleases();
        } catch (e) {
            showMessage('Upload error: ' + e, 'error');
        } finally {
            isExporting = false;
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
        messageType = type;
        setTimeout(() => message = '', 4000);
    }

    onMount(loadReleases);
</script>

<div class="h-full flex flex-col">
    <div class="flex justify-between items-center mb-6">
        <div>
            <h2 class="text-xl font-gothic text-[#34251c]">{$t('adminReleaseHeading')}</h2>
            <p class="text-[10px] text-[#5f4636] uppercase tracking-wider mt-1">Version management</p>
        </div>
        <div class="flex gap-4 items-center">
            {#if message}
                <span class="text-xs transition-opacity duration-500 {messageType === 'error' ? 'text-red-700' : 'text-[#34251c]'}" in:fade>{message}</span>
            {/if}
            {#if isTauri}
                <button onclick={handleCleanupMedia} class="btn-gothic border-red-900/40 text-red-700">
                    Cleanup media
                </button>
                <button onclick={handlePull} class="btn-gothic border-blue-900/40 text-blue-700">
                    {$t('adminReleaseDownloadActive')}
                </button>
                <button onclick={handleExport} class="btn-gothic border-amber-900/40 text-amber-600" disabled={isExporting}>
                    {isExporting ? $t('adminReleaseCreating') : $t('adminReleaseCreate')}
                </button>
            {:else}
                <span class="text-[10px] text-[#5f4636] italic">Upload a .db file via the button below</span>
                <label class="btn-gothic border-amber-900/40 text-amber-600 cursor-pointer">
                    {$t('adminReleaseUpload')}
                    <input type="file" accept=".db,.sqlite" class="hidden" onchange={handleUpload} />
                </label>
            {/if}
        </div>
    </div>

    <div class="flex-1 bg-[#fff9f0]/50 border border-[#34251c]/10 overflow-hidden flex flex-col">
        <div class="grid grid-cols-12 bg-[#f8f1e7] border-b border-[#34251c]/20 p-3 text-[10px] uppercase tracking-wide text-[#5f4636] font-bold">
            <div class="col-span-1">Ver.</div>
            <div class="col-span-1">Status</div>
            <div class="col-span-3">Description / ID</div>
            <div class="col-span-3">Created</div>
            <div class="col-span-4 text-right">Actions</div>
        </div>

        <div class="overflow-y-auto flex-1 p-2 space-y-1">
            {#if isLoading}
                <div class="p-4 text-center text-[#5f4636] text-xs animate-pulse">{$t('adminLoading')}</div>
            {:else if releases.length === 0}
                <div class="p-10 text-center text-[#5f4636] text-xs opacity-75">
                    {$t('adminReleaseEmpty')}<br>
                    {$t('adminReleaseEmptyHint')}
                </div>
            {:else}
                {#each releases as release}
                    <div class="grid grid-cols-12 items-center p-3 border border-[#34251c]/5 hover:bg-[#34251c]/5 transition-colors group {release.isActive ? 'bg-[#34251c]/5' : ''}">
                        <div class="col-span-1 text-[#34251c] font-bold">{release.version}</div>
                        <div class="col-span-1">
                            {#if release.isActive}
                                <span class="text-[9px] bg-green-50 text-green-800 border border-green-700/30 px-1 py-0.5 rounded">ACTIVE</span>
                            {/if}
                        </div>
                        <div class="col-span-3 text-xs text-[#34251c] truncate pr-2" title={release.id}>
                            <span class="block">{release.description || '—'}</span>
                            <span class="text-[9px] text-[#5f4636] font-mono">{release.id.slice(0, 8)}...</span>
                        </div>
                        <div class="col-span-3 text-xs text-[#5f4636]">{formatDate(release.createdAt)}</div>
                        <div class="col-span-4 flex justify-end gap-2 opacity-60 group-hover:opacity-100 transition-opacity">
                            {#if !release.isActive}
                                <button
                                    onclick={() => handleActivate(release)}
                                    class="btn-xs border border-[#34251c]/20 hover:bg-[#34251c]/10 text-[#34251c]"
                                    title={$t('adminReleaseMakeActive')}
                                >
                                    Activate
                                </button>
                            {/if}
                        </div>
                    </div>
                {/each}
            {/if}
        </div>
    </div>

    <div class="mt-4 p-4 border border-[#34251c]/10 bg-[#fff9f0]/30 text-xs text-[#5f4636]">
        <h4 class="font-bold mb-2 uppercase tracking-wide text-[#34251c]">Curator's Guide</h4>
        <ul class="list-disc pl-4 space-y-1">
            <li><strong class="text-[#34251c]">Create release:</strong> {$t('adminReleaseInfoCreate')}</li>
            <li><strong class="text-[#34251c]">Download active:</strong> {$t('adminReleaseInfoDownload')}</li>
        </ul>
    </div>
</div>

<style>
    .btn-gothic {
        padding: 0.5rem 1rem;
        border: 1px solid rgba(198, 95, 60, 0.3);
        font-size: 11px;
        text-transform: uppercase;
        cursor: pointer;
        transition: all 0.2s;
        background: transparent;
    }

    .btn-gothic:hover {
        background-color: rgba(198, 95, 60, 0.1);
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
