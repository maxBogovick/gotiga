import { writable } from 'svelte/store';

// true while the circle is traveling across the header
export const birdWalking = writable(false);

// reference to the yellow circle DOM element (set by RavenWatcher)
export const ravenCircleEl = writable<HTMLElement | null>(null);
