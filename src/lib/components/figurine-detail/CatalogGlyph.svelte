<script lang="ts">
  let { name }: { name: string } = $props();
</script>

<svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  {#if name === 'star'}
    <path d="M12 3.2l1.85 5.2h5.45l-4.4 3.25 1.7 5.15L12 13.7 7.4 16.8l1.7-5.15-4.4-3.25h5.45L12 3.2z" />
  {:else if name === 'face'}
    <circle cx="12" cy="12" r="8.2" />
    <path d="M9 10.2h.01M15 10.2h.01M8.8 14.2c.9 1.4 2.1 2.1 3.2 2.1s2.3-.7 3.2-2.1" />
  {:else if name === 'pencil'}
    <path d="M14.2 4.6l5.2 5.2-10.1 10.1H4.1v-5.2L14.2 4.6z" />
    <path d="M12.4 6.4l5.2 5.2" />
  {:else if name === 'needle'}
    <path d="M14.5 4.2c2.2 2.2 2.4 5.4.4 7.4L7.4 19.1c-.8.8-2.1.8-2.9 0s-.8-2.1 0-2.9l7.5-7.5c2-2 5.2-1.8 7.4.4" />
    <path d="M9.2 12.8l2.2 2.2" />
    <path d="M16.8 6.2l1.6-1.6" />
  {:else if name === 'lock'}
    <rect x="6.2" y="10.5" width="11.6" height="8.3" rx="1.4" />
    <path d="M8.6 10.5V8.2a3.4 3.4 0 0 1 6.8 0v2.3" />
  {:else if name === 'figure'}
    <circle cx="12" cy="5.2" r="2.1" />
    <path d="M12 8.2v5.6M8.2 11.2h7.6M9.2 20.2L12 13.8l2.8 6.4" />
  {:else if name === 'gift'}
    <rect x="4.2" y="10.2" width="15.6" height="9.6" rx="0.8" />
    <path d="M12 10.2v9.6M4.2 14.4h15.6" />
    <path d="M12 10.2c-1.8-3.4-5.4-3.6-5.4-1.4S10.2 11 12 10.2c1.8-3.4 5.4-3.6 5.4-1.4S13.8 11 12 10.2z" />
  {:else if name === 'heart'}
    <path d="M12 19.4S4.4 14.4 4.4 9.2A3.9 3.9 0 0 1 12 7.1a3.9 3.9 0 0 1 7.6 2.1C19.6 14.4 12 19.4 12 19.4z" />
  {:else if name === 'house'}
    <path d="M4.2 11.4L12 4.4l7.8 7" />
    <path d="M6.4 10.2V19.6h11.2V10.2" />
    <path d="M10.2 19.6v-5.4h3.6v5.4" />
  {:else if name === 'spool'}
    <rect x="8.2" y="4.2" width="7.6" height="3.2" rx="0.6" />
    <rect x="8.2" y="16.6" width="7.6" height="3.2" rx="0.6" />
    <path d="M9.4 7.4v9.2M14.6 7.4v9.2" />
    <path d="M9.4 10.4h8.2M9.4 13.6h8.2" />
  {:else if name === 'scissors'}
    <circle cx="7.2" cy="7.2" r="2.4" />
    <circle cx="7.2" cy="16.8" r="2.4" />
    <path d="M9.2 8.6L20 19.2M9.2 15.4L20 4.8" />
  {:else if name === 'frame'}
    <rect x="4.6" y="5.4" width="14.8" height="13.2" rx="0.6" />
    <rect x="7.2" y="8" width="9.6" height="8" rx="0.3" />
  {:else if name === 'quill'}
    <path d="M5.2 19.2c4.6-1.2 8.4-5 10.6-9.2 1.4-2.6 1.2-5.4-.6-6.4-1.8-1-4.4.2-6.2 2.6C6.8 10.4 5.4 14.8 5.2 19.2z" />
    <path d="M8.4 15.6c1.6-1.8 3.6-3.2 5.6-4.2" />
  {:else if name === 'seal'}
    <circle cx="12" cy="11.2" r="6.4" />
    <circle cx="12" cy="11.2" r="3.4" />
    <path d="M9.4 16.8l-1.2 4.2 3.8-1.6 3.8 1.6-1.2-4.2" />
  {:else if name === 'crate'}
    <path d="M4.4 8.2h15.2v11.4H4.4z" />
    <path d="M4.4 8.2L12 4.4l7.6 3.8" />
    <path d="M12 4.4v15.2M4.4 12.6h15.2" />
  {:else}
    <circle cx="12" cy="12" r="3" />
  {/if}
</svg>
