<script lang="ts">
    import { onMount } from 'svelte';
    import { api } from '$lib/api';
    import type { Figurine, FigurineListItem, GazetteSeed, ShowingRoom } from '$lib/types/api';
    import { fade, slide } from 'svelte/transition';
    import SettingsModal from '$lib/components/SettingsModal.svelte';
    import TextEditor from '$lib/components/admin/TextEditor.svelte';
    import ReleaseManager from '$lib/components/admin/ReleaseManager.svelte';
    import ProfileEditor from '$lib/components/admin/ProfileEditor.svelte';
    import MediaLibrary from '$lib/components/admin/MediaLibrary.svelte';
    import HomeContentEditor from '$lib/components/admin/HomeContentEditor.svelte';
    import OrdersPanel from '$lib/components/admin/OrdersPanel.svelte';
    import CommissionsPanel from '$lib/components/admin/CommissionsPanel.svelte';
    import ShowingsPanel from '$lib/components/admin/ShowingsPanel.svelte';
    import BookingsPanel from '$lib/components/admin/BookingsPanel.svelte';
    import AnalyticsPanel from '$lib/components/admin/AnalyticsPanel.svelte';
    import AttentionMarksPanel from '$lib/components/admin/AttentionMarksPanel.svelte';
    import UsersPanel from '$lib/components/admin/UsersPanel.svelte';
    import CommentsPanel from '$lib/components/admin/CommentsPanel.svelte';
    import ImpressionsPanel from '$lib/components/admin/ImpressionsPanel.svelte';
    import SmtpSettingsPanel from '$lib/components/admin/SmtpSettingsPanel.svelte';
    import ContactSettingsPanel from '$lib/components/admin/ContactSettingsPanel.svelte';
    import WaitlistPanel from '$lib/components/admin/WaitlistPanel.svelte';
    import SubscribersPanel from '$lib/components/admin/SubscribersPanel.svelte';
    import ContactMessagesPanel from '$lib/components/admin/ContactMessagesPanel.svelte';
    import BookingRulesPanel from '$lib/components/admin/BookingRulesPanel.svelte';
    import MessagesPanel from '$lib/components/admin/MessagesPanel.svelte';
    import DesignEditor from '$lib/components/admin/DesignEditor.svelte';
    import HomeLayoutEditor from '$lib/components/admin/HomeLayoutEditor.svelte';
    import ReelThemePanel from '$lib/components/admin/ReelThemePanel.svelte';
    import CopyEditor from '$lib/components/admin/CopyEditor.svelte';
    import ProgrammePanel from '$lib/components/admin/ProgrammePanel.svelte';
    import GazettePanel from '$lib/components/admin/GazettePanel.svelte';
    import LogsPanel from '$lib/components/admin/LogsPanel.svelte';
    import { t, registerAdminDicts, type TranslationKey } from '$lib/i18n';
    import { enAdmin } from '$lib/i18n/en.admin';
    import { ruAdmin } from '$lib/i18n/ru.admin';

    registerAdminDicts(enAdmin, ruAdmin);
    import FigurineForm from '$lib/components/admin/FigurineForm.svelte';
    import SlugsPanel from '$lib/components/admin/SlugsPanel.svelte';
    import ShowingRoomsPanel from '$lib/components/admin/ShowingRoomsPanel.svelte';
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
            if (rememberMe) {
                // Persist across tabs/restarts only when explicitly requested.
                localStorage.setItem('gotiga_api_key', token);
                localStorage.setItem('gotiga_admin_persist', '1');
            } else {
                // Session-only: token dies with the tab (read via sessionStorage fallback).
                sessionStorage.setItem('gotiga_api_key', token);
                sessionStorage.setItem('gotiga_admin', '1');
            }
            isAuthenticated = true;
            await loadFigurines();
            await loadShowingRooms();
        } catch {
            loginError = $t('adminLoginError');
        } finally {
            loginLoading = false;
        }
    }

    function handleLogout() {
        sessionStorage.removeItem('gotiga_admin');
        sessionStorage.removeItem('gotiga_api_key');
        localStorage.removeItem('gotiga_admin_persist');
        localStorage.removeItem('gotiga_api_key');
        isAuthenticated = false;
        selectedFigurine = null;
    }

    // === STATE ===
    let figurines = $state<FigurineListItem[]>([]);
    let selectedFigurine = $state<Figurine | null>(null);

    let savedSnapshot = $state<string>('');
    // Raised by the form while an upload is in flight — blocks switching works.
    let formBusy = $state(false);
    let showSettings = $state(false);
    let bulkPanelOpen = $state(false);
    let bulkBusy = $state(false);
    let bulkParallaxValue = $state(0.5);
    let message = $state({ text: '', type: 'info' });
    // The sidebar's single source of truth: which tabs exist, in what order, under
    // which heading. Hash routing (see onMount) validates against this same list, so
    // a tab can never be reachable by URL yet missing from the nav — the state the
    // «Брони» tab was in before this list existed.
    // Grouped by the job the author comes here to do, not by which panel component
    // happens to render it.
    const TAB_GROUPS = [
        {
            label: 'adminGroupCollection',
            tabs: [
                ['registry',  'adminTabRegistry'],
                ['slugs',     'adminTabSlugs'],
                ['media',     'adminTabMedia'],
                ['rooms',     'adminTabShowingRooms'],
                ['showings',  'adminTabShowings'],
            ],
        },
        {
            label: 'adminGroupHome',
            tabs: [
                ['home',        'adminTabHome'],
                ['home-layout', 'adminTabHomeLayout'],
                ['reel-theme',  'adminTabReelTheme'],
                ['programme',   'adminTabProgramme'],
                ['gazette',     'adminTabGazette'],
                ['marks',       'adminTabMarks'],
            ],
        },
        {
            label: 'adminGroupPages',
            tabs: [
                ['author',   'adminTabAuthor'],
                ['workshop', 'adminTabWorkshop'],
                ['design',   'adminTabDesign'],
                ['copy',     'adminTabCopy'],
            ],
        },
        {
            label: 'adminGroupRequests',
            tabs: [
                ['orders',      'adminTabOrders'],
                ['commissions', 'adminTabCommissions'],
                ['bookings',    'adminTabBookings'],
                ['waitlist',    'adminTabWaitlist'],
            ],
        },
        {
            label: 'adminGroupGuests',
            tabs: [
                ['messages',        'adminTabMessages'],
                ['contactMessages', 'adminTabContactMessages'],
                ['comments',        'adminTabComments'],
                ['impressions',     'adminTabImpressions'],
                ['subscribers',     'adminTabSubscribers'],
                ['users',           'adminUsersTab'],
            ],
        },
        {
            label: 'adminGroupSystem',
            tabs: [
                ['analytics',     'adminTabAnalytics'],
                ['logs',          'adminTabLogs'],
                ['server',        'adminTabServer'],
                ['contact',       'adminTabContact'],
                ['booking-rules', 'adminTabBookingRules'],
                ['releases',      'adminTabReleases'],
            ],
        },
    ] as const satisfies readonly { label: TranslationKey; tabs: readonly (readonly [string, TranslationKey])[] }[];

    type AdminTab = (typeof TAB_GROUPS)[number]['tabs'][number][0];
    const TAB_IDS: readonly string[] = TAB_GROUPS.flatMap((g) => g.tabs.map(([id]) => id));

    let activeTab = $state<AdminTab>('registry');
    let gazetteSeed = $state<GazetteSeed | null>(null);
    let activeAuthorSubTab = $state<'profile' | 'texts'>('profile');
    let newOrdersCount = $state(0);
    let newCommissionsCount = $state(0);
    let pendingBookingsCount = $state(0);
    let pendingCommentsCount = $state(0);
    let pendingImpressionsCount = $state(0);

    // Unattended work waiting in a tab. Red = someone is waiting on an answer;
    // orange = something needs moderating before guests see it.
    const BADGE_TONE: Partial<Record<AdminTab, string>> = {
        orders: 'bg-red-500',
        commissions: 'bg-red-500',
        bookings: 'bg-red-500',
        comments: 'bg-orange-600',
        impressions: 'bg-orange-600',
    };
    function tabBadge(tab: AdminTab): number {
        switch (tab) {
            case 'orders':      return newOrdersCount;
            case 'commissions': return newCommissionsCount;
            case 'bookings':    return pendingBookingsCount;
            case 'comments':    return pendingCommentsCount;
            case 'impressions': return pendingImpressionsCount;
            default:            return 0;
        }
    }
    let searchQuery = $state('');
    let isDeleting = $state(false);
    let sidebarCollapsed = $state(false);

    let hasUnsaved = $derived(
        selectedFigurine !== null && JSON.stringify(selectedFigurine) !== savedSnapshot
    );

    let filteredFigurines = $derived(
        searchQuery.trim()
            ? figurines.filter(f => f.name.toLowerCase().includes(searchQuery.toLowerCase()))
            : figurines
    );

    let materialSuggestions = $derived(
        [...new Set(figurines.map(f => f.material).filter((v): v is string => !!v))].sort()
    );
    let techniqueSuggestions = $derived(
        [...new Set(figurines.map(f => f.technique).filter((v): v is string => !!v))].sort()
    );
    let dimensionsSuggestions = $derived(
        [...new Set(figurines.map(f => f.dimensions).filter((v): v is string => !!v))].sort()
    );

    const emptyFigurine: Figurine = {
        id: '',
        name: '',
        shortText: '',
        fullDescription: '',
        dimensions: '',
        material: '',
        technique: '',
        year: new Date().getFullYear(),
        passportNumber: '',
        edition: '',
        createdPeriod: '',
        careInstructions: '',
        provenanceNote: '',
        authenticityNote: '',
        includedItems: '',
        ambiencePath: null,
        videoUrl: null,
        secretText: '',
        status: 'available',
        sortOrder: 0,
        isVisible: true,
        isFeatured: false,
        series: null,
        displayLayout: null,
        displayConfig: null,
        catalogLists: null,
        openFromMin: null,
        openUntilMin: null,
        firstLookUntil: null,
        images: [],
        processSteps: [],
        relatedItems: []
    };

    // === Showing rooms (named shared windows) ===
    let showingRoomsList = $state<ShowingRoom[]>([]);

    async function loadShowingRooms() {
        try { showingRoomsList = await api.getShowingRooms(); } catch { return; }
        // A work pointing at a room that was just deleted falls back to always-open.
        if (selectedFigurine?.showingRoomId && !showingRoomsList.some((r) => r.id === selectedFigurine!.showingRoomId)) {
            selectedFigurine.showingRoomId = null;
        }
    }

    async function loadFigurines() {
        try {
            figurines = await api.getAllFigurinesAdmin();
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

    // True while a photo/video/audio/folder upload for the CURRENTLY selected figurine
    // is in flight. Switching selectedFigurine mid-upload used to be how a photo could
    // end up attached to the wrong figurine: handlePickFile/handleFolderUpload await the
    // file picker and the upload, then write into `selectedFigurine.images` — if the admin
    // clicked a different figurine in the list during that wait, that write landed on
    // whichever figurine was current by the time the upload finished, not the one the
    // upload was started for. Blocking the switch here is simpler and safer than trying
    // to retarget an in-flight upload to its original figurine.
    function uploadBusy(): boolean {
        return formBusy;
    }

    async function editFigurine(id: string) {
        if (uploadBusy()) { showMessage($t('adminMsgUploadInProgress'), 'error'); return; }
        if (hasUnsaved && !confirm($t('adminMsgUnsavedLeave'))) return;
        const full = await api.getFigurine(id);
        if (full) {
            selectedFigurine = { ...full };
            savedSnapshot = JSON.stringify(selectedFigurine);
        }
    }

    function createNew() {
        if (uploadBusy()) { showMessage($t('adminMsgUploadInProgress'), 'error'); return; }
        if (hasUnsaved && !confirm($t('adminMsgUnsavedLeave'))) return;
        selectedFigurine = { ...emptyFigurine, id: crypto.randomUUID(), sortOrder: figurines.length };
        savedSnapshot = '';
    }

    function duplicateFigurine(fig: FigurineListItem) {
        if (uploadBusy()) { showMessage($t('adminMsgUploadInProgress'), 'error'); return; }
        if (hasUnsaved && !confirm($t('adminMsgUnsavedLeave'))) return;
        api.getFigurine(fig.id).then(full => {
            if (!full) return;
            selectedFigurine = {
                ...full,
                id: crypto.randomUUID(),
                name: full.name + $t('adminRegistryCopySuffix'),
                // Drop the original's slug so the backend auto-generates one from
                // the new copy name instead of appending -2 to the source slug.
                slug: null,
                sortOrder: figurines.length,
                isVisible: false,
            };
            savedSnapshot = '';
        });
    }

    async function deleteFigurine(fig: { id: string }) {
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

    async function refreshAfterBulkOp() {
        await loadFigurines();
        if (selectedFigurine) {
            const fresh = await api.getFigurine(selectedFigurine.id);
            if (fresh) {
                selectedFigurine = { ...fresh };
                savedSnapshot = JSON.stringify(selectedFigurine);
            }
        }
    }

    async function runBulkOp(confirmMsg: string, action: () => Promise<{ affected: number }>) {
        if (!confirm(confirmMsg)) return;
        bulkBusy = true;
        try {
            const res = await action();
            await refreshAfterBulkOp();
            showMessage(`${$t('adminBulkDone')}: ${res.affected}`, 'success');
        } catch (e: unknown) {
            const msg = e instanceof Error ? e.message : String(e);
            showMessage($t('adminMsgError') + msg, 'error');
        } finally {
            bulkBusy = false;
        }
    }

    async function bulkClearDarkness() {
        await runBulkOp($t('adminBulkClearDarknessConfirm'), () => api.bulkClearDarkness());
    }

    async function bulkClearShowings() {
        await runBulkOp($t('adminBulkClearShowingsConfirm'), () => api.bulkClearShowings());
    }

    async function bulkResetParallax() {
        await runBulkOp($t('adminBulkResetParallaxConfirm'), () => api.bulkResetParallax());
    }

    async function bulkSetParallax() {
        await runBulkOp($t('adminBulkSetParallaxConfirm'), () => api.bulkSetParallax(bulkParallaxValue));
    }

    async function bulkRecalculateParallax() {
        if (!confirm($t('adminBulkRecalculateParallaxConfirm'))) return;
        bulkBusy = true;
        try {
            const res = await api.bulkRecalculateParallax();
            await refreshAfterBulkOp();
            showMessage(`${$t('adminBulkDone')}: ${res.generated}/${res.results.length}`, 'success');
        } catch (e: unknown) {
            const msg = e instanceof Error ? e.message : String(e);
            showMessage($t('adminMsgError') + msg, 'error');
        } finally {
            bulkBusy = false;
        }
    }

    async function bulkSetSecondAngle() {
        await runBulkOp($t('adminBulkSetSecondAngleConfirm'), () => api.bulkSetSecondAngle());
    }

    // Rebuild the "Хранитель" semantic-search vectors for every visible work.
    // Server-only (candle). Cheap after the first run — unchanged text is skipped.
    async function reindexKeeper() {
        if (!confirm($t('adminReindexKeeperConfirm'))) return;
        bulkBusy = true;
        try {
            const res = await api.reindexEmbeddings();
            showMessage(
                `${$t('adminReindexKeeperDone')}: ${res.indexed} / ${res.total}` +
                    (res.failed ? ` · ⚠ ${res.failed}` : ''),
                'success',
            );
        } catch (e: unknown) {
            const msg = e instanceof Error ? e.message : String(e);
            showMessage($t('adminMsgError') + msg, 'error');
        } finally {
            bulkBusy = false;
        }
    }

    // The form saved the work: the snapshot it was compared against is now the
    // saved state, and the registry list may show a new name/status.
    async function afterFigurineSaved() {
        savedSnapshot = JSON.stringify(selectedFigurine);
        await loadFigurines();
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

    function layGazette(seed: GazetteSeed) {
        gazetteSeed = seed;
        activeTab = 'gazette';
    }

    onMount(() => {
        // Check session
        const session = sessionStorage.getItem('gotiga_admin');
        const persisted = localStorage.getItem('gotiga_admin_persist');
        const hasKey = localStorage.getItem('gotiga_api_key') || sessionStorage.getItem('gotiga_api_key');
        if ((session === '1' || persisted === '1') && hasKey) {
            isAuthenticated = true;
            loadFigurines();
            loadShowingRooms();
        }
        // Restore sidebar collapsed state
        sidebarCollapsed = localStorage.getItem('gotiga_admin_sidebar_collapsed') === '1';
        // Hash-based tab routing (e.g. Telegram notification links)
        const hash = window.location.hash.replace('#', '');
        if (TAB_IDS.includes(hash)) {
            activeTab = hash as AdminTab;
        }
    });
</script>

<!-- ===== LOGIN SCREEN ===== -->
{#if !isAuthenticated}
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
<div class="h-screen bg-[#f8f1e7] text-[#34251c] font-cinzel flex flex-row overflow-hidden">

    <!-- Sidebar -->
    <aside class="shrink-0 flex flex-col border-r border-[#34251c]/20 bg-[#f2e8da] overflow-hidden transition-[width] duration-200 {sidebarCollapsed ? 'w-0 border-r-0' : 'w-52'}">
      <div class="w-52 flex flex-col h-full overflow-hidden">

        <!-- Branding -->
        <div class="px-4 pt-5 pb-4 border-b border-[#34251c]/15 flex items-start justify-between">
            <div>
                <h1 class="text-lg font-gothic leading-tight">{$t('adminTitle')}</h1>
                <p class="text-[9px] tracking-[0.1em] text-[#5f4636] uppercase mt-1">{$t('adminSubtitle')}</p>
            </div>
            <button
                onclick={() => { sidebarCollapsed = true; localStorage.setItem('gotiga_admin_sidebar_collapsed', '1'); }}
                class="mt-1.5 shrink-0 w-5 h-5 flex items-center justify-center text-[#5f4636]/40 hover:text-[#34251c] hover:bg-[#34251c]/8 rounded transition-colors text-sm leading-none"
                title="Collapse sidebar">‹</button>
        </div>

        <!-- Nav -->
        <nav class="flex-1 px-3 py-4 flex flex-col gap-5 overflow-y-auto">
            {#each TAB_GROUPS as group}
              <div>
                <span class="block px-2 mb-1 text-[8px] uppercase tracking-[0.12em] text-[#5f4636]/50 font-medium">{$t(group.label)}</span>
                {#each group.tabs as [tab, labelKey]}
                  <button
                    onclick={() => activeTab = tab}
                    class="w-full text-left flex items-center justify-between px-2 py-1.5 text-xs uppercase tracking-wide transition-colors
                           {activeTab === tab
                             ? 'border-l-2 border-[#c65f3c] bg-[#c65f3c]/10 text-[#34251c] pl-[6px]'
                             : 'border-l-2 border-transparent text-[#5f4636] hover:text-[#34251c] hover:bg-[#34251c]/5 pl-[6px]'}"
                  >
                    <span>{$t(labelKey)}</span>
                    {#if tabBadge(tab) > 0 && activeTab !== tab}
                      <span class="inline-flex items-center justify-center min-w-[16px] h-4 px-1 rounded-full {BADGE_TONE[tab]} text-white text-[9px] font-bold leading-none">
                        {tabBadge(tab) > 99 ? '99+' : tabBadge(tab)}
                      </span>
                    {/if}
                  </button>
                {/each}
              </div>
            {/each}
        </nav>

        <!-- Bottom actions -->
        <div class="px-3 py-4 border-t border-[#34251c]/15 flex flex-col gap-2">
            <LangSwitcher />
            <button onclick={() => showSettings = true} class="btn-gothic text-[10px] w-full text-left opacity-75 hover:opacity-100" title={$t('adminSettings')}>⚙ {$t('adminSettings')}</button>
            <button onclick={handleLogout} class="btn-gothic text-[10px] w-full text-left opacity-75 hover:opacity-100">{$t('adminLogout')}</button>
            <a href="/" class="btn-gothic text-[10px] opacity-60 hover:opacity-100">{$t('adminToMuseum')}</a>
        </div>

      </div><!-- /inner w-52 -->
    </aside>

    <SettingsModal isOpen={showSettings} onClose={() => showSettings = false} />

    <!-- Content -->
    <div class="flex-1 overflow-hidden p-6 relative">

        {#if sidebarCollapsed}
        <button
            onclick={() => { sidebarCollapsed = false; localStorage.setItem('gotiga_admin_sidebar_collapsed', '0'); }}
            class="absolute top-3 left-1 z-20 w-6 h-6 flex items-center justify-center bg-[#f2e8da] border border-[#34251c]/20 text-[#5f4636] hover:text-[#34251c] hover:bg-[#e8dece] transition-colors text-sm leading-none"
            title="Expand sidebar">›</button>
        {/if}

        {#if activeTab === 'registry'}
        <div class="grid grid-cols-12 gap-6 h-full" in:fade>

            <!-- Sidebar -->
            <aside class="col-span-3 flex flex-col gap-3 border-r border-[#34251c]/10 pr-5 overflow-hidden">
                <div class="flex justify-between items-center shrink-0">
                    <h2 class="text-xs uppercase tracking-wide text-[#5f4636]">{$t('adminRegistryHeading')}</h2>
                    <div class="flex items-center gap-1.5">
                        <button onclick={() => bulkPanelOpen = !bulkPanelOpen}
                            class="text-[10px] uppercase tracking-wide px-2 py-1 border transition-colors
                                {bulkPanelOpen ? 'bg-[#34251c]/10 border-[#34251c]/30 text-[#34251c]' : 'border-[#34251c]/15 text-[#5f4636] hover:border-[#34251c]/30'}"
                            title={$t('adminBulkHeading')}>⚙ {$t('adminBulkHeading')}</button>
                        <button onclick={createNew} class="btn-gothic text-[10px]">{$t('adminRegistryNew')}</button>
                    </div>
                </div>

                {#if bulkPanelOpen}
                <div class="shrink-0 border border-[#34251c]/15 bg-[#f2e8da] p-3 space-y-2 text-[10px]">
                    <div class="uppercase tracking-wide text-[#5f4636] opacity-75">{$t('adminBulkScope')}</div>

                    <button onclick={bulkClearDarkness} disabled={bulkBusy}
                        class="w-full text-left px-2 py-1.5 border border-[#34251c]/15 hover:border-[#34251c]/40 hover:bg-[#34251c]/5 disabled:opacity-40 transition-colors">
                        {$t('adminBulkClearDarkness')}
                    </button>

                    <button onclick={bulkResetParallax} disabled={bulkBusy}
                        class="w-full text-left px-2 py-1.5 border border-[#34251c]/15 hover:border-[#34251c]/40 hover:bg-[#34251c]/5 disabled:opacity-40 transition-colors">
                        {$t('adminBulkResetParallax')}
                    </button>

                    <div class="flex items-center gap-2 px-2 py-1.5 border border-[#34251c]/15">
                        <input type="range" min="0" max="1" step="0.01" bind:value={bulkParallaxValue}
                            class="flex-1" disabled={bulkBusy} />
                        <span class="w-8 text-right tabular-nums">{bulkParallaxValue.toFixed(2)}</span>
                        <button onclick={bulkSetParallax} disabled={bulkBusy}
                            class="uppercase text-[#5f4636] hover:text-[#34251c] disabled:opacity-40 transition-colors shrink-0">
                            {$t('adminBulkApply')}
                        </button>
                    </div>

                    <button onclick={bulkSetSecondAngle} disabled={bulkBusy}
                        class="w-full text-left px-2 py-1.5 border border-[#34251c]/15 hover:border-[#34251c]/40 hover:bg-[#34251c]/5 disabled:opacity-40 transition-colors">
                        {$t('adminBulkSetSecondAngle')}
                    </button>

                    <button onclick={bulkRecalculateParallax} disabled={bulkBusy}
                        class="w-full text-left px-2 py-1.5 border border-[#34251c]/15 hover:border-[#34251c]/40 hover:bg-[#34251c]/5 disabled:opacity-40 transition-colors">
                        {$t('adminBulkRecalculateParallax')}
                    </button>

                    <button onclick={reindexKeeper} disabled={bulkBusy}
                        class="w-full text-left px-2 py-1.5 border border-[#34251c]/15 hover:border-[#34251c]/40 hover:bg-[#34251c]/5 disabled:opacity-40 transition-colors">
                        {$t('adminReindexKeeper')}
                    </button>

                    <button onclick={bulkClearShowings} disabled={bulkBusy}
                        class="w-full text-left px-2 py-1.5 border border-[#34251c]/15 hover:border-red-700/40 hover:bg-red-50 disabled:opacity-40 transition-colors">
                        {$t('adminBulkClearShowings')}
                    </button>
                </div>
                {/if}

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
            <main class="col-span-9 bg-[#fff9f0]/50 border border-[#34251c]/10 relative h-full flex flex-col overflow-hidden">
                {#if selectedFigurine}
                    <FigurineForm
                        figurine={selectedFigurine}
                        {figurines}
                        showingRooms={showingRoomsList}
                        unsaved={hasUnsaved}
                        deleting={isDeleting}
                        bind:busy={formBusy}
                        onSaved={afterFigurineSaved}
                        onDelete={() => deleteFigurine(selectedFigurine!)}
                        onCancel={cancelEdit}
                        onMessage={showMessage}
                        onLayGazette={layGazette}
                    />
                {:else}
                    <div class="h-full flex flex-col items-center justify-center text-[#5f4636] opacity-60">
                        <span class="text-5xl mb-4">📜</span>
                        <p class="text-sm">{$t('adminRegistrySelectPrompt')}</p>
                    </div>
                {/if}
            </main>
        </div>

        {:else if activeTab === 'slugs'}
            <SlugsPanel {figurines} onReload={loadFigurines} onMessage={showMessage} />

        {:else if activeTab === 'rooms'}
            <ShowingRoomsPanel rooms={showingRoomsList} onReload={loadShowingRooms} />

        {:else if activeTab === 'home'}
            <HomeContentEditor />

        {:else if activeTab === 'reel-theme'}
            <div in:fade class="h-full overflow-hidden"><ReelThemePanel /></div>

        {:else if activeTab === 'home-layout'}
            <div in:fade class="h-full overflow-y-auto"><HomeLayoutEditor /></div>

        {:else if activeTab === 'programme'}
            <div in:fade class="h-full overflow-auto"><ProgrammePanel /></div>

        {:else if activeTab === 'gazette'}
            <div in:fade class="h-full overflow-hidden"><GazettePanel seed={gazetteSeed} onSeedConsumed={() => (gazetteSeed = null)} /></div>

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
            <div in:fade class="h-full"><OrdersPanel onNewCount={(n: number) => newOrdersCount = n} /></div>
        {:else if activeTab === 'commissions'}
            <div in:fade class="h-full"><CommissionsPanel onNewCount={(n: number) => newCommissionsCount = n} /></div>
        {:else if activeTab === 'showings'}
            <div in:fade class="h-full"><ShowingsPanel onLayGazette={layGazette} /></div>
        {:else if activeTab === 'bookings'}
            <div in:fade class="h-full"><BookingsPanel onPendingCount={(n: number) => pendingBookingsCount = n} /></div>
        {:else if activeTab === 'releases'}
            <div in:fade class="h-full"><ReleaseManager /></div>
        {:else if activeTab === 'analytics'}
            <div in:fade class="h-full overflow-hidden"><AnalyticsPanel /></div>
        {:else if activeTab === 'marks'}
            <div in:fade class="h-full overflow-y-auto"><AttentionMarksPanel /></div>
        {:else if activeTab === 'users'}
            <div in:fade class="h-full overflow-y-auto"><UsersPanel /></div>
        {:else if activeTab === 'comments'}
            <div in:fade class="h-full overflow-y-auto"><CommentsPanel onPendingCount={(n) => pendingCommentsCount = n} /></div>
        {:else if activeTab === 'impressions'}
            <div in:fade class="h-full overflow-y-auto"><ImpressionsPanel onPendingCount={(n) => pendingImpressionsCount = n} /></div>
        {:else if activeTab === 'server'}
            <div in:fade class="h-full overflow-y-auto"><SmtpSettingsPanel /></div>
        {:else if activeTab === 'logs'}
            <div in:fade class="h-full overflow-hidden"><LogsPanel /></div>
        {:else if activeTab === 'waitlist'}
            <div in:fade class="h-full"><WaitlistPanel /></div>
        {:else if activeTab === 'subscribers'}
            <div in:fade class="h-full"><SubscribersPanel /></div>
        {:else if activeTab === 'contactMessages'}
            <div in:fade class="h-full"><ContactMessagesPanel /></div>
        {:else if activeTab === 'messages'}
            <div in:fade class="h-full overflow-y-auto"><MessagesPanel /></div>
        {:else if activeTab === 'booking-rules'}
            <div in:fade class="h-full overflow-y-auto"><BookingRulesPanel /></div>
        {:else if activeTab === 'contact'}
            <div in:fade class="h-full overflow-y-auto"><ContactSettingsPanel /></div>
        {:else if activeTab === 'design'}
            <div in:fade class="h-full overflow-hidden"><DesignEditor /></div>
        {:else if activeTab === 'copy'}
            <div in:fade class="h-full"><CopyEditor /></div>
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
