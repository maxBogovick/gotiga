<script lang="ts">
  import '../app.css';
  import { fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { page } from '$app/state';

  let { children } = $props();

  // Ключ для анимации переходов
  let key = $derived(page.url.pathname);
</script>

<div class="min-h-screen bg-cabinet-bg">
  {#key key}
    <main
      class="min-h-screen"
      in:fade={{ duration: 400, delay: 200, easing: cubicOut }}
      out:fade={{ duration: 300, easing: cubicOut }}
    >
      {@render children()}
    </main>
  {/key}
</div>
