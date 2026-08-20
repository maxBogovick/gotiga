import { getIconById, iconLabel, svgForCanvas, type IconCategory } from '$lib/data/visualIcons';

const CATEGORY_IDS: IconCategory[] = ['animals', 'dishes', 'seasons', 'symbols'];

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

/**
 * Render the user's four chosen signs as a parchment "key card" PNG and trigger
 * a download. Shared by the registration sealing step and the success reminder.
 * `finalSelections` is the four icon_ids in category order.
 */
export async function downloadKeyCard(finalSelections: string[], userName: string, lang: string): Promise<void> {
  const icons = CATEGORY_IDS.map((cat, i) => getIconById(cat, finalSelections[i]));
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
      ctx.font = '9px "Instrument Sans", sans-serif';
      ctx.textAlign = 'center';
      ctx.fillText(iconLabel(icon, lang), x, 118);
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
}
