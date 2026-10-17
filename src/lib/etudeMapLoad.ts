// Где страница берёт карту этюдов: из кэша браузера, из отдельного потока или,
// если ни того ни другого нет, считает сама.
//
// Карта зависит только от версии рисунка и числа этюдов (деревья обходят
// знаки), поэтому кэш ключуется этими двумя числами. Вернувшийся гость
// получает карту сразу, без расчёта.
import { MAP_VERSION, buildMap, type BuiltMap, type MapMotion } from './etudeMap';

const CACHE = 'etude-map';

export interface LoadedMap {
  picture: string;
  clouds: string[];
  motion: MapMotion;
  /** Освободить адреса картинок, когда карта уходит со страницы. */
  release: () => void;
}

const key = (count: number, part: string) => `/__etude-map/v${MAP_VERSION}/${count}/${part}`;

async function fromCache(count: number): Promise<BuiltMap | null> {
  if (typeof caches === 'undefined') return null;
  try {
    const box = await caches.open(CACHE);
    const [picture, motion, ...clouds] = await Promise.all(
      ['map.webp', 'motion.json', 'cloud-0.webp', 'cloud-1.webp', 'cloud-2.webp'].map((p) =>
        box.match(key(count, p)),
      ),
    );
    if (!picture || !motion) return null;
    return {
      picture: await picture.blob(),
      motion: await motion.json(),
      clouds: await Promise.all(clouds.filter((c): c is Response => !!c).map((c) => c.blob())),
    };
  } catch {
    return null;
  }
}

async function toCache(count: number, built: BuiltMap) {
  if (typeof caches === 'undefined') return;
  try {
    const box = await caches.open(CACHE);
    // Прежние версии рисунка больше не понадобятся. Другое число этюдов не
    // трогаем: оно соседний режим («Обучение» и «Прохождение» — две карты), и
    // стиравшая его запись заставляла считать карту заново на каждое
    // переключение режима.
    const current = `/__etude-map/v${MAP_VERSION}/`;
    for (const request of await box.keys()) {
      if (!new URL(request.url).pathname.startsWith(current)) await box.delete(request);
    }
    await box.put(key(count, 'map.webp'), new Response(built.picture));
    await box.put(
      key(count, 'motion.json'),
      new Response(JSON.stringify(built.motion), { headers: { 'Content-Type': 'application/json' } }),
    );
    await Promise.all(built.clouds.map((c, i) => box.put(key(count, `cloud-${i}.webp`), new Response(c))));
  } catch {
    // Кэш — удобство: без него карта просто считается заново.
  }
}

function inWorker(count: number): Promise<BuiltMap> {
  return new Promise((resolve, reject) => {
    const worker = new Worker(new URL('./etudeMap.worker.ts', import.meta.url), { type: 'module' });
    worker.onmessage = (e) => {
      worker.terminate();
      if (e.data?.ok) resolve(e.data.built);
      else reject(new Error(e.data?.error ?? 'etude map worker failed'));
    };
    worker.onerror = (e) => {
      worker.terminate();
      reject(e);
    };
    worker.postMessage({ count });
  });
}

export async function loadEtudeMap(count: number): Promise<LoadedMap> {
  let built = await fromCache(count);
  if (!built) {
    try {
      built = await inWorker(count);
    } catch {
      built = await buildMap(count);
    }
    void toCache(count, built);
  }
  const picture = URL.createObjectURL(built.picture);
  const clouds = built.clouds.map((c) => URL.createObjectURL(c));
  return {
    picture,
    clouds,
    motion: built.motion,
    release: () => {
      URL.revokeObjectURL(picture);
      clouds.forEach((c) => URL.revokeObjectURL(c));
    },
  };
}
