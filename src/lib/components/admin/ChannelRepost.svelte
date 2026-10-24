<script lang="ts">
  /**
   * «Опубликовать повторно в Telegram»: выпускает работу, байку или карту во
   * все каналы сразу, мимо журнала объявлений. Текст берётся из сохранённого,
   * на языке каждого канала — как у обычного объявления, поэтому несохранённая
   * правка в пост не попадёт (подсказка у кнопки это говорит).
   */
  import { api } from '$lib/api';
  import { t } from '$lib/i18n';
  import type { ChannelRepostResult } from '$lib/types/api';

  let {
    kind,
    id,
    disabled = false,
    class: extra = '',
  }: {
    kind: 'work' | 'leaf' | 'card';
    id: string;
    disabled?: boolean;
    class?: string;
  } = $props();

  let busy = $state(false);
  let outcome = $state<{ ok: boolean; text: string } | null>(null);

  // Исход относился к прежней вещи: открыли другую — старая строка не нужна.
  $effect(() => {
    void id;
    outcome = null;
  });

  function summarize(r: ChannelRepostResult): { ok: boolean; text: string } {
    const langs = (status: string) =>
      r.results
        .filter((x) => x.status === status)
        .map((x) => x.lang.toUpperCase())
        .join(', ');
    const parts: string[] = [];
    const sent = langs('sent');
    const local = langs('local');
    const hidden = langs('not_on_display');
    const failed = r.results.filter((x) => x.status === 'failed');
    if (sent) parts.push(`${$t('adminRepostSent')}: ${sent}`);
    if (local) parts.push(`${$t('adminRepostLocal')}: ${local}`);
    if (hidden) parts.push(`${$t('adminRepostHidden')}: ${hidden}`);
    for (const f of failed) parts.push(`${$t('adminRepostFailed')}: ${f.lang.toUpperCase()} — ${f.error ?? ''}`);
    return { ok: failed.length === 0 && !hidden, text: parts.join(' · ') };
  }

  async function run() {
    if (busy || !confirm($t('adminRepostConfirm'))) return;
    busy = true;
    outcome = null;
    try {
      outcome = summarize(await api.adminChannelRepost(kind, id));
    } catch (e) {
      outcome = { ok: false, text: String(e) };
    } finally {
      busy = false;
    }
  }
</script>

<span class="repost {extra}">
  <button
    type="button"
    class="repost-btn"
    onclick={run}
    disabled={disabled || busy}
    title={$t('adminRepostHint')}
  >{busy ? $t('adminRepostBusy') : $t('adminRepostButton')}</button>
  {#if outcome}
    <span class="repost-out" class:bad={!outcome.ok} role="status">{outcome.text}</span>
  {/if}
</span>

<style>
  .repost {
    display: inline-flex;
    align-items: center;
    gap: 0.6rem;
    max-width: 100%;
  }
  .repost-btn {
    padding: 0.5rem 0.9rem;
    font-size: 10px;
    line-height: 1;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    white-space: nowrap;
    flex-shrink: 0;
    color: #34251c;
    background: transparent;
    border: 1px solid rgba(52, 37, 28, 0.25);
    cursor: pointer;
  }
  .repost-btn:hover:not(:disabled) {
    background: rgba(52, 37, 28, 0.06);
  }
  .repost-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .repost-out {
    font-size: 11px;
    line-height: 1.3;
    color: #2f5d3a;
  }
  .repost-out.bad {
    color: #8f2f22;
  }
</style>
