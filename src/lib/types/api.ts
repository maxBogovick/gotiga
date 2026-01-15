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
  year: number | null;
  status: 'available' | 'sold' | 'reserved';
  images: FigurineImage[];
}

export interface FigurineImage {
  id: string;
  imageType: 'face' | 'detail' | 'full';
  url: string;
  altText: string | null;
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
  zoneType: 'showcase' | 'desk' | 'shelf' | 'note';
  x: number;
  y: number;
  width: number;
  height: number;
  targetRoute: string;
}
