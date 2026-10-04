<script lang="ts">
  import { t } from '$lib/i18n';

  /**
   * Листок «куда написать» — один на все просьбы после байки: следующая
   * небылица, продолжение, выбранная читателями. Три листка с одним смыслом,
   * написанные порознь, разошлись бы на первой правке.
   *
   * Возраст спрашивается здесь же, потому что без него дом адрес не берёт —
   * то же правило, что у книги дома и у слежки за эскизом.
   */
  let {
    id,
    submitLabel,
    busy = false,
    error = '',
    onsubmit,
  }: {
    id: string;
    submitLabel: string;
    busy?: boolean;
    error?: string;
    onsubmit: (email: string, ageConfirmed: boolean) => void;
  } = $props();

  let email = $state('');
  let age = $state(false);
</script>

<form
  class="slip"
  onsubmit={(e) => {
    e.preventDefault();
    onsubmit(email.trim(), age);
  }}
>
  <label class="sr" for="{id}-email">{$t('talesLetterEmail')}</label>
  <div class="row">
    <input
      id="{id}-email"
      name="email"
      type="email"
      autocomplete="email"
      required
      placeholder={$t('talesLetterEmail')}
      bind:value={email}
      disabled={busy}
    />
    <button type="submit" class="send" disabled={busy}>{busy ? '…' : submitLabel}</button>
  </div>
  <label class="age">
    <input type="checkbox" name="age-confirm" bind:checked={age} disabled={busy} />
    {$t('formAgeConfirm')}
  </label>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
</form>

<style>
  .slip {
    display: grid;
    gap: 8px;
    max-width: 26rem;
    margin-top: 12px;
  }
  .row {
    display: flex;
    gap: 8px;
  }
  input[type='email'] {
    flex: 1;
    min-width: 0;
    background: #fff9f0;
    border: 1px solid rgba(52, 37, 28, 0.18);
    padding: 0.55rem 0.75rem;
    color: var(--ink, #34251c);
    font: inherit;
    font-size: 14px;
    transition: border-color 0.2s;
  }
  input[type='email']:focus {
    outline: none;
    border-color: rgba(198, 95, 60, 0.6);
  }
  .send {
    flex-shrink: 0;
    padding: 0 14px;
    background: var(--brown, #34251c);
    border: 1px solid var(--brown, #34251c);
    color: #f8f1e7;
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    cursor: pointer;
    transition: background 0.2s, border-color 0.2s;
  }
  .send:hover { background: var(--deep, #6f3b24); border-color: var(--deep, #6f3b24); }
  .send:disabled { opacity: 0.55; cursor: default; }
  .age {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    font-size: 12px;
    line-height: 1.4;
    color: var(--muted2, #5f4636);
  }
  .age input { margin-top: 2px; accent-color: var(--copper, #c65f3c); }
  .err {
    margin: 0;
    font-size: 12px;
    color: #8a2a2a;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
  }
</style>
