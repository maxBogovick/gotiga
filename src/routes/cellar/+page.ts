// The cellar is written, not fetched — no API call, no `load` data. It is the one
// room that looks the same for every visitor, so it prerenders to flat HTML.
//
// The flag mirrors every other public route on purpose: the root layout turns SSR
// off for the SPA/Tauri profile, and a child cannot prerender under a parent with
// SSR disabled. Same switch, same profile, no fight.
export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';
