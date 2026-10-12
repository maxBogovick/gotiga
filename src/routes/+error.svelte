<script lang="ts">
  import { page } from '$app/state';
  import NotFound from '$lib/components/NotFound.svelte';
  import { t } from '$lib/i18n';
  import { onMount } from 'svelte';
  import { reportError, reportNotFound } from '$lib/analytics';

  let status = $derived(page.status);
  let title   = $derived(status === 404 ? $t('notFoundTitle') : $t('errorTitle'));
  let message = $derived(
    status === 404
      ? $t('errorNotFound')
      : `${$t('errorGeneric')} ${status}`
  );

  // A dead address is the report's "broken links" list; any other error page
  // is a failed render. Reported here, where every error route ends, rather
  // than in the client hook, which only sees the unexpected ones.
  onMount(() => {
    if (status === 404) reportNotFound();
    else reportError('render', `${status} ${page.error?.message ?? ''}`);
  });
</script>

<NotFound {title} {message} />
