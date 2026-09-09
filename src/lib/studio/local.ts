/**
 * Локальный верстак: рамки и картинки живут в браузере, пока человек работает.
 *
 * Почему не `localStorage`: там пять мегабайт и только строки, а деталь рамы —
 * это Blob, и их у одной рамы до двадцати. IndexedDB держит блобы и не имеет
 * такого потолка.
 *
 * Почему не сразу на сервер: правка идёт непрерывно — тянут ползунок, и число
 * меняется десять раз в секунду. Класть это в сеть значит слать десять запросов
 * в секунду ради работы, которую человек ещё и выбросит. На сервер уходит то,
 * что человек СЛОЖИЛ В ЯЩИК, — своим решением и своей кнопкой.
 *
 * Отказ хранилища не считается поломкой: в приватном окне IndexedDB может быть
 * закрыт совсем. Тогда `available()` говорит «нет», студия говорит об этом
 * словами и предлагает ящик — но молча работу не теряет.
 */

import type { BattleFrame } from '$lib/types/api';

const DB = 'gotiga-studio';
const VERSION = 2;
/** Рамки: JSON и когда правили. */
const FRAMES = 'frames';
/** Картинки: блоб под своим ключом. Ключ рамка носит у себя в теле. */
const BLOBS = 'blobs';
/** Опись своих картинок: имя, роль, вес. Блоб лежит рядом, под тем же ключом.
 *  Порознь, потому что блоб читают редко и он тяжёлый, а опись — на каждый
 *  показ склада. */
const ASSETS = 'assets';

/** Одна рамка на верстаке. `id` свой, локальный: у неё ещё нет имени на
 *  сервере, и связывать её с серверным идентификатором нечем, пока не
 *  выложена. */
export interface LocalFrame {
	id: string;
	name: string;
	body: BattleFrame;
	updatedAt: number;
	/** Идентификатор на сервере, если рамку уже клали в ящик. */
	remoteId?: string | null;
	/** Ушла ли рамка на полный стол. Дорога односторонняя, и помнить об этом
	 *  должна сама рамка: иначе человек, вернувшись завтра, увидит пять
	 *  ползунков вместо своей резьбы. */
	advanced?: boolean;
}

let opening: Promise<IDBDatabase | null> | null = null;

function open(): Promise<IDBDatabase | null> {
	if (opening) return opening;
	opening = new Promise((resolve) => {
		if (typeof indexedDB === 'undefined') {
			resolve(null);
			return;
		}
		let request: IDBOpenDBRequest;
		try {
			request = indexedDB.open(DB, VERSION);
		} catch {
			// Приватное окно некоторых браузеров бросает прямо здесь.
			resolve(null);
			return;
		}
		request.onupgradeneeded = () => {
			const db = request.result;
			if (!db.objectStoreNames.contains(FRAMES)) db.createObjectStore(FRAMES, { keyPath: 'id' });
			if (!db.objectStoreNames.contains(BLOBS)) db.createObjectStore(BLOBS);
			if (!db.objectStoreNames.contains(ASSETS)) db.createObjectStore(ASSETS, { keyPath: 'key' });
		};
		request.onsuccess = () => resolve(request.result);
		request.onerror = () => resolve(null);
		// Открытие, которого никто не дождался, — тоже отказ: страница не должна
		// висеть, пока браузер решает.
		setTimeout(() => resolve(null), 3000);
	});
	return opening;
}

/** Работает ли верстак вообще. Спрашивают один раз, при входе в студию. */
export async function available(): Promise<boolean> {
	return (await open()) !== null;
}

function run<T>(
	store: string,
	mode: IDBTransactionMode,
	work: (s: IDBObjectStore) => IDBRequest<T>,
): Promise<T | null> {
	return open().then(
		(db) =>
			new Promise<T | null>((resolve) => {
				if (!db) {
					resolve(null);
					return;
				}
				try {
					const tx = db.transaction(store, mode);
					const request = work(tx.objectStore(store));
					request.onsuccess = () => resolve(request.result as T);
					request.onerror = () => resolve(null);
				} catch {
					resolve(null);
				}
			}),
	);
}

// ── Рамки ────────────────────────────────────────────────────────────────────

export async function listFrames(): Promise<LocalFrame[]> {
	const rows = (await run<LocalFrame[]>(FRAMES, 'readonly', (s) => s.getAll())) ?? [];
	return rows.sort((a, b) => b.updatedAt - a.updatedAt);
}

export async function getFrame(id: string): Promise<LocalFrame | null> {
	return (await run<LocalFrame>(FRAMES, 'readonly', (s) => s.get(id))) ?? null;
}

/** Записать рамку. Без надписи «сохранено»: она просто не теряется. */
export async function putFrame(frame: LocalFrame): Promise<void> {
	await run(FRAMES, 'readwrite', (s) => s.put({ ...frame, updatedAt: Date.now() }));
}

export async function dropFrame(id: string): Promise<void> {
	await run(FRAMES, 'readwrite', (s) => s.delete(id));
}

// ── Картинки ─────────────────────────────────────────────────────────────────
//
// Ключ картинки — строка `local:<uuid>`, и она же стоит в теле рамы там, где у
// выложенной рамы стоит адрес на складе. Отрисовщику всё равно, что показывать,
// если ему дали то, что браузер умеет загрузить, — поэтому локальная работа
// выглядит ровно так же, как выложенная, и подмены не случается при выкладке.

export function localKey(): string {
	return `local:${crypto.randomUUID()}`;
}

export function isLocal(url: string | null | undefined): boolean {
	return typeof url === 'string' && url.startsWith('local:');
}

export async function putBlob(key: string, blob: Blob): Promise<void> {
	await run(BLOBS, 'readwrite', (s) => s.put(blob, key));
}

export async function getBlob(key: string): Promise<Blob | null> {
	return (await run<Blob>(BLOBS, 'readonly', (s) => s.get(key))) ?? null;
}

export async function dropBlob(key: string): Promise<void> {
	await run(BLOBS, 'readwrite', (s) => s.delete(key));
}

export async function listBlobKeys(): Promise<string[]> {
	const keys = (await run<IDBValidKey[]>(BLOBS, 'readonly', (s) => s.getAllKeys())) ?? [];
	return keys.map(String);
}

/**
 * Показать локальную картинку: `local:…` → `blob:…`.
 *
 * Адреса живут в одной таблице на страницу и отзываются вместе (`forgetUrls`).
 * Без этого каждая перерисовка карты плодила бы новый `blob:`-адрес, а старые
 * держали бы картинку в памяти до перезагрузки.
 */
const shown = new Map<string, string>();

export async function showLocal(key: string): Promise<string | null> {
	const had = shown.get(key);
	if (had) return had;
	const blob = await getBlob(key);
	if (!blob) return null;
	const url = URL.createObjectURL(blob);
	shown.set(key, url);
	return url;
}

export function forgetUrls(): void {
	for (const url of shown.values()) URL.revokeObjectURL(url);
	shown.clear();
}

// ── Свой склад в браузере ────────────────────────────────────────────────────
//
// То же, что ящик на сервере, только у себя: картинка лежит здесь, пока её не
// решили отдать дому. Роль та же, что у деталей дома, — чтобы деталь, уехавшая
// в ящик, не поменяла породу по дороге.

export interface LocalAsset {
	key: string;
	name: string;
	role: string;
	bytes: number;
	width: number;
	height: number;
	createdAt: number;
}

export async function listAssets(): Promise<LocalAsset[]> {
	const rows = (await run<LocalAsset[]>(ASSETS, 'readonly', (s) => s.getAll())) ?? [];
	return rows.sort((a, b) => b.createdAt - a.createdAt);
}

export async function putAsset(row: LocalAsset): Promise<void> {
	await run(ASSETS, 'readwrite', (s) => s.put(row));
}

export async function dropAsset(key: string): Promise<void> {
	await run(ASSETS, 'readwrite', (s) => s.delete(key));
	await dropBlob(key);
}

/** Положить свою картинку на верстак. Размер читается у самой картинки: имя
 *  файла о нём не говорит, а склад считается по байтам. */
export async function keepAsset(file: File, role = 'other'): Promise<LocalAsset> {
	const key = localKey();
	await putBlob(key, file);
	let width = 0;
	let height = 0;
	try {
		const bitmap = await createImageBitmap(file);
		width = bitmap.width;
		height = bitmap.height;
		bitmap.close?.();
	} catch {
		// Не прочиталось — не повод не сохранить: показывать размер необязательно.
	}
	const row: LocalAsset = {
		key,
		name: file.name.replace(/\.[^.]+$/, '').slice(0, 80) || 'деталь',
		role,
		bytes: file.size,
		width,
		height,
		createdAt: Date.now(),
	};
	await putAsset(row);
	return row;
}

/** Сколько занято локально. Не потолок — отчёт: браузер решает сам, сколько
 *  даст, и врать человеку числом, которого мы не назначали, незачем. */
export async function localUsed(): Promise<number> {
	const keys = await listBlobKeys();
	let sum = 0;
	for (const key of keys) {
		const blob = await getBlob(key);
		sum += blob?.size ?? 0;
	}
	return sum;
}

/**
 * Убрать картинки, на которые не ссылается ни одна рамка верстака.
 *
 * Нужен потому, что деталь заменяют чаще, чем думают: взял другой уголок —
 * прежний блоб остался лежать. Локальный ящик не бесконечен, и мусор в нём
 * никто не увидит, пока браузер не откажет.
 */
export async function sweep(): Promise<number> {
	const frames = await listFrames();
	const used = new Set<string>();
	for (const frame of frames) {
		for (const value of Object.values(frame.body ?? {})) {
			if (typeof value === 'string' && isLocal(value)) used.add(value);
		}
		for (const ornament of frame.body?.ornaments ?? []) {
			if (isLocal(ornament.image)) used.add(ornament.image);
		}
	}
	// Картинки СКЛАДА держатся сами по себе: они лежат не ради рамы, а ради
	// того, что из них ещё сделают.
	for (const one of await listAssets()) used.add(one.key);
	let gone = 0;
	for (const key of await listBlobKeys()) {
		if (used.has(key)) continue;
		await dropBlob(key);
		gone += 1;
	}
	return gone;
}
