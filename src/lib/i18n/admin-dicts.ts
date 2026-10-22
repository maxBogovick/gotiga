import { LANGS, type Lang } from './index';

type Dict = Record<string, string>;

/* Admin dictionaries and, for the copy editor, the public ones too, gathered by
 * file name. A glob rather than one import per language: a language whose
 * dictionary has not been written yet is simply absent and falls back to
 * English, instead of breaking the admin bundle. Each file exports its
 * dictionary under the language code (`ru`, `es`) or `<code>Admin`. */
const adminFiles = import.meta.glob<Record<string, Dict>>('./*.admin.ts', { eager: true });
const publicFiles = import.meta.glob<Record<string, Dict>>(
	['./en.ts', './ru.ts', './uk.ts', './es.ts', './de.ts', './it.ts'],
	{ eager: true },
);

/** Admin strings per language, for `registerAdminDicts`. */
export function adminDicts(): Partial<Record<Lang, Dict>> {
	const out: Partial<Record<Lang, Dict>> = {};
	for (const l of LANGS) {
		const d = adminFiles[`./${l}.admin.ts`]?.[`${l}Admin`];
		if (d) out[l] = d;
	}
	return out;
}

/** Public plus admin strings per language, with English under anything missing. */
export function fullDicts(): Record<Lang, Dict> {
	const admin = adminDicts();
	const en = { ...(publicFiles['./en.ts']?.en ?? {}), ...(admin.en ?? {}) };
	const out = {} as Record<Lang, Dict>;
	for (const l of LANGS) {
		out[l] = l === 'en' ? en : { ...en, ...(publicFiles[`./${l}.ts`]?.[l] ?? {}), ...(admin[l] ?? {}) };
	}
	return out;
}
