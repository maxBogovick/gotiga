// Связи между разделами сайта: работа, её карта в игре, этюды, где эта карта
// стоит, и байки о той же работе.
//
// Всё связано через работу (`figurineId`): у карты одна работа, у байки одна
// работа, а этюд называет карты по слагу. Поэтому разделы ссылаются друг на
// друга по героине, а не пунктом меню.
//
// Данные читаются в браузере один раз на несколько минут и делятся между всеми
// местами страницы: на странице работы, байки и на главной блоки спрашивают
// одно и то же, и четыре одинаковых запроса за полкой были бы лишними.
import { api } from '$lib/api';
import type { BattleCard, BattleChallenge, BattleFrame, GazetteLeaf } from '$lib/types/api';

export interface World {
  cards: BattleCard[];
  frames: BattleFrame[];
  etudes: BattleChallenge[];
  tales: GazetteLeaf[];
}

/** Сколько держать прочитанное: полка меняется редко, а SPA живёт часами. */
const KEEP_MS = 5 * 60 * 1000;

let pending: Promise<World> | null = null;
let readAt = 0;

export function loadWorld(): Promise<World> {
  if (pending && Date.now() - readAt < KEEP_MS) return pending;
  readAt = Date.now();
  pending = Promise.all([
    api.getBattleCards(),
    api.getBattleFrames(),
    api.getBattleChallenges(),
    api.getTales().catch(() => [] as GazetteLeaf[]),
  ]).then(([cards, frames, etudes, tales]) => ({
    cards,
    frames: frames.frames,
    etudes,
    tales,
  }));
  return pending;
}

/** Карта, сделанная из этой работы. Строения (стены, башни) — не героини. */
export function cardOfWork(world: World, figurineId: string | null | undefined): BattleCard | null {
  if (!figurineId) return null;
  return (
    world.cards.find((c) => c.figurineId === figurineId && c.kind !== 'structure') ?? null
  );
}

/** Этюды, где эта карта стоит на поле или в руке — у любой из сторон. */
export function etudesWithCard(world: World, card: BattleCard): BattleChallenge[] {
  return world.etudes.filter((e) => {
    const s = e.setup;
    return (
      s.keeperBoard.some((p) => p.card === card.slug) ||
      s.playerBoard.some((p) => p.card === card.slug) ||
      s.keeperHand.includes(card.slug) ||
      s.playerHand.includes(card.slug)
    );
  });
}

/** Байки о работе, в порядке полки. */
export function talesOfWork(world: World, figurineId: string | null | undefined): GazetteLeaf[] {
  if (!figurineId) return [];
  return world.tales
    .filter((t) => t.figurineId === figurineId)
    .sort((a, b) => (a.shelfOrder ?? 1e9) - (b.shelfOrder ?? 1e9));
}

/** Карты этюда (без строений), у которых есть работа: о них есть что почитать. */
export function heroesOfEtude(world: World, etude: BattleChallenge): BattleCard[] {
  const s = etude.setup;
  const slugs = new Set([
    ...s.keeperBoard.map((p) => p.card),
    ...s.keeperHand,
    ...s.playerBoard.map((p) => p.card),
    ...s.playerHand,
  ]);
  return world.cards.filter((c) => slugs.has(c.slug) && c.kind !== 'structure' && c.figurineId);
}

export const cardHref = (card: BattleCard) => `/battles?card=${card.id}`;
export const etudeHref = (etude: BattleChallenge) => `/battles/etude?play=${etude.id}`;
export const workHref = (card: BattleCard) =>
  card.figurineSlug || card.figurineId ? `/figurines/${card.figurineSlug || card.figurineId}` : null;
