<script lang="ts">
  import { t, lang, brandName } from '$lib/i18n';
  import { getIconById, iconLabel, type IconCategory } from '$lib/data/visualIcons';
  import { downloadKeyCard } from '$lib/utils/keyCard';

  interface Props {
    finalSelections: string[]; // 4 icon IDs in category order
    userName: string;
    onContinue: () => void;
  }

  let { finalSelections, userName, onContinue }: Props = $props();

  const CATEGORY_IDS: IconCategory[] = ['animals', 'dishes', 'seasons', 'symbols'];

  function getIcon(i: number) {
    return getIconById(CATEGORY_IDS[i], finalSelections[i]);
  }

  let downloading = $state(false);

  async function downloadCard() {
    downloading = true;
    try {
      await downloadKeyCard(finalSelections, userName, $lang);
    } finally {
      downloading = false;
    }
  }
</script>

<div class="success">
  <div class="seal">✦</div>

  <h1 class="title">{$t('authSuccessTitle')}</h1>
  <p class="text">{$t('authSuccessText')}</p>

  <div class="reminder-section">
    <p class="reminder-label">{$t('authSaveReminder')}</p>
    <p class="reminder-desc">{$t('authSaveReminderDesc')}</p>

    <div class="card">
      <p class="card-name">{userName}</p>
      <div class="card-icons">
        {#each CATEGORY_IDS as _catId, i}
          {@const icon = getIcon(i)}
          {#if icon}
            <div class="card-icon">
              {@html icon.svg}
              <span>{iconLabel(icon, $lang)}</span>
            </div>
          {/if}
        {/each}
      </div>
      <p class="card-brand">{$brandName.toUpperCase()}</p>
    </div>

    <div class="card-actions">
      <button class="btn-download" onclick={downloadCard} disabled={downloading}>
        {downloading ? '…' : `↓ ${$t('authDownloadCard')}`}
      </button>
      <span class="saved-note">✓ {$t('authSavedLocally')}</span>
    </div>
  </div>

  <button class="btn-primary" onclick={onContinue}>
    {$t('authGoToArchive')}
  </button>
</div>

<style>
  .success {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 0.5rem;
  }

  .seal {
    font-size: 2.5rem;
    color: #c65f3c;
    line-height: 1;
    margin-bottom: 0.5rem;
    animation: seal-drop 0.6s ease-out;
  }

  @keyframes seal-drop {
    from { transform: scale(0) rotate(-30deg); opacity: 0; }
    to   { transform: scale(1) rotate(0deg);   opacity: 1; }
  }

  .title {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1.6rem;
    font-weight: 400;
    color: #34251c;
    margin: 0;
  }

  .text {
    font-size: 0.9rem;
    color: #6f3b24;
    font-style: italic;
    margin: 0 0 1.5rem;
  }

  .reminder-section {
    width: 100%;
    background: #f4ead8;
    border: 1px solid #d8c6b1;
    padding: 1.25rem;
    margin-bottom: 1.5rem;
  }

  .reminder-label {
    font-size: 0.78rem;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: #6f3b24;
    font-family: Inter, sans-serif;
    margin: 0 0 0.25rem;
  }

  .reminder-desc {
    font-size: 0.82rem;
    color: #9a7c5c;
    margin: 0 0 1rem;
    font-family: Inter, sans-serif;
  }

  .card {
    background: #fdf8f2;
    border: 1px solid #d8c6b1;
    outline: 3px solid #f4ead8;
    outline-offset: -5px;
    padding: 1.25rem;
    text-align: center;
    margin-bottom: 0.75rem;
  }

  .card-name {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 1rem;
    color: #34251c;
    margin: 0 0 1rem;
    letter-spacing: 0.08em;
  }

  .card-icons {
    display: flex;
    justify-content: center;
    gap: 1.25rem;
    margin-bottom: 1rem;
  }

  .card-icon {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.3rem;
    color: #6f3b24;
  }

  .card-icon :global(svg) { width: 36px; height: 36px; }

  .card-icon span {
    font-size: 0.6rem;
    font-family: Inter, sans-serif;
    color: #9a7c5c;
    letter-spacing: 0.04em;
  }

  .card-brand {
    font-family: 'Fraunces', Georgia, serif;
    font-size: 0.65rem;
    letter-spacing: 0.3em;
    color: #d8c6b1;
    margin: 0;
    text-transform: uppercase;
  }

  .card-actions {
    display: flex;
    align-items: center;
    gap: 1rem;
    justify-content: center;
  }

  .btn-download {
    background: transparent;
    border: 1px solid #d8c6b1;
    color: #6f3b24;
    padding: 0.4rem 0.9rem;
    font-size: 0.8rem;
    font-family: Inter, sans-serif;
    cursor: pointer;
    letter-spacing: 0.04em;
    transition: border-color 0.2s;
  }
  .btn-download:hover:not(:disabled) { border-color: #6f3b24; }
  .btn-download:disabled { opacity: 0.5; cursor: not-allowed; }

  .saved-note {
    font-size: 0.75rem;
    color: #9a7c5c;
    font-family: Inter, sans-serif;
  }

  .btn-primary {
    background: #34251c;
    color: #f8f1e7;
    border: none;
    padding: 0.7rem 2rem;
    font-family: Georgia, serif;
    font-size: 0.9rem;
    cursor: pointer;
    letter-spacing: 0.08em;
    transition: background 0.2s;
    width: 100%;
  }
  .btn-primary:hover { background: #6f3b24; }
</style>
