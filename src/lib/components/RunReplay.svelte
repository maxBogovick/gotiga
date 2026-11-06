<script lang="ts">
  // Пересмотр сыгранной волны по действиям: та же сцена, что в бою, только без
  // ходов игрока. Сервер отдаёт доску на любой ступени журнала (`/battles/matches/:id/replay`),
  // страница лишь листает ступени.
  import { onDestroy } from 'svelte';
  import { t } from '$lib/i18n';
  import { api } from '$lib/api';
  import BattleScene from '$lib/components/BattleScene.svelte';
  import type { BattleCard, BattleFrame, BattleMatch, MatchReplay, Motion } from '$lib/types/api';

  let {
    matchId,
    token,
    title,
    cards,
    frames,
    motions,
    onclose,
  }: {
    matchId: string;
    token: string;
    title: string;
    cards: BattleCard[];
    frames: BattleFrame[];
    motions: Motion[];
    onclose: () => void;
  } = $props();

  let replay = $state<MatchReplay | null>(null);
  let busy = $state(false);
  let failed = $state(false);
  let playing = $state(false);
  let timer: ReturnType<typeof setInterval> | null = null;

  async function stepTo(upto: number) {
    if (busy) return;
    busy = true;
    try {
      replay = await api.replayBattleMatch(token, matchId, Math.max(0, upto));
      failed = false;
    } catch {
      failed = true;
      stop();
    } finally {
      busy = false;
    }
  }

  function stop() {
    playing = false;
    if (timer) clearInterval(timer);
    timer = null;
  }
  function play() {
    if (!replay) return;
    if (replay.upto >= replay.total) void stepTo(0);
    playing = true;
    timer = setInterval(() => {
      if (!replay || replay.upto >= replay.total) return stop();
      void stepTo(replay.upto + 1);
    }, 1400);
  }

  $effect(() => {
    matchId;
    void stepTo(0);
    return stop;
  });
  onDestroy(stop);

  /** Партия притворяется живой, чтобы доску рисовал тот же компонент. */
  const shown = $derived<BattleMatch | null>(
    replay
      ? {
          id: 'replay',
          challengeId: null,
          seq: 0,
          state: replay.state,
          legalActions: [],
          events: replay.events,
          outcome: replay.outcome,
          rewardDust: 0,
        }
      : null,
  );

  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    return { destroy: () => node.remove() };
  }
  function focusClose(node: HTMLElement) {
    node.focus();
  }
  const f = (key: string, vars: Record<string, string | number>) =>
    Object.entries(vars).reduce((s, [k, v]) => s.replace(`{${k}}`, String(v)), $t(key as never));
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div use:portal class="rp" role="dialog" aria-modal="true" aria-label={title}>
  <header class="rp-top">
    <h2>{title}</h2>
    <button type="button" class="rp-x" aria-label={$t('runClose')} use:focusClose onclick={onclose}>×</button>
  </header>

  <div class="rp-body">
    {#if failed}
      <p class="rp-note" role="alert">{$t('runReplayFail')}</p>
    {:else if shown}
      <div class="rp-scene">
        <BattleScene match={shown} {cards} {frames} {motions} {busy} control="player" onact={() => {}} />
      </div>
    {:else}
      <p class="rp-note">…</p>
    {/if}
  </div>

  {#if replay}
    <footer class="rp-bar">
      {#if replay.diverged}<p class="rp-warn">{$t('runReplayDiverged')}</p>{/if}
      <div class="rp-controls">
        <button type="button" disabled={busy || replay.upto === 0} aria-label={$t('runReplayFirst')} onclick={() => (stop(), stepTo(0))}>⏮</button>
        <button type="button" disabled={busy || replay.upto === 0} aria-label={$t('runReplayBack')} onclick={() => (stop(), stepTo(replay!.upto - 1))}>‹</button>
        {#if playing}
          <button type="button" class="rp-main" aria-label={$t('runReplayPause')} onclick={stop}>⏸</button>
        {:else}
          <button type="button" class="rp-main" aria-label={$t('runReplayPlay')} onclick={play}>▶</button>
        {/if}
        <button type="button" disabled={busy || replay.upto >= replay.total} aria-label={$t('runReplayForward')} onclick={() => (stop(), stepTo(replay!.upto + 1))}>›</button>
        <button type="button" disabled={busy || replay.upto >= replay.total} aria-label={$t('runReplayLast')} onclick={() => (stop(), stepTo(replay!.total))}>⏭</button>
      </div>
      <input
        class="rp-range"
        type="range"
        min="0"
        max={replay.total}
        value={replay.upto}
        aria-label={f('runReplayStep', { n: replay.upto, total: replay.total })}
        onchange={(e) => (stop(), stepTo(Number((e.currentTarget as HTMLInputElement).value)))}
      />
      <p class="rp-count">{f('runReplayStep', { n: replay.upto, total: replay.total })}</p>
    </footer>
  {/if}
</div>

<style>
  .rp {
    position: fixed;
    inset: 0;
    z-index: 90;
    display: flex;
    flex-direction: column;
    background: #f8f1e7;
    color: #34251c;
  }
  .rp-top {
    display: flex;
    flex: none;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.5rem 0.9rem;
    border-bottom: 1px solid #d8c6b1;
  }
  .rp-top h2 {
    margin: 0;
    font-family: Fraunces, Georgia, serif;
    font-size: 1.1rem;
  }
  .rp-x {
    width: 2.4rem;
    height: 2.4rem;
    border: 1px solid #d8c6b1;
    border-radius: 50%;
    background: #fcf7ee;
    color: #6f3b24;
    font-size: 1.4rem;
    line-height: 1;
    cursor: pointer;
  }
  .rp-body {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 0.5rem;
  }
  .rp-scene {
    max-width: 56rem;
    margin: 0 auto;
  }
  .rp-note {
    margin: 2rem 1rem;
    text-align: center;
  }
  .rp-bar {
    flex: none;
    display: grid;
    gap: 0.4rem;
    justify-items: center;
    padding: 0.6rem 0.9rem calc(0.8rem + env(safe-area-inset-bottom, 0px));
    border-top: 1px solid #d8c6b1;
    background: #fcf7ee;
  }
  .rp-controls {
    display: flex;
    gap: 0.4rem;
  }
  .rp-controls button {
    min-width: 2.8rem;
    min-height: 2.8rem;
    border: 1px solid #d8c6b1;
    background: #f8f1e7;
    color: #6f3b24;
    font-size: 1.1rem;
    cursor: pointer;
  }
  .rp-controls .rp-main {
    min-width: 4rem;
    border-color: #c65f3c;
    background: #c65f3c;
    color: #fff7ec;
  }
  .rp-controls button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .rp-range {
    width: min(32rem, 100%);
    accent-color: #c65f3c;
  }
  .rp-count {
    margin: 0;
    font-size: 0.85rem;
    color: #6f3b24;
  }
  .rp-warn {
    margin: 0;
    font-size: 0.85rem;
    color: #8f2f22;
  }
</style>
