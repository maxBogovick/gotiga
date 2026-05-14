<script lang="ts">
  import { page } from '$app/state';
  import NotFound from '$lib/components/NotFound.svelte';

  let status = $derived(page.status);
  let message = $derived(
    status === 404
      ? 'Сей экспонат утрачен или никогда не существовал в реестре.'
      : `Хранилище содержит повреждённую запись. Код: ${status}`
  );
</script>

<NotFound
  title={status === 404 ? 'Увы' : 'Ошибка'}
  {message}
/>
