import { invoke } from '@tauri-apps/api/core';
import type {
  FigurineListItem,
  Figurine,
  AuthorText,
  WorkshopItem,
  CabinetZone
} from './types/api';

/**
 * Custom error class for API errors
 */
export class ApiError extends Error {
  constructor(
    message: string,
    public readonly code?: string,
    public readonly originalError?: unknown
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

/**
 * Wraps Tauri invoke calls with error handling
 */
async function safeInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error(`[API] ${command} failed:`, error);
    throw new ApiError(`Ошибка при выполнении ${command}: ${message}`, command, error);
  }
}

/**
 * API client for Cabinet of Curiosities
 */
export const api = {
  // ═══════════════════════════════════════════════════════════════
  // FIGURINES
  // ═══════════════════════════════════════════════════════════════

  /**
   * Get list of all figurines (for showcase)
   */
  async getAllFigurines(): Promise<FigurineListItem[]> {
    return safeInvoke<FigurineListItem[]>('get_all_figurines');
  },

  /**
   * Get detailed figurine info by ID
   * @returns Figurine or null if not found
   */
  async getFigurine(id: string): Promise<Figurine | null> {
    if (!id) {
      throw new ApiError('ID фигуры не указан', 'INVALID_ID');
    }
    return safeInvoke<Figurine | null>('get_figurine', { id });
  },

  // ═══════════════════════════════════════════════════════════════
  // TEXTS
  // ═══════════════════════════════════════════════════════════════

  /**
   * Get author presence texts
   */
  async getAuthorTexts(): Promise<AuthorText[]> {
    return safeInvoke<AuthorText[]>('get_author_texts');
  },

  /**
   * Get workshop content items
   */
  async getWorkshopContent(): Promise<WorkshopItem[]> {
    return safeInvoke<WorkshopItem[]>('get_workshop_content');
  },

  // ═══════════════════════════════════════════════════════════════
  // CABINET ZONES
  // ═══════════════════════════════════════════════════════════════

  /**
   * Get interactive zones for the cabinet room
   */
  async getCabinetZones(): Promise<CabinetZone[]> {
    return safeInvoke<CabinetZone[]>('get_cabinet_zones');
  },
} as const;

// Type for the api object
export type Api = typeof api;
