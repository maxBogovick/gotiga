// Закрытые списки мастера заявки.
// ЭТОТ ФАЙЛ И `src-tauri/server/src/commission_wizard.rs` МЕНЯЮТСЯ ТОЛЬКО ВМЕСТЕ:
// сервер отбрасывает ключи, которых нет в его таблице.

export const CONTACT_CHANNELS = [
	'email',
	'telegram',
	'whatsapp',
	'viber',
	'signal',
	'instagram',
	'facebook',
	'x',
	'phone'
] as const;
export type ContactChannel = (typeof CONTACT_CHANNELS)[number];

export interface RequestContact {
	channel: ContactChannel;
	value: string;
}

export const REQUEST_WISHES = ['similar', 'new'] as const;
export type RequestWish = (typeof REQUEST_WISHES)[number];

export const KEEP_CHANGE_KEYS = ['look', 'pose', 'clothes', 'colors', 'props', 'size'] as const;
export const WHO_KEYS = ['witch', 'granny', 'gnome', 'girl', 'old_man', 'creature', 'other'] as const;
export const MOOD_KEYS = ['kind', 'sly', 'dark', 'funny', 'calm'] as const;
export const PROPS_KEYS = [
	'broom',
	'cat',
	'pumpkin',
	'book',
	'lantern',
	'herbs',
	'bird',
	'other'
] as const;

export interface RequestChoices {
	keep?: (typeof KEEP_CHANGE_KEYS)[number][];
	change?: (typeof KEEP_CHANGE_KEYS)[number][];
	who?: (typeof WHO_KEYS)[number][];
	mood?: (typeof MOOD_KEYS)[number][];
	props?: (typeof PROPS_KEYS)[number][];
}

export const SIZE_KEYS = ['xs', 's', 'm', 'l', 'xl', 'unsure'] as const;
export type SizeKey = (typeof SIZE_KEYS)[number];

export const DEADLINE_KINDS = ['none', 'date', 'asap'] as const;
export type DeadlineKind = (typeof DEADLINE_KINDS)[number];

export const BUDGET_KEYS = ['lt100', '100_200', '200_400', 'gt400', 'discuss'] as const;
export type BudgetKey = (typeof BUDGET_KEYS)[number];

export const BUDGET_CURRENCIES = ['USD', 'EUR'] as const;
export type BudgetCurrency = (typeof BUDGET_CURRENCIES)[number];

/** Не больше стольких ключей в одном списке выбора. */
export const MAX_CHOICES_PER_LIST = 12;
/** Не больше стольких фото в заявке гостя. */
export const MAX_GUEST_PHOTOS = 3;

/** Ссылка, открывающая разговор с человеком по выбранному способу связи.
 *  Значения уже приведены сервером: telegram/instagram/x — имя без @;
 *  whatsapp/viber/signal/phone — «+» и цифры; facebook — имя или `profile.php?id=N`. */
export function contactHref(channel: ContactChannel, value: string): string {
	const v = value.trim();
	const digits = v.replace(/\D/g, '');
	switch (channel) {
		case 'email':
			return `mailto:${v}`;
		case 'telegram':
			return `https://t.me/${encodeURIComponent(v.replace(/^@/, ''))}`;
		case 'whatsapp':
			return `https://wa.me/${digits}`;
		case 'viber':
			return `viber://chat?number=%2B${digits}`;
		case 'signal':
			return `https://signal.me/#p/+${digits}`;
		case 'instagram':
			return `https://instagram.com/${encodeURIComponent(v.replace(/^@/, ''))}`;
		case 'facebook':
			return `https://facebook.com/${v}`;
		case 'x':
			return `https://x.com/${encodeURIComponent(v.replace(/^@/, ''))}`;
		case 'phone':
			return `tel:+${digits}`;
	}
}
