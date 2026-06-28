import type { DisplayConfig, BlockStyle } from '$lib/types/api';
import { READING_FONTS } from '$lib/stores/reading-font.svelte';
export type { BlockStyle };

// Orderable / hideable content blocks (lower sections)
export const BLOCK_IDS = ['description', 'making', 'video', 'showings', 'related', 'comments'] as const;
export type BlockId = typeof BLOCK_IDS[number];

// Upper page elements — styleable only, never ordered or hidden
export const UPPER_ZONE_IDS = ['name', 'shortText'] as const;
export type UpperZoneId = typeof UPPER_ZONE_IDS[number];

export type ZoneId = UpperZoneId | BlockId;

// Build font stack lookup from the site's reading-font system
const FONT_STACK: Record<string, string> = Object.fromEntries(
  READING_FONTS.map(f => [f.id, f.stack])
);

const FONT_SIZES: Record<NonNullable<BlockStyle['fontSize']>, string> = {
  sm:   '0.875rem',
  base: '1rem',
  lg:   '1.125rem',
  xl:   '1.25rem',
};

function styleForZone(s: BlockStyle): string {
  const parts: string[] = [];
  if (s.color) parts.push(`color:${s.color}`);
  if (s.fontSize && s.fontSize !== 'base') parts.push(`font-size:${FONT_SIZES[s.fontSize]}`);
  if (s.font) parts.push(`font-family:${FONT_STACK[s.font] ?? 'inherit'}`);
  return parts.join(';');
}

/**
 * For DC-block wrapper divs: outputs CSS custom properties that targeted
 * rules in figurine-detail.css pick up for descendant text elements.
 * (Direct `color` on a wrapper can't beat explicit child `color: var(...)` rules.)
 */
export function computeBlockStyle(config: DisplayConfig | null, blockId: BlockId): string {
  const s = config?.blockStyles?.[blockId];
  if (!s) return '';
  const parts: string[] = [];
  if (s.color) parts.push(`--dc-block-color:${s.color}`);
  if (s.fontSize && s.fontSize !== 'base') parts.push(`--dc-block-size:${FONT_SIZES[s.fontSize]}`);
  if (s.font) parts.push(`--dc-block-font:${FONT_STACK[s.font] ?? 'inherit'}`);
  return parts.join(';');
}

/**
 * For upper-zone elements (name, shortText): outputs direct CSS properties
 * applied inline on the element itself (inline style always wins regardless of
 * CSS specificity, so no custom-property trick needed).
 */
export function computeElementStyle(config: DisplayConfig | null, zoneId: ZoneId): string {
  const s = config?.blockStyles?.[zoneId];
  if (!s) return '';
  return styleForZone(s);
}

export function isBlockVisible(config: DisplayConfig | null, blockId: BlockId): boolean {
  return !(config?.hiddenBlocks ?? []).includes(blockId);
}

/**
 * Computes CSS `order` custom-property declarations for the lower-section flex container.
 * Returns an inline style string like `--order-description:0;--order-making:1;...`
 */
export function computeSectionOrderStyle(config: DisplayConfig | null): string {
  const order = config?.blockOrder ?? [];
  if (!order.length) return '';
  return BLOCK_IDS.map((blockId, defaultIdx) => {
    const pos = order.indexOf(blockId as BlockId);
    return `--order-${blockId}:${pos >= 0 ? pos : defaultIdx}`;
  }).join(';');
}
