<script lang="ts">
  // Уведомления о смене статуса брони: ответ автора виден на любой странице.
  import { fly } from 'svelte/transition';
  import { allClaims } from '$lib/stores/all-claims.svelte';
  import { figurineNameIn } from '$lib/workText';
  import { t, lang } from '$lib/i18n';
</script>

<!-- Уведомления о смене статуса брони. Область объявлена сразу и живёт всегда: читалка
     объявляет то, что в неё положили, а область, появившаяся вместе с текстом, молчит. -->
<div class="notif-stack" aria-live="polite" aria-atomic="false">
  {#each allClaims.notifications as notif (notif.id)}
    <div
      class="notif notif--{notif.newStatus}"
      in:fly={{ x: 60, duration: 350 }}
      out:fly={{ x: 60, duration: 250 }}
    >
      <div class="notif-body">
        <span class="notif-name">{figurineNameIn(notif, $lang)}</span>
        <span class="notif-text">
          {notif.newStatus === 'confirmed' ? $t('notifConfirmed')
          : notif.newStatus === 'rejected'  ? $t('notifRejected')
          : notif.newStatus === 'cancelled' ? $t('notifCancelled')
          : $t('notifCompleted')}
        </span>
      </div>
      <div class="notif-actions">
        {#if notif.newStatus === 'rejected'}
          <a href="/figurines/{notif.figurineId}" class="notif-link">{$t('notifTryAgain')}</a>
        {:else if notif.newStatus === 'completed'}
          <a href="/figurines/{notif.figurineId}" class="notif-link">{$t('notifViewFigurine')}</a>
        {/if}
        <button class="notif-dismiss" onclick={() => allClaims.dismissNotification(notif.id)} aria-label={$t('uiDismiss')}>✕</button>
      </div>
    </div>
  {/each}
</div>

<style>
  /* ── Notifications ───────────────────────────────────────── */
  .notif-stack {
    position: fixed;
    bottom: 20px;
    right: 20px;
    z-index: 500;
    display: flex;
    flex-direction: column;
    gap: 8px;
    pointer-events: none;
  }

  .notif {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    width: 280px;
    padding: 12px 14px;
    background: var(--color-canvas-sunken);
    border: 1px solid var(--color-border-default);
    border-left-width: 3px;
    box-shadow: var(--shadow-md);
    font-family: Georgia, serif;
    color: var(--color-ink-primary);
    pointer-events: all;
  }

  .notif--confirmed { border-left-color: #3a7a40; }
  .notif--rejected  { border-left-color: #c65f3c; }
  .notif--cancelled { border-left-color: #9a8070; }
  .notif--completed { border-left-color: #3a7060; }

  .notif-body { flex: 1; min-width: 0; }

  .notif-name {
    display: block;
    font-size: 0.9rem;
    font-weight: 500;
    color: var(--color-ink-primary);
    margin-bottom: 2px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .notif-text {
    display: block;
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 11px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--color-ink-secondary);
  }

  .notif-actions {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 4px;
    flex-shrink: 0;
  }

  .notif-link {
    font-family: 'Instrument Sans', system-ui, sans-serif;
    font-size: 11px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--color-ember-deep);
    text-decoration: none;
    white-space: nowrap;
    padding: 4px 0;
    transition: opacity 0.15s;
  }
  .notif-link:hover { opacity: 0.75; }

  .notif-dismiss {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    margin: -14px -14px -14px 0;
    background: none;
    border: none;
    cursor: pointer;
    font-size: 14px;
    color: var(--color-ink-tertiary);
    padding: 0;
    line-height: 1;
    transition: color 0.15s;
  }
  .notif-dismiss:hover { color: var(--color-ember-deep); }

  /* Уведомление стоит над нижней панелью разделов, а не под ней. */
  @media (max-width: 1024px) {
    .notif-stack { bottom: calc(var(--section-bar-h, 0px) + 12px); }
  }

  @media (max-width: 720px) {
    .notif-stack { right: 12px; bottom: calc(var(--section-bar-h, 0px) + 12px); }
    .notif { width: calc(100vw - 24px); max-width: 280px; }
  }
</style>
