import { api, ApiError } from '$lib/api';
import { authStore } from '$lib/stores/auth.svelte';

/**
 * Записать названную почту в имя — один раз, в тот момент, когда её спросило
 * дело (бронь, заказ, заявка).
 *
 * Отдельная функция, потому что мест таких три, и «сохранить, а потом отправить»
 * в каждом из них написанное по-своему разошлось бы на первой же правке.
 *
 * `taken` — единственный случай, который должен остановить дело: чужой адрес в
 * своё имя не записывают. Сорвавшаяся сеть дело не держит — почта всё равно
 * уедет в саму заявку, а имя узнает её в следующий раз.
 */
export async function keepEmail(email: string): Promise<'ok' | 'taken' | 'failed'> {
  const token = authStore.token;
  if (!token || !authStore.needsEmail) return 'ok';
  try {
    authStore.setSession(token, await api.nameEmail(token, email));
    return 'ok';
  } catch (e) {
    return e instanceof ApiError && e.status === 409 ? 'taken' : 'failed';
  }
}
