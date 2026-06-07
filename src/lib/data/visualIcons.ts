export type IconCategory = 'animals' | 'dishes' | 'seasons' | 'colors';

export interface VisualIcon {
  id: string;
  labelEn: string;
  labelRu: string;
  svg: string; // inline SVG path data (viewBox="0 0 24 24", stroke-based)
}

export interface IconCategoryDef {
  id: IconCategory;
  labelEn: string;
  labelRu: string;
  icons: VisualIcon[];
}

// All SVGs: 24×24 viewBox, stroke="currentColor" fill="none" strokeWidth="1.5"
const S = 'stroke="currentColor" fill="none" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"';

export const VISUAL_CATEGORIES: IconCategoryDef[] = [
  {
    id: 'animals',
    labelEn: 'A creature',
    labelRu: 'Существо',
    icons: [
      {
        id: 'wolf',
        labelEn: 'Wolf', labelRu: 'Волк',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M3 6 l3-3 l2 2 l3-1 l1 3 c2-1 4 0 5 2 l2-1 l1 3-2 1c0 3-2 5-5 6l-2 1-2-1c-3-1-5-3-5-6L2 11l1-3 2 1z"/><circle cx="9" cy="11" r="1" fill="currentColor"/><circle cx="14" cy="11" r="1" fill="currentColor"/></svg>`,
      },
      {
        id: 'raven',
        labelEn: 'Raven', labelRu: 'Ворон',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 4c-2 0-4 1-5 3l-2 1 1 2 2-1c0 2 1 4 3 5v3l-2 2h8l-2-2v-3c2-1 3-3 3-5l2 1 1-2-2-1c-1-2-3-3-5-3z"/><path d="M10 9 l1 1 M14 9 l-1 1"/></svg>`,
      },
      {
        id: 'fox',
        labelEn: 'Fox', labelRu: 'Лиса',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M3 4 l4 4 c-1 1-2 3-2 5 0 3 3 6 7 6s7-3 7-6c0-2-1-4-2-5l4-4-5 2c-1-1-2-2-4-2s-3 1-4 2z"/><circle cx="10" cy="12" r="1" fill="currentColor"/><circle cx="14" cy="12" r="1" fill="currentColor"/><path d="M11 15 q1 1 2 0"/></svg>`,
      },
      {
        id: 'owl',
        labelEn: 'Owl', labelRu: 'Сова',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M8 4 l-3 2 2 1c-1 1-2 3-2 5 0 4 3 7 7 7s7-3 7-7c0-2-1-4-2-5l2-1-3-2-2 2c-1-1-2-1-2 0z"/><circle cx="10" cy="11" r="2"/><circle cx="14" cy="11" r="2"/><circle cx="10" cy="11" r="1" fill="currentColor"/><circle cx="14" cy="11" r="1" fill="currentColor"/><path d="M11 15 q1 1 2 0"/></svg>`,
      },
      {
        id: 'snake',
        labelEn: 'Snake', labelRu: 'Змея',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M5 18 c0-3 4-3 4-6s-4-3-4-6c0-2 2-3 4-3 3 0 5 2 5 4s-2 3-2 5 2 3 5 3"/><path d="M18 15 l2-1-1-2"/></svg>`,
      },
      {
        id: 'deer',
        labelEn: 'Deer', labelRu: 'Олень',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 9 v7 l-2 4 M12 16 l2 4"/><path d="M12 9 c-1-2-1-4 0-5 M12 9 c1-2 1-4 0-5"/><path d="M7 5 l-2-1 M7 6 l-2 1 M17 5 l2-1 M17 6 l2 1"/><ellipse cx="12" cy="12" rx="3" ry="2"/></svg>`,
      },
      {
        id: 'bat',
        labelEn: 'Bat', labelRu: 'Летучая мышь',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 10 c-1-1-3-2-5-2-1 0-3 1-4 2l1 2c1-1 2-1 3-1-1 1-1 2-1 3h12c0-1 0-2-1-3 1 0 2 0 3 1l1-2c-1-1-3-2-4-2-2 0-4 1-5 2z"/><path d="M10 14 q2 3 4 0"/><circle cx="10" cy="11" r="0.5" fill="currentColor"/><circle cx="14" cy="11" r="0.5" fill="currentColor"/></svg>`,
      },
      {
        id: 'cat',
        labelEn: 'Cat', labelRu: 'Кот',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M7 4 l-2 4 1 1c-1 1-1 2-1 3 0 3 3 6 7 6s7-3 7-6c0-1 0-2-1-3l1-1-2-4-2 3c-1-1-3-1-4 0z"/><circle cx="10" cy="12" r="1" fill="currentColor"/><circle cx="14" cy="12" r="1" fill="currentColor"/><path d="M10 15 q2 2 4 0"/><path d="M9 11 l-2-1 M15 11 l2-1 M12 11 v-1"/></svg>`,
      },
    ],
  },
  {
    id: 'dishes',
    labelEn: 'A dish',
    labelRu: 'Блюдо',
    icons: [
      {
        id: 'mushroom',
        labelEn: 'Mushroom', labelRu: 'Гриб',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 4 c-4 0-7 3-7 6h14c0-3-3-6-7-6z"/><path d="M9 10 v6 q0 2 3 2t3-2v-6"/></svg>`,
      },
      {
        id: 'apple',
        labelEn: 'Apple', labelRu: 'Яблоко',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 6 c-3 0-6 3-6 7 0 3 2 6 4 6h4c2 0 4-3 4-6 0-4-3-7-6-7z"/><path d="M12 6 c0-2 1-3 3-3"/><path d="M15 8 c1-1 2-1 2 0"/></svg>`,
      },
      {
        id: 'bread',
        labelEn: 'Bread', labelRu: 'Хлеб',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M5 11 c0-3 3-5 7-5s7 2 7 5v6H5z"/><path d="M5 13 h14"/><path d="M9 13 v4 M15 13 v4"/></svg>`,
      },
      {
        id: 'cup',
        labelEn: 'Tea cup', labelRu: 'Чай',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M5 8 h11 l-1 8 q0 2-2 2H8q-2 0-2-2z"/><path d="M16 10 h2 q2 0 2 2t-2 2h-2"/><path d="M4 18 h12"/><path d="M8 5 q0-2 2-2 M12 5 q0-2 2-2"/></svg>`,
      },
      {
        id: 'fish',
        labelEn: 'Fish', labelRu: 'Рыба',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M4 12 c2-4 6-5 9-4l3-4v8l-3-4c-3 1-7 0-9 4z"/><circle cx="17" cy="10" r="0.5" fill="currentColor"/></svg>`,
      },
      {
        id: 'berry',
        labelEn: 'Berries', labelRu: 'Ягоды',
        svg: `<svg viewBox="0 0 24 24" ${S}><circle cx="9" cy="14" r="3"/><circle cx="15" cy="14" r="3"/><circle cx="12" cy="10" r="3"/><path d="M12 7 v-3 M9 11 l-2-2 M15 11 l2-2"/></svg>`,
      },
      {
        id: 'honey',
        labelEn: 'Honey', labelRu: 'Мёд',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M9 4 h6 l2 4-2 4H9L7 8z"/><path d="M8 12 l-1 5q0 2 5 2t5-2l-1-5"/><path d="M10 8 h4"/></svg>`,
      },
      {
        id: 'herb',
        labelEn: 'Herb', labelRu: 'Трава',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 20 v-8"/><path d="M12 14 c-2-3-5-3-6-1 1 3 4 3 6 1z"/><path d="M12 11 c2-3 5-3 6-1-1 3-4 3-6 1z"/><path d="M12 17 c-1-2-3-2-4-1 1 2 3 2 4 1z"/></svg>`,
      },
    ],
  },
  {
    id: 'seasons',
    labelEn: 'A season',
    labelRu: 'Время года',
    icons: [
      {
        id: 'snowflake',
        labelEn: 'Snowflake', labelRu: 'Снежинка',
        svg: `<svg viewBox="0 0 24 24" ${S}><line x1="12" y1="3" x2="12" y2="21"/><line x1="3" y1="12" x2="21" y2="12"/><line x1="5.6" y1="5.6" x2="18.4" y2="18.4"/><line x1="18.4" y1="5.6" x2="5.6" y2="18.4"/><path d="M9 6 l3-3 3 3 M9 18 l3 3 3-3 M6 9 l-3 3 3 3 M18 9 l3 3-3 3"/></svg>`,
      },
      {
        id: 'bare_tree',
        labelEn: 'Bare tree', labelRu: 'Голое дерево',
        svg: `<svg viewBox="0 0 24 24" ${S}><line x1="12" y1="20" x2="12" y2="8"/><path d="M12 14 l-4-4 M12 11 l4-4 M12 17 l-3-2 M12 17 l3-2"/></svg>`,
      },
      {
        id: 'sprout',
        labelEn: 'Sprout', labelRu: 'Росток',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 20 v-9"/><path d="M12 14 c0-4 4-6 7-5-1 4-4 6-7 5z"/><path d="M12 11 c0-4-4-6-7-5 1 4 4 6 7 5z"/></svg>`,
      },
      {
        id: 'rain',
        labelEn: 'Rain', labelRu: 'Дождь',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M6 12 H18 a4 4 0 0 0 0-8 5 5 0 0 0-10 1"/><line x1="8" y1="15" x2="6" y2="19"/><line x1="12" y1="15" x2="10" y2="19"/><line x1="16" y1="15" x2="14" y2="19"/></svg>`,
      },
      {
        id: 'sun',
        labelEn: 'Sun', labelRu: 'Солнце',
        svg: `<svg viewBox="0 0 24 24" ${S}><circle cx="12" cy="12" r="4"/><line x1="12" y1="3" x2="12" y2="5"/><line x1="12" y1="19" x2="12" y2="21"/><line x1="3" y1="12" x2="5" y2="12"/><line x1="19" y1="12" x2="21" y2="12"/><line x1="5.6" y1="5.6" x2="7" y2="7"/><line x1="17" y1="17" x2="18.4" y2="18.4"/><line x1="18.4" y1="5.6" x2="17" y2="7"/><line x1="7" y1="17" x2="5.6" y2="18.4"/></svg>`,
      },
      {
        id: 'wheat',
        labelEn: 'Wheat', labelRu: 'Пшеница',
        svg: `<svg viewBox="0 0 24 24" ${S}><line x1="12" y1="20" x2="12" y2="6"/><path d="M12 16 c-2-1-3-3-2-5 2 1 3 3 2 5z"/><path d="M12 16 c2-1 3-3 2-5-2 1-3 3-2 5z"/><path d="M12 12 c-2-1-3-3-2-5 2 1 3 3 2 5z"/><path d="M12 12 c2-1 3-3 2-5-2 1-3 3-2 5z"/><path d="M11 6 l1-3 1 3"/></svg>`,
      },
      {
        id: 'leaf',
        labelEn: 'Autumn leaf', labelRu: 'Лист',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M6 20 c0-6 3-10 8-12 3-1 6-1 7-3-1 5-3 9-8 12-2 1-5 2-7 3z"/><line x1="6" y1="20" x2="12" y2="12"/></svg>`,
      },
      {
        id: 'acorn',
        labelEn: 'Acorn', labelRu: 'Жёлудь',
        svg: `<svg viewBox="0 0 24 24" ${S}><ellipse cx="12" cy="14" rx="4" ry="5"/><path d="M8 12 q0-3 4-3t4 3"/><path d="M8 10 q4-1 8 0"/><line x1="12" y1="9" x2="12" y2="6"/><path d="M10 6 q2-2 4 0"/></svg>`,
      },
    ],
  },
  {
    id: 'colors',
    labelEn: 'A colour',
    labelRu: 'Цвет',
    icons: [
      {
        id: 'red',
        labelEn: 'Crimson', labelRu: 'Алый',
        svg: `<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="8" fill="#c0392b" stroke="#8b0000" stroke-width="1.5"/></svg>`,
      },
      {
        id: 'blue',
        labelEn: 'Midnight blue', labelRu: 'Синий',
        svg: `<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="8" fill="#1a3a5c" stroke="#0d2240" stroke-width="1.5"/></svg>`,
      },
      {
        id: 'green',
        labelEn: 'Forest green', labelRu: 'Зелёный',
        svg: `<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="8" fill="#2d6a3f" stroke="#1a4a28" stroke-width="1.5"/></svg>`,
      },
      {
        id: 'amber',
        labelEn: 'Amber', labelRu: 'Янтарный',
        svg: `<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="8" fill="#d4820a" stroke="#9a5a00" stroke-width="1.5"/></svg>`,
      },
      {
        id: 'violet',
        labelEn: 'Violet', labelRu: 'Фиолетовый',
        svg: `<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="8" fill="#6b3fa0" stroke="#4a2870" stroke-width="1.5"/></svg>`,
      },
      {
        id: 'copper',
        labelEn: 'Copper', labelRu: 'Медный',
        svg: `<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="8" fill="#b55a2a" stroke="#7a3810" stroke-width="1.5"/></svg>`,
      },
      {
        id: 'black',
        labelEn: 'Charcoal', labelRu: 'Тёмный',
        svg: `<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="8" fill="#1c1c1e" stroke="#000" stroke-width="1.5"/></svg>`,
      },
      {
        id: 'ivory',
        labelEn: 'Ivory', labelRu: 'Слоновая кость',
        svg: `<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="8" fill="#f8f1e7" stroke="#d8c6b1" stroke-width="1.5"/></svg>`,
      },
    ],
  },
];

export function getCategoryById(id: IconCategory): IconCategoryDef | undefined {
  return VISUAL_CATEGORIES.find(c => c.id === id);
}

export function getIconById(categoryId: IconCategory, iconId: string): VisualIcon | undefined {
  return getCategoryById(categoryId)?.icons.find(i => i.id === iconId);
}

/** Pick the right label for the current language. */
export function iconLabel(icon: VisualIcon, lang: string): string {
  return lang === 'ru' ? icon.labelRu : icon.labelEn;
}

export function categoryLabel(cat: IconCategoryDef, lang: string): string {
  return lang === 'ru' ? cat.labelRu : cat.labelEn;
}

/**
 * Prepare an SVG string for canvas rendering:
 * - replace `currentColor` with a concrete hex so it shows up without CSS
 * - add explicit width/height so the browser knows the intrinsic size
 */
export function svgForCanvas(svg: string, size: number, color = '#6f3b24'): string {
  return svg
    .replace(/currentColor/g, color)
    // xmlns is required when loading SVG as <img> via data: URL (not needed for inline SVG)
    .replace('<svg ', `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" `);
}
