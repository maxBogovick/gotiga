// src/lib/api.ts
import { invoke } from '@tauri-apps/api/core';
import type {
    FigurineListItem,
    Figurine,
    AuthorText,
    WorkshopItem,
    CabinetZone,
    AppSettings, ServerRelease
} from './types/api';

export const api = {
    // === READ ===
    async getAllFigurines(): Promise<FigurineListItem[]> {
        return invoke('get_all_figurines');
    },

    async getFigurine(id: string): Promise<Figurine | null> {
        return invoke('get_figurine', { id });
    },

    async getAuthorTexts(): Promise<AuthorText[]> {
        return invoke('get_author_texts');
    },

    async getWorkshopContent(): Promise<WorkshopItem[]> {
        return invoke('get_workshop_content');
    },

    async getCabinetZones(): Promise<CabinetZone[]> {
        return invoke('get_cabinet_zones');
    },

    // === WRITE (ADMIN) ===

    async saveFigurine(figurine: Figurine): Promise<void> {
        return invoke('save_figurine', { figurine });
    },

    async importMedia(filePath: string, mediaType: 'images' | 'videos' | 'audio'): Promise<string> {
        return invoke('import_media', { filePath, mediaType });
    },

    async saveCabinetZone(zone: CabinetZone): Promise<void> {
        return invoke('save_cabinet_zone', { zone });
    },

    async deleteCabinetZone(id: string): Promise<void> {
        return invoke('delete_cabinet_zone', { id });
    },

    // Used for both Author Texts and Workshop Items
    async saveText(item: WorkshopItem | AuthorText, category: 'author' | 'workshop'): Promise<void> {
        // Ensure AuthorText fits WorkshopItem shape if needed, or backend handles it
        // Backend expects generic structure: id, content, caption, imageUrl
        const dto = {
            id: item.id,
            content: item.content,
            caption: (item as WorkshopItem).caption || null,
            imageUrl: (item as WorkshopItem).imageUrl || null
        };
        return invoke('save_text', { dto, category });
    },

    async deleteText(id: string): Promise<void> {
        return invoke('delete_text', { id });
    },

    async getMainBackground(): Promise<string | null> {
        return invoke('get_main_background');
    },

    async setMainBackground(filePath: string): Promise<string> {
        return invoke('set_main_background', { filePath });
    },

    // === SYNC & SETTINGS ===

    async getSettings(): Promise<AppSettings> {
        return invoke('get_settings');
    },

    async saveSettings(settings: AppSettings): Promise<void> {
        return invoke('save_settings', { settings });
    },

    async exportRelease(): Promise<string> {
        return invoke('export_release');
    },

    async pullUpdates(): Promise<string> {
        return invoke('pull_updates');
    },

    async pushFigurine(figurine: Figurine): Promise<string> {
        return invoke('push_figurine', { figurine });
    },

    async getServerReleases(): Promise<ServerRelease[]> {
        return invoke('get_server_releases');
    },

    async activateServerRelease(id: string): Promise<void> {
        return invoke('activate_server_release', { id });
    }
};
