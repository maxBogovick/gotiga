// Что можно искать и о чём можно спрашивать: работы, небылицы, вестник, карты
// и этюды. Читается один раз на несколько минут и делится между поиском в шапке
// и окном «Спросить сайт»: оба спрашивают одно и то же, и два набора запросов
// за одними данными были бы лишними.
//
// Пока ни поиск, ни окно не открывали, не спрашивается ничего: шапка стоит на
// каждой странице.
import { api } from '$lib/api';
import { loadWorld } from '$lib/world';
import type { SiteIndex } from '$lib/site-search';
import type { BattleChallenge, GazetteLeaf } from '$lib/types/api';

export interface AskIndex extends SiteIndex {
  etudes: BattleChallenge[];
}

const KEEP_MS = 5 * 60 * 1000;
let pending: Promise<AskIndex> | null = null;
let readAt = 0;

export function loadIndex(): Promise<AskIndex> {
  if (pending && Date.now() - readAt < KEEP_MS) return pending;
  readAt = Date.now();
  pending = Promise.all([
    api.getAllFigurines(),
    loadWorld().catch(() => ({ cards: [], tales: [] as GazetteLeaf[], etudes: [] as BattleChallenge[] })),
    api.getGazettePage(1, 60).then((p) => p.items).catch(() => [] as GazetteLeaf[]),
  ])
    .then(([works, world, leaves]) => ({
      works,
      cards: world.cards,
      tales: world.tales,
      gazette: leaves.filter((l) => l.kind !== 'tale'),
      etudes: world.etudes,
    }))
    .catch((e) => {
      pending = null;
      throw e;
    });
  return pending;
}
