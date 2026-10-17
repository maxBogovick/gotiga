<script lang="ts">
  import { page } from '$app/state';
  import { api } from '$lib/api';
  import ConfirmLetter from '$lib/components/ConfirmLetter.svelte';
</script>

<ConfirmLetter token={page.params.token ?? ''} kind="book" confirm={(t) => api.confirmSubscription(t)} />
