import type { Reroute } from '@sveltejs/kit';
import { stripLang } from '$lib/i18n/path';

/**
 * `/ru/figurines/x` открывает тот же маршрут, что `/figurines/x`; адрес в
 * браузере и в `event.url` остаётся с приставкой, и по ней страница узнаёт свой
 * язык (`+layout.ts`). Так не надо переносить маршруты в подпапку `[[lang]]`
 * и переписывать ссылки в каждом из них. Работает и при пререндере, и в браузере.
 */
export const reroute: Reroute = ({ url }) => stripLang(url.pathname);
