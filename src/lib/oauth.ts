/**
 * Вход через Google и Apple: сторона вкладки.
 *
 * Страница сама ничего не грузит у провайдера — она просит у сервера ссылку и
 * уходит по ней обычным переходом. Привязка (`binder`) остаётся здесь, в
 * `sessionStorage` этой вкладки, и уходит на сервер вместе с кодом провайдера
 * (его страница возврата берёт из `#`-части адреса): без любой из двух половин
 * сервер не обменяет код. Поэтому ссылка, присланная посторонним, чужим входом
 * не награждает — у открывшего её привязки нет, а у приславшего нет кода.
 */
import { api } from '$lib/api';
import type { OauthProvider } from '$lib/types/api';

// Ключ в sessionStorage, а не в localStorage: ждёт ОДНА вкладка, та самая,
// которой достанется результат. Новая вкладка чужого входа не продолжает.
const KEPT = 'gotiga_oauth';

export interface KeptOauth {
    state: string;
    binder: string;
    provider: OauthProvider;
    /** `login` — вход, `link` — привязка к вошедшему имени. */
    mode: 'login' | 'link';
    /** Куда вернуться после входа. Только свой путь, не адрес. */
    from: string;
    /** До какого часа привязка ещё что-то значит. */
    dies: number;
}

/** Путь внутри сайта. Всё, что похоже на чужой адрес, заменяется главной. */
export function ownPath(raw: string | null | undefined, fallback = '/'): string {
    if (!raw || !raw.startsWith('/') || raw.startsWith('//') || raw.includes('\\')) return fallback;
    return raw;
}

/**
 * Уйти к провайдеру. Возвращает только при неудаче (при успехе страница
 * меняется), и тогда причина — в исключении: вызывающий печатает свою строку.
 */
export async function beginOauth(
    provider: OauthProvider,
    opts: {
        mode?: 'login' | 'link';
        sessionToken?: string | null;
        from?: string | null;
        /** Отметил ли человек «мне 16 или больше»: без неё новое имя не заводится. */
        ageConfirmed?: boolean;
    } = {}
): Promise<void> {
    const mode = opts.mode ?? 'login';
    const start = await api.oauthStart(
        provider,
        mode === 'link' ? opts.sessionToken : null,
        opts.ageConfirmed === true
    );
    const kept: KeptOauth = {
        state: start.state,
        binder: start.binder,
        provider,
        mode,
        from: ownPath(opts.from, mode === 'link' ? '/profile/account' : '/'),
        dies: Date.parse(start.expiresAt) || Date.now() + 10 * 60 * 1000,
    };
    // Без хранилища (приватное окно с отключённым storage) вход невозможен:
    // привязку негде оставить. Лучше сказать сразу, чем увести человека к
    // провайдеру и вернуть ни с чем.
    sessionStorage.setItem(KEPT, JSON.stringify(kept));
    window.location.assign(start.url);
}

/** Что вкладка помнит об уходе к провайдеру. */
export function keptOauth(): KeptOauth | null {
    try {
        const kept = JSON.parse(sessionStorage.getItem(KEPT) ?? 'null') as KeptOauth | null;
        if (!kept?.state || !kept.binder || !kept.provider) return null;
        return kept;
    } catch {
        return null;
    }
}

export function forgetOauth(): void {
    try {
        sessionStorage.removeItem(KEPT);
    } catch {
        /* нечего забывать */
    }
}
