// Page-turn sounds, synthesised on the fly — no audio files to ship or fetch.
//
// Each variant shapes filtered white noise into the friction "shhhk" of paper
// (plus, for some, a low flap or a string of riffles) and pans it a touch in the
// turn's direction. They are tuned to read as different *kinds* of book rather
// than as sound effects, and stay deliberately quiet.
//
// If real recorded samples are ever preferred, add a file-backed branch in
// playTurnSound — the rest of the app only knows the ids below.

export type TurnSoundId = 'thin' | 'tome' | 'parchment' | 'riffle' | 'cloth';

export const TURN_SOUND_IDS: TurnSoundId[] = ['thin', 'tome', 'parchment', 'riffle', 'cloth'];

type Direction = 'forward' | 'backward';

let ctx: AudioContext | null = null;
let noise: AudioBuffer | null = null;

function audioContext(): AudioContext | null {
  if (typeof window === 'undefined') return null;
  const AC = window.AudioContext ?? (window as typeof window & { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
  if (!AC) return null;
  if (!ctx) ctx = new AC();
  return ctx;
}

function noiseBuffer(c: AudioContext): AudioBuffer {
  if (noise) return noise;
  const length = Math.floor(c.sampleRate * 1.0);
  const buffer = c.createBuffer(1, length, c.sampleRate);
  const data = buffer.getChannelData(0);
  for (let i = 0; i < length; i++) data[i] = Math.random() * 2 - 1;
  noise = buffer;
  return buffer;
}

function source(c: AudioContext, rate = 1): AudioBufferSourceNode {
  const s = c.createBufferSource();
  s.buffer = noiseBuffer(c);
  s.playbackRate.value = rate;
  return s;
}

function filter(c: AudioContext, type: BiquadFilterType, freq: number, q = 0.7): BiquadFilterNode {
  const f = c.createBiquadFilter();
  f.type = type;
  f.frequency.value = freq;
  f.Q.value = q;
  return f;
}

// Pan toward the side the leaf travels, then to the speakers.
function toOutput(c: AudioContext, node: AudioNode, dir: Direction): AudioNode {
  const panner = c.createStereoPanner?.();
  if (panner) {
    panner.pan.value = dir === 'forward' ? 0.3 : -0.3;
    node.connect(panner).connect(c.destination);
    return panner;
  }
  node.connect(c.destination);
  return node;
}

// ── Variant generators ──────────────────────────────────────────────────────

// Thin leaf: a quick, bright flick — high, short, a faint second tick.
function thin(c: AudioContext, dir: Direction, t: number) {
  const s = source(c, 1.4);
  const hp = filter(c, 'highpass', 2200);
  const bp = filter(c, 'bandpass', 3600, 1.1);
  const g = c.createGain();
  g.gain.setValueAtTime(0.0001, t);
  g.gain.linearRampToValueAtTime(0.12, t + 0.01);
  g.gain.exponentialRampToValueAtTime(0.02, t + 0.1);
  g.gain.linearRampToValueAtTime(0.05, t + 0.14);
  g.gain.exponentialRampToValueAtTime(0.0001, t + 0.24);
  s.connect(hp).connect(bp).connect(g);
  toOutput(c, g, dir);
  s.start(t);
  s.stop(t + 0.3);
}

// Heavy tome: a low body "whump" under a slower rustle.
function tome(c: AudioContext, dir: Direction, t: number) {
  // body
  const sb = source(c, 0.55);
  const lp = filter(c, 'lowpass', 420);
  const gb = c.createGain();
  gb.gain.setValueAtTime(0.0001, t);
  gb.gain.linearRampToValueAtTime(0.1, t + 0.025);
  gb.gain.exponentialRampToValueAtTime(0.0001, t + 0.3);
  sb.connect(lp).connect(gb);
  toOutput(c, gb, dir);
  sb.start(t);
  sb.stop(t + 0.4);
  // rustle
  const sr = source(c, 0.9);
  const bp = filter(c, 'bandpass', 1700, 0.8);
  const gr = c.createGain();
  gr.gain.setValueAtTime(0.0001, t + 0.02);
  gr.gain.linearRampToValueAtTime(0.07, t + 0.09);
  gr.gain.exponentialRampToValueAtTime(0.0001, t + 0.5);
  sr.connect(bp).connect(gr);
  toOutput(c, gr, dir);
  sr.start(t + 0.02);
  sr.stop(t + 0.55);
}

// Parchment: a slow, dry drag with a faint crackle (amplitude wobble).
function parchment(c: AudioContext, dir: Direction, t: number) {
  const s = source(c, 0.8);
  const bp = filter(c, 'bandpass', 2200, 0.6);
  const g = c.createGain();
  g.gain.setValueAtTime(0.0001, t);
  g.gain.linearRampToValueAtTime(0.085, t + 0.08);
  g.gain.linearRampToValueAtTime(0.05, t + 0.3);
  g.gain.linearRampToValueAtTime(0.075, t + 0.42);
  g.gain.exponentialRampToValueAtTime(0.0001, t + 0.62);
  // crackle: a fast tremolo on a small extra gain
  const lfo = c.createOscillator();
  lfo.type = 'square';
  lfo.frequency.value = 34;
  const lfoGain = c.createGain();
  lfoGain.gain.value = 0.02;
  lfo.connect(lfoGain).connect(g.gain);
  s.connect(bp).connect(g);
  toOutput(c, g, dir);
  lfo.start(t);
  lfo.stop(t + 0.62);
  s.start(t);
  s.stop(t + 0.66);
}

// Riffle: a short string of rapid taps, as if many pages flick past.
function riffle(c: AudioContext, dir: Direction, t: number) {
  const count = 6;
  for (let i = 0; i < count; i++) {
    const at = t + i * 0.045 + Math.random() * 0.01;
    const s = source(c, 1.2 + Math.random() * 0.3);
    const hp = filter(c, 'highpass', 2600);
    const g = c.createGain();
    const peak = 0.09 * (1 - i / (count + 2));
    g.gain.setValueAtTime(0.0001, at);
    g.gain.linearRampToValueAtTime(peak, at + 0.006);
    g.gain.exponentialRampToValueAtTime(0.0001, at + 0.05);
    s.connect(hp).connect(g);
    toOutput(c, g, dir);
    s.start(at);
    s.stop(at + 0.07);
  }
}

// Cloth-bound: soft, muffled, low — a gentle fabric-spine turn.
function cloth(c: AudioContext, dir: Direction, t: number) {
  const s = source(c, 0.7);
  const lp = filter(c, 'lowpass', 900);
  const g = c.createGain();
  g.gain.setValueAtTime(0.0001, t);
  g.gain.linearRampToValueAtTime(0.08, t + 0.04);
  g.gain.exponentialRampToValueAtTime(0.0001, t + 0.45);
  s.connect(lp).connect(g);
  toOutput(c, g, dir);
  s.start(t);
  s.stop(t + 0.5);
}

const generators: Record<TurnSoundId, (c: AudioContext, dir: Direction, t: number) => void> = {
  thin,
  tome,
  parchment,
  riffle,
  cloth,
};

export function playTurnSound(id: TurnSoundId, direction: Direction = 'forward') {
  const gen = generators[id];
  if (!gen) return;
  const c = audioContext();
  if (!c) return;
  // The user gesture that triggered playback can resume an auto-suspended context.
  if (c.state === 'suspended') c.resume().catch(() => {});
  gen(c, direction, c.currentTime);
}
