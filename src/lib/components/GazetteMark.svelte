<script lang="ts">
  /**
   * A house stamp for a world-desk source — invented, not a favicon.
   * A laid picture (markUrl) covers the glyph until the keeper takes it off.
   */
  import { resolveMediaUrl } from '$lib/api';
  import { isGazetteMarkKey, markLetter, type GazetteMarkKey } from '$lib/gazette-marks';

  let {
    markKey = 'letter',
    markUrl = null,
    letter = '',
    size = 36,
    label = '',
  }: {
    markKey?: string | null;
    markUrl?: string | null;
    letter?: string;
    size?: number;
    label?: string;
  } = $props();

  let key = $derived<GazetteMarkKey>(isGazetteMarkKey(markKey) ? markKey : 'letter');
  let picture = $derived(resolveMediaUrl(markUrl));
  let glyph = $derived(markLetter(letter));
</script>

<span
  class="gz-mark"
  style="width:{size}px;height:{size}px"
  title={label}
  aria-hidden="true"
>
  {#if picture}
    <img src={picture} alt="" />
  {:else}
    <svg viewBox="0 0 36 36" fill="none">
      <rect x="1.2" y="1.2" width="33.6" height="33.6" rx="3" fill="#f3e6d0" stroke="#d8c6b1" stroke-width="1.2"/>
      <path d="M1.2 1.2h2.4v33.6H1.2z" fill="#c65f3c" opacity="0.85"/>
      {#if key === 'pillar'}
        <path d="M13 28V12.5h10V28" stroke="#6f3b24" stroke-width="1.4"/>
        <path d="M11 12.5h14M12 10.5h12" stroke="#6f3b24" stroke-width="1.4" stroke-linecap="square"/>
        <path d="M11 28h14" stroke="#c65f3c" stroke-width="1.3"/>
      {:else if key === 'hive'}
        <circle cx="18" cy="13" r="3.1" stroke="#6f3b24" stroke-width="1.3"/>
        <circle cx="13.2" cy="20.2" r="3.1" stroke="#6f3b24" stroke-width="1.3"/>
        <circle cx="22.8" cy="20.2" r="3.1" stroke="#6f3b24" stroke-width="1.3"/>
        <circle cx="18" cy="22.4" r="2.2" stroke="#c65f3c" stroke-width="1.2"/>
      {:else if key === 'boom'}
        <path d="M18 11v14" stroke="#6f3b24" stroke-width="1.3" stroke-linecap="round"/>
        <path d="M18 14c4 2.2 6.5 5 8 9M18 14c-4 2.2-6.5 5-8 9" stroke="#6f3b24" stroke-width="1.3" stroke-linecap="round"/>
        <path d="M18 17c2.6 1.4 4.2 3.2 5.2 5.6M18 17c-2.6 1.4-4.2 3.2-5.2 5.6" stroke="#c65f3c" stroke-width="1.2" stroke-linecap="round"/>
      {:else if key === 'quill'}
        <path d="M11 25.5L24.5 11" stroke="#6f3b24" stroke-width="1.4" stroke-linecap="round"/>
        <path d="M22.5 11.5l2.4-.2-.2 2.4" stroke="#6f3b24" stroke-width="1.3" stroke-linecap="round"/>
        <path d="M12.5 24l3.2-1.4" stroke="#c65f3c" stroke-width="1.2" stroke-linecap="round"/>
      {:else if key === 'lens'}
        <circle cx="18" cy="18" r="7.2" stroke="#6f3b24" stroke-width="1.4"/>
        <circle cx="18" cy="18" r="3.2" stroke="#c65f3c" stroke-width="1.2"/>
        <path d="M23.2 23.2L27 27" stroke="#6f3b24" stroke-width="1.4" stroke-linecap="round"/>
      {:else if key === 'shard'}
        <path d="M18 9.5L27 26H9z" stroke="#6f3b24" stroke-width="1.4" stroke-linejoin="round"/>
        <path d="M18 14v9" stroke="#c65f3c" stroke-width="1.2"/>
      {:else if key === 'coil'}
        <path d="M18 12.5c4.6 0 7.2 2.6 7.2 5.6 0 4.4-4 6.6-7.2 6.6-3.8 0-6.4-2.2-6.4-5.2 0-2.2 1.8-3.6 3.8-3.6 1.8 0 3 1 3 2.5" stroke="#6f3b24" stroke-width="1.4" stroke-linecap="round"/>
        <circle cx="18.6" cy="19.2" r="1.3" fill="#c65f3c"/>
      {:else}
        <text x="18" y="23.5" text-anchor="middle" fill="#6f3b24" font-family="Georgia, serif" font-size="14">{glyph}</text>
      {/if}
    </svg>
  {/if}
</span>

<style>
  .gz-mark {
    display: inline-block;
    flex-shrink: 0;
    overflow: hidden;
    border-radius: 3px;
    line-height: 0;
  }
  .gz-mark svg,
  .gz-mark img {
    display: block;
    width: 100%;
    height: 100%;
  }
  .gz-mark img {
    object-fit: cover;
    background: #f3e6d0;
  }
</style>
