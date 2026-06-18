export type IconCategory = 'animals' | 'dishes' | 'seasons' | 'symbols';

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
  icons: VisualIcon[]; // master pool — each user is shown a random subset (see POOL_PER_CATEGORY)
}

/**
 * How many icons from each category's master pool are shown to a single user.
 * Must match the server's POOL_PER_CATEGORY (services/mod.rs). The personal
 * subset is generated at registration, persisted, and replayed at login.
 */
export const POOL_PER_CATEGORY = 8;

// All SVGs: 24×24 viewBox, stroke="currentColor" fill="none" strokeWidth="1.5"
const S = 'stroke="currentColor" fill="none" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"';

export const VISUAL_CATEGORIES: IconCategoryDef[] = [
  {
    id: 'animals',
    labelEn: 'A creature',
    labelRu: 'Существо',
    icons: [
      { id: 'wolf', labelEn: 'Wolf', labelRu: 'Волк',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M3 6 l3-3 l2 2 l3-1 l1 3 c2-1 4 0 5 2 l2-1 l1 3-2 1c0 3-2 5-5 6l-2 1-2-1c-3-1-5-3-5-6L2 11l1-3 2 1z"/><circle cx="9" cy="11" r="1" fill="currentColor"/><circle cx="14" cy="11" r="1" fill="currentColor"/></svg>` },
      { id: 'raven', labelEn: 'Raven', labelRu: 'Ворон',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 4c-2 0-4 1-5 3l-2 1 1 2 2-1c0 2 1 4 3 5v3l-2 2h8l-2-2v-3c2-1 3-3 3-5l2 1 1-2-2-1c-1-2-3-3-5-3z"/><path d="M10 9 l1 1 M14 9 l-1 1"/></svg>` },
      { id: 'fox', labelEn: 'Fox', labelRu: 'Лиса',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M3 4 l4 4 c-1 1-2 3-2 5 0 3 3 6 7 6s7-3 7-6c0-2-1-4-2-5l4-4-5 2c-1-1-2-2-4-2s-3 1-4 2z"/><circle cx="10" cy="12" r="1" fill="currentColor"/><circle cx="14" cy="12" r="1" fill="currentColor"/><path d="M11 15 q1 1 2 0"/></svg>` },
      { id: 'owl', labelEn: 'Owl', labelRu: 'Сова',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M8 4 l-3 2 2 1c-1 1-2 3-2 5 0 4 3 7 7 7s7-3 7-7c0-2-1-4-2-5l2-1-3-2-2 2c-1-1-2-1-2 0z"/><circle cx="10" cy="11" r="2"/><circle cx="14" cy="11" r="2"/><circle cx="10" cy="11" r="1" fill="currentColor"/><circle cx="14" cy="11" r="1" fill="currentColor"/><path d="M11 15 q1 1 2 0"/></svg>` },
      { id: 'snake', labelEn: 'Snake', labelRu: 'Змея',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M5 18 c0-3 4-3 4-6s-4-3-4-6c0-2 2-3 4-3 3 0 5 2 5 4s-2 3-2 5 2 3 5 3"/><path d="M18 15 l2-1-1-2"/></svg>` },
      { id: 'deer', labelEn: 'Deer', labelRu: 'Олень',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 9 v7 l-2 4 M12 16 l2 4"/><path d="M12 9 c-1-2-1-4 0-5 M12 9 c1-2 1-4 0-5"/><path d="M7 5 l-2-1 M7 6 l-2 1 M17 5 l2-1 M17 6 l2 1"/><ellipse cx="12" cy="12" rx="3" ry="2"/></svg>` },
      { id: 'bat', labelEn: 'Bat', labelRu: 'Летучая мышь',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 10 c-1-1-3-2-5-2-1 0-3 1-4 2l1 2c1-1 2-1 3-1-1 1-1 2-1 3h12c0-1 0-2-1-3 1 0 2 0 3 1l1-2c-1-1-3-2-4-2-2 0-4 1-5 2z"/><path d="M10 14 q2 3 4 0"/><circle cx="10" cy="11" r="0.5" fill="currentColor"/><circle cx="14" cy="11" r="0.5" fill="currentColor"/></svg>` },
      { id: 'cat', labelEn: 'Cat', labelRu: 'Кот',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M7 4 l-2 4 1 1c-1 1-1 2-1 3 0 3 3 6 7 6s7-3 7-6c0-1 0-2-1-3l1-1-2-4-2 3c-1-1-3-1-4 0z"/><circle cx="10" cy="12" r="1" fill="currentColor"/><circle cx="14" cy="12" r="1" fill="currentColor"/><path d="M10 15 q2 2 4 0"/><path d="M9 11 l-2-1 M15 11 l2-1 M12 11 v-1"/></svg>` },
      { id: 'bear', labelEn: 'Bear', labelRu: 'Медведь',
        svg: `<svg viewBox="0 0 24 24" ${S}><circle cx="6" cy="6" r="2"/><circle cx="18" cy="6" r="2"/><path d="M6 9 c0-2 3-4 6-4s6 2 6 4c1 1 2 3 2 5 0 4-4 7-8 7s-8-3-8-7c0-2 1-4 2-5z"/><circle cx="9" cy="12" r="1" fill="currentColor"/><circle cx="15" cy="12" r="1" fill="currentColor"/><ellipse cx="12" cy="16" rx="2.5" ry="2"/><circle cx="12" cy="15" r="0.8" fill="currentColor"/></svg>` },
      { id: 'hare', labelEn: 'Hare', labelRu: 'Заяц',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M9 9 c0-4-1-7 0-7s2 3 2 6 M15 9 c0-4 1-7 0-7s-2 3-2 6"/><circle cx="12" cy="14" r="5"/><circle cx="10" cy="13" r="1" fill="currentColor"/><circle cx="14" cy="13" r="1" fill="currentColor"/><path d="M11 16 q1 1 2 0"/></svg>` },
      { id: 'boar', labelEn: 'Boar', labelRu: 'Кабан',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M4 10 c0-3 3-5 8-5s8 2 8 6c0 4-4 6-8 6-3 0-6-1-7-3"/><ellipse cx="6" cy="13" rx="2.5" ry="2"/><circle cx="5.5" cy="13" r="0.6" fill="currentColor"/><circle cx="6.5" cy="13" r="0.6" fill="currentColor"/><circle cx="13" cy="10" r="1" fill="currentColor"/><path d="M8 15 l-1 2 M9 15 l1 2"/><path d="M16 5 l2-1 M19 7 l2-1"/></svg>` },
      { id: 'lynx', labelEn: 'Lynx', labelRu: 'Рысь',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M7 3 l-1 4 M17 3 l1 4"/><path d="M6 7 l-1 3 1 1c-1 1-1 2-1 3 0 3 3 6 7 6s7-3 7-6c0-1 0-2-1-3l1-1-1-3-2 2c-1-1-3-1-4 0z"/><circle cx="10" cy="12" r="1" fill="currentColor"/><circle cx="14" cy="12" r="1" fill="currentColor"/><path d="M10 15 q2 2 4 0"/></svg>` },
      { id: 'crow', labelEn: 'Crow', labelRu: 'Грач',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M3 8 l5 2 c1-3 3-4 6-4 4 0 6 3 6 6 0 4-3 6-6 6-2 0-4-1-5-3l-4 1 2-3-4-1z"/><path d="M14 9 l3-1"/><circle cx="13" cy="10" r="0.6" fill="currentColor"/></svg>` },
      { id: 'moth', labelEn: 'Moth', labelRu: 'Мотылёк',
        svg: `<svg viewBox="0 0 24 24" ${S}><line x1="12" y1="8" x2="12" y2="18"/><path d="M12 9 q-7-5-8 1 t8 5 M12 9 q7-5 8 1 t-8 5"/><path d="M12 8 l-2-4 M12 8 l2-4"/></svg>` },
      { id: 'spider', labelEn: 'Spider', labelRu: 'Паук',
        svg: `<svg viewBox="0 0 24 24" ${S}><circle cx="12" cy="13" r="3"/><circle cx="12" cy="8" r="1.5"/><path d="M9 12 l-4-2 M9 13 l-5 1 M9 15 l-4 3 M15 12 l4-2 M15 13 l5 1 M15 15 l4 3"/></svg>` },
      { id: 'frog', labelEn: 'Frog', labelRu: 'Лягушка',
        svg: `<svg viewBox="0 0 24 24" ${S}><circle cx="8" cy="7" r="2"/><circle cx="16" cy="7" r="2"/><circle cx="8" cy="7" r="0.6" fill="currentColor"/><circle cx="16" cy="7" r="0.6" fill="currentColor"/><path d="M6 9 c-1 2-2 4-2 6 0 3 4 5 8 5s8-2 8-5c0-2-1-4-2-6"/><path d="M9 16 q3 2 6 0"/><path d="M5 18 l-2 2 M19 18 l2 2"/></svg>` },
      { id: 'hound', labelEn: 'Hound', labelRu: 'Пёс',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M5 7 c-1 0-2 2-2 4l1 1c0 4 3 7 8 7 5 0 8-3 8-8 0-3-2-5-5-5-2 0-3 1-4 2-1-1-2-2-3-2z"/><path d="M5 7 c-1 1-1 3 0 4"/><circle cx="15" cy="11" r="1" fill="currentColor"/><path d="M17 14 l2 1 M17 15 l2-1"/></svg>` },
      { id: 'horse', labelEn: 'Horse', labelRu: 'Конь',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M6 20 c-1-5 0-9 3-12 1-1 2-2 2-4 1 1 2 2 4 2 3 0 5 2 5 5 0 2-1 3-3 4-1 3-2 5-2 5"/><path d="M11 4 c-2 1-4 3-5 5 M9 6 l-3-1"/><circle cx="16" cy="8" r="0.6" fill="currentColor"/></svg>` },
      { id: 'goat', labelEn: 'Goat', labelRu: 'Коза',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M8 6 c-1-2-3-3-5-2 1 2 3 3 5 3 M16 6 c1-2 3-3 5-2-1 2-3 3-5 3"/><path d="M8 7 c0-1 2-2 4-2s4 1 4 2c1 1 1 3 1 5 0 3-2 5-5 5s-5-2-5-5c0-2 0-4 1-5z"/><circle cx="10" cy="11" r="0.8" fill="currentColor"/><circle cx="14" cy="11" r="0.8" fill="currentColor"/><path d="M11 16 l1 3 1-3"/></svg>` },
      { id: 'ram', labelEn: 'Ram', labelRu: 'Баран',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M8 8 c0-2 2-3 4-3s4 1 4 3c1 1 1 3 1 5 0 3-2 5-5 5s-5-2-5-5c0-2 0-4 1-5z"/><path d="M8 8 c-3 0-5-2-5-4 0-1 1-2 2-1 1 1 0 3 1 4 M16 8 c3 0 5-2 5-4 0-1-1-2-2-1-1 1 0 3-1 4"/><circle cx="10" cy="12" r="0.8" fill="currentColor"/><circle cx="14" cy="12" r="0.8" fill="currentColor"/></svg>` },
      { id: 'hawk', labelEn: 'Hawk', labelRu: 'Ястреб',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 5 c1 0 2 1 2 3 l4-2-1 3 3 0-3 3 c0 4-3 7-5 8-2-1-5-4-5-8l-3-3 3 0-1-3 4 2c0-2 1-3 2-3z"/><path d="M12 14 l-1 2 M12 14 l1 2"/></svg>` },
      { id: 'mouse', labelEn: 'Mouse', labelRu: 'Мышь',
        svg: `<svg viewBox="0 0 24 24" ${S}><circle cx="7" cy="8" r="3"/><circle cx="15" cy="8" r="3"/><path d="M10 9 c1 1 3 1 5 1 4 0 6 3 6 6 M9 10 c-2 1-3 4-3 6 M5 16 h13"/><circle cx="11" cy="13" r="0.6" fill="currentColor"/><path d="M18 21 c2-1 3-3 3-5"/></svg>` },
      { id: 'beetle', labelEn: 'Beetle', labelRu: 'Жук',
        svg: `<svg viewBox="0 0 24 24" ${S}><ellipse cx="12" cy="13" rx="4" ry="6"/><line x1="12" y1="7" x2="12" y2="19"/><circle cx="12" cy="6" r="2"/><path d="M11 4 l-2-2 M13 4 l2-2"/><path d="M8 10 l-4-2 M8 13 l-4 0 M8 16 l-4 2 M16 10 l4-2 M16 13 l4 0 M16 16 l4 2"/></svg>` },
      { id: 'stag_beetle', labelEn: 'Stag beetle', labelRu: 'Жук-олень',
        svg: `<svg viewBox="0 0 24 24" ${S}><ellipse cx="12" cy="14" rx="4" ry="5"/><circle cx="12" cy="8" r="2"/><path d="M10 7 c-2-1-4-3-4-5 2 1 3 2 3 4 M14 7 c2-1 4-3 4-5-2 1-3 2-3 4"/><path d="M8 12 l-4-1 M8 15 l-4 1 M16 12 l4-1 M16 15 l4 1"/></svg>` },
    ],
  },
  {
    id: 'dishes',
    labelEn: 'A dish',
    labelRu: 'Блюдо',
    icons: [
      { id: 'mushroom', labelEn: 'Mushroom', labelRu: 'Гриб',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 4 c-4 0-7 3-7 6h14c0-3-3-6-7-6z"/><path d="M9 10 v6 q0 2 3 2t3-2v-6"/></svg>` },
      { id: 'apple', labelEn: 'Apple', labelRu: 'Яблоко',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 6 c-3 0-6 3-6 7 0 3 2 6 4 6h4c2 0 4-3 4-6 0-4-3-7-6-7z"/><path d="M12 6 c0-2 1-3 3-3"/><path d="M15 8 c1-1 2-1 2 0"/></svg>` },
      { id: 'bread', labelEn: 'Bread', labelRu: 'Хлеб',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M5 11 c0-3 3-5 7-5s7 2 7 5v6H5z"/><path d="M5 13 h14"/><path d="M9 13 v4 M15 13 v4"/></svg>` },
      { id: 'cup', labelEn: 'Tea cup', labelRu: 'Чай',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M5 8 h11 l-1 8 q0 2-2 2H8q-2 0-2-2z"/><path d="M16 10 h2 q2 0 2 2t-2 2h-2"/><path d="M4 18 h12"/><path d="M8 5 q0-2 2-2 M12 5 q0-2 2-2"/></svg>` },
      { id: 'fish', labelEn: 'Fish', labelRu: 'Рыба',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M4 12 c2-4 6-5 9-4l3-4v8l-3-4c-3 1-7 0-9 4z"/><circle cx="17" cy="10" r="0.5" fill="currentColor"/></svg>` },
      { id: 'berry', labelEn: 'Berries', labelRu: 'Ягоды',
        svg: `<svg viewBox="0 0 24 24" ${S}><circle cx="9" cy="14" r="3"/><circle cx="15" cy="14" r="3"/><circle cx="12" cy="10" r="3"/><path d="M12 7 v-3 M9 11 l-2-2 M15 11 l2-2"/></svg>` },
      { id: 'honey', labelEn: 'Honey', labelRu: 'Мёд',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M9 4 h6 l2 4-2 4H9L7 8z"/><path d="M8 12 l-1 5q0 2 5 2t5-2l-1-5"/><path d="M10 8 h4"/></svg>` },
      { id: 'herb', labelEn: 'Herb', labelRu: 'Трава',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 20 v-8"/><path d="M12 14 c-2-3-5-3-6-1 1 3 4 3 6 1z"/><path d="M12 11 c2-3 5-3 6-1-1 3-4 3-6 1z"/><path d="M12 17 c-1-2-3-2-4-1 1 2 3 2 4 1z"/></svg>` },
      { id: 'pear', labelEn: 'Pear', labelRu: 'Груша',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 7 c-1 0-2 1-2 3 -1 1-3 3-3 6 0 3 2 5 5 5s5-2 5-5c0-3-2-5-3-6 0-2-1-3-2-3z"/><path d="M12 7 c0-2 1-3 2-4"/><path d="M14 4 c1-1 2 0 1 1"/></svg>` },
      { id: 'plum', labelEn: 'Plum', labelRu: 'Слива',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 6 c-3 0-5 3-5 7s2 6 5 6 5-2 5-6-2-7-5-7z"/><path d="M12 6 v13"/><path d="M12 6 c1-2 2-3 4-3"/></svg>` },
      { id: 'egg', labelEn: 'Egg', labelRu: 'Яйцо',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 4 c-3 0-6 5-6 10 0 4 3 6 6 6s6-2 6-6c0-5-3-10-6-10z"/></svg>` },
      { id: 'cheese', labelEn: 'Cheese', labelRu: 'Сыр',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M4 16 l14-7 c2 1 2 5 0 7l-14 0z"/><path d="M4 16 v-2 l14-7"/><circle cx="9" cy="14" r="1"/><circle cx="13" cy="13" r="1"/></svg>` },
      { id: 'grapes', labelEn: 'Grapes', labelRu: 'Виноград',
        svg: `<svg viewBox="0 0 24 24" ${S}><circle cx="9" cy="11" r="2"/><circle cx="13" cy="11" r="2"/><circle cx="11" cy="14" r="2"/><circle cx="15" cy="14" r="2"/><circle cx="13" cy="17" r="2"/><path d="M13 9 c0-3 1-5 4-5"/></svg>` },
      { id: 'carrot', labelEn: 'Carrot', labelRu: 'Морковь',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M7 12 l9 9 c2-2 3-6 1-9s-7-3-10-1z"/><path d="M7 12 v-5 M7 9 l-3-2 M7 9 l3-3"/><path d="M10 14 l1 1 M13 16 l1 1"/></svg>` },
      { id: 'onion', labelEn: 'Onion', labelRu: 'Лук',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 8 c-3 0-5 3-5 6 0 3 2 6 5 6s5-3 5-6c0-3-2-6-5-6z"/><path d="M12 8 c-1 0-2 4-2 6s1 6 2 6 2-4 2-6-1-6-2-6z"/><path d="M10 7 l2-3 2 3"/></svg>` },
      { id: 'pumpkin', labelEn: 'Pumpkin', labelRu: 'Тыква',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 8 c-4 0-6 3-6 6s2 6 6 6 6-3 6-6-2-6-6-6z"/><path d="M9 8 c-1 1-1 11 0 12 M15 8 c1 1 1 11 0 12"/><path d="M12 8 v-3 c0-1 1-2 3-2"/></svg>` },
      { id: 'walnut', labelEn: 'Walnut', labelRu: 'Орех',
        svg: `<svg viewBox="0 0 24 24" ${S}><circle cx="12" cy="13" r="7"/><path d="M12 6 v14 M8 8 c2 2 2 8 0 10 M16 8 c-2 2-2 8 0 10"/></svg>` },
      { id: 'pie', labelEn: 'Pie slice', labelRu: 'Пирог',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M4 18 l8-12 8 12z"/><path d="M4 18 h16"/><circle cx="10" cy="15" r="0.6" fill="currentColor"/><circle cx="13" cy="13" r="0.6" fill="currentColor"/><circle cx="12" cy="16" r="0.6" fill="currentColor"/></svg>` },
      { id: 'soup', labelEn: 'Soup', labelRu: 'Похлёбка',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M4 12 h16 c0 4-3 7-8 7s-8-3-8-7z"/><path d="M2 12 h20"/><path d="M9 8 c-1-1 1-2 0-3 M13 8 c-1-1 1-2 0-3 M17 8 c-1-1 1-2 0-3"/></svg>` },
      { id: 'wine', labelEn: 'Wine', labelRu: 'Вино',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M8 4 h8 c0 4-2 7-4 7s-4-3-4-7z"/><path d="M8 6 h8"/><path d="M12 11 v7 M9 18 h6"/></svg>` },
      { id: 'milk', labelEn: 'Milk', labelRu: 'Молоко',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M9 4 h6 v3 l2 4 v8 q0 1-1 1H8q-1 0-1-1v-8 l2-4z"/><path d="M7 13 h10"/></svg>` },
      { id: 'salt', labelEn: 'Salt', labelRu: 'Соль',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M8 9 c0-3 1-5 4-5s4 2 4 5 v9q0 1-1 1H9q-1 0-1-1z"/><path d="M8 12 h8"/><circle cx="11" cy="7" r="0.4" fill="currentColor"/><circle cx="13" cy="7" r="0.4" fill="currentColor"/><circle cx="12" cy="6" r="0.4" fill="currentColor"/></svg>` },
      { id: 'pepper', labelEn: 'Pepper', labelRu: 'Перец',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M16 6 c-1 0-2 1-3 2 -4-1-8 2-8 7 0 3 2 4 4 4 5 0 9-5 9-10"/><path d="M16 6 c0-2 1-3 3-3 M13 8 c1-2 3-2 4-2"/></svg>` },
      { id: 'garlic', labelEn: 'Garlic', labelRu: 'Чеснок',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 7 c-3 1-5 5-5 8 0 3 2 4 5 4s5-1 5-4c0-3-2-7-5-8z"/><path d="M12 7 c0 4 0 9 0 12 M9 9 c-1 3-1 7 0 10 M15 9 c1 3 1 7 0 10"/><path d="M12 7 c0-2 0-3 1-4 M12 7 c0-2 0-3-1-4"/></svg>` },
    ],
  },
  {
    id: 'seasons',
    labelEn: 'A season',
    labelRu: 'Время года',
    icons: [
      { id: 'snowflake', labelEn: 'Snowflake', labelRu: 'Снежинка',
        svg: `<svg viewBox="0 0 24 24" ${S}><line x1="12" y1="3" x2="12" y2="21"/><line x1="3" y1="12" x2="21" y2="12"/><line x1="5.6" y1="5.6" x2="18.4" y2="18.4"/><line x1="18.4" y1="5.6" x2="5.6" y2="18.4"/><path d="M9 6 l3-3 3 3 M9 18 l3 3 3-3 M6 9 l-3 3 3 3 M18 9 l3 3-3 3"/></svg>` },
      { id: 'bare_tree', labelEn: 'Bare tree', labelRu: 'Голое дерево',
        svg: `<svg viewBox="0 0 24 24" ${S}><line x1="12" y1="20" x2="12" y2="8"/><path d="M12 14 l-4-4 M12 11 l4-4 M12 17 l-3-2 M12 17 l3-2"/></svg>` },
      { id: 'sprout', labelEn: 'Sprout', labelRu: 'Росток',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 20 v-9"/><path d="M12 14 c0-4 4-6 7-5-1 4-4 6-7 5z"/><path d="M12 11 c0-4-4-6-7-5 1 4 4 6 7 5z"/></svg>` },
      { id: 'rain', labelEn: 'Rain', labelRu: 'Дождь',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M6 12 H18 a4 4 0 0 0 0-8 5 5 0 0 0-10 1"/><line x1="8" y1="15" x2="6" y2="19"/><line x1="12" y1="15" x2="10" y2="19"/><line x1="16" y1="15" x2="14" y2="19"/></svg>` },
      { id: 'sun', labelEn: 'Sun', labelRu: 'Солнце',
        svg: `<svg viewBox="0 0 24 24" ${S}><circle cx="12" cy="12" r="4"/><line x1="12" y1="3" x2="12" y2="5"/><line x1="12" y1="19" x2="12" y2="21"/><line x1="3" y1="12" x2="5" y2="12"/><line x1="19" y1="12" x2="21" y2="12"/><line x1="5.6" y1="5.6" x2="7" y2="7"/><line x1="17" y1="17" x2="18.4" y2="18.4"/><line x1="18.4" y1="5.6" x2="17" y2="7"/><line x1="7" y1="17" x2="5.6" y2="18.4"/></svg>` },
      { id: 'wheat', labelEn: 'Wheat', labelRu: 'Пшеница',
        svg: `<svg viewBox="0 0 24 24" ${S}><line x1="12" y1="20" x2="12" y2="6"/><path d="M12 16 c-2-1-3-3-2-5 2 1 3 3 2 5z"/><path d="M12 16 c2-1 3-3 2-5-2 1-3 3-2 5z"/><path d="M12 12 c-2-1-3-3-2-5 2 1 3 3 2 5z"/><path d="M12 12 c2-1 3-3 2-5-2 1-3 3-2 5z"/><path d="M11 6 l1-3 1 3"/></svg>` },
      { id: 'leaf', labelEn: 'Autumn leaf', labelRu: 'Лист',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M6 20 c0-6 3-10 8-12 3-1 6-1 7-3-1 5-3 9-8 12-2 1-5 2-7 3z"/><line x1="6" y1="20" x2="12" y2="12"/></svg>` },
      { id: 'acorn', labelEn: 'Acorn', labelRu: 'Жёлудь',
        svg: `<svg viewBox="0 0 24 24" ${S}><ellipse cx="12" cy="14" rx="4" ry="5"/><path d="M8 12 q0-3 4-3t4 3"/><path d="M8 10 q4-1 8 0"/><line x1="12" y1="9" x2="12" y2="6"/><path d="M10 6 q2-2 4 0"/></svg>` },
      { id: 'icicle', labelEn: 'Icicle', labelRu: 'Сосулька',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M4 5 h16"/><path d="M8 5 l1 9-1 3-1-3z M13 5 l1 13-1 2-1-2z M18 5 l1 6-1 2-1-2z"/></svg>` },
      { id: 'frost_pane', labelEn: 'Frost pane', labelRu: 'Морозный узор',
        svg: `<svg viewBox="0 0 24 24" ${S}><rect x="4" y="4" width="16" height="16" rx="1"/><path d="M12 8 v8 M8 12 h8 M9 9 l6 6 M15 9 l-6 6"/></svg>` },
      { id: 'bud', labelEn: 'Bud', labelRu: 'Бутон',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 20 v-8"/><path d="M12 12 c-3 0-5-2-5-5 0-2 2-3 3-2 1-2 3-2 4 0 1-1 3 0 3 2 0 3-2 5-5 5z"/><path d="M9 16 c-2 0-3-1-3-3 2 0 3 1 3 3z"/></svg>` },
      { id: 'blossom', labelEn: 'Blossom', labelRu: 'Цветок',
        svg: `<svg viewBox="0 0 24 24" ${S}><circle cx="12" cy="12" r="2.5"/><path d="M12 9.5 c0-3 0-5 0-5 M12 14.5 c0 3 0 5 0 5 M9.5 12 c-3 0-5 0-5 0 M14.5 12 c3 0 5 0 5 0 M10 10 l-3-3 M14 10 l3-3 M10 14 l-3 3 M14 14 l3 3"/></svg>` },
      { id: 'cloud', labelEn: 'Cloud', labelRu: 'Облако',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M7 17 H17 a4 4 0 0 0 0-8 5 5 0 0 0-9.5 1.5 A3.5 3.5 0 0 0 7 17z"/></svg>` },
      { id: 'lightning', labelEn: 'Lightning', labelRu: 'Молния',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M13 3 L6 13 h5 l-2 8 8-12 h-5 z"/></svg>` },
      { id: 'mist', labelEn: 'Mist', labelRu: 'Туман',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M4 8 h12 M8 12 h12 M4 16 h10 M6 20 h12"/></svg>` },
      { id: 'pinecone', labelEn: 'Pine cone', labelRu: 'Шишка',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 3 c-3 2-5 5-5 9 0 4 2 8 5 8s5-4 5-8c0-4-2-7-5-9z"/><path d="M9 9 q3 2 6 0 M8 13 q4 2 8 0 M9 17 q3 1 6 0"/></svg>` },
      { id: 'fern', labelEn: 'Fern', labelRu: 'Папоротник',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M5 20 c2-8 7-13 14-16"/><path d="M5 20 l2-3 M7 16 l3-2 M9 12 l3-2 M11 9 l3-2 M14 6 l2-1"/><path d="M5 20 l-1-3 M7 16 l-2-2 M9 12 l-2-2 M11 9 l-2-2"/></svg>` },
      { id: 'sheaf', labelEn: 'Sheaf', labelRu: 'Сноп',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 20 v-12 M9 20 c0-5-1-9-2-12 M15 20 c0-5 1-9 2-12"/><path d="M7 8 l-2-3 2 1 M12 8 l0-4 M17 8 l2-3-2 1"/><path d="M7 16 h10"/></svg>` },
      { id: 'crescent', labelEn: 'Crescent', labelRu: 'Месяц',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M16 4 a9 9 0 1 0 4 13 7 7 0 0 1-4-13z"/></svg>` },
      { id: 'dewdrop', labelEn: 'Dewdrop', labelRu: 'Капля росы',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 3 c4 5 6 8 6 11 a6 6 0 0 1-12 0 c0-3 2-6 6-11z"/><path d="M9 15 a3 3 0 0 0 2 3"/></svg>` },
      { id: 'hail', labelEn: 'Hail', labelRu: 'Град',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M6 11 H18 a4 4 0 0 0 0-8 5 5 0 0 0-10 1"/><circle cx="8" cy="17" r="1.2"/><circle cx="12" cy="19" r="1.2"/><circle cx="16" cy="17" r="1.2"/></svg>` },
      { id: 'gust', labelEn: 'Gust', labelRu: 'Ветер',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M3 9 h11 a2 2 0 1 0-2-2 M3 13 h14 a2 2 0 1 1-2 2 M3 17 h8 a2 2 0 1 1-2 2"/></svg>` },
      { id: 'ember', labelEn: 'Ember', labelRu: 'Уголёк',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 3 c1 3-2 4-2 7 0 1 1 2 2 2 0-2 2-2 2-4 2 2 3 4 3 6 0 4-3 6-6 6s-6-2-6-6c0-4 3-6 5-11z"/></svg>` },
      { id: 'catkin', labelEn: 'Catkin', labelRu: 'Серёжка',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M10 21 c-2-4-3-8-3-12 M10 9 c0-3 2-5 5-6"/><circle cx="8" cy="6" r="2"/><circle cx="12" cy="9" r="2"/><circle cx="9" cy="13" r="2"/><circle cx="13" cy="15" r="2"/></svg>` },
    ],
  },
  {
    id: 'symbols',
    labelEn: 'A mark',
    labelRu: 'Знак',
    icons: [
      { id: 'key', labelEn: 'Key', labelRu: 'Ключ',
        svg: `<svg viewBox="0 0 24 24" ${S}><circle cx="7" cy="9" r="4"/><path d="M10 12 l9 9 M16 18 l2-2 M19 21 l2-2"/></svg>` },
      { id: 'candle', labelEn: 'Candle', labelRu: 'Свеча',
        svg: `<svg viewBox="0 0 24 24" ${S}><rect x="9" y="8" width="6" height="12" rx="1"/><line x1="12" y1="8" x2="12" y2="5"/><path d="M12 5 c1-1 1-2 0-3-1 1-1 2 0 3z"/></svg>` },
      { id: 'hourglass', labelEn: 'Hourglass', labelRu: 'Часы',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M6 4 h12 M6 20 h12"/><path d="M7 4 c0 4 5 6 5 8 M17 4 c0 4-5 6-5 8 M7 20 c0-4 5-6 5-8 M17 20 c0-4-5-6-5-8"/></svg>` },
      { id: 'skull', labelEn: 'Skull', labelRu: 'Череп',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M5 11 c0-4 3-7 7-7s7 3 7 7c0 3-2 5-3 6 v2q0 1-1 1h-6q-1 0-1-1v-2 c-1-1-3-3-3-6z"/><circle cx="9" cy="11" r="1.5"/><circle cx="15" cy="11" r="1.5"/><path d="M12 13 l-1 2 h2z M10 18 v2 M14 18 v2"/></svg>` },
      { id: 'moon', labelEn: 'Moon', labelRu: 'Луна',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M16 4 a9 9 0 1 0 4 13 7 7 0 0 1-4-13z"/></svg>` },
      { id: 'star', labelEn: 'Star', labelRu: 'Звезда',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 3 l2.5 6 6.5 .5-5 4.5 1.5 6.5-5.5-3.5-5.5 3.5 1.5-6.5-5-4.5 6.5-.5z"/></svg>` },
      { id: 'cross', labelEn: 'Cross', labelRu: 'Крест',
        svg: `<svg viewBox="0 0 24 24" ${S}><line x1="12" y1="3" x2="12" y2="21"/><line x1="6" y1="9" x2="18" y2="9"/></svg>` },
      { id: 'anchor', labelEn: 'Anchor', labelRu: 'Якорь',
        svg: `<svg viewBox="0 0 24 24" ${S}><circle cx="12" cy="5" r="2"/><line x1="12" y1="7" x2="12" y2="20"/><path d="M5 13 c0 4 3 7 7 7s7-3 7-7"/><line x1="8" y1="10" x2="16" y2="10"/></svg>` },
      { id: 'bell', labelEn: 'Bell', labelRu: 'Колокол',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 4 c-3 0-5 2-5 5 0 4-1 6-2 8 h14 c-1-2-2-4-2-8 0-3-2-5-5-5z"/><path d="M10 20 a2 2 0 0 0 4 0"/><line x1="12" y1="3" x2="12" y2="4"/></svg>` },
      { id: 'clock', labelEn: 'Clock', labelRu: 'Часы',
        svg: `<svg viewBox="0 0 24 24" ${S}><circle cx="12" cy="12" r="8"/><path d="M12 8 v4 l3 2"/></svg>` },
      { id: 'feather', labelEn: 'Feather', labelRu: 'Перо',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M19 5 c-7 0-12 5-12 11 l-3 3 M19 5 c0 7-5 11-11 11"/><path d="M16 8 l-7 7 M12 8 l-3 3"/></svg>` },
      { id: 'inkpot', labelEn: 'Inkpot', labelRu: 'Чернильница',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M7 11 h10 v5 a4 4 0 0 1-4 4h-2 a4 4 0 0 1-4-4z"/><path d="M9 11 v-2 h6 v2"/><path d="M15 9 l4-5"/></svg>` },
      { id: 'scroll', labelEn: 'Scroll', labelRu: 'Свиток',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M6 5 h10 a2 2 0 0 1 2 2 v10 a2 2 0 0 0 2 2 H8 a2 2 0 0 1-2-2z"/><path d="M6 5 a2 2 0 0 0-2 2 v2 h4 M9 9 h6 M9 13 h6"/></svg>` },
      { id: 'dagger', labelEn: 'Dagger', labelRu: 'Кинжал',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M12 3 l2 11 -2 3 -2-3z"/><line x1="7" y1="16" x2="17" y2="16"/><line x1="12" y1="17" x2="12" y2="21"/></svg>` },
      { id: 'crown', labelEn: 'Crown', labelRu: 'Корона',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M4 8 l3 8 h10 l3-8 -5 4 -3-7 -3 7z"/><line x1="7" y1="19" x2="17" y2="19"/></svg>` },
      { id: 'eye', labelEn: 'Eye', labelRu: 'Око',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M2 12 c3-5 7-7 10-7s7 2 10 7c-3 5-7 7-10 7s-7-2-10-7z"/><circle cx="12" cy="12" r="3"/></svg>` },
      { id: 'lantern', labelEn: 'Lantern', labelRu: 'Фонарь',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M9 3 h6 M12 3 v2"/><rect x="7" y="6" width="10" height="11" rx="1"/><path d="M7 9 h10 M7 14 h10"/><path d="M10 17 v2 a2 2 0 0 0 4 0 v-2"/></svg>` },
      { id: 'mask', labelEn: 'Mask', labelRu: 'Маска',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M5 6 h14 c0 6-2 14-7 14s-7-8-7-14z"/><path d="M8 10 q2-2 4 0 M12 10 q2-2 4 0"/><path d="M10 16 q2 2 4 0"/></svg>` },
      { id: 'book', labelEn: 'Book', labelRu: 'Книга',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M5 5 c2-1 5-1 7 0 v13 c-2-1-5-1-7 0z"/><path d="M19 5 c-2-1-5-1-7 0 v13 c2-1 5-1 7 0z"/></svg>` },
      { id: 'chalice', labelEn: 'Chalice', labelRu: 'Чаша',
        svg: `<svg viewBox="0 0 24 24" ${S}><path d="M7 4 h10 c0 5-2 8-5 8s-5-3-5-8z"/><path d="M12 12 v5 M8 20 h8 M10 17 h4"/></svg>` },
      { id: 'compass', labelEn: 'Compass', labelRu: 'Компас',
        svg: `<svg viewBox="0 0 24 24" ${S}><circle cx="12" cy="12" r="9"/><path d="M15 9 l-2 5-4 2 2-5z"/><circle cx="12" cy="12" r="0.8" fill="currentColor"/></svg>` },
      { id: 'keyhole', labelEn: 'Keyhole', labelRu: 'Замочная скважина',
        svg: `<svg viewBox="0 0 24 24" ${S}><rect x="4" y="3" width="16" height="18" rx="2"/><circle cx="12" cy="10" r="2.5"/><path d="M11 12 l-1 5 h4 l-1-5"/></svg>` },
      { id: 'ring', labelEn: 'Ring', labelRu: 'Кольцо',
        svg: `<svg viewBox="0 0 24 24" ${S}><circle cx="12" cy="14" r="6"/><path d="M9 8 l3-4 3 4z"/></svg>` },
      { id: 'coin', labelEn: 'Coin', labelRu: 'Монета',
        svg: `<svg viewBox="0 0 24 24" ${S}><circle cx="12" cy="12" r="8"/><circle cx="12" cy="12" r="5"/><path d="M12 9 v6 M10 11 h4"/></svg>` },
    ],
  },
];

export function getCategoryById(id: IconCategory): IconCategoryDef | undefined {
  return VISUAL_CATEGORIES.find(c => c.id === id);
}

export function getIconById(categoryId: IconCategory, iconId: string): VisualIcon | undefined {
  return getCategoryById(categoryId)?.icons.find(i => i.id === iconId);
}

/**
 * Generate a personal subset for registration / password reset: a shuffled
 * selection of POOL_PER_CATEGORY icon_ids per category, in the fixed category
 * order (animals, dishes, seasons, symbols). The full pool is the alphabet the
 * server replays at login, so it must be sent alongside the chosen selections.
 */
export function generatePersonalPool(): string[][] {
  return VISUAL_CATEGORIES.map(cat => {
    const ids = cat.icons.map(i => i.id);
    for (let i = ids.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [ids[i], ids[j]] = [ids[j], ids[i]];
    }
    return ids.slice(0, POOL_PER_CATEGORY);
  });
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
