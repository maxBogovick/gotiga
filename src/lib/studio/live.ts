/**
 * Два обличья одной рамы.
 *
 * В ящике браузера картинка лежит блобом под ключом `local:<uuid>`, а показать
 * её можно только адресом `blob:…`, который живёт до перезагрузки страницы.
 * Значит, у рамы два обличья: ХРАНИМОЕ (ключи) и ЖИВОЕ (адреса).
 *
 * Обе замены сделаны здесь и только здесь. Порознь они однажды разойдутся, и в
 * ящик ляжет `blob:`-адрес, который завтра не откроется ничем: он умирает
 * вместе со вкладкой, а рама остаётся.
 *
 * Отрисовщик про всё это не знает вовсе — ему дают то, что браузер умеет
 * загрузить, и локальная работа выглядит ровно так же, как выложенная.
 */

import type { BattleFrame } from '$lib/types/api';
import { getBlob, isLocal } from './local';

/** Живой адрес → ключ в ящике. Обратной дороги без этого нет: `blob:`-адрес
 *  ничего о себе не рассказывает. */
const backwards = new Map<string, string>();
/** Ключ → живой адрес, чтобы не плодить адрес на каждую перерисовку. */
const forwards = new Map<string, string>();

export async function liveUrl(key: string): Promise<string | null> {
	const had = forwards.get(key);
	if (had) return had;
	const blob = await getBlob(key);
	if (!blob) return null;
	const url = URL.createObjectURL(blob);
	forwards.set(key, url);
	backwards.set(url, key);
	return url;
}

/** Отпустить все живые адреса. Зовётся при уходе со стола: без этого каждая
 *  картинка держится в памяти до перезагрузки. */
export function forgetLive(): void {
	for (const url of forwards.values()) URL.revokeObjectURL(url);
	forwards.clear();
	backwards.clear();
}

/** Запомнить только что положенный блоб — чтобы его адрес умел возвращаться в
 *  ключ, не побывав ни разу в ящике. */
export function rememberLive(key: string, url: string): void {
	forwards.set(key, url);
	backwards.set(url, key);
}

type Bag = Record<string, unknown>;

async function walk(value: unknown, swap: (s: string) => Promise<string> | string): Promise<unknown> {
	if (typeof value === 'string') return swap(value);
	if (Array.isArray(value)) return Promise.all(value.map((one) => walk(one, swap)));
	if (value && typeof value === 'object') {
		const out: Bag = {};
		for (const [key, one] of Object.entries(value as Bag)) out[key] = await walk(one, swap);
		return out;
	}
	return value;
}

/** Рама, готовая к показу: ключи заменены живыми адресами. Ключ, для которого
 *  блоба уже нет, остаётся как есть — пустая картинка честнее подмены. */
export async function toLive(frame: BattleFrame): Promise<BattleFrame> {
	return (await walk(frame, async (s) => (isLocal(s) ? ((await liveUrl(s)) ?? s) : s))) as BattleFrame;
}

/** Рама, готовая лечь в ящик: живые адреса заменены ключами. */
export async function toStored(frame: BattleFrame): Promise<BattleFrame> {
	return (await walk(frame, (s) => backwards.get(s) ?? s)) as BattleFrame;
}

/**
 * Есть ли у картинки прозрачность.
 *
 * Тот же вопрос и тот же ответ, что у сервера (`build_frame_asset`): рама без
 * дыры, надетая поверх карты, закрыла бы её целиком, и человеку надо сказать
 * об этом сразу, а не после выкладки. Считается по выборке — каждый 37-й
 * пиксель: шаг, который не может попасть в такт ширине строки, а ответ на
 * тысяче пикселей тот же, что на двух миллионах.
 */
export async function hasAlpha(file: File): Promise<boolean> {
	try {
		const bitmap = await createImageBitmap(file);
		const canvas = document.createElement('canvas');
		canvas.width = bitmap.width;
		canvas.height = bitmap.height;
		const ctx = canvas.getContext('2d', { willReadFrequently: true });
		if (!ctx) return false;
		ctx.drawImage(bitmap, 0, 0);
		const { data } = ctx.getImageData(0, 0, bitmap.width, bitmap.height);
		bitmap.close?.();
		for (let i = 3; i < data.length; i += 4 * 37) {
			if (data[i] < 250) return true;
		}
		return false;
	} catch {
		// Не прочиталось — пусть решает человек, а не догадка: «нет дыры»
		// заставит раму лечь под карту, что заметно сразу.
		return false;
	}
}
