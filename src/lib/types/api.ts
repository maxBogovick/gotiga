// Типы данных от backend

export interface FigurineListItem {
  id: string;
  name: string;
  status: 'available' | 'sold' | 'reserved';
  faceImageUrl: string | null;
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
  videoUrl?: string | null;
  secretText: string | null;
  status: 'available' | 'sold' | 'reserved';
  sortOrder: number;
  isVisible: boolean;
  images: FigurineImage[];
  processSteps: ProcessStep[];
  relatedItems: FigurineListItem[];
}

export interface FigurineImage {
  id: string;
  imageType: 'face' | 'detail' | 'full';
  url: string;
  altText: string | null;
}

export interface ProcessStep {
  id: string;
  stepType: 'sketch' | 'prototype' | 'modeling' | 'painting' | 'finish';
  description: string | null;
  imageUrl: string;
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
  zoneType: 'showcase' | 'desk' | 'shelf' | 'note' | 'curator' | 'cabinet' | 'portrait' | 'windows';
  x: number;
  y: number;
  width: number;
  height: number;
  targetRoute: string;
}
