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
    LoginChallengeResponse,
    LoginVerifyResponse,
    UserDto,
    UserBookingDto,
    UserOrderDto,
    AdminUsersPage,
    AdminUserDetail,
    ResetTokenResponse,
    CommentDto,
    AdminCommentsPage,
    SubmitCommentRequest,
    ModerateCommentRequest,
    AdminCommentDto,
    SmtpSettings,
    ContactSettings,
    BookingRules,
    RescheduleBookingRequest,
    CreateWaitlistRequest,
    WaitlistEntryDto,
    MessageThreadDto,
    ThreadMessageDto,
    ThreadDetailDto,
    ThemeConfig,
    CopyOverrides,
    CommissionRequest,
    CommissionDto,
    CommissionCreatedResponse,
    CommissionsPage,
    EditCommissionRequest,
    AttachmentInput,
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
    // Admin token may live in sessionStorage when "remember me" is off — it must not
    // outlive the tab. Fall back to it so API calls stay authorized either way.
    const sessionKey = typeof sessionStorage !== 'undefined'
        ? sessionStorage.getItem('gotiga_api_key')
        : null;
    return {
        serverUrl: localStorage.getItem('gotiga_server_url') ?? '',
        apiKey: localStorage.getItem('gotiga_api_key') ?? sessionKey ?? '',
    };
}

function getWebHomeContent(): HomeContent {
    if (typeof localStorage === 'undefined') {
        return {
            title: null,
            kicker: null,
            lead: null,
            heroFigurineId: null,
            heroCaptionTitle: null,
            heroCaptionMeta: null,
            heroCaptionCta: null,
            heroMode: null,
        };
    }
    return {
        title: localStorage.getItem('gotiga_home_title'),
        kicker: localStorage.getItem('gotiga_home_kicker'),
        lead: localStorage.getItem('gotiga_home_lead'),
        heroFigurineId: localStorage.getItem('gotiga_home_hero_figurine_id'),
        heroCaptionTitle: localStorage.getItem('gotiga_home_hero_caption_title'),
        heroCaptionMeta: localStorage.getItem('gotiga_home_hero_caption_meta'),
        heroCaptionCta: localStorage.getItem('gotiga_home_hero_caption_cta'),
        heroMode: (localStorage.getItem('gotiga_home_hero_mode') as HomeContent['heroMode']) || null,
    };
}

function webApiBase(): string {
    const { serverUrl } = getWebSettings();
    if (serverUrl) return `${serverUrl}/api/v1`;
    // In the browser always use a same-origin relative path so requests go through the
    // nginx /api proxy. Only during prerender/SSR (Node — no window, no localStorage)
    // do we need an absolute origin, injected at build time via VITE_API_BASE.
    if (typeof window === 'undefined') {
        const buildBase = import.meta.env.VITE_API_BASE;
        if (buildBase) return `${buildBase.replace(/\/$/, '')}/api/v1`;
    }
    return '/api/v1';
}

/**
 * Resolve a media path (avatar, image, …) to a loadable URL.
 * Relative `/static/` paths are prefixed with the configured server origin in web mode.
 * Shared helper — previously duplicated across SiteHeader/OrderModal/BookingModal/etc.
 */
export function resolveMediaUrl(url: string | null | undefined): string | null {
    if (!url) return null;
    const value = url.trim();
    if (!value) return null;

    const serverUrl = typeof localStorage !== 'undefined'
        ? (localStorage.getItem('gotiga_server_url') ?? '').replace(/\/$/, '')
        : '';

    if (
        value.startsWith('http://') ||
        value.startsWith('https://')
    ) {
        if (serverUrl) {
            try {
                const parsed = new URL(value);
                if (parsed.pathname.startsWith('/static/') || parsed.pathname.startsWith('/api/v1/assets/')) {
                    return `${serverUrl}${parsed.pathname}${parsed.search}${parsed.hash}`;
                }
            } catch {
                return value;
            }
        }
        return value;
    }
    if (value.startsWith('data:') || value.startsWith('blob:') || value.startsWith('file:')) {
        return value;
    }

    if (value.startsWith('//') && typeof location !== 'undefined') {
        return `${location.protocol}${value}`;
    }

    if (value.startsWith('/static/')) {
        return serverUrl ? `${serverUrl}${value}` : value;
    }

    if (value.startsWith('static/')) {
        return serverUrl ? `${serverUrl}/${value}` : `/${value}`;
    }

    if (value.startsWith('/api/v1/assets/')) {
        return serverUrl ? `${serverUrl}${value}` : value;
    }

    // Legacy/local stored media paths, e.g. "images/original/x.jpg" or
    // "backgrounds/x.jpg". In web mode those are served from the API server's
    // static mount, not from the Svelte route.
    if (/^(images|backgrounds|avatars|profile-uploads|uploads)\//.test(value)) {
        return serverUrl ? `${serverUrl}/static/${value}` : `/static/${value}`;
    }

    return value;
}

function webPublicUrl(url: unknown): string | null {
    if (typeof url !== 'string') return null;
    return resolveMediaUrl(url);
}

class ApiError extends Error {
    status: number;

    constructor(status: number, body: string) {
        super(`API ${status}: ${body}`);
        this.name = 'ApiError';
        this.status = status;
    }
}

function isNotFoundError(err: unknown): boolean {
    return err instanceof ApiError && (err.status === 404 || err.status === 410);
}

async function webFetch<T>(path: string, options?: RequestInit): Promise<T> {
    const res = await fetch(`${webApiBase()}${path}`, options);
    if (!res.ok) {
        const text = await res.text().catch(() => '');
        throw new ApiError(res.status, text);
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
            localStorage.setItem('gotiga_home_hero_figurine_id', content.heroFigurineId ?? '');
            localStorage.setItem('gotiga_home_hero_caption_title', content.heroCaptionTitle ?? '');
            localStorage.setItem('gotiga_home_hero_caption_meta', content.heroCaptionMeta ?? '');
            localStorage.setItem('gotiga_home_hero_caption_cta', content.heroCaptionCta ?? '');
            localStorage.setItem('gotiga_home_hero_mode', content.heroMode ?? '');
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

    async submitOrder(order: OrderRequest): Promise<import('./types/api').OrderCreatedResponse> {
        return webFetch('/orders', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(order),
        });
    },

    async getNotifyByToken(token: string): Promise<import('./types/api').NotifyInfo | null> {
        try {
            return await webFetch(`/orders/notify/${token}`);
        } catch (err) {
            if (isNotFoundError(err)) return null;
            throw err;
        }
    },

    async cancelNotifyByToken(token: string): Promise<void> {
        await webFetch(`/orders/notify/${token}`, { method: 'POST' });
    },

    async listOrders(opts?: { status?: string; page?: number; perPage?: number }): Promise<import('./types/api').OrdersPage> {
        const p = new URLSearchParams();
        if (opts?.status)  p.set('status',  opts.status);
        if (opts?.page)    p.set('page',    String(opts.page));
        if (opts?.perPage) p.set('perPage', String(opts.perPage));
        const qs = p.toString() ? `?${p}` : '';
        return webFetch(`/admin/orders${qs}`, { headers: authHeaders() });
    },

    async updateOrderStatus(id: string, status: 'new' | 'seen' | 'replied', adminNotes?: string): Promise<void> {
        await webFetch(`/admin/orders/${id}`, {
            method: 'PATCH',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify({ status, adminNotes }),
        });
    },

    // === SCHEDULE & BOOKINGS (PUBLIC) ===

    async getFigurineSchedule(figurineId: string): Promise<FigurineSchedule> {
        return webFetch(`/figurines/${figurineId}/schedule`);
    },

    async submitBooking(req: CreateBookingRequest): Promise<import('./types/api').BookingCreatedResponse> {
        return webFetch(`/figurines/${req.figurineId}/book`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(req),
        });
    },

    async getBookingByToken(token: string): Promise<import('./types/api').BookingCancelInfo> {
        return webFetch(`/bookings/cancel/${encodeURIComponent(token)}`);
    },

    // Batch token lookup — one request for many claim tokens. Tokens not found are
    // simply absent from the returned map (same semantics as a 404 on the single GET).
    async getBookingsByTokens(tokens: string[]): Promise<Record<string, import('./types/api').BookingCancelInfo>> {
        if (tokens.length === 0) return {};
        return webFetch('/bookings/by-tokens', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ tokens }),
        });
    },

    async cancelBookingByToken(token: string): Promise<void> {
        await webFetch(`/bookings/cancel/${encodeURIComponent(token)}`, { method: 'POST' });
    },

    async rescheduleBookingByToken(token: string, req: RescheduleBookingRequest): Promise<import('./types/api').BookingCancelInfo> {
        return webFetch(`/bookings/cancel/${encodeURIComponent(token)}/reschedule`, {
            method: 'PATCH',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(req),
        });
    },

    async getBookingRules(): Promise<BookingRules> {
        return webFetch('/booking-rules');
    },

    async saveBookingRules(rules: BookingRules): Promise<void> {
        await webFetch('/admin/booking-rules', {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(rules),
        });
    },

    async joinWaitlist(figurineId: string, req: CreateWaitlistRequest): Promise<import('./types/api').WaitlistCreatedResponse> {
        return webFetch(`/figurines/${figurineId}/waitlist`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(req),
        });
    },

    async getWaitlistByToken(token: string): Promise<import('./types/api').WaitlistCancelInfo | null> {
        try {
            return await webFetch(`/waitlist/leave/${token}`);
        } catch (err) {
            if (isNotFoundError(err)) return null;
            throw err;
        }
    },

    async leaveWaitlistByToken(token: string): Promise<void> {
        await webFetch(`/waitlist/leave/${token}`, { method: 'POST' });
    },

    async adminListWaitlist(figurineId?: string): Promise<WaitlistEntryDto[]> {
        const qs = figurineId ? `?figurineId=${encodeURIComponent(figurineId)}` : '';
        return webFetch(`/admin/waitlist${qs}`, { headers: authHeaders() });
    },

    async adminRemoveFromWaitlist(id: string): Promise<void> {
        const res = await fetch(`${webApiBase()}/admin/waitlist/${id}`, {
            method: 'DELETE',
            headers: authHeaders(),
        });
        if (!res.ok) throw new Error(`Delete failed: ${res.status}`);
    },

    async adminNotifyWaitlist(figurineId: string): Promise<{ notified: number; total: number }> {
        return webFetch(`/admin/waitlist/${figurineId}/notify`, {
            method: 'POST',
            headers: authHeaders(),
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

    async updateBookingStatus(id: string, status: string, adminNotes?: string, curatorConditions?: string): Promise<void> {
        await webFetch(`/admin/bookings/${id}/status`, {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify({ status, adminNotes: adminNotes ?? null, curatorConditions: curatorConditions ?? null }),
        });
    },

    // === USER AUTH ===

    async userRegister(email: string, displayName: string, selections: [string, string, string, string]): Promise<{ user: UserDto }> {
        return webFetch('/auth/register', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ email, displayName, selections }),
        });
    },

    async userLoginChallenge(email: string): Promise<LoginChallengeResponse> {
        return webFetch('/auth/login/challenge', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ email }),
        });
    },

    async userLoginVerify(challengeId: string, tokens: [string, string, string, string]): Promise<LoginVerifyResponse> {
        return webFetch('/auth/login/verify', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ challengeId, tokens }),
        });
    },

    async userLogout(sessionToken: string): Promise<void> {
        await webFetch('/auth/logout', {
            method: 'POST',
            headers: { Authorization: `Bearer ${sessionToken}` },
        });
    },

    async userMe(sessionToken: string): Promise<UserDto> {
        return webFetch('/auth/me', {
            headers: { Authorization: `Bearer ${sessionToken}` },
        });
    },

    async userLinkBookings(sessionToken: string, cancelTokens: string[]): Promise<{ linked: number }> {
        return webFetch('/auth/link-bookings', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${sessionToken}` },
            body: JSON.stringify({ cancelTokens }),
        });
    },

    async userProfileBookings(sessionToken: string): Promise<UserBookingDto[]> {
        return webFetch('/profile/bookings', {
            headers: { Authorization: `Bearer ${sessionToken}` },
        });
    },

    async userProfileOrders(sessionToken: string): Promise<UserOrderDto[]> {
        return webFetch('/profile/orders', {
            headers: { Authorization: `Bearer ${sessionToken}` },
        });
    },

    async updateProfile(sessionToken: string, displayName: string): Promise<UserDto> {
        return webFetch('/profile/me', {
            method: 'PATCH',
            headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${sessionToken}` },
            body: JSON.stringify({ displayName }),
        });
    },

    async uploadAvatar(sessionToken: string, file: File): Promise<UserDto> {
        const form = new FormData();
        form.append('file', file);
        return webFetch('/profile/avatar', {
            method: 'POST',
            headers: { Authorization: `Bearer ${sessionToken}` },
            body: form,
        });
    },

    async deleteAccount(sessionToken: string): Promise<void> {
        await webFetch('/profile/me', {
            method: 'DELETE',
            headers: { Authorization: `Bearer ${sessionToken}` },
        });
    },

    // === ADMIN USER MANAGEMENT ===

    async adminListUsers(opts?: { search?: string; page?: number; perPage?: number }): Promise<AdminUsersPage> {
        const p = new URLSearchParams();
        if (opts?.search)   p.set('search',   opts.search);
        if (opts?.page)     p.set('page',     String(opts.page));
        if (opts?.perPage)  p.set('perPage',  String(opts.perPage));
        const qs = p.toString() ? `?${p}` : '';
        return webFetch(`/admin/users${qs}`, { headers: authHeaders() });
    },

    async adminGetUser(id: string): Promise<AdminUserDetail> {
        return webFetch(`/admin/users/${id}`, { headers: authHeaders() });
    },

    async adminRevokeUserSessions(id: string): Promise<{ revoked: number }> {
        return webFetch(`/admin/users/${id}/sessions`, {
            method: 'DELETE',
            headers: authHeaders(),
        });
    },

    async adminUpdateUserNotes(id: string, adminNotes: string | null): Promise<void> {
        await webFetch(`/admin/users/${id}/notes`, {
            method: 'PATCH',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify({ adminNotes }),
        });
    },

    async adminSetUserBlocked(id: string, blocked: boolean): Promise<void> {
        await webFetch(`/admin/users/${id}/block`, {
            method: 'PATCH',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify({ blocked }),
        });
    },

    async adminGenerateResetToken(id: string): Promise<ResetTokenResponse> {
        return webFetch(`/admin/users/${id}/reset-token`, {
            method: 'POST',
            headers: authHeaders(),
        });
    },

    async getUserThreads(sessionToken: string): Promise<{ threads: MessageThreadDto[]; unread: number }> {
        return webFetch('/profile/threads', { headers: { Authorization: `Bearer ${sessionToken}` } });
    },

    async getThread(sessionToken: string, threadId: string): Promise<ThreadDetailDto> {
        return webFetch(`/profile/threads/${threadId}`, { headers: { Authorization: `Bearer ${sessionToken}` } });
    },

    async createThread(sessionToken: string, subject: string, body: string, category?: string, attachmentUrls?: AttachmentInput[]): Promise<ThreadDetailDto> {
        return webFetch('/profile/threads', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${sessionToken}` },
            body: JSON.stringify({ subject, body, category, attachmentUrls }),
        });
    },

    async replyToThread(sessionToken: string, threadId: string, body: string, attachmentUrls?: AttachmentInput[]): Promise<ThreadMessageDto> {
        return webFetch(`/profile/threads/${threadId}/reply`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${sessionToken}` },
            body: JSON.stringify({ body, attachmentUrls }),
        });
    },

    /** Upload an image as a logged-in user (commission refs, chat attachments). */
    async uploadUserMedia(sessionToken: string, file: File): Promise<AttachmentInput> {
        const form = new FormData();
        form.append('file', file);
        const res = await fetch(`${webApiBase()}/profile/uploads`, {
            method: 'POST',
            headers: { Authorization: `Bearer ${sessionToken}` },
            body: form,
        });
        if (!res.ok) throw new Error(`Upload failed: ${res.status}`);
        const data = await res.json();
        return {
            url: webPublicUrl(data.url) ?? '',
            thumbUrl: webPublicUrl(data.thumbUrl),
        };
    },

    async adminListThreads(opts?: { category?: string; status?: string; page?: number; perPage?: number }): Promise<{ items: Array<{ thread: MessageThreadDto; user: { id: string; displayName: string; email: string } }>; total: number; page: number; perPage: number }> {
        const p = new URLSearchParams();
        if (opts?.category) p.set('category', opts.category);
        if (opts?.status)   p.set('status',   opts.status);
        if (opts?.page)     p.set('page',     String(opts.page));
        if (opts?.perPage)  p.set('perPage',  String(opts.perPage));
        const qs = p.toString() ? `?${p}` : '';
        return webFetch(`/admin/threads${qs}`, { headers: authHeaders() });
    },

    async adminGetThread(threadId: string): Promise<ThreadDetailDto> {
        return webFetch(`/admin/threads/${threadId}`, { headers: authHeaders() });
    },

    async adminCreateThreadForUser(userId: string, subject: string, body: string, category?: string, attachmentUrls?: AttachmentInput[]): Promise<ThreadDetailDto> {
        return webFetch(`/admin/users/${userId}/threads`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify({ subject, body, category, attachmentUrls }),
        });
    },

    async adminReplyToThread(threadId: string, body: string, attachmentUrls?: AttachmentInput[]): Promise<ThreadMessageDto> {
        return webFetch(`/admin/threads/${threadId}/reply`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify({ body, attachmentUrls }),
        });
    },

    // === COMMISSIONS ===
    async submitCommission(req: CommissionRequest, sessionToken?: string): Promise<CommissionCreatedResponse> {
        const headers: Record<string, string> = { 'Content-Type': 'application/json' };
        if (sessionToken) headers.Authorization = `Bearer ${sessionToken}`;
        return webFetch('/commissions', {
            method: 'POST',
            headers,
            body: JSON.stringify(req),
        });
    },

    async getCommissionByToken(token: string): Promise<CommissionDto> {
        return webFetch(`/commissions/${token}`);
    },

    async claimCommission(sessionToken: string, claimToken: string): Promise<CommissionDto> {
        return webFetch('/profile/commissions/claim', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${sessionToken}` },
            body: JSON.stringify({ claimToken }),
        });
    },

    async getUserCommissions(sessionToken: string): Promise<CommissionDto[]> {
        const data = await webFetch<{ commissions: CommissionDto[] }>('/profile/commissions', {
            headers: { Authorization: `Bearer ${sessionToken}` },
        });
        return data.commissions;
    },

    async editCommission(sessionToken: string, id: string, req: EditCommissionRequest): Promise<CommissionDto> {
        return webFetch(`/profile/commissions/${id}`, {
            method: 'PATCH',
            headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${sessionToken}` },
            body: JSON.stringify(req),
        });
    },

    async deleteCommission(sessionToken: string, id: string): Promise<void> {
        await webFetch(`/profile/commissions/${id}`, {
            method: 'DELETE',
            headers: { Authorization: `Bearer ${sessionToken}` },
        });
    },

    async adminDeleteCommission(id: string): Promise<void> {
        await webFetch(`/admin/commissions/${id}`, {
            method: 'DELETE',
            headers: authHeaders(),
        });
    },

    async adminListCommissions(opts?: { status?: string; page?: number; perPage?: number }): Promise<CommissionsPage> {
        const p = new URLSearchParams();
        if (opts?.status)  p.set('status',  opts.status);
        if (opts?.page)    p.set('page',    String(opts.page));
        if (opts?.perPage) p.set('perPage', String(opts.perPage));
        const qs = p.toString() ? `?${p}` : '';
        return webFetch(`/admin/commissions${qs}`, { headers: authHeaders() });
    },

    async updateCommissionStatus(id: string, status: CommissionDto['status'], opts?: { adminNotes?: string; figurineId?: string }): Promise<CommissionDto> {
        return webFetch(`/admin/commissions/${id}`, {
            method: 'PATCH',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify({ status, adminNotes: opts?.adminNotes, figurineId: opts?.figurineId }),
        });
    },

    async adminResolveThread(threadId: string): Promise<void> {
        await webFetch(`/admin/threads/${threadId}/resolve`, { method: 'POST', headers: authHeaders() });
    },

    async adminReopenThread(threadId: string): Promise<void> {
        await webFetch(`/admin/threads/${threadId}/reopen`, { method: 'POST', headers: authHeaders() });
    },

    async validateResetToken(token: string): Promise<{ id: string; email: string; displayName: string }> {
        return webFetch(`/auth/reset-token/${token}`);
    },

    async applyPasswordReset(token: string, selections: [string, string, string, string]): Promise<void> {
        await webFetch('/auth/reset-password', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ token, selections }),
        });
    },

    // === COMMENTS ===

    async getComments(figurineId: string, newestFirst = false): Promise<CommentDto[]> {
        try {
            const qs = newestFirst ? '?sort=newest' : '';
            return await webFetch(`/figurines/${figurineId}/comments${qs}`);
        } catch {
            return [];
        }
    },

    async submitComment(figurineId: string, req: SubmitCommentRequest, sessionToken?: string | null): Promise<void> {
        const headers: Record<string, string> = { 'Content-Type': 'application/json' };
        if (sessionToken) headers['Authorization'] = `Bearer ${sessionToken}`;
        await webFetch(`/figurines/${figurineId}/comments`, {
            method: 'POST',
            headers,
            body: JSON.stringify(req),
        });
    },

    async adminListComments(opts?: { pending?: boolean; figurineId?: string; sort?: 'newest' | 'oldest'; page?: number; perPage?: number }): Promise<AdminCommentsPage> {
        const p = new URLSearchParams();
        if (opts?.pending)     p.set('pending',    'true');
        if (opts?.figurineId)  p.set('figurineId', opts.figurineId);
        if (opts?.sort)        p.set('sort',        opts.sort);
        if (opts?.page)        p.set('page',        String(opts.page));
        if (opts?.perPage)     p.set('perPage',     String(opts.perPage));
        const qs = p.toString() ? `?${p}` : '';
        return webFetch(`/admin/comments${qs}`, { headers: authHeaders() });
    },

    async adminModerateComment(id: string, req: ModerateCommentRequest): Promise<AdminCommentDto> {
        return webFetch(`/admin/comments/${id}`, {
            method: 'PATCH',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(req),
        });
    },

    async adminDeleteComment(id: string): Promise<void> {
        const res = await fetch(`${webApiBase()}/admin/comments/${id}`, {
            method: 'DELETE',
            headers: authHeaders(),
        });
        if (!res.ok) throw new Error(`Delete failed: ${res.status}`);
    },

    // === SMTP SETTINGS ===

    async getSmtpSettings(): Promise<SmtpSettings> {
        return webFetch('/admin/settings/smtp', { headers: authHeaders() });
    },

    async saveSmtpSettings(s: SmtpSettings): Promise<SmtpSettings> {
        return webFetch('/admin/settings/smtp', {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(s),
        });
    },

    // === CONTACT SETTINGS ===

    async getContactSettings(): Promise<ContactSettings> {
        return webFetch('/settings/contact');
    },

    async saveContactSettings(s: ContactSettings): Promise<ContactSettings> {
        return webFetch('/admin/settings/contact', {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(s),
        });
    },

    // === THEME CONFIG ===

    async getThemeConfig(): Promise<ThemeConfig> {
        return webFetch('/settings/theme');
    },

    async saveThemeConfig(config: ThemeConfig): Promise<ThemeConfig> {
        return webFetch('/admin/settings/theme', {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(config),
        });
    },

    // === COPY OVERRIDES ===

    async getCopyOverrides(): Promise<CopyOverrides> {
        return webFetch('/settings/copy');
    },

    async saveCopyOverrides(overrides: CopyOverrides): Promise<CopyOverrides> {
        return webFetch('/admin/settings/copy', {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify(overrides),
        });
    },
};
