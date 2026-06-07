<script lang="ts">
  import { t, lang } from '$lib/i18n';
  import { getIconById, iconLabel, svgForCanvas, type IconCategory } from '$lib/data/visualIcons';

  interface Props {
    finalSelections: string[]; // 4 icon IDs in category order
    userName: string;
    onContinue: () => void;
  }

  let { finalSelections, userName, onContinue }: Props = $props();

  const CATEGORY_IDS: IconCategory[] = ['animals', 'dishes', 'seasons', 'colors'];

  function getIcon(i: number) {
    return getIconById(CATEGORY_IDS[i], finalSelections[i]);
  }

  function loadSvgAsImage(svg: string, size: number): Promise<HTMLImageElement | null> {
    return new Promise((resolve) => {
      const prepared = svgForCanvas(svg, size);
      const dataUrl = 'data:image/svg+xml;charset=utf-8,' + encodeURIComponent(prepared);
      const img = new Image();
      img.onload = () => resolve(img);
      img.onerror = () => resolve(null);
      img.src = dataUrl;
    });
  }

  let downloading = $state(false);

  async function downloadCard() {
    downloading = true;
    try {
      const icons = CATEGORY_IDS.map((_, i) => getIcon(i));
      const ICON_SIZE = 48;

      // Load all SVGs as Images first so canvas drawing is synchronous
      const images = await Promise.all(
        icons.map(icon => icon ? loadSvgAsImage(icon.svg, ICON_SIZE) : Promise.resolve(null))
      );

      const scale = 2;
      const W = 400, H = 210;
      const canvas = document.createElement('canvas');
      canvas.width = W * scale;
      canvas.height = H * scale;
      const ctx = canvas.getContext('2d')!;
      ctx.scale(scale, scale);

      // Background + double border
      ctx.fillStyle = '#fdf8f2';
      ctx.fillRect(0, 0, W, H);
      ctx.strokeStyle = '#d8c6b1';
      ctx.lineWidth = 1;
      ctx.strokeRect(1, 1, W - 2, H - 2);
      ctx.strokeRect(5, 5, W - 10, H - 10);

      // Name
      ctx.fillStyle = '#34251c';
      ctx.font = '15px Georgia, serif';
      ctx.textAlign = 'center';
      ctx.fillText(userName, W / 2, 36);

      // Separator
      ctx.strokeStyle = '#e8d8c0';
      ctx.beginPath();
      ctx.moveTo(40, 46); ctx.lineTo(W - 40, 46);
      ctx.stroke();

      // Icons + labels
      const xPositions = [70, 155, 245, 330];
      icons.forEach((icon, i) => {
        const x = xPositions[i];
        const img = images[i];
        if (img) ctx.drawImage(img, x - ICON_SIZE / 2, 56, ICON_SIZE, ICON_SIZE);
        if (icon) {
          ctx.fillStyle = '#6f3b24';
          ctx.font = '9px Inter, sans-serif';
          ctx.textAlign = 'center';
          ctx.fillText(iconLabel(icon, $lang), x, 118);
        }
      });

      // Brand
      ctx.fillStyle = '#d8c6b1';
      ctx.font = '8px Georgia, serif';
      ctx.textAlign = 'center';
      ctx.fillText('G O T I G A', W / 2, 178);

      const link = document.createElement('a');
      link.download = 'gotiga-key.png';
      link.href = canvas.toDataURL('image/png');
      link.click();
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
      <p class="card-brand">GOTIGA</p>
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
