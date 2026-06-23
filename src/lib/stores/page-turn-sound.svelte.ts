// Reader's choice of page-turn sound (or silence), persisted in localStorage.
// 'off' is honoured everywhere; any of the synth variants live in page-turn-sounds.

import { TURN_SOUND_IDS, type TurnSoundId } from '$lib/audio/page-turn-sounds';

export type TurnSoundPref = TurnSoundId | 'off';

const STORAGE_KEY = 'gotiga_turn_sound';
// Silence by default — this place is built on it; the reader opts in to sound.
const DEFAULT: TurnSoundPref = 'off';

function isValid(v: string | null): v is TurnSoundPref {
  return v === 'off' || (v !== null && (TURN_SOUND_IDS as string[]).includes(v));
}

let pref = $state<TurnSoundPref>(DEFAULT);
let loaded = false;

export const turnSound = {
  /** Reactive: the chosen variant id, or 'off'. */
  get value(): TurnSoundPref {
    return pref;
  },
  set(next: TurnSoundPref) {
    pref = next;
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {}
  },
  /** Read the saved preference once (call from a client lifecycle hook). */
  load() {
    if (loaded) return;
    loaded = true;
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (isValid(saved)) pref = saved;
    } catch {}
  },
};
