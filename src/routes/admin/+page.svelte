<script lang="ts">
    import { setOwnerBrowser } from '$lib/analytics';
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
    import HeaderDecorPanel from '$lib/components/admin/HeaderDecorPanel.svelte';
    import HeaderPortraitPanel from '$lib/components/admin/HeaderPortraitPanel.svelte';
    import ReelThemePanel from '$lib/components/admin/ReelThemePanel.svelte';
    import CopyEditor from '$lib/components/admin/CopyEditor.svelte';
    import ProgrammePanel from '$lib/components/admin/ProgrammePanel.svelte';
    import GazettePanel from '$lib/components/admin/GazettePanel.svelte';
    import TalesPanel from '$lib/components/admin/TalesPanel.svelte';
    import BattlesPanel from '$lib/components/admin/BattlesPanel.svelte';
    import StudioPanel from '$lib/components/admin/StudioPanel.svelte';
    import LogsPanel from '$lib/components/admin/LogsPanel.svelte';
    import ToolsPanel from '$lib/components/admin/ToolsPanel.svelte';
    import { t, registerAdminDicts, type TranslationKey } from '$lib/i18n';
    import { adminDicts } from '$lib/i18n/admin-dicts';

    registerAdminDicts(adminDicts());
    import FigurineForm from '$lib/components/admin/FigurineForm.svelte';
    import SlugsPanel from '$lib/components/admin/SlugsPanel.svelte';
    import ShowingRoomsPanel from '$lib/components/admin/ShowingRoomsPanel.svelte';
    import ReadinessPanel from '$lib/components/admin/ReadinessPanel.svelte';
    import { adminReadiness, workGapText, workGapHint } from '$lib/stores/admin-readiness.svelte';
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
            // The author's own browser stops counting as a visitor (the switch
            // is in the Analytics panel).
            setOwnerBrowser(true);
            await loadFigurines();
            await loadShowingRooms();
            void adminReadiness.load();
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
    /** Вкладка формы работы, которую просит помощник («Что не доделано»). */
    let formTabAsk = $state<'media' | 'object' | null>(null);

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
                ['todo',      'adminTabTodo'],
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
                ['tales',       'adminTabTales'],
                ['battles',     'adminTabBattles'],
                ['studio',      'adminTabStudio'],
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
                ['tools',         'adminTabTools'],
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
    let activeAuthorSubTab = $state<'profile' | 'texts' | 'header' | 'portrait'>('profile');
    let newOrdersCount = $state(0);
    let newCommissionsCount = $state(0);
    let pendingBookingsCount = $state(0);
    let pendingCommentsCount = $state(0);
    let pendingImpressionsCount = $state(0);
    /** Допуск + тройка + жалобы одним числом: на вкладке важно «ждёт ли
     *  что-нибудь», а чего именно — видно уже внутри. */
    let studioWaitingCount = $state(0);

    // Unattended work waiting in a tab. Red = someone is waiting on an answer;
    // orange = something needs moderating before guests see it.
    const BADGE_TONE: Partial<Record<AdminTab, string>> = {
        orders: 'bg-red-500',
        commissions: 'bg-red-500',
        bookings: 'bg-red-500',
        comments: 'bg-orange-600',
        impressions: 'bg-orange-600',
        // Оранжевый — модерация: кто-то ждёт не ответа, а решения.
        studio: 'bg-orange-600',
        // Янтарный — никто не ждёт, но что-то забыли доделать.
        todo: 'bg-amber-600',
    };
    function tabBadge(tab: AdminTab): number {
        switch (tab) {
            case 'orders':      return newOrdersCount;
            case 'commissions': return newCommissionsCount;
            case 'bookings':    return pendingBookingsCount;
            case 'comments':    return pendingCommentsCount;
            case 'impressions': return pendingImpressionsCount;
            case 'studio':      return studioWaitingCount;
            case 'todo':        return adminReadiness.count;
            default:            return 0;
        }
    }
    let searchQuery = $state('');
    let isDeleting = $state(false);
    let sidebarCollapsed = $state(false);

    let hasUnsaved = $derived(
        selectedFigurine !== null && JSON.stringify(selectedFigurine) !== savedSnapshot
    );

    // Registry list: thumbnails, a status/problem filter and a sort. The view and sort are
    // per-browser conveniences (localStorage), never state anyone else depends on.
    type RegistryView = 'list' | 'grid';
    type RegistryFilter = 'all' | FigurineListItem['status'] | 'hidden' | 'gaps';
    type RegistrySort = 'site' | 'newest' | 'name';
    let registryView = $state<RegistryView>('list');
    let registryFilter = $state<RegistryFilter>('all');
    let registrySort = $state<RegistrySort>('site');

    function setRegistryView(v: RegistryView) {
        registryView = v;
        try { localStorage.setItem('gotiga_admin_registry_view', v); } catch { /* private window */ }
    }
    function setRegistrySort(v: RegistrySort) {
        registrySort = v;
        try { localStorage.setItem('gotiga_admin_registry_sort', v); } catch { /* private window */ }
    }

    const REGISTRY_STATUSES: { key: FigurineListItem['status']; label: TranslationKey }[] = [
        { key: 'available', label: 'adminFieldStatusAvail' },
        { key: 'reserved', label: 'adminFieldStatusRes' },
        { key: 'sold', label: 'adminFieldStatusSold' },
        { key: 'in_progress', label: 'adminFieldStatusWip' },
    ];
    const STATUS_DOT: Record<FigurineListItem['status'], string> = {
        available: 'bg-emerald-700/80',
        reserved: 'bg-amber-600',
        sold: 'bg-[#34251c]/45',
        in_progress: 'bg-[#c65f3c]',
    };

    function registryMatches(f: FigurineListItem, filter: RegistryFilter): boolean {
        if (filter === 'all') return true;
        if (filter === 'hidden') return f.isVisible === false;
        if (filter === 'gaps') return adminReadiness.byWork.has(f.id);
        return f.status === filter;
    }
    let registryCounts = $derived.by(() => {
        const c: Record<string, number> = { all: figurines.length, hidden: 0, gaps: 0 };
        for (const f of figurines) {
            c[f.status] = (c[f.status] ?? 0) + 1;
            if (f.isVisible === false) c.hidden++;
            if (adminReadiness.byWork.has(f.id)) c.gaps++;
        }
        return c;
    });

    let filteredFigurines = $derived.by(() => {
        const q = searchQuery.trim().toLowerCase();
        const list = figurines.filter(
            (f) => registryMatches(f, registryFilter) && (!q || f.name.toLowerCase().includes(q))
        );
        // `figurines` already arrives in site order (sortOrder, newest first).
        if (registrySort === 'newest') {
            return list.slice().sort((a, b) => (b.createdAt ?? '').localeCompare(a.createdAt ?? ''));
        }
        if (registrySort === 'name') return list.slice().sort((a, b) => a.name.localeCompare(b.name));
        return list;
    });

    function thumbSrc(path: string | null | undefined): string {
        if (!path) return '';
        if (path.startsWith('http')) return path;
        try {
            const serverUrl = localStorage.getItem('gotiga_server_url') ?? '';
            return serverUrl && path.startsWith('/') ? `${serverUrl}${path}` : path;
        } catch {
            return path;
        }
    }

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

    // ── Registry order ────────────────────────────────────────────────────
    // The order is one list: dragging a work, ▲▼ and "renumber" all end in `applyOrder`,
    // which writes 1, 2, 3, … for the whole list. Nudging one work's number by ±1 (the old
    // ▲▼) left ties, and a tie is settled by creation date — so a work could stay put.
    async function applyOrder(ids: string[]) {
        const byId = new Map(figurines.map((f) => [f.id, f]));
        const before = figurines;
        figurines = ids.map((id, i) => ({ ...byId.get(id)!, sortOrder: i + 1 }));
        try {
            await api.reorderFigurines(ids);
            syncOpenWorkOrder(ids);
            showMessage($t('adminRegistryReordered'), 'success');
        } catch (e) {
            figurines = before;
            showMessage($t('adminRegistryReorderFailed') + e, 'error');
        }
    }

    /** The open form holds its own copy of sortOrder; saving it later must not undo the move. */
    function syncOpenWorkOrder(ids: string[]) {
        if (!selectedFigurine) return;
        const idx = ids.indexOf(selectedFigurine.id);
        if (idx < 0) return;
        selectedFigurine.sortOrder = idx + 1;
        if (savedSnapshot) {
            try {
                const snap = JSON.parse(savedSnapshot);
                snap.sortOrder = idx + 1;
                savedSnapshot = JSON.stringify(snap);
            } catch { /* leave the snapshot */ }
        }
    }

    /** Put `id` just before/after `targetId` in the whole registry order and save it. */
    async function moveTo(id: string, targetId: string, where: 'before' | 'after') {
        if (id === targetId) return;
        const ids = figurines.map((f) => f.id).filter((x) => x !== id);
        const at = ids.indexOf(targetId);
        if (at < 0) return;
        ids.splice(where === 'before' ? at : at + 1, 0, id);
        if (ids.every((x, i) => x === figurines[i].id)) return;
        await applyOrder(ids);
    }

    /** ▲▼: swap with the neighbour as the list currently shows it. */
    async function moveFigurine(id: string, direction: 1 | -1) {
        const shown = registrySort === 'site' ? filteredFigurines : figurines;
        const idx = shown.findIndex((f) => f.id === id);
        const neighbour = shown[idx + direction];
        if (idx < 0 || !neighbour) return;
        await moveTo(id, neighbour.id, direction === -1 ? 'before' : 'after');
    }

    function renumberRegistry() {
        const msg = $t('adminRegistryRenumberConfirm').replace('{n}', String(figurines.length));
        if (!confirm(msg)) return;
        void applyOrder(figurines.map((f) => f.id));
    }

    // Drag and drop. Only meaningful when the list shows the whole registry in site order:
    // dropping inside a filtered or re-sorted view would have no single place to land.
    let canDrag = $derived(registrySort === 'site' && registryFilter === 'all' && !searchQuery.trim());
    let dragId = $state<string | null>(null);
    let dropMark = $state<{ id: string; where: 'before' | 'after' } | null>(null);

    function dragOver(e: DragEvent, id: string) {
        if (!dragId || dragId === id) return;
        e.preventDefault();
        const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
        const where = registryView === 'grid'
            ? (e.clientX < r.left + r.width / 2 ? 'before' : 'after')
            : (e.clientY < r.top + r.height / 2 ? 'before' : 'after');
        if (dropMark?.id !== id || dropMark.where !== where) dropMark = { id, where };
    }
    function dragEnd() {
        dragId = null;
        dropMark = null;
    }
    async function drop(e: DragEvent) {
        e.preventDefault();
        const from = dragId;
        const mark = dropMark;
        dragEnd();
        if (from && mark) await moveTo(from, mark.id, mark.where);
    }

    // Ties (or a start other than 1) are what "renumber" repairs.
    let registryHasTies = $derived(
        new Set(figurines.map((f) => f.sortOrder ?? 0)).size !== figurines.length
    );

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

    /** Открыть работу в форме. `false` — не открылась (идёт загрузка, отказались
     *  бросить правку, работы нет). */
    async function editFigurine(id: string): Promise<boolean> {
        if (uploadBusy()) { showMessage($t('adminMsgUploadInProgress'), 'error'); return false; }
        if (hasUnsaved && !confirm($t('adminMsgUnsavedLeave'))) return false;
        const full = await api.getFigurineFresh(id);
        if (!full) return false;
        selectedFigurine = { ...full };
        savedSnapshot = JSON.stringify(selectedFigurine);
        return true;
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
        api.getFigurineFresh(fig.id).then(full => {
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
            const fresh = await api.getFigurineFresh(selectedFigurine.id);
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
        adminReadiness.afterSave();
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
        // A tale is a gazette leaf, but it is not written in the newsroom —
        // «Записать небылицу» from a work must land at the writing desk.
        activeTab = seed.kind === 'tale' ? 'tales' : 'gazette';
    }

    onMount(() => {
        // Check session
        const session = sessionStorage.getItem('gotiga_admin');
        const persisted = localStorage.getItem('gotiga_admin_persist');
        const hasKey = localStorage.getItem('gotiga_api_key') || sessionStorage.getItem('gotiga_api_key');
        if ((session === '1' || persisted === '1') && hasKey) {
            isAuthenticated = true;
            setOwnerBrowser(true);
            loadFigurines();
            loadShowingRooms();
            void adminReadiness.load();
            // Значок студии спрашивается сразу: остальные вкладки сообщают своё
            // число, только когда их откроют, а тут «ждёт ли что-нибудь» нужно
            // знать ДО того, как зайдёшь.
            api.adminStudioWaiting()
                .then((w) => (studioWaitingCount = w.admissions + w.queue + w.reports))
                .catch(() => {});
        }
        // Restore sidebar collapsed state
        sidebarCollapsed = localStorage.getItem('gotiga_admin_sidebar_collapsed') === '1';
        try {
            const v = localStorage.getItem('gotiga_admin_registry_view');
            if (v === 'list' || v === 'grid') registryView = v;
            const so = localStorage.getItem('gotiga_admin_registry_sort');
            if (so === 'site' || so === 'newest' || so === 'name') registrySort = so;
        } catch { /* private window */ }
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

                <!-- Filters: status and "what is missing" -->
                <div class="shrink-0 flex flex-wrap gap-1">
                    {#each [{ key: 'all', label: $t('adminRegistryFilterAll') }, ...REGISTRY_STATUSES.map((s) => ({ key: s.key, label: $t(s.label) })), { key: 'hidden', label: $t('adminRegistryHidden') }, { key: 'gaps', label: $t('adminRegistryGaps') }] as chip (chip.key)}
                        {@const n = registryCounts[chip.key] ?? 0}
                        {#if chip.key === 'all' || n > 0 || registryFilter === chip.key}
                            <button
                                onclick={() => registryFilter = chip.key as RegistryFilter}
                                class="text-[9px] uppercase tracking-wide px-1.5 py-1 border transition-colors
                                    {registryFilter === chip.key
                                        ? 'bg-[#34251c] border-[#34251c] text-[#f8f1e7]'
                                        : 'border-[#34251c]/15 text-[#5f4636] hover:border-[#34251c]/40'}">
                                {chip.label} <span class="opacity-60 tabular-nums">{n}</span>
                            </button>
                        {/if}
                    {/each}
                </div>

                <!-- View + sort -->
                <div class="shrink-0 flex items-center justify-between gap-2">
                    <select
                        value={registrySort}
                        onchange={(e) => setRegistrySort(e.currentTarget.value as RegistrySort)}
                        class="min-w-0 bg-[#f8f1e7] border border-[#34251c]/15 px-1.5 py-1 text-[10px] uppercase tracking-wide text-[#5f4636] outline-none focus:border-[#34251c]/40">
                        <option value="site">{$t('adminRegistrySortSite')}</option>
                        <option value="newest">{$t('adminRegistrySortNewest')}</option>
                        <option value="name">{$t('adminRegistrySortName')}</option>
                    </select>
                    {#if registryHasTies}
                        <button onclick={renumberRegistry}
                            title={$t('adminRegistryRenumberHint')}
                            class="shrink-0 text-[10px] uppercase tracking-wide px-1.5 py-1 border border-amber-700/40 bg-amber-50 text-amber-900 hover:bg-amber-100 transition-colors">
                            {$t('adminRegistryRenumber')}
                        </button>
                    {/if}
                    <div class="flex border border-[#34251c]/15 shrink-0" role="group">
                        {#each [{ v: 'list', label: $t('adminRegistryViewList'), icon: '☰' }, { v: 'grid', label: $t('adminRegistryViewGrid'), icon: '▦' }] as m (m.v)}
                            <button
                                onclick={() => setRegistryView(m.v as RegistryView)}
                                title={m.label}
                                aria-label={m.label}
                                aria-pressed={registryView === m.v}
                                class="px-2 py-1 text-xs leading-none transition-colors
                                    {registryView === m.v ? 'bg-[#34251c]/10 text-[#34251c]' : 'text-[#5f4636] hover:text-[#34251c]'}">{m.icon}</button>
                        {/each}
                    </div>
                </div>

                {#if !canDrag}
                    <div class="shrink-0 -mt-1 text-[9px] leading-snug text-[#5f4636]/60">{$t('adminRegistryDragOff')}</div>
                {/if}

                <div class="flex-1 overflow-y-auto pr-1 {registryView === 'grid' ? 'grid grid-cols-2 gap-2 content-start' : 'space-y-1'}">
                    {#each filteredFigurines as fig (fig.id)}
                        {@const selected = selectedFigurine?.id === fig.id}
                        {@const gapsOf = adminReadiness.work(fig.id)}
                        {@const mark = dropMark?.id === fig.id ? dropMark.where : null}
                        <!-- svelte-ignore a11y_no_static_element_interactions -->
                        <div class="relative group/row {registryView === 'grid' ? '' : 'flex'}
                                {dragId === fig.id ? 'opacity-40' : ''}
                                {mark && registryView === 'grid' ? (mark === 'before' ? 'shadow-[-3px_0_0_0_#c65f3c]' : 'shadow-[3px_0_0_0_#c65f3c]') : ''}
                                {mark && registryView !== 'grid' ? (mark === 'before' ? 'shadow-[0_-3px_0_0_#c65f3c]' : 'shadow-[0_3px_0_0_#c65f3c]') : ''}"
                            draggable={canDrag}
                            title={canDrag ? $t('adminRegistryDragHint') : undefined}
                            ondragstart={(e) => { dragId = fig.id; e.dataTransfer?.setData('text/plain', fig.id); if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'; }}
                            ondragover={(e) => dragOver(e, fig.id)}
                            ondrop={drop}
                            ondragend={dragEnd}>
                            <button
                                onclick={() => editFigurine(fig.id)}
                                class="text-left border transition-colors min-w-0
                                    {registryView === 'grid' ? 'w-full flex flex-col' : 'flex-1 flex items-center gap-3 p-1.5'}
                                    {selected ? 'bg-[#34251c]/5 border-[#34251c]/40' : 'border-transparent hover:border-[#34251c]/25 hover:bg-[#34251c]/[0.03]'}"
                            >
                                <!-- Thumbnail -->
                                <span class="relative shrink-0 overflow-hidden bg-[#f1e3d1] border border-[#34251c]/10
                                    {registryView === 'grid' ? 'w-full aspect-[4/5]' : 'w-14 h-[4.5rem]'}
                                    {fig.status === 'sold' ? 'saturate-[0.55]' : ''} {fig.isVisible === false ? 'opacity-50' : ''}">
                                    {#if fig.faceImageUrl}
                                        <img src={thumbSrc(fig.faceImageUrl)} alt="" loading="lazy" decoding="async"
                                            class="w-full h-full object-cover" />
                                    {:else}
                                        <span class="absolute inset-0 flex items-center justify-center text-[#34251c]/25 text-lg">◌</span>
                                    {/if}
                                    <span class="absolute top-1 left-1 w-2 h-2 rounded-full ring-1 ring-[#f8f1e7] {STATUS_DOT[fig.status]}"
                                        title={fig.status}></span>
                                </span>

                                <!-- Text -->
                                <span class="min-w-0 flex-1 {registryView === 'grid' ? 'px-1.5 py-1.5' : ''}">
                                    <span class="block text-xs font-bold leading-snug text-[#34251c] group-hover/row:text-[#6f3b24] transition-colors line-clamp-2">
                                        {fig.name}
                                        {#if hasUnsaved && selected}<span class="text-amber-700 ml-1">*</span>{/if}
                                    </span>
                                    <span class="mt-0.5 flex flex-wrap items-center gap-x-2 gap-y-0.5 text-[9px] uppercase tracking-wide text-[#5f4636]/80">
                                        <span>{fig.status === 'available' ? $t('adminFieldStatusAvail') : fig.status === 'reserved' ? $t('adminFieldStatusRes') : fig.status === 'sold' ? $t('adminFieldStatusSold') : $t('adminFieldStatusWip')}</span>
                                        <span class="tabular-nums opacity-70" title={$t('adminRegistryOrderNo')}>№{fig.sortOrder ?? 0}</span>
                                        {#if fig.isVisible === false}<span class="px-1 border border-[#34251c]/30 text-[#34251c]/80">{$t('adminRegistryHidden')}</span>{/if}
                                        {#each gapsOf?.gaps ?? [] as gap (gap)}
                                            <span class="normal-case tracking-normal text-amber-800" title={workGapHint($t, gap)}>{workGapText($t, gap, gapsOf!)}</span>
                                        {/each}
                                    </span>
                                </span>
                            </button>

                            <!-- Actions (on hover / keyboard focus) -->
                            <div class="absolute right-1 flex bg-[#f8f1e7]/95 {registryView === 'grid' ? 'top-1' : 'bottom-1'} border border-[#34251c]/15 opacity-0 group-hover/row:opacity-100 focus-within:opacity-100 transition-opacity">
                                <button onclick={() => moveFigurine(fig.id, -1)}
                                    class="w-6 h-6 hover:bg-[#34251c]/10 text-[#5f4636] hover:text-[#34251c] text-[9px] flex items-center justify-center"
                                    title={$t('adminRegistryTooltipUp')}>▲</button>
                                <button onclick={() => moveFigurine(fig.id, 1)}
                                    class="w-6 h-6 hover:bg-[#34251c]/10 text-[#5f4636] hover:text-[#34251c] text-[9px] flex items-center justify-center"
                                    title={$t('adminRegistryTooltipDown')}>▼</button>
                                <button onclick={() => duplicateFigurine(fig)}
                                    class="w-6 h-6 hover:bg-[#34251c]/10 text-[#5f4636] hover:text-amber-700 text-[10px] flex items-center justify-center"
                                    title={$t('adminRegistryDuplicate')}>⎘</button>
                                <button onclick={() => deleteFigurine(fig)} disabled={isDeleting}
                                    class="w-6 h-6 hover:bg-red-50 text-[#5f4636] hover:text-red-700 text-[9px] flex items-center justify-center"
                                    title={$t('adminRegistryDelete')}>✕</button>
                            </div>
                        </div>
                    {/each}

                    {#if filteredFigurines.length === 0}
                        <div class="col-span-full text-center text-[#5f4636] text-xs py-6 opacity-70">
                            {searchQuery || registryFilter !== 'all' ? $t('adminRegistryNotFound') : $t('adminRegistryEmpty')}
                        </div>
                    {/if}
                </div>

                <div class="pt-3 border-t border-[#34251c]/10 shrink-0 text-[10px] text-[#5f4636] text-center opacity-75">
                    {#if filteredFigurines.length !== figurines.length}{filteredFigurines.length} / {/if}{figurines.length} {$t('adminRegistryCount')}
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
                        bind:askTab={formTabAsk}
                    />
                {:else}
                    <div class="h-full flex flex-col items-center justify-center text-[#5f4636] opacity-60">
                        <span class="text-5xl mb-4">📜</span>
                        <p class="text-sm">{$t('adminRegistrySelectPrompt')}</p>
                    </div>
                {/if}
            </main>
        </div>

        {:else if activeTab === 'todo'}
            <div in:fade class="h-full overflow-auto">
                <ReadinessPanel
                    onOpenWork={async (id, tab) => { activeTab = 'registry'; if (await editFigurine(id)) formTabAsk = tab; }}
                    onOpenTale={(id) => layGazette({ kind: 'tale', leafId: id })}
                />
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

        {:else if activeTab === 'tales'}
            <div in:fade class="h-full overflow-hidden"><TalesPanel seed={gazetteSeed} onSeedConsumed={() => (gazetteSeed = null)} /></div>
        {:else if activeTab === 'battles'}
            <div in:fade class="h-full overflow-hidden"><BattlesPanel /></div>
        {:else if activeTab === 'studio'}
            <div in:fade class="h-full overflow-hidden"><StudioPanel /></div>

        {:else if activeTab === 'media'}
            <div in:fade class="h-full">
                <MediaLibrary onEditFigurine={(id) => { activeTab = 'registry'; editFigurine(id); }} />
            </div>
        {:else if activeTab === 'author'}
            <div in:fade class="h-full flex flex-col">
                <!-- Sub-tabs for author section -->
                <div class="flex gap-1 border-b border-[#34251c]/10 px-4 pt-2 flex-shrink-0">
                    {#each [[('profile'), $t('adminSubProfile')],[('texts'), $t('adminSubTexts')],[('header'), $t('adminSubHeader')],[('portrait'), $t('adminSubPortrait')]] as [sub, label]}
                        {@const authorSubTab = activeAuthorSubTab}
                        <button
                            onclick={() => activeAuthorSubTab = sub as 'profile' | 'texts' | 'header' | 'portrait'}
                            class="px-3 py-1.5 text-[10px] uppercase tracking-wide border-b-2 transition-colors -mb-px
                                {authorSubTab === sub ? 'border-[#34251c]/50 text-[#34251c]' : 'border-transparent text-[#5f4636] hover:text-[#34251c]'}"
                        >{label}</button>
                    {/each}
                </div>
                <div class="flex-1 overflow-hidden">
                    {#if activeAuthorSubTab === 'profile'}
                        <ProfileEditor />
                    {:else if activeAuthorSubTab === 'header'}
                        <HeaderDecorPanel />
                    {:else if activeAuthorSubTab === 'portrait'}
                        <HeaderPortraitPanel />
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
        {:else if activeTab === 'tools'}
            <div in:fade class="h-full overflow-y-auto"><ToolsPanel /></div>
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
