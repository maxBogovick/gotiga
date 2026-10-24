/**
 * Надписи на загруженной обложке книги: название и имя автора.
 *
 * Все места и величины — доли обложки (`%` ширины и высоты, кегль в `cqi`),
 * поэтому надпись одинакова и в 64 px на печати меню, и в 460 px на развороте.
 * Стиль считает одно место: полка, разворот и стол правят и рисуют одним и тем
 * же кодом, иначе предпросмотр однажды разошёлся бы с обложкой.
 */
import { fontStack } from '$lib/fonts';
import type { BookCoverLabel, BookCoverText } from '$lib/types/api';

export type BookCoverKey = 'title' | 'author';

/** Пределы чисел — те же, что держит сервер (`normalize_cover_label`). */
export const COVER_LIMITS = {
  width: [10, 100],
  size: [2, 30],
  weight: [100, 900],
  lineHeight: [0.7, 2.5],
  spacing: [-0.1, 0.5],
  shadow: [0, 1],
  blur: [0, 6],
  radius: [0, 10],
  pad: [0, 10],
} as const;

/** Что получает новая надпись: название ниже середины крупно, автор у низа мельче. */
export function defaultLabel(key: BookCoverKey): BookCoverLabel {
  const title = key === 'title';
  return {
    shown: true,
    text: '',
    x: 50,
    y: title ? 82 : 93,
    width: title ? 78 : 56,
    font: 'cormorant',
    size: title ? 8.5 : 4.6,
    weight: title ? 600 : 500,
    lineHeight: title ? 1.08 : 1.15,
    spacing: title ? 0.01 : 0.08,
    italic: !title,
    shadow: 0,
    align: 'center',
    color: '#2a1d14',
    plate: { on: true, color: '#f3ead9', opacity: 0.55, blur: 1.5, radius: 1.5, pad: title ? 3 : 2 },
  };
}

/** Надпись с недостающим из значений по умолчанию: сервер хранит только то, что назначено. */
export function labelOf(text: BookCoverText | null | undefined, key: BookCoverKey): BookCoverLabel | null {
  const own = text?.[key];
  if (!own) return null;
  const base = defaultLabel(key);
  return { ...base, ...own, plate: { ...base.plate, ...(own.plate ?? {}) } };
}

/** Показывается ли надпись: нужна и запись, и непустой текст. */
export function labelShown(label: BookCoverLabel | null, text: string): label is BookCoverLabel {
  return !!label && label.shown && text.trim().length > 0;
}

/** Тень под буквами: близкая и широкая, обе тёмные; сила — одно число. */
function letterShadow(strength: number): string {
  if (!(strength > 0)) return 'none';
  return `0 0.25cqi 0.7cqi rgba(0,0,0,${0.85 * strength}), 0 0 2.6cqi rgba(0,0,0,${0.7 * strength})`;
}

export function labelStyle(label: BookCoverLabel): string {
  return [
    `left:${label.x}%`,
    `top:${label.y}%`,
    `width:${label.width}%`,
    `font-family:${fontStack(label.font) === 'inherit' ? 'inherit' : fontStack(label.font)}`,
    `font-size:${label.size}cqi`,
    `font-weight:${label.weight}`,
    `font-style:${label.italic ? 'italic' : 'normal'}`,
    `line-height:${label.lineHeight}`,
    `letter-spacing:${label.spacing}em`,
    `text-align:${label.align}`,
    `color:${label.color}`,
    `text-shadow:${letterShadow(label.shadow ?? 0)}`,
  ].join(';');
}

function rgba(hex: string, alpha: number): string {
  const n = parseInt(hex.slice(1), 16);
  return `rgba(${(n >> 16) & 255},${(n >> 8) & 255},${n & 255},${alpha})`;
}

/** Табличка: матовое стекло — полупрозрачный тон и размытие того, что под ней. */
export function plateStyle(label: BookCoverLabel): string {
  const p = label.plate;
  if (!p.on) return '';
  const blur = p.blur > 0 ? `blur(${p.blur}cqi) saturate(1.1)` : 'none';
  return [
    `padding:${p.pad}cqi ${p.pad * 1.4}cqi`,
    `border-radius:${p.radius}cqi`,
    `background:${rgba(p.color, p.opacity)}`,
    `backdrop-filter:${blur}`,
    `-webkit-backdrop-filter:${blur}`,
    `box-shadow:0 0 0 1px ${rgba(p.color, Math.min(1, p.opacity + 0.25))} inset, 0 0.6cqi 2cqi rgba(0,0,0,0.28)`,
  ].join(';');
}
