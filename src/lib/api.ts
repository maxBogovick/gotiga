// src/lib/api.ts
import type {
    FigurineListItem,
    Figurine,
    AuthorText,
    WorkshopItem,
    CabinetZone,
    AppSettings,
    ServerRelease,
    AuthorProfile,
    HomeContent,
    OrderRequest,
    MediaInventory,
    MediaCleanupReport,
    MediaReplaceResult,
    FigurineSchedule,
    CreateBookingRequest,
    BookingsPage,
    ShowingDto,
    SaveShowingRequest,
} from './types/api';

export type { AppSettings };
export type ImportedMedia = {
    url: string;
    originalUrl?: string | null;
    thumbUrl?: string | null;
};

// Tauri 2.x injects __TAURI_INTERNALS__ into the webview
export const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

// --- Tauri helpers ---
async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
    const { invoke: tauriInvoke } = await import('@tauri-apps/api/core');
    return tauriInvoke<T>(cmd, args);
}

// --- Web helpers ---
function getWebSettings(): AppSettings {
    if (typeof localStorage === 'undefined') return { serverUrl: '', apiKey: '' };
    return {
        serverUrl: localStorage.getItem('gotiga_server_url') ?? '',
        apiKey: localStorage.getItem('gotiga_api_key') ?? '',
    };
}

function getWebHomeContent(): HomeContent {
    if (typeof localStorage === 'undefined') return { title: null, kicker: null, lead: null };
    return {
        title: localStorage.getItem('gotiga_home_title'),
        kicker: localStorage.getItem('gotiga_home_kicker'),
        lead: localStorage.getItem('gotiga_home_lead'),
    };
}

function webApiBase(): string {
    const { serverUrl } = getWebSettings();
    return serverUrl ? `${serverUrl}/api/v1` : '/api/v1';
}

function webPublicUrl(url: unknown): string | null {
    if (typeof url !== 'string' || !url) return null;
    if (url.startsWith('http')) return url;
    if (url.startsWith('/static/')) {
        const { serverUrl } = getWebSettings();
        return serverUrl ? `${serverUrl}${url}` : url;
    }
    return url;
}

async function webFetch<T>(path: string, options?: RequestInit): Promise<T> {
    const res = await fetch(`${webApiBase()}${path}`, options);
    if (!res.ok) {
        const text = await res.text().catch(() => '');
        throw new Error(`API ${res.status}: ${text}`);
    }
    const ct = res.headers.get('content-type') ?? '';
    if (!ct.includes('json') || res.status === 204) return undefined as T;
    const text = await res.text();
    if (!text) return undefined as T;
    return JSON.parse(text);
}

function authHeaders(): Record<string, string> {
    const { apiKey } = getWebSettings();
    return apiKey ? { Authorization: `Bearer ${apiKey}` } : {};
}

export const api = {
    // === AUTH ===
    async adminLogin(login: string, password: string): Promise<string> {
        const data = await webFetch<{ token: string }>('/admin/login', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ login, password }),
        });
        return data.token;
    },

    // === READ (public) ===
    async getAllFigurines(): Promise<FigurineListItem[]> {
        if (isTauri) return invoke('get_all_figurines');
        return webFetch('/figurines?visible=true');
    },

    async getInProgressFigurines(): Promise<FigurineListItem[]> {
        if (isTauri) return invoke<FigurineListItem[]>('get_all_figurines')
            .then(all => all.filter(f => f.status === 'in_progress'));
        return webFetch('/figurines/in-progress');
    },

    async getAllFigurinesAdmin(): Promise<FigurineListItem[]> {
        if (isTauri) return invoke('get_all_figurines');
        return webFetch('/figurines?visible=false', {
            headers: authHeaders(),
        });
    },

    async getFigurine(id: string): Promise<Figurine | null> {
        if (isTauri) return invoke('get_figurine', { id });
        try {
            return await webFetch(`/figurines/${id}`);
        } catch (e: unknown) {
            if (e instanceof Error && e.message.includes('404')) return null;
            throw e;
        }
    },

    async getAuthorTexts(): Promise<AuthorText[]> {
        if (isTauri) return invoke('get_author_texts');
        return webFetch('/content/texts/author');
    },

    async getWorkshopContent(): Promise<WorkshopItem[]> {
        if (isTauri) return invoke('get_workshop_content');
        return webFetch('/content/texts/workshop');
    },

    async getCabinetZones(): Promise<CabinetZone[]> {
        if (isTauri) return invoke('get_cabinet_zones');
        return webFetch('/cabinet/zones');
    },

    // === WRITE (ADMIN) ===
    async saveFigurine(figurine: Figurine): Promise<void> {
        if (isTauri) return invoke('save_figurine', { figurine });
        await webFetch('/figurines', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(figurine),
        });
    },

    async deleteFigurine(id: string): Promise<void> {
        if (isTauri) return invoke('delete_figurine', { id });
        const res = await fetch(`${webApiBase()}/figurines/${id}`, {
            method: 'DELETE',
            headers: authHeaders(),
        });
        if (!res.ok) throw new Error(`Delete failed: ${res.status}`);
    },

    async cleanupUnusedMedia(): Promise<string[]> {
        if (isTauri) return invoke('cleanup_unused_media');
        throw new Error('Очистка локальных медиа доступна только в Tauri-приложении.');
    },

    async getMediaInventory(): Promise<MediaInventory> {
        if (isTauri) return invoke('get_media_inventory');
        return webFetch('/admin/media', { headers: authHeaders() });
    },

    async getUnusedMediaReport(): Promise<MediaCleanupReport> {
        if (isTauri) return invoke('get_unused_media_report');
        return webFetch('/admin/media/cleanup-report', { headers: authHeaders() });
    },

    async cleanupReportedUnusedMedia(): Promise<string[]> {
        if (isTauri) return invoke('cleanup_reported_unused_media');
        const data = await webFetch<{ removed: string[] }>('/admin/media/cleanup', {
            method: 'POST',
            headers: authHeaders(),
        });
        return data.removed;
    },

    async replaceMediaEverywhere(oldPath: string, replacementFilePath: string | File): Promise<MediaReplaceResult> {
        if (isTauri) {
            return invoke('replace_media_everywhere', { oldPath, replacementFilePath });
        }
        const form = new FormData();
        form.append('targetPath', oldPath);
        form.append('file', replacementFilePath as File);
        const res = await fetch(`${webApiBase()}/admin/media/replace`, {
            method: 'POST',
            headers: authHeaders(),
            body: form,
        });
        if (!res.ok) throw new Error(`Replace failed: ${res.status}`);
        return res.json();
    },

    // fileOrPath is a local path string in Tauri, a File object on web
    async importMedia(fileOrPath: string | File, mediaType: 'images' | 'videos' | 'audio'): Promise<string> {
        const media = await api.importMediaWithVariants(fileOrPath, mediaType);
        return media.url;
    },

    async importMediaWithVariants(fileOrPath: string | File, mediaType: 'images' | 'videos' | 'audio'): Promise<ImportedMedia> {
        if (isTauri) {
            const url = await invoke<string>('import_media', { filePath: fileOrPath as string, mediaType });
            return { url };
        }
        const file = fileOrPath as File;
        const form = new FormData();
        form.append('file', file);
        const res = await fetch(`${webApiBase()}/upload`, {
            method: 'POST',
            headers: authHeaders(),
            body: form,
        });
        if (!res.ok) throw new Error(`Upload failed: ${res.status}`);
        const data = await res.json();
        return {
            url: webPublicUrl(data.url) ?? '',
            originalUrl: webPublicUrl(data.originalUrl),
            thumbUrl: webPublicUrl(data.thumbUrl),
        };
    },

    async saveCabinetZone(zone: CabinetZone): Promise<void> {
        if (isTauri) return invoke('save_cabinet_zone', { zone });
        await webFetch('/cabinet/zones', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(zone),
        });
    },

    async deleteCabinetZone(id: string): Promise<void> {
        if (isTauri) return invoke('delete_cabinet_zone', { id });
        const res = await fetch(`${webApiBase()}/cabinet/zones/${id}`, {
            method: 'DELETE',
            headers: authHeaders(),
        });
        if (!res.ok) throw new Error(`Delete failed: ${res.status}`);
    },

    async saveText(item: WorkshopItem | AuthorText, category: 'author' | 'workshop'): Promise<void> {
        const dto = {
            id: item.id,
            content: item.content,
            caption: (item as WorkshopItem).caption ?? null,
            imageUrl: (item as WorkshopItem).imageUrl ?? null,
        };
        if (isTauri) return invoke('save_text', { dto, category });
        await webFetch(`/content/texts/${category}`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(dto),
        });
    },

    async deleteText(id: string): Promise<void> {
        if (isTauri) return invoke('delete_text', { id });
        const res = await fetch(`${webApiBase()}/content/texts/${id}`, {
            method: 'DELETE',
            headers: authHeaders(),
        });
        if (!res.ok) throw new Error(`Delete failed: ${res.status}`);
    },

    async getMainBackground(): Promise<string | null> {
        if (isTauri) return invoke('get_main_background');
        try {
            const data = await webFetch<{ url: string | null }>('/main-background');
            return data.url;
        } catch {
            return null;
        }
    },

    async getHomeContent(): Promise<HomeContent> {
        if (isTauri) return invoke('get_home_content');
        try {
            return await webFetch('/home-content');
        } catch {
            return getWebHomeContent();
        }
    },

    async saveHomeContent(content: HomeContent): Promise<void> {
        if (isTauri) return invoke('save_home_content', { content });
        try {
            await webFetch('/home-content', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json', ...authHeaders() },
                body: JSON.stringify(content),
            });
        } catch {
            if (typeof localStorage === 'undefined') return;
            localStorage.setItem('gotiga_home_title', content.title ?? '');
            localStorage.setItem('gotiga_home_kicker', content.kicker ?? '');
            localStorage.setItem('gotiga_home_lead', content.lead ?? '');
        }
    },

    async setMainBackground(fileOrPath: string | File): Promise<string> {
        if (isTauri) return invoke('set_main_background', { filePath: fileOrPath as string });
        const file = fileOrPath as File;
        const form = new FormData();
        form.append('file', file);
        const res = await fetch(`${webApiBase()}/main-background`, {
            method: 'POST',
            headers: authHeaders(),
            body: form,
        });
        if (!res.ok) throw new Error(`Upload failed: ${res.status}`);
        const data = await res.json();
        return data.url as string;
    },

    // === SYNC & SETTINGS ===
    async getSettings(): Promise<AppSettings> {
        if (isTauri) return invoke('get_settings');
        return getWebSettings();
    },

    async saveSettings(settings: AppSettings): Promise<void> {
        if (isTauri) return invoke('save_settings', { settings });
        localStorage.setItem('gotiga_server_url', settings.serverUrl);
        localStorage.setItem('gotiga_api_key', settings.apiKey);
    },

    async exportRelease(): Promise<string> {
        if (isTauri) return invoke('export_release');
        throw new Error('В веб-режиме данные сохраняются напрямую на сервер без создания релизов.');
    },

    async pullUpdates(): Promise<string> {
        if (isTauri) return invoke('pull_updates');
        throw new Error('В веб-режиме данные загружаются напрямую с сервера.');
    },

    async pushFigurine(figurine: Figurine): Promise<string> {
        if (isTauri) return invoke('push_figurine', { figurine });
        await api.saveFigurine(figurine);
        return 'Сохранено на сервере';
    },

    async getServerReleases(): Promise<ServerRelease[]> {
        if (isTauri) return invoke('get_server_releases');
        try {
            return await webFetch('/admin/releases', { headers: authHeaders() });
        } catch {
            return [];
        }
    },

    async activateServerRelease(id: string): Promise<void> {
        if (isTauri) return invoke('activate_server_release', { id });
        await webFetch(`/admin/releases/${id}/activate`, {
            method: 'POST',
            headers: authHeaders(),
        });
    },

    async getAuthorProfile(): Promise<AuthorProfile> {
        if (isTauri) return invoke('get_author_profile');
        return webFetch('/author/profile');
    },

    async saveAuthorProfile(profile: AuthorProfile): Promise<void> {
        if (isTauri) return invoke('save_author_profile', { profile });
        await webFetch('/author/profile', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(profile),
        });
    },

    async submitOrder(order: OrderRequest): Promise<void> {
        await webFetch('/orders', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(order),
        });
    },

    async listOrders(opts?: { status?: string; page?: number; perPage?: number }): Promise<import('./types/api').OrdersPage> {
        const p = new URLSearchParams();
        if (opts?.status)  p.set('status',  opts.status);
        if (opts?.page)    p.set('page',    String(opts.page));
        if (opts?.perPage) p.set('perPage', String(opts.perPage));
        const qs = p.toString() ? `?${p}` : '';
        return webFetch(`/admin/orders${qs}`, { headers: authHeaders() });
    },

    async updateOrderStatus(id: string, status: 'new' | 'seen' | 'replied'): Promise<void> {
        await webFetch(`/admin/orders/${id}`, {
            method: 'PATCH',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify({ status }),
        });
    },

    // === SCHEDULE & BOOKINGS (PUBLIC) ===

    async getFigurineSchedule(figurineId: string): Promise<FigurineSchedule> {
        try {
            return await webFetch(`/figurines/${figurineId}/schedule`);
        } catch {
            return { entries: [] };
        }
    },

    async submitBooking(req: CreateBookingRequest): Promise<void> {
        await webFetch(`/figurines/${req.figurineId}/book`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(req),
        });
    },

    // === SHOWINGS (ADMIN) ===

    async listShowings(): Promise<ShowingDto[]> {
        return webFetch('/admin/showings', { headers: authHeaders() });
    },

    async saveShowing(req: SaveShowingRequest): Promise<ShowingDto> {
        return webFetch('/admin/showings', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(req),
        });
    },

    async deleteShowing(id: string): Promise<void> {
        const res = await fetch(`${webApiBase()}/admin/showings/${id}`, {
            method: 'DELETE',
            headers: authHeaders(),
        });
        if (!res.ok) throw new Error(`Delete failed: ${res.status}`);
    },

    // === BOOKINGS (ADMIN) ===

    async listBookings(opts?: { status?: string; figurineId?: string; page?: number; perPage?: number }): Promise<BookingsPage> {
        const p = new URLSearchParams();
        if (opts?.status)     p.set('status',     opts.status);
        if (opts?.figurineId) p.set('figurineId', opts.figurineId);
        if (opts?.page)       p.set('page',       String(opts.page));
        if (opts?.perPage)    p.set('perPage',     String(opts.perPage));
        const qs = p.toString() ? `?${p}` : '';
        return webFetch(`/admin/bookings${qs}`, { headers: authHeaders() });
    },

    async updateBookingStatus(id: string, status: string, adminNotes?: string): Promise<void> {
        await webFetch(`/admin/bookings/${id}/status`, {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify({ status, adminNotes: adminNotes ?? null }),
        });
    },
};
