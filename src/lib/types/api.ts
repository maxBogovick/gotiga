// src/lib/types/api.ts

export interface FigurineListItem {
    id: string;
    name: string;
    status: 'available' | 'sold' | 'reserved';
    faceImageUrl: string | null;
    year?: number | null;
    sortOrder?: number;
    series?: string | null;
    technique?: string | null;
    material?: string | null;
}

export interface FigurineImage {
    id: string;
    imageType: 'face' | 'detail' | 'full';
    url: string;
    originalUrl: string | null;
    thumbUrl: string | null;
    altText: string | null;
}

export interface ProcessStep {
    id: string;
    stepType: 'sketch' | 'prototype' | 'modeling' | 'painting' | 'finish';
    description: string | null;
    imageUrl: string;
}

export interface Figurine {
    id: string;
    name: string;
    shortText: string | null;
    fullDescription: string | null;
    dimensions: string | null;
    material: string | null;
    technique: string | null;
    year: number | null;
    ambiencePath: string | null;
    videoUrl: string | null;
    secretText: string | null;
    status: 'available' | 'sold' | 'reserved';
    sortOrder: number;
    isVisible: boolean;
    images: FigurineImage[];
    processSteps: ProcessStep[];
    relatedItems: FigurineListItem[];
}

export interface AuthorText {
    id: string;
    content: string;
}

export interface WorkshopItem {
    id: string;
    content: string;
    caption: string | null;
    imageUrl: string | null;
}

export interface CabinetZone {
    id: string;
    zoneType: string;
    x: number;
    y: number;
    width: number;
    height: number;
    targetRoute: string;
}

export interface ServerRelease {
    id: string;
    version: number;
    createdAt: string;
    description: string | null;
    isActive: boolean;
}

export interface AppSettings {
    serverUrl: string;
    apiKey: string;
}

export interface HomeContent {
    title: string | null;
    kicker: string | null;
    lead: string | null;
}

export interface AuthorProfile {
    name: string;
    tagline: string | null;
    bio: string | null;
    photoUrl: string | null;
    instagram: string | null;
    telegram: string | null;
    vk: string | null;
    email: string | null;
}

export interface OrderRequest {
    figurineId: string;
    figurineName: string;
    requesterName: string;
    requesterEmail: string;
    message: string | null;
    mode: 'request' | 'question' | 'notify';
}

export interface Order {
    id: string;
    figurineId: string;
    figurineName: string;
    requesterName: string;
    requesterEmail: string;
    message: string | null;
    mode: 'request' | 'question' | 'notify';
    status: 'new' | 'seen' | 'replied';
    createdAt: string;
}

export interface MediaUsage {
    path: string;
    label: string;
    entityType: string;
    entityId: string;
    field: string;
}

export interface MediaFile {
    path: string;
    url: string;
    mediaType: 'image' | 'video' | 'audio' | 'other';
    variant: string | null;
    sizeBytes: number;
    exists: boolean;
    usages: MediaUsage[];
}

export interface MediaInventory {
    files: MediaFile[];
    orphanCount: number;
    usedCount: number;
    totalSizeBytes: number;
}

export interface MediaCleanupReport {
    files: MediaFile[];
    totalSizeBytes: number;
}

export interface MediaReplaceResult {
    oldPath: string;
    newPath: string;
    updatedReferences: number;
    importedPaths: string[];
}
