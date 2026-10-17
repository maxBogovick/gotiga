// Отдельный поток для карты этюдов: рельеф, деревья и облака считаются
// здесь, а страница получает готовые картинки. Расчёт занимает от трёхсот
// миллисекунд до нескольких секунд на слабом процессоре, и в главном потоке
// всё это время страница не отвечала бы.
import { buildMap } from './etudeMap';

self.onmessage = async (e: MessageEvent<{ count: number }>) => {
  try {
    const built = await buildMap(e.data.count);
    self.postMessage({ ok: true, built });
  } catch (error) {
    self.postMessage({ ok: false, error: String(error) });
  }
};
