<script lang="ts">
    import { onMount } from 'svelte';
    import { api, isTauri } from '$lib/api';
    import type { Figurine, FigurineListItem, ShowingRoom } from '$lib/types/api';
    import { fade, slide } from 'svelte/transition';
    import SettingsModal from '$lib/components/SettingsModal.svelte';
    import KeyholeVeil from '$lib/components/KeyholeVeil.svelte';
    import { themeConfig } from '$lib/stores/theme.svelte';
    import ZoneEditor from '$lib/components/admin/ZoneEditor.svelte';
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
    import UsersPanel from '$lib/components/admin/UsersPanel.svelte';
    import CommentsPanel from '$lib/components/admin/CommentsPanel.svelte';
    import SmtpSettingsPanel from '$lib/components/admin/SmtpSettingsPanel.svelte';
    import ContactSettingsPanel from '$lib/components/admin/ContactSettingsPanel.svelte';
    import WaitlistPanel from '$lib/components/admin/WaitlistPanel.svelte';
    import BookingRulesPanel from '$lib/components/admin/BookingRulesPanel.svelte';
    import MessagesPanel from '$lib/components/admin/MessagesPanel.svelte';
    import FigurineShowingsEditor from '$lib/components/admin/FigurineShowingsEditor.svelte';
    import DesignEditor from '$lib/components/admin/DesignEditor.svelte';
    import CopyEditor from '$lib/components/admin/CopyEditor.svelte';
    import WorkshopFeaturePanel from '$lib/components/admin/WorkshopFeaturePanel.svelte';
    import ProgrammePanel from '$lib/components/admin/ProgrammePanel.svelte';
    import LogsPanel from '$lib/components/admin/LogsPanel.svelte';
    import { t, lang } from '$lib/i18n';
    import SealedDoor from '$lib/components/SealedDoor.svelte';
    import { resolveWindow, isShowingOpen, roomToWindow } from '$lib/showing-window';
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
    let isSaving = $state(false);
    let showingsEditor = $state<FigurineShowingsEditor | null>(null);
    let showSettings = $state(false);
    let message = $state({ text: '', type: 'info' });
    let activeTab = $state<'registry' | 'rooms' | 'home' | 'workshop-feature' | 'zones' | 'author' | 'workshop' | 'media' | 'releases' | 'orders' | 'commissions' | 'showings' | 'bookings' | 'waitlist' | 'analytics' | 'users' | 'comments' | 'messages' | 'server' | 'logs' | 'booking-rules' | 'contact' | 'design' | 'copy' | 'programme'>('registry');
    let activeAuthorSubTab = $state<'profile' | 'texts'>('profile');
    let newOrdersCount = $state(0);
    let newCommissionsCount = $state(0);
    let newBookingsCount = $state(0);
    let pendingCommentsCount = $state(0);
    let searchQuery = $state('');
    let isDeleting = $state(false);
    let uploadingVideo = $state(false);
    let uploadingAudio = $state(false);
    let externalVideoUrl = $state('');
    let folderUploadProgress = $state<{ done: number; total: number } | null>(null);

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

    function loadImageAspect(url: string): Promise<number | null> {
        if (!url) return Promise.resolve(null);
        return new Promise((resolve) => {
            const img = new Image();
            img.onload = () => {
                resolve(img.naturalWidth && img.naturalHeight
                    ? img.naturalWidth / img.naturalHeight
                    : null);
            };
            img.onerror = () => resolve(null);
            img.src = url;
        });
    }

    async function confirmDepthAspectMatches(imageUrl: string, depthUrl: string): Promise<boolean> {
        const [imageAspect, depthAspect] = await Promise.all([
            loadImageAspect(resolveUrl(imageUrl)),
            loadImageAspect(resolveUrl(depthUrl)),
        ]);
        if (!imageAspect || !depthAspect) return true;
        const drift = Math.abs(imageAspect - depthAspect) / imageAspect;
        return drift <= 0.03 || confirm($t('adminMediaDepthAspectWarning'));
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
        openFromMin: null,
        openUntilMin: null,
        sealedDoorImage: null,
        images: [],
        processSteps: [],
        relatedItems: []
    };

    // Showing window ("the house wakes"): stored as minutes-from-midnight, edited
    // as a HH:MM clock. Empty input → null. Both null → always open (ungated).
    function minToTime(min: number | null | undefined): string {
        if (min == null) return '';
        const m = ((Math.round(min) % 1440) + 1440) % 1440;
        return `${String(Math.floor(m / 60)).padStart(2, '0')}:${String(m % 60).padStart(2, '0')}`;
    }
    function timeToMin(value: string): number | null {
        if (!value) return null;
        const [h, m] = value.split(':').map(Number);
        if (Number.isNaN(h) || Number.isNaN(m)) return null;
        return h * 60 + m;
    }

    // === Showing rooms (named shared windows) ===
    let showingRoomsList = $state<ShowingRoom[]>([]);

    async function loadShowingRooms() {
        try { showingRoomsList = await api.getShowingRooms(); } catch {}
    }

    // The figurine's window mode: '' = always open, 'custom' = own hours, else a room id.
    function figWindowMode(f: Figurine | null): string {
        if (!f) return '';
        if (f.showingRoomId) return f.showingRoomId;
        if (f.openFromMin != null && f.openUntilMin != null) return 'custom';
        return '';
    }

    // Room and custom hours are mutually exclusive: switching mode clears the other.
    function setFigWindowMode(value: string) {
        if (!selectedFigurine) return;
        if (value === '') {
            selectedFigurine.showingRoomId = null;
            selectedFigurine.openFromMin = null;
            selectedFigurine.openUntilMin = null;
        } else if (value === 'custom') {
            selectedFigurine.showingRoomId = null;
            if (selectedFigurine.openFromMin == null) selectedFigurine.openFromMin = 0;
            if (selectedFigurine.openUntilMin == null) selectedFigurine.openUntilMin = 4 * 60;
        } else {
            selectedFigurine.showingRoomId = value;
            selectedFigurine.openFromMin = null;
            selectedFigurine.openUntilMin = null;
        }
    }

    function addShowingRoom() {
        showingRoomsList = [
            ...showingRoomsList,
            { id: crypto.randomUUID(), name: '', openFromMin: 23 * 60, openUntilMin: 4 * 60 },
        ];
    }

    async function saveShowingRoom(room: ShowingRoom) {
        await api.saveShowingRoom(room);
        await loadShowingRooms();
    }

    async function deleteShowingRoom(id: string) {
        await api.deleteShowingRoom(id);
        // A work pointing at the deleted room falls back to always-open.
        if (selectedFigurine?.showingRoomId === id) selectedFigurine.showingRoomId = null;
        await loadShowingRooms();
    }

    // --- Room schedule: weekday mask + date mode (Task B) ---
    let roomLocale = $derived($lang === 'ru' ? 'ru-RU' : 'en-US');
    // Mon..Sun short labels (2024-01-01 was a Monday).
    let weekdayLabels = $derived(
        Array.from({ length: 7 }, (_, i) =>
            new Intl.DateTimeFormat(roomLocale, { weekday: 'short' }).format(new Date(2024, 0, 1 + i))
        )
    );
    const dayBit = (mask: number | null | undefined, i: number) => (((mask ?? 0) >> i) & 1) === 1;
    function toggleDay(room: ShowingRoom, i: number) {
        const next = (room.openDaysMask ?? 0) ^ (1 << i);
        room.openDaysMask = next === 0 ? null : next; // no days set → every day
    }

    function roomDateMode(room: ShowingRoom): 'none' | 'annual' | 'range' {
        if (room.openMonthDay) return 'annual';
        if (room.openDateFrom || room.openDateUntil) return 'range';
        return 'none';
    }
    function setRoomDateMode(room: ShowingRoom, mode: string) {
        room.openMonthDay = null;
        room.openDateFrom = null;
        room.openDateUntil = null;
        if (mode === 'annual') {
            const now = new Date();
            room.openMonthDay = `${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`;
        } else if (mode === 'range') {
            room.openDateFrom = new Date().toISOString().slice(0, 10);
        }
    }
    // Annual date <-> "MM-DD": a date input wants YYYY-MM-DD, so pad/strip the year.
    const annualToInput = (md: string | null | undefined) => (md ? `2000-${md}` : '');
    const inputToAnnual = (v: string) => (v ? v.slice(5) : null);

    // --- Preview clock (Task E): see a window "as a guest would" at any moment,
    // without touching the system clock or saving anything. ---
    let previewAt = $state<Date>(new Date());
    function toLocalInput(d: Date): string {
        const p = (n: number) => String(n).padStart(2, '0');
        return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}`;
    }
    // A room's full window (hours + days + date) for preview evaluation.
    // The selected figurine's effective window (room or own hours).
    let previewFigWindow = $derived(
        selectedFigurine
            ? resolveWindow(
                  { openFromMin: selectedFigurine.openFromMin, openUntilMin: selectedFigurine.openUntilMin, showingRoomId: selectedFigurine.showingRoomId },
                  showingRoomsList
              )
            : {}
    );
    let previewFigOpen = $derived(isShowingOpen(previewFigWindow, previewAt));

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

    // Picks a media source for the current platform: a local path string in Tauri,
    // a File on web. Returns null when the user cancels (Tauri) — the web picker
    // rejects with 'no file' instead, which callers swallow.
    async function pickMediaSource(type: 'images' | 'videos' | 'audio'): Promise<string | File | null> {
        if (isTauri) {
            const { open } = await import('@tauri-apps/plugin-dialog');
            const filters = [];
            if (type === 'images') filters.push({ name: 'Images', extensions: ['jpg', 'png', 'webp'] });
            else if (type === 'videos') filters.push({ name: 'Videos', extensions: ['mp4', 'webm', 'mov'] });
            else filters.push({ name: 'Audio', extensions: ['mp3', 'wav', 'ogg', 'm4a'] });
            const selected = await open({ multiple: false, filters });
            if (!selected || typeof selected !== 'string') return null;
            return selected;
        }
        return await pickFileWeb(type);
    }

    async function handlePickFile(type: 'images' | 'videos' | 'audio', stepIndex?: number) {
        if (!selectedFigurine) return;
        if (type === 'videos') uploadingVideo = true;
        if (type === 'audio') uploadingAudio = true;
        try {
            const fileOrPath = await pickMediaSource(type);
            if (fileOrPath === null) return;

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
                    altText: '',
                    depthUrl: null,
                    parallaxIntensity: null,
                    focalX: null,
                    focalY: null,
                    revealRadius: null,
                    darkness: null
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

    async function handleFolderUpload() {
        if (!selectedFigurine) return;
        if (isTauri) {
            const { open } = await import('@tauri-apps/plugin-dialog');
            const { invoke: inv } = await import('@tauri-apps/api/core');
            const dir = await open({ directory: true, multiple: false });
            if (!dir || typeof dir !== 'string') return;
            let imagePaths: string[] = [];
            try {
                imagePaths = await inv<string[]>('list_image_files', { dirPath: dir });
            } catch (e) {
                showMessage($t('adminMsgError') + e, 'error');
                return;
            }
            if (imagePaths.length === 0) return;
            folderUploadProgress = { done: 0, total: imagePaths.length };
            for (const filePath of imagePaths) {
                try {
                    const imported = await api.importMediaWithVariants(filePath, 'images');
                    const variants = deriveImageVariants(imported.url);
                    selectedFigurine.images = [...selectedFigurine.images, {
                        id: crypto.randomUUID(),
                        imageType: 'full',
                        url: imported.url,
                        originalUrl: imported.originalUrl ?? variants.originalUrl,
                        thumbUrl: imported.thumbUrl ?? variants.thumbUrl,
                        altText: '',
                        depthUrl: null,
                        parallaxIntensity: null,
                        focalX: null,
                        focalY: null,
                        revealRadius: null,
                        darkness: null
                    }];
                } catch (e) {
                    showMessage($t('adminMsgError') + String(e), 'error');
                }
                folderUploadProgress = { done: (folderUploadProgress?.done ?? 0) + 1, total: imagePaths.length };
            }
            folderUploadProgress = null;
            showMessage($t('adminMsgFileUploaded'), 'success');
        } else {
            // Web mode: native folder picker via webkitdirectory
            const input = document.createElement('input');
            input.type = 'file';
            input.accept = 'image/jpeg,image/png,image/webp';
            input.multiple = true;
            (input as HTMLInputElement & { webkitdirectory: boolean }).webkitdirectory = true;
            input.onchange = async () => {
                const files = Array.from(input.files ?? [])
                    .filter(f => /\.(jpe?g|png|webp)$/i.test(f.name))
                    .sort((a, b) => a.name.localeCompare(b.name));
                if (files.length === 0) return;
                folderUploadProgress = { done: 0, total: files.length };
                for (const file of files) {
                    try {
                        const imported = await api.importMediaWithVariants(file, 'images');
                        const variants = deriveImageVariants(imported.url);
                        selectedFigurine!.images = [...selectedFigurine!.images, {
                            id: crypto.randomUUID(),
                            imageType: 'full',
                            url: imported.url,
                            originalUrl: imported.originalUrl ?? variants.originalUrl,
                            thumbUrl: imported.thumbUrl ?? variants.thumbUrl,
                            altText: '',
                            depthUrl: null,
                            parallaxIntensity: null,
                            focalX: null,
                            focalY: null,
                            revealRadius: null,
                            darkness: null
                        }];
                    } catch (e) {
                        showMessage($t('adminMsgError') + String(e), 'error');
                    }
                    folderUploadProgress = { done: (folderUploadProgress?.done ?? 0) + 1, total: files.length };
                }
                folderUploadProgress = null;
                showMessage($t('adminMsgFileUploaded'), 'success');
            };
            input.click();
        }
    }

    // Attach a precomputed depth map to a single image (LivingDaguerreotype 2.5D
    // parallax). It's just a grayscale image upload — the offline batch produces
    // higher-fidelity maps, this is the manual path. NULL falls back to luminance.
    async function handlePickDepth(imgIdx: number) {
        if (!selectedFigurine) return;
        try {
            const fileOrPath = await pickMediaSource('images');
            if (fileOrPath === null) return;
            const imported = await api.importMediaWithVariants(fileOrPath, 'images');
            const targetImage = selectedFigurine.images[imgIdx];
            if (targetImage && !(await confirmDepthAspectMatches(targetImage.url, imported.url))) {
                showMessage($t('adminMediaDepthCancelled'), 'info');
                return;
            }
            selectedFigurine.images[imgIdx].depthUrl = imported.url;
            selectedFigurine.images = [...selectedFigurine.images];
            showMessage($t('adminMediaDepthUploaded'), 'success');
        } catch (e: unknown) {
            const msg = e instanceof Error ? e.message : String(e);
            if (msg !== 'no file') showMessage($t('adminMsgError') + msg, 'error');
        }
    }

    function clearDepth(imgIdx: number) {
        if (!selectedFigurine) return;
        selectedFigurine.images[imgIdx].depthUrl = null;
        selectedFigurine.images = [...selectedFigurine.images];
    }

    let generatingDepth = $state(false);

    // Generate depth maps for every image of the current figurine via the Rust
    // API (Depth-Anything on CPU). Requires the figurine to already exist server
    // side; then refresh each image's depthUrl so the badge/preview update.
    async function generateDepth() {
        if (!selectedFigurine) return;
        if (hasUnsaved) {
            showMessage($t('adminMediaDepthGenSaveFirst'), 'info');
            return;
        }
        generatingDepth = true;
        try {
            const res = await api.generateFigurineDepth(selectedFigurine.id);
            // Pull fresh depthUrls (the API just wrote them) without clobbering
            // any in-form edits: merge by image id.
            const fresh = await api.getFigurine(selectedFigurine.id);
            const byId = new Map((fresh?.images ?? []).map(i => [i.id, i.depthUrl ?? null]));
            selectedFigurine.images = selectedFigurine.images.map(img => ({
                ...img,
                depthUrl: byId.get(img.id) ?? img.depthUrl ?? null,
            }));
            showMessage(`${$t('adminMediaDepthGenDone')}: ${res.generated}/${res.results.length}`, 'success');
        } catch (e: unknown) {
            const msg = e instanceof Error ? e.message : String(e);
            showMessage($t('adminMsgError') + msg, 'error');
        } finally {
            generatingDepth = false;
        }
    }

    function setParallaxIntensity(imgIdx: number, value: string) {
        if (!selectedFigurine) return;
        const parsed = Number(value);
        selectedFigurine.images[imgIdx].parallaxIntensity = Number.isFinite(parsed)
            ? Math.max(0, Math.min(1, parsed))
            : null;
        selectedFigurine.images = [...selectedFigurine.images];
    }

    function resetParallaxIntensity(imgIdx: number) {
        if (!selectedFigurine) return;
        selectedFigurine.images[imgIdx].parallaxIntensity = null;
        selectedFigurine.images = [...selectedFigurine.images];
    }

    function parallaxValue(value: number | null | undefined): number {
        if (typeof value !== 'number' || !Number.isFinite(value)) return 0.6;
        return Math.max(0, Math.min(1, value));
    }

    // "Keyhole" reveal — the focal fragment shown on the archive/home card while
    // the work is still sealed (unseen). Frame-relative 0..1, edited over a 4/3
    // `contain` preview that mirrors the live card exactly.
    function setFocalPoint(imgIdx: number, x: number, y: number) {
        if (!selectedFigurine) return;
        selectedFigurine.images[imgIdx].focalX = Math.round(x * 1000) / 1000;
        selectedFigurine.images[imgIdx].focalY = Math.round(y * 1000) / 1000;
        selectedFigurine.images = [...selectedFigurine.images];
    }

    function setRevealRadius(imgIdx: number, value: string) {
        if (!selectedFigurine) return;
        const parsed = Number(value);
        selectedFigurine.images[imgIdx].revealRadius = Number.isFinite(parsed)
            ? Math.max(0.08, Math.min(1, parsed))
            : null;
        selectedFigurine.images = [...selectedFigurine.images];
    }

    function resetReveal(imgIdx: number) {
        if (!selectedFigurine) return;
        selectedFigurine.images[imgIdx].focalX = null;
        selectedFigurine.images[imgIdx].focalY = null;
        selectedFigurine.images[imgIdx].revealRadius = null;
        selectedFigurine.images[imgIdx].darkness = null;
        selectedFigurine.images = [...selectedFigurine.images];
    }

    function revealRadiusValue(value: number | null | undefined): number {
        if (typeof value !== 'number' || !Number.isFinite(value)) return 0.3;
        return Math.max(0.08, Math.min(1, value));
    }

    // Per-image darkness override. Empty/non-finite → null = inherit the global
    // keyhole darkness (theme setting). Mirrors the renderer's 0.88 default.
    function setDarkness(imgIdx: number, value: string) {
        if (!selectedFigurine) return;
        const parsed = Number(value);
        selectedFigurine.images[imgIdx].darkness = Number.isFinite(parsed)
            ? Math.max(0, Math.min(1, parsed))
            : null;
        selectedFigurine.images = [...selectedFigurine.images];
    }

    // The global keyhole darkness shows through when no per-image override is set,
    // so the stepper lands on it rather than a bare default.
    function darknessValue(value: number | null | undefined): number {
        if (typeof value === 'number' && Number.isFinite(value)) return Math.max(0, Math.min(1, value));
        const global = $themeConfig.effects?.keyholeDarkness;
        return typeof global === 'number' && Number.isFinite(global) ? global : 0.88;
    }

    // "Window" / "Shadow" are stepper buttons, not sliders: a range input cannot
    // shrink below its intrinsic width, so it overflowed and went dead in the
    // narrow per-image column. Each tap nudges by a fixed step, clamped to the
    // same bounds the renderer enforces.
    const REVEAL_MIN = 0.08, REVEAL_MAX = 1, DARK_MIN = 0.4, DARK_MAX = 1, KEYHOLE_STEP = 0.05;
    function nudgeRevealRadius(imgIdx: number, delta: number) {
        if (!selectedFigurine) return;
        const next = revealRadiusValue(selectedFigurine.images[imgIdx].revealRadius) + delta;
        setRevealRadius(imgIdx, String(Math.round(next * 100) / 100));
    }
    function nudgeDarkness(imgIdx: number, delta: number) {
        if (!selectedFigurine) return;
        const next = Math.max(DARK_MIN, Math.min(DARK_MAX, darknessValue(selectedFigurine.images[imgIdx].darkness) + delta));
        setDarkness(imgIdx, String(Math.round(next * 100) / 100));
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
            // Сначала коммитим незакрытую инлайн-форму показа (если в ней есть данные).
            // При невалидных данных flush() вернёт false и покажет свою ошибку инлайн —
            // прерываем сохранение, чтобы автор их поправил.
            if (showingsEditor && !(await showingsEditor.flush())) {
                return;
            }
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
        const hasKey = localStorage.getItem('gotiga_api_key') || sessionStorage.getItem('gotiga_api_key');
        if (isTauri) {
            isAuthenticated = true;
            loadFigurines();
            loadShowingRooms();
        } else if ((session === '1' || persisted === '1') && hasKey) {
            isAuthenticated = true;
            loadFigurines();
            loadShowingRooms();
        }
        // Hash-based tab routing (e.g. Telegram notification links)
        const hash = window.location.hash.replace('#', '');
        const validTabs = ['registry','rooms','home','workshop-feature','zones','author','workshop','media','releases','orders','commissions','showings','bookings','waitlist','analytics','users','comments','messages','server','booking-rules','contact'];
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
<div class="h-screen bg-[#f8f1e7] text-[#34251c] font-cinzel flex flex-row overflow-hidden">

    <!-- Sidebar -->
    <aside class="w-52 shrink-0 flex flex-col border-r border-[#34251c]/20 bg-[#f2e8da] overflow-y-auto">

        <!-- Branding -->
        <div class="px-4 pt-5 pb-4 border-b border-[#34251c]/15">
            <h1 class="text-lg font-gothic leading-tight">{$t('adminTitle')}</h1>
            <p class="text-[9px] tracking-[0.1em] text-[#5f4636] uppercase mt-1">{$t('adminSubtitle')}</p>
        </div>

        <!-- Nav -->
        <nav class="flex-1 px-3 py-4 flex flex-col gap-5">
            {#each [
              {
                label: $t('adminGroupFigurines'),
                tabs: [
                  ['registry', $t('adminTabRegistry')],
                  ['rooms',    $t('adminTabShowingRooms')],
                  ['zones',    $t('adminTabZones')],
                  ['releases', $t('adminTabReleases')],
                ]
              },
              {
                label: $t('adminGroupShowcase'),
                tabs: [
                  ['home',     $t('adminTabHome')],
                  ['programme', $t('adminTabProgramme')],
                  ['workshop-feature', $t('adminTabWorkshopFeature')],
                  ['author',   $t('adminTabAuthor')],
                  ['workshop', $t('adminTabWorkshop')],
                  ['media',    $t('adminTabMedia')],
                ]
              },
              {
                label: $t('adminGroupActivity'),
                tabs: [
                  ['orders',    $t('adminTabOrders')],
                  ['commissions', $t('adminTabCommissions')],
                  ['showings',  $t('adminTabShowings')],
                  ['bookings',      $t('adminTabBookings')],
                  ['waitlist',      $t('adminTabWaitlist')],
                  ['comments',      $t('adminTabComments')],
                  ['messages',      $t('adminTabMessages')],
                  ['analytics',     $t('adminTabAnalytics')],
                  ['users',         $t('adminUsersTab')],
                ]
              },
              {
                label: $t('adminGroupSystem'),
                tabs: [
                  ['server',        $t('adminTabServer')],
                  ['logs',          $t('adminTabLogs')],
                  ['booking-rules', $t('adminTabBookingRules')],
                  ['contact',       $t('adminTabContact')],
                ]
              },
              {
                label: $t('adminGroupDesign'),
                tabs: [
                  ['design', $t('adminTabDesign')],
                  ['copy',   $t('adminTabCopy')],
                ]
              },
            ] as group}
              <div>
                <span class="block px-2 mb-1 text-[8px] uppercase tracking-[0.12em] text-[#5f4636]/50 font-medium">{group.label}</span>
                {#each group.tabs as [tab, label]}
                  <button
                    onclick={() => activeTab = tab as typeof activeTab}
                    class="w-full text-left flex items-center justify-between px-2 py-1.5 text-xs uppercase tracking-wide transition-colors
                           {activeTab === tab
                             ? 'border-l-2 border-[#c65f3c] bg-[#c65f3c]/10 text-[#34251c] pl-[6px]'
                             : 'border-l-2 border-transparent text-[#5f4636] hover:text-[#34251c] hover:bg-[#34251c]/5 pl-[6px]'}"
                  >
                    <span>{label}</span>
                    {#if tab === 'orders' && newOrdersCount > 0 && activeTab !== 'orders'}
                      <span class="inline-flex items-center justify-center min-w-[16px] h-4 px-1 rounded-full bg-red-500 text-white text-[9px] font-bold leading-none">
                        {newOrdersCount > 99 ? '99+' : newOrdersCount}
                      </span>
                    {/if}
                    {#if tab === 'commissions' && newCommissionsCount > 0 && activeTab !== 'commissions'}
                      <span class="inline-flex items-center justify-center min-w-[16px] h-4 px-1 rounded-full bg-red-500 text-white text-[9px] font-bold leading-none">
                        {newCommissionsCount > 99 ? '99+' : newCommissionsCount}
                      </span>
                    {/if}
                    {#if tab === 'bookings' && newBookingsCount > 0 && activeTab !== 'bookings'}
                      <span class="inline-flex items-center justify-center min-w-[16px] h-4 px-1 rounded-full bg-amber-500 text-white text-[9px] font-bold leading-none">
                        {newBookingsCount > 99 ? '99+' : newBookingsCount}
                      </span>
                    {/if}
                    {#if tab === 'comments' && pendingCommentsCount > 0 && activeTab !== 'comments'}
                      <span class="inline-flex items-center justify-center min-w-[16px] h-4 px-1 rounded-full bg-orange-600 text-white text-[9px] font-bold leading-none">
                        {pendingCommentsCount > 99 ? '99+' : pendingCommentsCount}
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
            {#if !isTauri}
                <button onclick={handleLogout} class="btn-gothic text-[10px] w-full text-left opacity-75 hover:opacity-100">{$t('adminLogout')}</button>
            {/if}
            <a href="/" class="btn-gothic text-[10px] opacity-60 hover:opacity-100">{$t('adminToMuseum')}</a>
        </div>
    </aside>

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
                                    <input bind:value={selectedFigurine.dimensions} class="input-gothic" placeholder="20×15×10 cm" list="suggest-dimensions" autocomplete="off" />
                                    <datalist id="suggest-dimensions">
                                        {#each dimensionsSuggestions as s}
                                            <option value={s} />
                                        {/each}
                                    </datalist>
                                </label>
                                <label class="block">
                                    <span class="label">{$t('adminFieldMaterial')}</span>
                                    <input bind:value={selectedFigurine.material} class="input-gothic" list="suggest-material" autocomplete="off" />
                                    <datalist id="suggest-material">
                                        {#each materialSuggestions as s}
                                            <option value={s} />
                                        {/each}
                                    </datalist>
                                </label>
                                <label class="block">
                                    <span class="label">{$t('adminFieldTechnique')}</span>
                                    <input bind:value={selectedFigurine.technique} class="input-gothic" list="suggest-technique" autocomplete="off" />
                                    <datalist id="suggest-technique">
                                        {#each techniqueSuggestions as s}
                                            <option value={s} />
                                        {/each}
                                    </datalist>
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

                                <!-- "The house wakes": showing window. A work is either always
                                     open, has its own hours, or belongs to a named room (shared
                                     window). Room and custom hours are mutually exclusive. -->
                                <div class="border-t border-[#34251c]/10 pt-3 mt-1">
                                    <span class="label">{$t('adminFieldShowingWindow')}</span>
                                    <label class="block">
                                        <select
                                            value={figWindowMode(selectedFigurine)}
                                            onchange={(e) => setFigWindowMode(e.currentTarget.value)}
                                            class="input-gothic"
                                        >
                                            <option value="">{$t('adminShowingModeAlways')}</option>
                                            <option value="custom">{$t('adminShowingModeCustom')}</option>
                                            {#each showingRoomsList as room (room.id)}
                                                {#if room.name}
                                                    <option value={room.id}>{room.name} ({minToTime(room.openFromMin)}–{minToTime(room.openUntilMin)})</option>
                                                {/if}
                                            {/each}
                                        </select>
                                    </label>

                                    {#if figWindowMode(selectedFigurine) === 'custom'}
                                        <div class="flex gap-4 items-end mt-2">
                                            <label class="block flex-1">
                                                <span class="text-[10px] uppercase tracking-wide text-[#7c6554]">{$t('adminFieldShowingFrom')}</span>
                                                <input
                                                    type="time"
                                                    value={minToTime(selectedFigurine.openFromMin)}
                                                    oninput={(e) => selectedFigurine!.openFromMin = timeToMin(e.currentTarget.value)}
                                                    class="input-gothic"
                                                />
                                            </label>
                                            <label class="block flex-1">
                                                <span class="text-[10px] uppercase tracking-wide text-[#7c6554]">{$t('adminFieldShowingUntil')}</span>
                                                <input
                                                    type="time"
                                                    value={minToTime(selectedFigurine.openUntilMin)}
                                                    oninput={(e) => selectedFigurine!.openUntilMin = timeToMin(e.currentTarget.value)}
                                                    class="input-gothic"
                                                />
                                            </label>
                                        </div>
                                        <p class="text-[10px] text-[#7c6554] mt-1 leading-snug">{$t('adminFieldShowingHint')}</p>
                                    {/if}

                                    <label class="block mt-2">
                                        <span class="text-[10px] uppercase tracking-wide text-[#7c6554]">{$t('adminFieldSealedDoorImage')}</span>
                                        <input
                                            bind:value={selectedFigurine.sealedDoorImage}
                                            placeholder="https://…"
                                            class="input-gothic"
                                        />
                                    </label>
                                    <p class="text-[10px] text-[#7c6554] mt-2 leading-snug">{$t('adminShowingRoomsManageHint')}</p>

                                    <!-- Preview: this work as a guest would see it at a chosen moment. -->
                                    <div class="mt-3 border-t border-[#34251c]/10 pt-3">
                                        <div class="flex flex-wrap items-end gap-3">
                                            <label class="block">
                                                <span class="text-[10px] uppercase tracking-wide text-[#7c6554]">{$t('adminPreviewAt')}</span>
                                                <input type="datetime-local" value={toLocalInput(previewAt)} oninput={(e) => { if (e.currentTarget.value) previewAt = new Date(e.currentTarget.value); }} class="input-gothic" />
                                            </label>
                                            <button type="button" class="text-[11px] uppercase tracking-wide text-[#6f3b24] pb-2" onclick={() => previewAt = new Date()}>{$t('adminPreviewNow')}</button>
                                            <span class="text-[10px] uppercase tracking-wide px-2 py-1 rounded pb-1 {previewFigOpen ? 'bg-emerald-600/15 text-emerald-700' : 'bg-[#6f3b24]/12 text-[#6f3b24]'}">
                                                {previewFigOpen ? $t('adminPreviewOpen') : $t('adminPreviewClosed')}
                                            </span>
                                        </div>
                                        {#if !previewFigOpen}
                                            <div class="relative w-40 aspect-[3/4] mt-3 rounded-[3px] overflow-hidden border border-[#34251c]/15">
                                                <SealedDoor
                                                    openFromMin={previewFigWindow.openFromMin}
                                                    openUntilMin={previewFigWindow.openUntilMin}
                                                    daysMask={previewFigWindow.daysMask}
                                                    monthDay={previewFigWindow.monthDay}
                                                    dateFrom={previewFigWindow.dateFrom}
                                                    dateUntil={previewFigWindow.dateUntil}
                                                    doorImageUrl={selectedFigurine.sealedDoorImage}
                                                    name={selectedFigurine.name}
                                                    now={previewAt}
                                                    compact
                                                />
                                            </div>
                                        {/if}
                                    </div>
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

                        <div class="border-t border-[#34251c]/10 pt-8 mb-8">
                            <h3 class="text-xl font-gothic mb-2">{$t('adminPassportHeading')}</h3>
                            <p class="text-xs text-[#5f4636]/75 mb-5 max-w-2xl">{$t('adminPassportHint')}</p>
                            <div class="grid grid-cols-1 lg:grid-cols-3 gap-4 mb-5">
                                <label class="block">
                                    <span class="label">{$t('passportNumber')}</span>
                                    <input bind:value={selectedFigurine.passportNumber} class="input-gothic" placeholder="RTN-2026-001" />
                                </label>
                                <label class="block">
                                    <span class="label">{$t('passportEdition')}</span>
                                    <input bind:value={selectedFigurine.edition} class="input-gothic" placeholder="1 of 1" />
                                </label>
                                <label class="block">
                                    <span class="label">{$t('passportCreated')}</span>
                                    <input bind:value={selectedFigurine.createdPeriod} class="input-gothic" placeholder="Spring 2026" />
                                </label>
                            </div>
                            <div class="grid grid-cols-1 lg:grid-cols-2 gap-5">
                                <label class="block">
                                    <span class="label">{$t('passportProvenance')}</span>
                                    <textarea bind:value={selectedFigurine.provenanceNote} class="input-gothic h-24"></textarea>
                                </label>
                                <label class="block">
                                    <span class="label">{$t('passportAuthenticity')}</span>
                                    <textarea bind:value={selectedFigurine.authenticityNote} class="input-gothic h-24"></textarea>
                                </label>
                                <label class="block">
                                    <span class="label">{$t('passportCare')}</span>
                                    <textarea bind:value={selectedFigurine.careInstructions} class="input-gothic h-24"></textarea>
                                </label>
                                <label class="block">
                                    <span class="label">{$t('passportIncluded')}</span>
                                    <textarea bind:value={selectedFigurine.includedItems} class="input-gothic h-24"></textarea>
                                </label>
                            </div>
                        </div>

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
                                    <div class="flex gap-2">
                                        {#if !isTauri && selectedFigurine.images.length > 0}
                                            <button onclick={generateDepth} disabled={generatingDepth}
                                                title={$t('adminMediaDepthHint')}
                                                class="btn-gothic text-[10px] disabled:opacity-60 disabled:cursor-wait">
                                                {generatingDepth ? $t('adminMediaDepthGenRunning') : $t('adminMediaDepthGen')}</button>
                                        {/if}
                                        <button onclick={() => handlePickFile('images')} class="btn-gothic text-[10px]" disabled={!!folderUploadProgress}>{$t('adminMediaAddPhoto')}</button>
                                        <button onclick={handleFolderUpload} class="btn-gothic text-[10px]" disabled={!!folderUploadProgress}>
                                            {folderUploadProgress
                                                ? $t('adminMediaFolderProgress').replace('{done}', String(folderUploadProgress.done)).replace('{total}', String(folderUploadProgress.total))
                                                : $t('adminMediaAddFolder')}
                                        </button>
                                    </div>
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
                                                {#if img.depthUrl}
                                                    <div class="absolute top-0 left-0 bg-[#34251c]/85 text-[#f3e9d8] text-[8px] px-1 py-0.5 leading-none tracking-wider font-bold pointer-events-none"
                                                        title={$t('adminMediaDepthHint')}>{$t('adminMediaDepthBadge')}</div>
                                                {/if}
                                            </div>
                                            <!-- Alt text -->
                                            <input
                                                bind:value={img.altText}
                                                type="text"
                                                placeholder={$t('adminMediaAltPlaceholder')}
                                                class="w-28 bg-[#f8f1e7] border border-[#34251c]/10 px-1.5 py-1 text-[9px] text-[#5f4636] focus:border-[#34251c]/30 outline-none"
                                            />
                                            <!-- Depth map (2.5D parallax) -->
                                            <div class="w-28 flex items-center gap-1" title={$t('adminMediaDepthHint')}>
                                                {#if img.depthUrl}
                                                    <img src={resolveUrl(img.depthUrl)} alt="" class="w-6 h-6 object-cover border border-[#34251c]/20 shrink-0" />
                                                    <button onclick={() => handlePickDepth(imgIdx)}
                                                        class="flex-1 text-[8px] uppercase text-[#5f4636] hover:text-[#34251c] px-1 py-0.5 border border-[#34251c]/15">{$t('adminMediaDepthReplace')}</button>
                                                    <button onclick={() => clearDepth(imgIdx)}
                                                        class="text-[8px] text-red-800/70 hover:text-red-900 px-1 py-0.5 border border-red-800/20 shrink-0">✕</button>
                                                {:else}
                                                    <button onclick={() => handlePickDepth(imgIdx)}
                                                        class="w-full text-[8px] uppercase text-[#5f4636] hover:text-[#34251c] px-1 py-0.5 border border-dashed border-[#34251c]/20">{$t('adminMediaDepthAdd')}</button>
                                                {/if}
                                            </div>
                                            <div class="w-28 space-y-1" title={$t('adminMediaParallaxHint')}>
                                                <div class="flex items-center justify-between gap-1">
                                                    <span class="text-[8px] uppercase tracking-[0.08em] text-[#5f4636]">{$t('adminMediaParallax')}</span>
                                                    <button
                                                        type="button"
                                                        onclick={() => resetParallaxIntensity(imgIdx)}
                                                        class="text-[8px] text-[#5f4636] hover:text-[#34251c]"
                                                        disabled={img.parallaxIntensity == null}
                                                    >
                                                        {$t('adminMediaParallaxReset')}
                                                    </button>
                                                </div>
                                                <div class="flex items-center gap-1">
                                                    <input
                                                        type="range"
                                                        min="0"
                                                        max="1"
                                                        step="0.05"
                                                        value={parallaxValue(img.parallaxIntensity)}
                                                        oninput={(e) => setParallaxIntensity(imgIdx, (e.currentTarget as HTMLInputElement).value)}
                                                        class="w-full accent-[#6f3b24]"
                                                    />
                                                    <span class="w-7 text-right text-[8px] tabular-nums text-[#5f4636]">
                                                        {parallaxValue(img.parallaxIntensity).toFixed(2)}
                                                    </span>
                                                </div>
                                            </div>
                                            {#if img.imageType === 'face'}
                                                <!-- Keyhole reveal — only the cover image is teased on the card -->
                                                <div class="w-28 space-y-1" title={$t('adminMediaKeyholeHint')}>
                                                    <div class="flex items-center justify-between gap-1">
                                                        <span class="text-[8px] uppercase tracking-[0.08em] text-[#5f4636]">{$t('adminMediaKeyhole')}</span>
                                                        <button
                                                            type="button"
                                                            onclick={() => resetReveal(imgIdx)}
                                                            class="text-[8px] text-[#5f4636] hover:text-[#34251c]"
                                                            disabled={img.focalX == null && img.focalY == null && img.revealRadius == null && img.darkness == null}
                                                        >
                                                            {$t('adminMediaParallaxReset')}
                                                        </button>
                                                    </div>
                                                    <!-- 4/3 contain preview mirrors the live card; click/drag to place focus -->
                                                    <div class="relative w-28 border border-[#34251c]/20 overflow-hidden bg-[#f1e3d1]" style="aspect-ratio: 4 / 3;">
                                                        <img src={resolveUrl(img.thumbUrl ?? img.url)} alt="" class="w-full h-full object-contain" />
                                                        <KeyholeVeil
                                                            focalX={img.focalX}
                                                            focalY={img.focalY}
                                                            revealRadius={img.revealRadius}
                                                            darkness={darknessValue(img.darkness)}
                                                            editable
                                                            onpick={(x, y) => setFocalPoint(imgIdx, x, y)}
                                                        />
                                                    </div>
                                                    <!-- Window size — stepper (−/+); a range input is unusable in this narrow column -->
                                                    <div class="flex items-center gap-1" title={$t('adminMediaKeyholeRadiusHint')}>
                                                        <span class="text-[7px] uppercase tracking-[0.06em] text-[#5f4636] w-9 shrink-0">{$t('adminMediaKeyholeRadius')}</span>
                                                        <div class="flex items-center flex-1 border border-[#34251c]/20 bg-[#f8f1e7] rounded-sm overflow-hidden">
                                                            <button type="button" aria-label="−"
                                                                onclick={() => nudgeRevealRadius(imgIdx, -KEYHOLE_STEP)}
                                                                disabled={revealRadiusValue(img.revealRadius) <= REVEAL_MIN}
                                                                class="px-1.5 py-0.5 text-[11px] leading-none text-[#5f4636] hover:bg-[#34251c]/10 disabled:opacity-30 disabled:hover:bg-transparent">−</button>
                                                            <span class="flex-1 text-center text-[8px] tabular-nums text-[#5f4636]">{revealRadiusValue(img.revealRadius).toFixed(2)}</span>
                                                            <button type="button" aria-label="+"
                                                                onclick={() => nudgeRevealRadius(imgIdx, KEYHOLE_STEP)}
                                                                disabled={revealRadiusValue(img.revealRadius) >= REVEAL_MAX}
                                                                class="px-1.5 py-0.5 text-[11px] leading-none text-[#5f4636] hover:bg-[#34251c]/10 disabled:opacity-30 disabled:hover:bg-transparent">+</button>
                                                        </div>
                                                    </div>
                                                    <!-- Shadow depth — overrides the global darkness for this work -->
                                                    <div class="flex items-center gap-1" title={$t('adminMediaDarknessHint')}>
                                                        <span class="text-[7px] uppercase tracking-[0.06em] text-[#5f4636] w-9 shrink-0">{$t('adminMediaDarkness')}</span>
                                                        <div class="flex items-center flex-1 border border-[#34251c]/20 bg-[#f8f1e7] rounded-sm overflow-hidden">
                                                            <button type="button" aria-label="−"
                                                                onclick={() => nudgeDarkness(imgIdx, -KEYHOLE_STEP)}
                                                                disabled={darknessValue(img.darkness) <= DARK_MIN}
                                                                class="px-1.5 py-0.5 text-[11px] leading-none text-[#5f4636] hover:bg-[#34251c]/10 disabled:opacity-30 disabled:hover:bg-transparent">−</button>
                                                            <span class="flex-1 text-center text-[8px] tabular-nums {img.darkness == null ? 'text-[#5f4636]/45 italic' : 'text-[#5f4636]'}">{darknessValue(img.darkness).toFixed(2)}</span>
                                                            <button type="button" aria-label="+"
                                                                onclick={() => nudgeDarkness(imgIdx, KEYHOLE_STEP)}
                                                                disabled={darknessValue(img.darkness) >= DARK_MAX}
                                                                class="px-1.5 py-0.5 text-[11px] leading-none text-[#5f4636] hover:bg-[#34251c]/10 disabled:opacity-30 disabled:hover:bg-transparent">+</button>
                                                        </div>
                                                    </div>
                                                </div>
                                            {/if}
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
                          <FigurineShowingsEditor bind:this={showingsEditor} figurineId={selectedFigurine.id} />
                        {/if}

                        <!-- Action bar -->
                        <div class="flex justify-end gap-3 pb-10">
                            <button onclick={() => deleteFigurine(selectedFigurine!)} disabled={isDeleting}
                                class="btn-gothic mr-auto border-red-900/30 text-red-800/70 hover:bg-red-50 hover:text-red-800 hover:border-red-700/40">
                                {$t('adminFormDeleteWork')}
                            </button>
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

        {:else if activeTab === 'rooms'}
            <div in:fade class="h-full overflow-auto p-6 sm:p-8 max-w-3xl mx-auto w-full">
                <h2 class="font-['Fraunces'] text-2xl text-[#34251c] mb-1">{$t('adminTabShowingRooms')}</h2>
                <p class="text-[12px] text-[#7c6554] mb-4 leading-snug max-w-prose">{$t('adminShowingRoomsIntro')}</p>

                <!-- Preview clock: evaluate every room "as a guest would" at this moment. -->
                <div class="flex flex-wrap items-end gap-3 mb-5 p-3 border border-[#34251c]/12 rounded-md bg-[#f3ead9]">
                    <label class="block">
                        <span class="text-[10px] uppercase tracking-wide text-[#7c6554]">{$t('adminPreviewAt')}</span>
                        <input type="datetime-local" value={toLocalInput(previewAt)} oninput={(e) => { if (e.currentTarget.value) previewAt = new Date(e.currentTarget.value); }} class="input-gothic" />
                    </label>
                    <button type="button" class="text-[11px] uppercase tracking-wide text-[#6f3b24] pb-2" onclick={() => previewAt = new Date()}>{$t('adminPreviewNow')}</button>
                </div>

                <div class="space-y-3">
                    {#each showingRoomsList as room (room.id)}
                        <div class="border border-[#34251c]/12 rounded-md p-3 bg-[#fff9f0] space-y-3">
                            <div class="flex flex-wrap gap-3 items-end">
                                <label class="block flex-1 min-w-[160px]">
                                    <span class="text-[10px] uppercase tracking-wide text-[#7c6554]">{$t('adminShowingRoomName')}</span>
                                    <input bind:value={room.name} class="input-gothic" placeholder={$t('adminShowingRoomNamePlaceholder')} />
                                </label>
                                <label class="block w-28">
                                    <span class="text-[10px] uppercase tracking-wide text-[#7c6554]">{$t('adminFieldShowingFrom')}</span>
                                    <input type="time" value={minToTime(room.openFromMin)} oninput={(e) => room.openFromMin = timeToMin(e.currentTarget.value) ?? 0} class="input-gothic" />
                                </label>
                                <label class="block w-28">
                                    <span class="text-[10px] uppercase tracking-wide text-[#7c6554]">{$t('adminFieldShowingUntil')}</span>
                                    <input type="time" value={minToTime(room.openUntilMin)} oninput={(e) => room.openUntilMin = timeToMin(e.currentTarget.value) ?? 0} class="input-gothic" />
                                </label>
                                <div class="flex items-center gap-3 pb-2 ml-auto">
                                    <span class="text-[10px] uppercase tracking-wide px-2 py-1 rounded {isShowingOpen(roomToWindow(room), previewAt) ? 'bg-emerald-600/15 text-emerald-700' : 'bg-[#6f3b24]/12 text-[#6f3b24]'}">
                                        {isShowingOpen(roomToWindow(room), previewAt) ? $t('adminPreviewOpen') : $t('adminPreviewClosed')}
                                    </span>
                                    <button type="button" class="text-[11px] uppercase tracking-wide text-[#c65f3c]" onclick={() => saveShowingRoom(room)}>{$t('adminSave')}</button>
                                    <button type="button" class="text-[11px] uppercase tracking-wide text-[#7c6554]" onclick={() => deleteShowingRoom(room.id)}>{$t('adminDelete')}</button>
                                </div>
                            </div>

                            <!-- Weekdays: empty = every day, pick e.g. Sat+Sun for weekends. -->
                            <div>
                                <span class="text-[10px] uppercase tracking-wide text-[#7c6554]">{$t('adminShowingRoomDays')}</span>
                                <div class="flex flex-wrap gap-1.5 mt-1">
                                    {#each weekdayLabels as label, i}
                                        <button
                                            type="button"
                                            class="px-2.5 py-1 rounded text-[11px] border transition-colors {dayBit(room.openDaysMask, i) ? 'bg-[#6f3b24] text-[#f8f1e7] border-[#6f3b24]' : 'border-[#34251c]/20 text-[#7c6554] hover:border-[#6f3b24]/40'}"
                                            onclick={() => toggleDay(room, i)}
                                        >{label}</button>
                                    {/each}
                                </div>
                            </div>

                            <!-- Calendar date: none / annual (MM-DD) / one-off range. -->
                            <div class="flex flex-wrap gap-3 items-end">
                                <label class="block">
                                    <span class="text-[10px] uppercase tracking-wide text-[#7c6554]">{$t('adminShowingRoomDate')}</span>
                                    <select value={roomDateMode(room)} onchange={(e) => setRoomDateMode(room, e.currentTarget.value)} class="input-gothic">
                                        <option value="none">{$t('adminShowingDateNone')}</option>
                                        <option value="annual">{$t('adminShowingDateAnnual')}</option>
                                        <option value="range">{$t('adminShowingDateRange')}</option>
                                    </select>
                                </label>
                                {#if roomDateMode(room) === 'annual'}
                                    <label class="block">
                                        <span class="text-[10px] uppercase tracking-wide text-[#7c6554]">{$t('adminShowingDateAnnual')}</span>
                                        <input type="date" value={annualToInput(room.openMonthDay)} oninput={(e) => room.openMonthDay = inputToAnnual(e.currentTarget.value)} class="input-gothic" />
                                    </label>
                                {:else if roomDateMode(room) === 'range'}
                                    <label class="block">
                                        <span class="text-[10px] uppercase tracking-wide text-[#7c6554]">{$t('adminFieldShowingFrom')}</span>
                                        <input type="date" bind:value={room.openDateFrom} class="input-gothic" />
                                    </label>
                                    <label class="block">
                                        <span class="text-[10px] uppercase tracking-wide text-[#7c6554]">{$t('adminFieldShowingUntil')}</span>
                                        <input type="date" bind:value={room.openDateUntil} class="input-gothic" />
                                    </label>
                                {/if}
                            </div>
                        </div>
                    {/each}
                    {#if showingRoomsList.length === 0}
                        <p class="text-[12px] italic text-[#7c6554]">{$t('adminShowingRoomsEmpty')}</p>
                    {/if}
                </div>

                <button type="button" class="mt-5 px-4 py-2 border border-[#6f3b24]/30 rounded-md text-[11px] uppercase tracking-wide text-[#6f3b24] hover:bg-[#6f3b24]/5" onclick={addShowingRoom}>+ {$t('adminShowingRoomAdd')}</button>
                <p class="text-[10px] text-[#7c6554] mt-3 leading-snug">{$t('adminFieldShowingHint')}</p>
            </div>

        {:else if activeTab === 'home'}
            <HomeContentEditor />

        {:else if activeTab === 'programme'}
            <div in:fade class="h-full overflow-auto"><ProgrammePanel /></div>

        {:else if activeTab === 'workshop-feature'}
            <div in:fade class="h-full"><WorkshopFeaturePanel /></div>

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
            <div in:fade class="h-full"><OrdersPanel onNewCount={(n: number) => newOrdersCount = n} /></div>
        {:else if activeTab === 'commissions'}
            <div in:fade class="h-full"><CommissionsPanel onNewCount={(n: number) => newCommissionsCount = n} /></div>
        {:else if activeTab === 'showings'}
            <div in:fade class="h-full"><ShowingsPanel /></div>
        {:else if activeTab === 'bookings'}
            <div in:fade class="h-full"><BookingsPanel onPendingCount={(n: number) => newBookingsCount = n} /></div>
        {:else if activeTab === 'releases'}
            <div in:fade class="h-full"><ReleaseManager /></div>
        {:else if activeTab === 'analytics'}
            <div in:fade class="h-full overflow-hidden"><AnalyticsPanel /></div>
        {:else if activeTab === 'users'}
            <div in:fade class="h-full overflow-y-auto"><UsersPanel /></div>
        {:else if activeTab === 'comments'}
            <div in:fade class="h-full overflow-y-auto"><CommentsPanel onPendingCount={(n) => pendingCommentsCount = n} /></div>
        {:else if activeTab === 'server'}
            <div in:fade class="h-full overflow-y-auto"><SmtpSettingsPanel /></div>
        {:else if activeTab === 'logs'}
            <div in:fade class="h-full overflow-hidden"><LogsPanel /></div>
        {:else if activeTab === 'waitlist'}
            <div in:fade class="h-full"><WaitlistPanel /></div>
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
