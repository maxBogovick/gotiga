<script lang="ts">
  import { page } from '$app/state';
  import NotFound from '$lib/components/NotFound.svelte';
  import { t } from '$lib/i18n';

  let status = $derived(page.status);
  let title   = $derived(status === 404 ? $t('notFoundTitle') : $t('errorTitle'));
  let message = $derived(
    status === 404
      ? $t('errorNotFound')
      : `${$t('errorGeneric')} ${status}`
  );
</script>

<NotFound {title} {message} />
