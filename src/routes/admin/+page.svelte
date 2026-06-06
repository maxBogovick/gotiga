<script lang="ts">
    import { onMount } from 'svelte';
    import { api, isTauri } from '$lib/api';
    import type { Figurine, FigurineListItem } from '$lib/types/api';
    import { fade, slide } from 'svelte/transition';
    import SettingsModal from '$lib/components/SettingsModal.svelte';
    import ZoneEditor from '$lib/components/admin/ZoneEditor.svelte';
    import TextEditor from '$lib/components/admin/TextEditor.svelte';
    import ReleaseManager from '$lib/components/admin/ReleaseManager.svelte';
    import ProfileEditor from '$lib/components/admin/ProfileEditor.svelte';
    import MediaLibrary from '$lib/components/admin/MediaLibrary.svelte';
    import HomeContentEditor from '$lib/components/admin/HomeContentEditor.svelte';
    import OrdersPanel from '$lib/components/admin/OrdersPanel.svelte';
    import ShowingsPanel from '$lib/components/admin/ShowingsPanel.svelte';
    import BookingsPanel from '$lib/components/admin/BookingsPanel.svelte';
    import AnalyticsPanel from '$lib/components/admin/AnalyticsPanel.svelte';
    import FigurineShowingsEditor from '$lib/components/admin/FigurineShowingsEditor.svelte';
    import { t } from '$lib/i18n';
    import LangSwitcher from '$lib/components/LangSwitcher.svelte';

    // === AUTH ===
    let isAuthenticated = $state(false);
    let loginForm = $state({ login: '', password: '' });
    let loginError = $state('');
    let loginLoading = $state(false);
    let rememberMe = $state(false);

    async function handleLogin() {
        loginLoading = true;
        loginError = '';
        try {
            const token = await api.adminLogin(loginForm.login, loginForm.password);
            localStorage.setItem('gotiga_api_key', token);
            if (rememberMe) {
                localStorage.setItem('gotiga_admin_persist', '1');
            } else {
                sessionStorage.setItem('gotiga_admin', '1');
            }
            isAuthenticated = true;
            await loadFigurines();
        } catch {
            loginError = $t('adminLoginError');
        } finally {
            loginLoading = false;
        }
    }

    function handleLogout() {
        sessionStorage.removeItem('gotiga_admin');
        localStorage.removeItem('gotiga_admin_persist');
        isAuthenticated = false;
        selectedFigurine = null;
    }

    // === STATE ===
    let figurines = $state<FigurineListItem[]>([]);
    let selectedFigurine = $state<Figurine | null>(null);
    let savedSnapshot = $state<string>('');
    let isSaving = $state(false);
    let showSettings = $state(false);
    let message = $state({ text: '', type: 'info' });
    let activeTab = $state<'registry' | 'home' | 'zones' | 'author' | 'workshop' | 'media' | 'releases' | 'orders' | 'showings' | 'bookings' | 'analytics'>('registry');
    let activeAuthorSubTab = $state<'profile' | 'texts'>('profile');
    let newOrdersCount = $state(0);
    let newBookingsCount = $state(0);
    let searchQuery = $state('');
    let isDeleting = $state(false);
    let uploadingVideo = $state(false);
    let uploadingAudio = $state(false);
    let externalVideoUrl = $state('');

    let hasUnsaved = $derived(
        selectedFigurine !== null && JSON.stringify(selectedFigurine) !== savedSnapshot
    );

    let filteredFigurines = $derived(
        searchQuery.trim()
            ? figurines.filter(f => f.name.toLowerCase().includes(searchQuery.toLowerCase()))
            : figurines
    );

    function resolveUrl(path: string | null): string {
        if (!path) return '';
        if (path.startsWith('http')) return path;
        if (path.startsWith('/static/')) {
            // Web-uploaded relative path — prepend server origin
            if (typeof localStorage !== 'undefined') {
                const serverUrl = localStorage.getItem('gotiga_server_url') ?? '';
                return serverUrl ? `${serverUrl}${path}` : path;
            }
        }
        return path;
    }

    const emptyFigurine: Figurine = {
        id: '',
        name: '',
        shortText: '',
        fullDescription: '',
        dimensions: '',
        material: '',
        technique: '',
        year: new Date().getFullYear(),
        ambiencePath: null,
        videoUrl: null,
        secretText: '',
        status: 'available',
        sortOrder: 0,
        isVisible: true,
        isFeatured: false,
        series: null,
        images: [],
        processSteps: [],
        relatedItems: []
    };

    async function loadFigurines() {
        try {
            figurines = isTauri
                ? await api.getAllFigurines()
                : await api.getAllFigurinesAdmin();
        } catch (e) {
            showMessage($t('adminMsgLoadError') + e, 'error');
        }
    }

    async function moveFigurine(id: string, direction: 1 | -1) {
        const full = await api.getFigurine(id);
        if (!full) return;
        full.sortOrder = (full.sortOrder ?? 0) + direction;
        try {
            await api.saveFigurine(full);
            await loadFigurines();
            if (selectedFigurine?.id === id) {
                selectedFigurine = { ...full };
                savedSnapshot = JSON.stringify(selectedFigurine);
            }
        } catch { /* silent */ }
    }

    async function editFigurine(id: string) {
        if (hasUnsaved && !confirm($t('adminMsgUnsavedLeave'))) return;
        const full = await api.getFigurine(id);
        if (full) {
            selectedFigurine = { ...full };
            savedSnapshot = JSON.stringify(selectedFigurine);
        }
    }

    function createNew() {
        if (hasUnsaved && !confirm($t('adminMsgUnsavedLeave'))) return;
        selectedFigurine = { ...emptyFigurine, id: crypto.randomUUID(), sortOrder: figurines.length };
        savedSnapshot = '';
    }

    function duplicateFigurine(fig: FigurineListItem) {
        if (hasUnsaved && !confirm($t('adminMsgUnsavedLeave'))) return;
        api.getFigurine(fig.id).then(full => {
            if (!full) return;
            selectedFigurine = {
                ...full,
                id: crypto.randomUUID(),
                name: full.name + $t('adminRegistryCopySuffix'),
                sortOrder: figurines.length,
                isVisible: false,
            };
            savedSnapshot = '';
        });
    }

    async function deleteFigurine(fig: FigurineListItem) {
        if (!confirm($t('adminRegistryDeleteConfirm'))) return;
        isDeleting = true;
        try {
            await api.deleteFigurine(fig.id);
            if (selectedFigurine?.id === fig.id) {
                selectedFigurine = null;
                savedSnapshot = '';
            }
            await loadFigurines();
            showMessage($t('adminMsgDeleteSuccess'), 'success');
        } catch (e) {
            showMessage($t('adminMsgDeleteError') + e, 'error');
        } finally {
            isDeleting = false;
        }
    }

    function pickFileWeb(type: 'images' | 'videos' | 'audio'): Promise<File> {
        return new Promise((resolve, reject) => {
            const input = document.createElement('input');
            input.type = 'file';
            if (type === 'images') input.accept = 'image/jpeg,image/png,image/webp';
            else if (type === 'videos') input.accept = 'video/mp4,video/webm';
            else input.accept = 'audio/mpeg,audio/wav,audio/ogg';
            input.onchange = () => {
                const file = input.files?.[0];
                if (file) resolve(file);
                else reject(new Error('no file'));
            };
            input.click();
        });
    }

    async function handlePickFile(type: 'images' | 'videos' | 'audio', stepIndex?: number) {
        if (!selectedFigurine) return;
        if (type === 'videos') uploadingVideo = true;
        if (type === 'audio') uploadingAudio = true;
        try {
            let fileOrPath: string | File;
            if (isTauri) {
                const { open } = await import('@tauri-apps/plugin-dialog');
                const filters = [];
                if (type === 'images') filters.push({ name: 'Images', extensions: ['jpg', 'png', 'webp'] });
                else if (type === 'videos') filters.push({ name: 'Videos', extensions: ['mp4', 'webm', 'mov'] });
                else filters.push({ name: 'Audio', extensions: ['mp3', 'wav', 'ogg', 'm4a'] });
                const selected = await open({ multiple: false, filters });
                if (!selected || typeof selected !== 'string') return;
                fileOrPath = selected;
            } else {
                fileOrPath = await pickFileWeb(type);
            }

            const imported = await api.importMediaWithVariants(fileOrPath, type === 'videos' ? 'videos' : type === 'audio' ? 'audio' : 'images');
            const localUrl = imported.url;

            if (type === 'videos') {
                selectedFigurine.videoUrl = localUrl;
            } else if (type === 'audio') {
                selectedFigurine.ambiencePath = localUrl;
            } else if (typeof stepIndex === 'number') {
                selectedFigurine.processSteps[stepIndex].imageUrl = localUrl;
            } else {
                const variants = deriveImageVariants(localUrl);
                selectedFigurine.images = [...selectedFigurine.images, {
                    id: crypto.randomUUID(),
                    imageType: 'full',
                    url: localUrl,
                    originalUrl: imported.originalUrl ?? variants.originalUrl,
                    thumbUrl: imported.thumbUrl ?? variants.thumbUrl,
                    altText: ''
                }];
            }
            showMessage($t('adminMsgFileUploaded'), 'success');
        } catch (e: unknown) {
            const msg = e instanceof Error ? e.message : String(e);
            if (msg !== 'no file') showMessage($t('adminMsgError') + msg, 'error');
        } finally {
            if (type === 'videos') uploadingVideo = false;
            if (type === 'audio') uploadingAudio = false;
        }
    }

    function deriveImageVariants(url: string): { originalUrl: string | null; thumbUrl: string | null } {
        const marker = 'images/preview/';
        const idx = url.indexOf(marker);
        if (idx === -1) return { originalUrl: null, thumbUrl: null };
        const prefix = url.slice(0, idx);
        const fileName = url.slice(idx + marker.length);
        return {
            originalUrl: `${prefix}images/original/${fileName}`,
            thumbUrl: `${prefix}images/thumb/${fileName}`
        };
    }

    function moveImage(index: number, direction: -1 | 1) {
        if (!selectedFigurine) return;
        const newIdx = index + direction;
        if (newIdx < 0 || newIdx >= selectedFigurine.images.length) return;
        const imgs = [...selectedFigurine.images];
        [imgs[index], imgs[newIdx]] = [imgs[newIdx], imgs[index]];
        selectedFigurine.images = imgs;
    }

    function addProcessStep() {
        if (!selectedFigurine) return;
        selectedFigurine.processSteps = [...selectedFigurine.processSteps, {
            id: crypto.randomUUID(), stepType: 'sketch', description: '', imageUrl: ''
        }];
    }

    function removeProcessStep(index: number) {
        if (!selectedFigurine) return;
        selectedFigurine.processSteps = selectedFigurine.processSteps.filter((_, i) => i !== index);
    }

    function setFaceImage(imageId: string) {
        if (!selectedFigurine) return;
        selectedFigurine.images = selectedFigurine.images.map(img => ({
            ...img, imageType: img.id === imageId ? 'face' : 'full'
        }));
    }

    async function save() {
        if (!selectedFigurine) return;
        isSaving = true;
        try {
            await api.saveFigurine(selectedFigurine);
            savedSnapshot = JSON.stringify(selectedFigurine);
            showMessage(isTauri ? $t('adminMsgSavedArchive') : $t('adminMsgSavedServer'), 'success');
            await loadFigurines();
        } catch (e) {
            showMessage($t('adminMsgError') + e, 'error');
        } finally {
            isSaving = false;
        }
    }

    function cancelEdit() {
        if (hasUnsaved && !confirm($t('adminMsgUnsavedCancel'))) return;
        selectedFigurine = null;
        savedSnapshot = '';
    }

    function showMessage(text: string, type = 'info') {
        message = { text, type };
        setTimeout(() => message.text = '', 3000);
    }

    onMount(() => {
        // Check session
        const session = sessionStorage.getItem('gotiga_admin');
        const persisted = localStorage.getItem('gotiga_admin_persist');
        if (isTauri) {
            isAuthenticated = true;
            loadFigurines();
        } else if ((session === '1' || persisted === '1') && localStorage.getItem('gotiga_api_key')) {
            isAuthenticated = true;
            loadFigurines();
        }
        // Hash-based tab routing (e.g. Telegram notification links)
        const hash = window.location.hash.replace('#', '');
        const validTabs = ['registry','home','zones','author','workshop','media','releases','orders','showings','bookings','analytics'];
        if (validTabs.includes(hash)) {
            activeTab = hash as typeof activeTab;
        }
    });
</script>

<!-- ===== LOGIN SCREEN ===== -->
{#if !isAuthenticated && !isTauri}
<div class="h-screen bg-[#f8f1e7] flex items-center justify-center font-cinzel">
    <div class="relative w-full max-w-sm p-10 border border-[#34251c]/20 bg-[#fff9f0] shadow-[0_0_80px_rgba(111,59,36,0.20)]">
        <!-- Corner marks -->
        <div class="absolute top-0 left-0 w-5 h-5 border-t border-l border-[#34251c]/40"></div>
        <div class="absolute top-0 right-0 w-5 h-5 border-t border-r border-[#34251c]/40"></div>
        <div class="absolute bottom-0 left-0 w-5 h-5 border-b border-l border-[#34251c]/40"></div>
        <div class="absolute bottom-0 right-0 w-5 h-5 border-b border-r border-[#34251c]/40"></div>

        <div class="text-center mb-10">
            <div class="text-5xl mb-4 opacity-60">🗝</div>
            <h1 class="text-2xl font-bold tracking-[0.08em] uppercase text-[#6f3b24]">{$t('adminLoginHeading')}</h1>
            <p class="text-[10px] tracking-wide text-[#5f4636] uppercase mt-1">{$t('adminLoginSub')}</p>
            <div class="w-full h-px bg-gradient-to-r from-transparent via-[#34251c]/30 to-transparent mt-6"></div>
        </div>

        <form onsubmit={(e) => { e.preventDefault(); handleLogin(); }} class="space-y-6">
            <label class="block">
                <span class="text-[10px] uppercase tracking-wide text-[#5f4636] block mb-2">{$t('adminLoginName')}</span>
                <input
                    bind:value={loginForm.login}
                    type="text"
                    autocomplete="username"
                    class="w-full bg-[#fff9f0] border border-[#34251c]/20 p-3 text-sm text-[#34251c] focus:border-[#34251c]/60 outline-none transition-colors"
                />
            </label>
            <label class="block">
                <span class="text-[10px] uppercase tracking-wide text-[#5f4636] block mb-2">{$t('adminLoginPassword')}</span>
                <input
                    bind:value={loginForm.password}
                    type="password"
                    autocomplete="current-password"
                    class="w-full bg-[#fff9f0] border border-[#34251c]/20 p-3 text-sm text-[#34251c] focus:border-[#34251c]/60 outline-none transition-colors"
                />
            </label>

            <label class="flex items-center gap-2 cursor-pointer select-none">
                <input type="checkbox" bind:checked={rememberMe} class="accent-[#34251c] w-3.5 h-3.5" />
                <span class="text-[10px] uppercase tracking-wide text-[#5f4636]">{$t('adminLoginRemember')}</span>
            </label>

            {#if loginError}
                <p class="text-red-700 text-xs text-center" in:fade>{loginError}</p>
            {/if}

            <button
                type="submit"
                disabled={loginLoading}
                class="w-full py-3 bg-[#34251c]/10 border border-[#34251c]/30 text-[#34251c] text-xs uppercase tracking-wide hover:bg-[#34251c]/20 transition-all disabled:opacity-70"
            >
                {$t(loginLoading ? 'adminLoginCheck' : 'adminLoginEnter')}
            </button>
        </form>
    </div>
</div>

<!-- ===== ADMIN UI ===== -->
{:else}
<div class="h-screen bg-[#f8f1e7] text-[#34251c] font-cinzel flex flex-col overflow-hidden">

    <!-- Header -->
    <header class="flex justify-between items-center px-6 py-4 border-b border-[#34251c]/20 bg-[#f8f1e7] z-10 shrink-0">
        <div>
            <h1 class="text-2xl font-gothic mb-0.5">{$t('adminTitle')}</h1>
            <p class="text-[10px] tracking-[0.08em] text-[#5f4636] uppercase">{$t('adminSubtitle')}</p>
        </div>

        <nav class="flex gap-1 bg-[#fff9f0] p-1 border border-[#34251c]/20 flex-wrap">
            {#each [
              ['analytics', '📊 Аналитика'],
              ['registry', $t('adminTabRegistry')],
              ['home',     'Home'],
              ['zones',    $t('adminTabZones')],
              ['author',   $t('adminTabAuthor')],
              ['workshop', $t('adminTabWorkshop')],
              ['media',    'Media'],
              ['orders',   'Orders'],
              ['showings', $t('adminTabShowings')],
              ['bookings', $t('adminTabBookings')],
              ['releases', $t('adminTabReleases')],
            ] as [tab, label]}
                <button
                    onclick={() => activeTab = tab as typeof activeTab}
                    class="relative px-4 py-2 text-xs uppercase tracking-wide transition-colors {activeTab === tab ? 'bg-[#c65f3c]/12 text-[#34251c]' : 'text-[#5f4636] hover:text-[#34251c]'}"
                >
                  {label}
                  {#if tab === 'orders' && newOrdersCount > 0 && activeTab !== 'orders'}
                    <span class="absolute -top-1 -right-1 inline-flex items-center justify-center min-w-[16px] h-4 px-1 rounded-full bg-red-500 text-white text-[9px] font-bold leading-none">
                      {newOrdersCount > 99 ? '99+' : newOrdersCount}
                    </span>
                  {/if}
                  {#if tab === 'bookings' && newBookingsCount > 0 && activeTab !== 'bookings'}
                    <span class="absolute -top-1 -right-1 inline-flex items-center justify-center min-w-[16px] h-4 px-1 rounded-full bg-amber-500 text-white text-[9px] font-bold leading-none">
                      {newBookingsCount > 99 ? '99+' : newBookingsCount}
                    </span>
                  {/if}
                </button>
            {/each}
        </nav>

        <div class="flex gap-2 items-center">
            <LangSwitcher />
            <button onclick={() => showSettings = true} class="btn-gothic text-lg px-3" title={$t('adminSettings')}>⚙</button>
            {#if !isTauri}
                <button onclick={handleLogout} class="btn-gothic text-[10px] opacity-75 hover:opacity-100">{$t('adminLogout')}</button>
            {/if}
            <a href="/" class="btn-gothic opacity-60">{$t('adminToMuseum')}</a>
        </div>
    </header>

    <SettingsModal isOpen={showSettings} onClose={() => showSettings = false} />

    <!-- Content -->
    <div class="flex-1 overflow-hidden p-6 relative">

        {#if activeTab === 'registry'}
        <div class="grid grid-cols-12 gap-6 h-full" in:fade>

            <!-- Sidebar -->
            <aside class="col-span-3 flex flex-col gap-3 border-r border-[#34251c]/10 pr-5 overflow-hidden">
                <div class="flex justify-between items-center shrink-0">
                    <h2 class="text-xs uppercase tracking-wide text-[#5f4636]">{$t('adminRegistryHeading')}</h2>
                    <button onclick={createNew} class="btn-gothic text-[10px]">{$t('adminRegistryNew')}</button>
                </div>

                <!-- Search -->
                <div class="shrink-0 relative">
                    <input
                        bind:value={searchQuery}
                        type="text"
                        placeholder={$t('adminRegistrySearch')}
                        class="w-full bg-[#f8f1e7] border border-[#34251c]/15 px-3 py-2 text-xs text-[#34251c] outline-none focus:border-[#34251c]/40 transition-colors"
                    />
                    {#if searchQuery}
                        <button
                            onclick={() => searchQuery = ''}
                            class="absolute right-2 top-1/2 -translate-y-1/2 text-[#5f4636] hover:text-[#34251c] text-xs"
                        >✕</button>
                    {/if}
                </div>

                <div class="flex-1 overflow-y-auto space-y-0.5 pr-1">
                    {#each filteredFigurines as fig (fig.id)}
                        <div class="flex items-stretch gap-0.5 group/row">
                            <!-- Main row button -->
                            <button
                                onclick={() => editFigurine(fig.id)}
                                class="flex-1 text-left px-3 py-2.5 border transition-colors min-w-0
                                    {selectedFigurine?.id === fig.id
                                        ? 'bg-[#34251c]/5 border-[#34251c]/30'
                                        : 'border-[#34251c]/8 hover:border-[#34251c]/30 border-transparent'}"
                            >
                                <div class="text-xs font-bold truncate group-hover/row:text-[#6f3b24] transition-colors">
                                    {fig.name}
                                    {#if hasUnsaved && selectedFigurine?.id === fig.id}
                                        <span class="text-amber-700 ml-1">*</span>
                                    {/if}
                                </div>
                                <div class="text-[9px] uppercase opacity-60 flex gap-2">
                                    <span>{fig.status}</span>
                                </div>
                            </button>

                            <!-- Action column (visible on hover) -->
                            <div class="flex flex-col opacity-0 group-hover/row:opacity-100 transition-opacity shrink-0">
                                <button onclick={() => moveFigurine(fig.id, -1)}
                                    class="flex-1 w-5 border border-[#34251c]/8 hover:bg-[#34251c]/10 text-[#5f4636] hover:text-[#34251c] text-[9px] flex items-center justify-center"
                                    title={$t('adminRegistryTooltipUp')}>▲</button>
                                <button onclick={() => moveFigurine(fig.id, 1)}
                                    class="flex-1 w-5 border border-[#34251c]/8 hover:bg-[#34251c]/10 text-[#5f4636] hover:text-[#34251c] text-[9px] flex items-center justify-center"
                                    title={$t('adminRegistryTooltipDown')}>▼</button>
                                <button onclick={() => duplicateFigurine(fig)}
                                    class="flex-1 w-5 border border-[#34251c]/8 hover:bg-[#34251c]/10 text-[#5f4636] hover:text-amber-700 text-[9px] flex items-center justify-center"
                                    title={$t('adminRegistryDuplicate')}>⎘</button>
                                <button onclick={() => deleteFigurine(fig)} disabled={isDeleting}
                                    class="flex-1 w-5 border border-[#34251c]/8 hover:bg-red-50 text-[#5f4636] hover:text-red-700 text-[9px] flex items-center justify-center"
                                    title={$t('adminRegistryDelete')}>✕</button>
                            </div>
                        </div>
                    {/each}

                    {#if filteredFigurines.length === 0}
                        <div class="text-center text-[#5f4636] text-xs py-6 opacity-70">
                            {searchQuery ? $t('adminRegistryNotFound') : $t('adminRegistryEmpty')}
                        </div>
                    {/if}
                </div>

                <div class="pt-3 border-t border-[#34251c]/10 shrink-0 text-[10px] text-[#5f4636] text-center opacity-75">
                    {figurines.length} {$t('adminRegistryCount')}
                </div>
            </aside>

            <!-- Editor -->
            <main class="col-span-9 bg-[#fff9f0]/50 border border-[#34251c]/10 relative h-full overflow-y-auto">
                {#if selectedFigurine}
                    <div class="p-8">
                        <!-- Unsaved indicator bar -->
                        {#if hasUnsaved}
                            <div class="mb-6 px-4 py-2 bg-amber-50 border border-amber-700/30 text-amber-800 text-[10px] uppercase tracking-wide flex items-center gap-2" in:fade>
                                <span class="w-1.5 h-1.5 rounded-full bg-amber-700 animate-pulse"></span>
                                {$t('adminRegistryUnsaved')}
                            </div>
                        {/if}

                        <div class="grid grid-cols-2 gap-6 mb-8">
                            <div class="space-y-4">
                                <label class="block">
                                    <span class="label">{$t('adminFieldName')}</span>
                                    <input bind:value={selectedFigurine.name} class="input-gothic" />
                                </label>
                                <label class="block">
                                    <span class="label">{$t('adminFieldYear')}</span>
                                    <input type="number" bind:value={selectedFigurine.year} class="input-gothic" />
                                </label>
                                <label class="block">
                                    <span class="label">{$t('adminFieldSeries')}</span>
                                    <input bind:value={selectedFigurine.series} class="input-gothic" placeholder="—" />
                                </label>
                                <label class="block">
                                    <span class="label">{$t('adminFieldStatus')}</span>
                                    <select bind:value={selectedFigurine.status} class="input-gothic">
                                        <option value="available">{$t('adminFieldStatusAvail')}</option>
                                        <option value="reserved">{$t('adminFieldStatusRes')}</option>
                                        <option value="in_progress">{$t('adminFieldStatusWip')}</option>
                                        <option value="sold">{$t('adminFieldStatusSold')}</option>
                                    </select>
                                </label>
                            </div>
                            <div class="space-y-4">
                                <label class="block">
                                    <span class="label">{$t('adminFieldDimensions')}</span>
                                    <input bind:value={selectedFigurine.dimensions} class="input-gothic" placeholder="20×15×10 cm" />
                                </label>
                                <label class="block">
                                    <span class="label">{$t('adminFieldMaterial')}</span>
                                    <input bind:value={selectedFigurine.material} class="input-gothic" />
                                </label>
                                <label class="block">
                                    <span class="label">{$t('adminFieldTechnique')}</span>
                                    <input bind:value={selectedFigurine.technique} class="input-gothic" />
                                </label>
                                <div class="flex gap-4">
                                    <label class="block flex-1">
                                        <span class="label">{$t('adminFieldSortOrder')}</span>
                                        <input type="number" bind:value={selectedFigurine.sortOrder} class="input-gothic" />
                                    </label>
                                    <label class="flex items-end gap-2 pb-3">
                                        <input type="checkbox" bind:checked={selectedFigurine.isVisible} class="accent-[#34251c] w-4 h-4" />
                                        <span class="text-xs text-[#34251c]">{$t('adminFieldVisible')}</span>
                                    </label>
                                    <label class="flex items-end gap-2 pb-3">
                                        <input type="checkbox" bind:checked={selectedFigurine.isFeatured} class="accent-[#c65f3c] w-4 h-4" />
                                        <span class="text-xs text-[#34251c]">{$t('adminFieldFeatured')}</span>
                                    </label>
                                </div>
                            </div>
                        </div>

                        <label class="block mb-6">
                            <span class="label">{$t('adminFieldQuote')}</span>
                            <textarea bind:value={selectedFigurine.shortText} class="input-gothic h-20"></textarea>
                        </label>

                        <label class="block mb-6">
                            <span class="label">{$t('adminFieldSecret')}</span>
                            <textarea bind:value={selectedFigurine.secretText} class="input-gothic h-16 opacity-70"></textarea>
                        </label>

                        <label class="block mb-8">
                            <span class="label">{$t('adminFieldHistory')}</span>
                            <textarea bind:value={selectedFigurine.fullDescription} class="input-gothic h-40"></textarea>
                        </label>

                        <!-- Media -->
                        <div class="border-t border-[#34251c]/10 pt-8 mb-8">
                            <h3 class="text-xl font-gothic mb-6">{$t('adminMediaHeading')}</h3>
                            <div class="grid grid-cols-2 gap-6 mb-6">
                                <!-- Video -->
                                <div class="p-4 border border-dashed border-[#34251c]/20 flex flex-col gap-2">
                                    <span class="label block">{$t('adminMediaVideo')}</span>
                                    {#if selectedFigurine.videoUrl}
                                        <video
                                            src={resolveUrl(selectedFigurine.videoUrl)}
                                            controls
                                            class="w-full max-h-36 bg-[#2f2117]"
                                            preload="metadata"
                                        >
                                            <track kind="captions" />
                                        </video>
                                        <div class="flex gap-2">
                                            <button
                                                onclick={() => handlePickFile('videos')}
                                                disabled={uploadingVideo}
                                                class="text-[10px] text-[#34251c]/85 hover:text-[#6f3b24] uppercase disabled:opacity-70"
                                            >{$t('adminMediaReplace')}</button>
                                            <button
                                                onclick={() => { selectedFigurine!.videoUrl = null; externalVideoUrl = ''; }}
                                                class="text-[10px] text-red-700 hover:text-red-900 uppercase"
                                            >{$t('adminMediaDeleteFile')}</button>
                                        </div>
                                    {:else}
                                        <div class="flex flex-col gap-2">
                                            <input
                                                type="url"
                                                bind:value={externalVideoUrl}
                                                placeholder="https://... external link"
                                                class="input-gothic text-xs"
                                            />
                                            {#if externalVideoUrl.trim()}
                                                <button
                                                    onclick={() => { selectedFigurine!.videoUrl = externalVideoUrl.trim(); externalVideoUrl = ''; }}
                                                    class="btn-gothic text-xs w-full"
                                                >{$t('adminMediaUseLink')}</button>
                                            {:else}
                                                <button
                                                    onclick={() => handlePickFile('videos')}
                                                    disabled={uploadingVideo}
                                                    class="btn-gothic text-xs w-full disabled:opacity-70"
                                                >{uploadingVideo ? '…' : $t('adminMediaPickMp4')}</button>
                                            {/if}
                                        </div>
                                    {/if}
                                </div>
                                <!-- Audio -->
                                <div class="p-4 border border-dashed border-[#34251c]/20 flex flex-col gap-2">
                                    <span class="label block">{$t('adminMediaAudio')}</span>
                                    {#if selectedFigurine.ambiencePath}
                                        <audio
                                            src={resolveUrl(selectedFigurine.ambiencePath)}
                                            controls
                                            class="w-full"
                                            preload="metadata"
                                        ></audio>
                                        <div class="flex gap-2">
                                            <button
                                                onclick={() => handlePickFile('audio')}
                                                disabled={uploadingAudio}
                                                class="text-[10px] text-[#34251c]/85 hover:text-[#6f3b24] uppercase disabled:opacity-70"
                                            >{$t('adminMediaReplace')}</button>
                                            <button
                                                onclick={() => selectedFigurine!.ambiencePath = null}
                                                class="text-[10px] text-red-700 hover:text-red-900 uppercase"
                                            >{$t('adminMediaDeleteFile')}</button>
                                        </div>
                                    {:else}
                                        <button
                                            onclick={() => handlePickFile('audio')}
                                            disabled={uploadingAudio}
                                            class="btn-gothic text-xs w-full disabled:opacity-70"
                                        >{uploadingAudio ? '…' : $t('adminMediaPickMp3')}</button>
                                    {/if}
                                </div>
                            </div>

                            <!-- Images gallery -->
                            <div class="p-4 border border-dashed border-[#34251c]/20">
                                <div class="flex justify-between items-center mb-4">
                                    <span class="label">{$t('adminMediaPhotos')} ({selectedFigurine.images.length})</span>
                                    <button onclick={() => handlePickFile('images')} class="btn-gothic text-[10px]">{$t('adminMediaAddPhoto')}</button>
                                </div>
                                <div class="flex flex-wrap gap-3">
                                    {#each selectedFigurine.images as img, imgIdx}
                                        <div class="flex flex-col gap-1">
                                            <div class="w-28 h-28 border overflow-hidden relative group transition-colors
                                                {img.imageType === 'face' ? 'border-amber-500' : 'border-[#34251c]/20'}">
                                                <img src={resolveUrl(img.thumbUrl ?? img.url)} alt={img.altText ?? ''} class="w-full h-full object-cover" />

                                                <!-- Overlay controls -->
                                                <div class="absolute inset-0 bg-[#6f3b24]/40 opacity-0 group-hover:opacity-100 transition-opacity flex flex-col items-center justify-center gap-1">
                                                    <button onclick={() => selectedFigurine!.images = selectedFigurine!.images.filter(i => i.id !== img.id)}
                                                        class="text-[10px] text-red-800 hover:text-red-950 uppercase px-2 py-0.5 border border-red-800/30 bg-[#fff9f0]/90 hover:bg-red-50">{$t('adminMediaDeleteFile')}</button>
                                                    {#if img.imageType !== 'face'}
                                                        <button onclick={() => setFaceImage(img.id)}
                                                            class="text-[9px] text-[#34251c] hover:text-amber-800 uppercase px-2 py-0.5 border border-[#34251c]/20 bg-[#fff9f0]/90">{$t('adminMediaCover')}</button>
                                                    {:else}
                                                        <span class="text-[9px] text-amber-900 bg-[#fff9f0]/90 px-2 py-0.5 uppercase">{$t('adminMediaCover')}</span>
                                                    {/if}
                                                    <div class="flex gap-1">
                                                        <button onclick={() => moveImage(imgIdx, -1)} disabled={imgIdx === 0}
                                                            class="text-[10px] text-[#34251c] hover:text-[#6f3b24] px-1.5 border border-[#34251c]/20 disabled:opacity-60">←</button>
                                                        <button onclick={() => moveImage(imgIdx, 1)} disabled={imgIdx === selectedFigurine.images.length - 1}
                                                            class="text-[10px] text-[#34251c] hover:text-[#6f3b24] px-1.5 border border-[#34251c]/20 disabled:opacity-60">→</button>
                                                    </div>
                                                </div>

                                                {#if img.imageType === 'face'}
                                                    <div class="absolute bottom-0 left-0 right-0 bg-amber-500/80 text-black text-[8px] text-center py-0.5 font-bold">{$t('adminMediaCoverBadge')}</div>
                                                {/if}
                                            </div>
                                            <!-- Alt text -->
                                            <input
                                                bind:value={img.altText}
                                                type="text"
                                                placeholder={$t('adminMediaAltPlaceholder')}
                                                class="w-28 bg-[#f8f1e7] border border-[#34251c]/10 px-1.5 py-1 text-[9px] text-[#5f4636] focus:border-[#34251c]/30 outline-none"
                                            />
                                        </div>
                                    {/each}
                                </div>
                            </div>
                        </div>

                        <!-- Process Steps -->
                        <div class="border-t border-[#34251c]/10 pt-8 mb-8">
                            <div class="flex justify-between items-center mb-6">
                                <h3 class="text-xl font-gothic">{$t('adminGrimoireHeading')}</h3>
                                <button onclick={addProcessStep} class="btn-gothic text-xs">{$t('adminGrimoireAddStep')}</button>
                            </div>
                            <div class="space-y-3">
                                {#each selectedFigurine.processSteps as step, i}
                                    <div class="p-4 bg-[#f8f1e7] border border-[#34251c]/10 flex gap-4 items-start">
                                        <div class="w-20 h-20 bg-[#f1e3d1] flex items-center justify-center border border-[#34251c]/20 relative group shrink-0">
                                            {#if step.imageUrl}
                                                <img src={resolveUrl(step.imageUrl)} alt="" class="w-full h-full object-cover" />
                                                <button onclick={() => step.imageUrl = ''} class="absolute top-0 right-0 bg-[#6f3b24]/30 text-[#fff9f0] p-0.5 text-[9px] opacity-0 group-hover:opacity-100">✕</button>
                                            {:else}
                                                <button onclick={() => handlePickFile('images', i)} class="text-[10px] uppercase text-[#5f4636] hover:text-[#34251c]">{$t('adminGrimoirePhoto')}</button>
                                            {/if}
                                        </div>
                                        <div class="flex-1 grid gap-2">
                                            <select bind:value={step.stepType} class="input-gothic text-xs py-1.5">
                                                <option value="sketch">Sketch</option>
                                                <option value="prototype">Prototype</option>
                                                <option value="modeling">Modeling</option>
                                                <option value="painting">Painting</option>
                                                <option value="finish">Finish</option>
                                            </select>
                                            <textarea bind:value={step.description} class="input-gothic h-14 text-xs" placeholder={$t('adminGrimoireStepDesc')}></textarea>
                                        </div>
                                        <button onclick={() => removeProcessStep(i)} class="text-[#5f4636] hover:text-red-500 self-center text-sm">✕</button>
                                    </div>
                                {/each}
                                {#if selectedFigurine.processSteps.length === 0}
                                    <div class="text-center text-[#5f4636] text-xs py-4 opacity-70">{$t('adminGrimoireEmpty')}</div>
                                {/if}
                            </div>
                        </div>

                        <!-- Showings for this figurine -->
                        {#if selectedFigurine.id}
                          <FigurineShowingsEditor figurineId={selectedFigurine.id} />
                        {/if}

                        <!-- Action bar -->
                        <div class="flex justify-end gap-3 pb-10">
                            <button onclick={cancelEdit} class="btn-gothic opacity-75">{$t('adminFormCancel')}</button>
                            {#if isTauri}
                                <button
                                    onclick={async () => {
                                        const isPushing = true;
                                        try {
                                            await api.pushFigurine(selectedFigurine!);
                                            savedSnapshot = JSON.stringify(selectedFigurine);
                                            showMessage($t('adminFormSentToCloud'), 'success');
                                        } catch(e) { showMessage($t('adminMsgError') + e, 'error'); }
                                    }}
                                    class="btn-gothic border-blue-900/40 text-blue-700 min-w-[160px]"
                                >{$t('adminFormToCloud')}</button>
                            {/if}
                            <button onclick={save} disabled={isSaving}
                                class="btn-gothic min-w-[200px] transition-colors
                                    {hasUnsaved ? 'bg-amber-50 border-amber-700/40 text-amber-900 hover:bg-amber-100' : 'bg-[#34251c]/10'}">
                                {isSaving ? $t('adminFormSaving') : hasUnsaved ? $t('adminFormSaveChanges') : $t('adminFormSaved')}
                            </button>
                        </div>
                    </div>
                {:else}
                    <div class="h-full flex flex-col items-center justify-center text-[#5f4636] opacity-60">
                        <span class="text-5xl mb-4">📜</span>
                        <p class="text-sm">{$t('adminRegistrySelectPrompt')}</p>
                    </div>
                {/if}
            </main>
        </div>

        {:else if activeTab === 'home'}
            <HomeContentEditor />

        {:else if activeTab === 'zones'}
            <div in:fade class="h-full"><ZoneEditor /></div>
        {:else if activeTab === 'media'}
            <div in:fade class="h-full">
                <MediaLibrary onEditFigurine={(id) => { activeTab = 'registry'; editFigurine(id); }} />
            </div>
        {:else if activeTab === 'author'}
            <div in:fade class="h-full flex flex-col">
                <!-- Sub-tabs for author section -->
                <div class="flex gap-1 border-b border-[#34251c]/10 px-4 pt-2 flex-shrink-0">
                    {#each [[('profile'), $t('adminSubProfile')],[('texts'), $t('adminSubTexts')]] as [sub, label]}
                        {@const authorSubTab = activeAuthorSubTab}
                        <button
                            onclick={() => activeAuthorSubTab = sub as 'profile' | 'texts'}
                            class="px-3 py-1.5 text-[10px] uppercase tracking-wide border-b-2 transition-colors -mb-px
                                {authorSubTab === sub ? 'border-[#34251c]/50 text-[#34251c]' : 'border-transparent text-[#5f4636] hover:text-[#34251c]'}"
                        >{label}</button>
                    {/each}
                </div>
                <div class="flex-1 overflow-hidden">
                    {#if activeAuthorSubTab === 'profile'}
                        <ProfileEditor />
                    {:else}
                        <TextEditor category="author" />
                    {/if}
                </div>
            </div>
        {:else if activeTab === 'workshop'}
            <div in:fade class="h-full"><TextEditor category="workshop" /></div>
        {:else if activeTab === 'orders'}
            <div in:fade class="h-full"><OrdersPanel onNewCount={(n) => newOrdersCount = n} /></div>
        {:else if activeTab === 'showings'}
            <div in:fade class="h-full"><ShowingsPanel /></div>
        {:else if activeTab === 'bookings'}
            <div in:fade class="h-full"><BookingsPanel onPendingCount={(n) => newBookingsCount = n} /></div>
        {:else if activeTab === 'releases'}
            <div in:fade class="h-full"><ReleaseManager /></div>
        {:else if activeTab === 'analytics'}
            <div in:fade class="h-full overflow-hidden"><AnalyticsPanel /></div>
        {/if}
    </div>

    {#if message.text}
        <div
            class="fixed bottom-6 right-6 px-5 py-3 border text-sm z-50
                {message.type === 'error' ? 'bg-red-50 border-red-700/30 text-red-800' : 'bg-[#fff9f0] border-[#34251c]/30 text-[#34251c]'}"
            in:slide={{ axis: 'x' }}
        >{message.text}</div>
    {/if}
</div>
{/if}

<style>
    .label {
        font-size: 10px;
        text-transform: uppercase;
        letter-spacing: 0.04em;
        color: #5f4636;
        margin-bottom: 0.35rem;
        display: block;
        font-weight: 700;
    }

    .input-gothic {
        width: 100%;
        background-color: #f8f1e7;
        border: 1px solid rgba(198, 95, 60, 0.2);
        padding: 0.65rem 0.75rem;
        font-size: 0.875rem;
        color: #34251c;
        outline: none;
        transition: border-color 0.2s;
        font-family: inherit;
    }

    .input-gothic:focus {
        border-color: rgba(198, 95, 60, 0.55);
    }

    textarea.input-gothic { resize: none; }

    .btn-gothic {
        padding: 0.45rem 1.25rem;
        border: 1px solid rgba(198, 95, 60, 0.3);
        font-size: 11px;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        cursor: pointer;
        transition: all 0.2s;
        background: transparent;
        color: #34251c;
        font-family: inherit;
    }

    .btn-gothic:hover { background-color: rgba(198, 95, 60, 0.06); }
    .btn-gothic:disabled { opacity: 0.3; cursor: not-allowed; }
</style>
