import { browser } from '$app/environment';

/**
 * Жетон посетителя — та же строка, которой читатель назван у работ
 * (`gotiga_visitor_token`): непрозрачное состояние браузера, не вход в дом.
 * Нужен затем, чтобы сервер мог отличить повторный голос от нового, и ни за
 * чем больше.
 */
const TOKEN_KEY = 'gotiga_visitor_token';

export function visitorToken(): string {
  if (!browser) return '';
  let token = localStorage.getItem(TOKEN_KEY);
  if (!token) {
    token = crypto.randomUUID();
    try {
      localStorage.setItem(TOKEN_KEY, token);
    } catch {
      // Приватное окно: жетон живёт до закрытия вкладки, и это допустимо —
      // голос без жетона всё равно не отправляется.
    }
  }
  return token;
}
