<script lang="ts">
  import '../app.css';
  import { fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { page } from '$app/state';

  let { children } = $props();

  // Key for page transitions
  let key = $derived(page.url.pathname);
</script>

<div class="min-h-screen bg-[#0a0806]">
  {#key key}
    <main
      class="min-h-screen"
      in:fade={{ duration: 600, delay: 300, easing: cubicOut }}
      out:fade={{ duration: 400, easing: cubicOut }}
    >
      {@render children()}
    </main>
  {/key}
</div>
