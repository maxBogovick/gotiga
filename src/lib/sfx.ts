// Короткие звуки и вибрация забега. Файлов нет: звук собирается из тонов Web Audio,
// поэтому он весит ноль байт и не требует загрузки. Включён по умолчанию, тихий;
// выбор игрока хранится в `gotiga_run_sound` ('0' — выключен).

const KEY = 'gotiga_run_sound';

let enabled: boolean | null = null;
let ctx: AudioContext | null = null;

export function soundOn(): boolean {
  if (enabled === null) {
    try {
      enabled = localStorage.getItem(KEY) !== '0';
    } catch {
      enabled = true;
    }
  }
  return enabled;
}

export function setSound(on: boolean) {
  enabled = on;
  try {
    localStorage.setItem(KEY, on ? '1' : '0');
  } catch {
    // Хранилище закрыто — выбор живёт до перезагрузки.
  }
}

type Tone = { from: number; to?: number; at?: number; dur: number; gain: number; type?: OscillatorType };

function play(tones: Tone[]) {
  if (!soundOn() || typeof window === 'undefined') return;
  try {
    ctx ??= new AudioContext();
    if (ctx.state === 'suspended') void ctx.resume();
    const now = ctx.currentTime;
    for (const tone of tones) {
      const start = now + (tone.at ?? 0);
      const osc = ctx.createOscillator();
      const amp = ctx.createGain();
      osc.type = tone.type ?? 'sine';
      osc.frequency.setValueAtTime(tone.from, start);
      if (tone.to) osc.frequency.exponentialRampToValueAtTime(tone.to, start + tone.dur);
      amp.gain.setValueAtTime(0.0001, start);
      amp.gain.exponentialRampToValueAtTime(tone.gain, start + 0.008);
      amp.gain.exponentialRampToValueAtTime(0.0001, start + tone.dur);
      osc.connect(amp).connect(ctx.destination);
      osc.start(start);
      osc.stop(start + tone.dur + 0.02);
    }
  } catch {
    // Звук недоступен (нет устройства или запрет браузера): игра идёт без него.
  }
}

function buzz(ms: number) {
  if (!soundOn()) return;
  try {
    navigator.vibrate?.(ms);
  } catch {
    // Вибрации нет — ничего не делаем.
  }
}

export const sfx = {
  /** Нажали карту: короткий светлый тон. */
  lift() {
    play([{ from: 340, to: 480, dur: 0.07, gain: 0.05 }]);
  },
  /** Карта встала на клетку: глухой стук и щелчок. */
  drop() {
    play([
      { from: 150, to: 70, dur: 0.1, gain: 0.14, type: 'triangle' },
      { from: 920, at: 0.01, dur: 0.025, gain: 0.03, type: 'square' },
    ]);
    buzz(12);
  },
  /** Начало волны: два тона вверх. */
  start() {
    play([
      { from: 196, to: 220, dur: 0.18, gain: 0.07, type: 'triangle' },
      { from: 294, to: 330, at: 0.14, dur: 0.3, gain: 0.07, type: 'triangle' },
    ]);
    buzz(30);
  },
};
