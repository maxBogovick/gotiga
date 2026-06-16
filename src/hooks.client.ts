import type { HandleClientError } from '@sveltejs/kit';

// Web build runs SPA (ssr=false), so only the client hook fires. This catches
// unexpected errors thrown during navigation/rendering that load()'s own try/catch
// didn't handle, logs them with context, and hands a safe shape to +error.svelte
// (exposed as page.error). It never throws itself.
export const handleError: HandleClientError = ({ error, event, status, message }) => {
  try {
    console.error('[gotiga] client error', {
      status,
      message,
      path: event?.url?.pathname,
      error,
    });
  } catch {
    // logging must never mask the original error
  }

  return { message: message ?? 'Unexpected error' };
};
