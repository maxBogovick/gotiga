<script lang="ts">
  import GazetteRoom from '$lib/components/GazetteRoom.svelte';

  let { data } = $props();
</script>

<GazetteRoom
  year={data.room.year}
  years={data.room.years}
  leaves={data.room.leaves}
  cuttings={data.room.cuttings}
/>
