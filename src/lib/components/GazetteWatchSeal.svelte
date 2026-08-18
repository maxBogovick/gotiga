<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import { t, lang } from '$lib/i18n';
  import { authStore } from '$lib/stores/auth.svelte';
  import { visitorBook } from '$lib/stores/visitor-book.svelte';
  import { workHref, sketchLaidOut } from '$lib/gazette';
  import type { GazetteLeaf } from '$lib/types/api';

  let {
    leaf,
    compact = false,
    showCopy = false,
  }: {
    leaf: GazetteLeaf;
    compact?: boolean;
    showCopy?: boolean;
  } = $props();

  const KEY = $derived(`gotiga_gazette_watch_${leaf.id}`);

  let open = $state(false);
  let email = $state('');
  let name = $state('');
  let ageConfirmed = $state(false);
  let busy = $state(false);
  let watching = $state(false);
  let error = $state('');
  let copied = $state(false);

  let laid = $derived(sketchLaidOut(leaf));
  let work = $derived(workHref(leaf, compact ? 'gazette' : 'gazette_leaf'));
  let known = $derived(!!(authStore.user?.email || visitorBook.signed));

  function readToken(): string | null {
    try {
      return localStorage.getItem(KEY);
    } catch {
      return null;
    }
  }
  function writeToken(token: string) {
    try {
      localStorage.setItem(KEY, token);
    } catch {
      /* ignore */
    }
  }
  function clearToken() {
    try {
      localStorage.removeItem(KEY);
    } catch {
      /* ignore */
    }
  }

  async function load() {
    visitorBook.load();
    const token = readToken();
    if (!token) return;
    try {
      const info = await api.getGazetteWatchByToken(token);
      watching = !!info;
      if (!info) clearToken();
    } catch {
      /* keep local mark */
      watching = true;
    }
  }

  onMount(() => {
    void load();
  });

  async function leaveName() {
    if (busy) return;
    error = '';
    const sessionEmail = authStore.user?.email?.trim() ?? '';
    const bookEmail = visitorBook.email.trim();
    const addr = (email.trim() || sessionEmail || bookEmail).trim();
    if (!addr) {
      if (known) {
        /* still need a stored email */
      } else {
        error = $t('gazetteWatchNeedEmail');
        open = true;
        return;
      }
    }
    if (!known && !ageConfirmed) {
      error = $t('formAgeConfirmRequired');
      open = true;
      return;
    }
    if (!addr) {
      error = $t('gazetteWatchNeedEmail');
      return;
    }
    busy = true;
    try {
      const res = await api.watchGazetteLeaf(
        leaf.slug,
        {
          email: addr,
          name: name.trim() || authStore.user?.displayName || visitorBook.name || null,
          lang: $lang,
          ageConfirmed: known ? true : ageConfirmed,
        },
        authStore.token,
      );
      writeToken(res.cancelToken);
      watching = true;
      open = false;
    } catch (e) {
      error = e instanceof Error ? e.message : $t('gazetteWatchError');
    } finally {
      busy = false;
    }
  }

  async function release() {
    const token = readToken();
    if (!token || busy) return;
    busy = true;
    try {
      await api.leaveGazetteWatchByToken(token);
      clearToken();
      watching = false;
    } catch {
      error = $t('gazetteWatchError');
    } finally {
      busy = false;
    }
  }

  async function copyAddress() {
    const url = `${window.location.origin}/gazette/${leaf.slug}`;
    try {
      await navigator.clipboard.writeText(url);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch {
      /* ignore */
    }
  }

  function onLeaveClick() {
    if (laid) return;
    if (watching) return;
    if (known) {
      void leaveName();
      return;
    }
    open = !open;
  }
</script>

{#if leaf.kind === 'sketch'}
<div class="seal" class:compact>
  {#if laid}
    {#if work}
      <a class="act laid" href={work}>{$t('gazetteWatchLaidOut')} →</a>
    {:else}
      <span class="act laid">{$t('gazetteWatchAlreadyTold')}</span>
    {/if}
  {:else}
    {#if watching}
      <p class="kept">
        <span class="disc" aria-hidden="true"></span>
        {$t('gazetteWatchLeft')}
        <button type="button" class="text" onclick={release} disabled={busy}>
          {$t('gazetteWatchRelease')}
        </button>
      </p>
    {:else}
      <button type="button" class="act" onclick={onLeaveClick} disabled={busy}>
        <span class="disc" aria-hidden="true"></span>
        {$t('gazetteWatchLeaveName')}
      </button>
    {/if}

    {#if open && !watching}
      <form
        class="slip"
        onsubmit={(e) => {
          e.preventDefault();
          void leaveName();
        }}
      >
        <p class="hint">{$t('gazetteWatchHint')}</p>
        <label class="sr" for="gz-watch-email-{leaf.id}">{$t('gazetteWatchEmail')}</label>
        <input
          id="gz-watch-email-{leaf.id}"
          type="email"
          autocomplete="email"
          placeholder={$t('gazetteWatchEmail')}
          bind:value={email}
        />
        <label class="sr" for="gz-watch-name-{leaf.id}">{$t('gazetteWatchName')}</label>
        <input
          id="gz-watch-name-{leaf.id}"
          type="text"
          autocomplete="name"
          placeholder={$t('gazetteWatchName')}
          bind:value={name}
        />
        <label class="age">
          <input type="checkbox" bind:checked={ageConfirmed} />
          {$t('formAgeConfirm')}
        </label>
        {#if error}<p class="err">{error}</p>{/if}
        <button type="submit" class="set" disabled={busy}>{$t('gazetteWatchSubmit')}</button>
      </form>
    {/if}
  {/if}

  {#if work && !laid && !compact}
    <a class="quiet" href={work}>{$t('gazetteWatchToBench')} →</a>
  {/if}
  {#if showCopy}
    <button type="button" class="quiet text" onclick={copyAddress}>
      {copied ? $t('gazetteWatchCopied') : $t('gazetteWatchCopy')}
    </button>
  {/if}
  {#if error && !open}<p class="err">{error}</p>{/if}
</div>
{/if}

<style>
  .seal {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 14px;
    margin-top: 8px;
  }
  .seal.compact { margin-top: 4px; }
  .disc {
    display: inline-block;
    width: 11px;
    height: 11px;
    margin-right: 6px;
    border-radius: 50%;
    background: radial-gradient(circle at 35% 30%, #d97a58, #6f3b24 72%);
    box-shadow: 0 0 0 1px #d8c6b1;
    vertical-align: -1px;
  }
  .act, .quiet, .text, .set {
    font: inherit;
    cursor: pointer;
    background: none;
    border: none;
    padding: 0;
    color: #6f3b24;
    text-decoration: none;
  }
  .act {
    font-size: 12px;
    letter-spacing: 0.04em;
    display: inline-flex;
    align-items: center;
  }
  .act.laid { border-bottom: 1px solid #d8c6b1; padding-bottom: 1px; }
  .quiet {
    font-size: 11px;
    letter-spacing: 0.04em;
    color: #8a6a55;
  }
  .kept {
    margin: 0;
    font-size: 12px;
    color: #5f4636;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .text { font-size: 11px; text-decoration: underline; text-underline-offset: 3px; }
  .slip {
    flex-basis: 100%;
    display: grid;
    gap: 8px;
    max-width: 320px;
    padding: 12px 14px;
    border: 1px solid #d8c6b1;
    background: #fff9f0;
    transform: rotate(-0.6deg);
  }
  .hint { margin: 0; font-size: 13px; color: #5f4636; line-height: 1.4; }
  input {
    width: 100%;
    background: #f8f1e7;
    border: 1px solid rgba(198, 95, 60, 0.25);
    padding: 0.55rem 0.7rem;
    color: #34251c;
    font: inherit;
    font-size: 13px;
  }
  .age {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    font-size: 12px;
    color: #5f4636;
  }
  .age input { width: auto; margin-top: 2px; }
  .set {
    justify-self: start;
    background: #6f3b24;
    color: #f8f1e7;
    padding: 6px 12px;
    border: 1px solid #6f3b24;
    font-size: 12px;
    letter-spacing: 0.04em;
  }
  .err { margin: 0; font-size: 12px; color: #8a2a2a; }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
  }
</style>
