<script lang="ts">
  // Один на шапку. Слушает ⌘K / Ctrl+K и подгружает окно при первом открытии:
  // код ответов и поиска нужен только тому, кто спросил.
  import { onMount } from 'svelte';
  import { siteAskUi } from '$lib/stores/site-ask-ui.svelte';

  onMount(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        siteAskUi.toggle('key');
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });
</script>

{#if siteAskUi.mounted}
  {#await import('$lib/components/SiteAskDialog.svelte') then { default: SiteAskDialog }}
    <SiteAskDialog />
  {/await}
{/if}
