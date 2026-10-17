// Карта этюдов: местность, дорога и всё, что на ней нарисовано.
//
// Карта одна и та же при каждом открытии: её рисует зерно (`MAP_SEED`), а не
// случай. Рельеф считается один раз на странице — высота в сетке, тени по
// склонам, береговая линия и морская штриховка — и кладётся на canvas. Горы,
// леса, реки и волны — SVG поверх, и места им выбирает та же сетка высот:
// гора стоит там, где рельеф поднят, а река течёт вниз по склону.
//
// Этюды стоят на дороге по порядку полки (`sort_order`): первый — у гавани
// на юго-западе, последний — в горах на северо-востоке. Сколько бы их ни
// было, дорога одна, меняется только шаг между ними.

export const MAP_W = 1600;
export const MAP_H = 1000;
export const MAP_SEED = 1873;
/** Во сколько раз картинка рельефа крупнее единиц карты. */
export const MAP_SCALE = 1.2;
/**
 * Версия рисунка. Готовая карта лежит в кэше браузера под этим номером;
 * правка рисунка без смены номера показала бы вернувшемуся старую карту.
 */
export const MAP_VERSION = 4;

// ── Случай с зерном и шум ───────────────────────────────────────────────

export function rng(seed: number) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function makeNoise(seed: number) {
  const r = rng(seed);
  const order = Array.from({ length: 256 }, (_, i) => i);
  for (let i = 255; i > 0; i--) {
    const j = Math.floor(r() * (i + 1));
    [order[i], order[j]] = [order[j], order[i]];
  }
  const p = new Uint8Array(512);
  for (let i = 0; i < 512; i++) p[i] = order[i & 255];
  const gx = new Float32Array(256);
  const gy = new Float32Array(256);
  for (let i = 0; i < 256; i++) {
    const a = r() * Math.PI * 2;
    gx[i] = Math.cos(a);
    gy[i] = Math.sin(a);
  }
  const fade = (t: number) => t * t * t * (t * (t * 6 - 15) + 10);
  function noise(x: number, y: number) {
    const xi = Math.floor(x);
    const yi = Math.floor(y);
    const xf = x - xi;
    const yf = y - yi;
    const X = xi & 255;
    const Y = yi & 255;
    const aa = p[p[X] + Y];
    const ab = p[p[X] + Y + 1];
    const ba = p[p[X + 1] + Y];
    const bb = p[p[X + 1] + Y + 1];
    const u = fade(xf);
    const v = fade(yf);
    const n00 = gx[aa] * xf + gy[aa] * yf;
    const n10 = gx[ba] * (xf - 1) + gy[ba] * yf;
    const n01 = gx[ab] * xf + gy[ab] * (yf - 1);
    const n11 = gx[bb] * (xf - 1) + gy[bb] * (yf - 1);
    const x1 = n00 + (n10 - n00) * u;
    const x2 = n01 + (n11 - n01) * u;
    return (x1 + (x2 - x1) * v) * 1.41;
  }
  function fbm(x: number, y: number, octaves = 5) {
    let amp = 0.5;
    let freq = 1;
    let sum = 0;
    let norm = 0;
    for (let i = 0; i < octaves; i++) {
      sum += amp * noise(x * freq + i * 17.3, y * freq - i * 9.1);
      norm += amp;
      amp *= 0.5;
      freq *= 2.03;
    }
    return sum / norm;
  }
  /**
   * Гребенчатый шум: у каждой октавы острый гребень там, где у обычного шума
   * ноль, и мелкие октавы растут только на гребнях крупных. Так получаются
   * хребет, отроги и долины между ними, а не бугры.
   */
  function ridged(x: number, y: number, octaves = 5) {
    let sum = 0;
    let amp = 0.55;
    let freq = 1;
    let weight = 1;
    for (let i = 0; i < octaves; i++) {
      let n = 1 - Math.abs(noise(x * freq + i * 31.7, y * freq - i * 17.3));
      n *= n;
      n *= weight;
      weight = Math.min(1, n * 1.8);
      sum += n * amp;
      amp *= 0.5;
      freq *= 2.1;
    }
    return sum;
  }
  return { noise, fbm, ridged };
}

// ── Дорога ──────────────────────────────────────────────────────────────

/** Опорные точки дороги в долях карты: от гавани к горной крепости. */
const ROAD_POINTS: [number, number][] = [
  [0.15, 0.8],
  [0.27, 0.7],
  [0.22, 0.52],
  [0.33, 0.36],
  [0.48, 0.4],
  [0.55, 0.58],
  [0.67, 0.71],
  [0.8, 0.6],
  [0.75, 0.42],
  [0.85, 0.25],
];

type Pt = { x: number; y: number };

function catmullRom(points: Pt[], perSegment: number): Pt[] {
  const out: Pt[] = [];
  const P = [points[0], ...points, points[points.length - 1]];
  for (let i = 1; i < P.length - 2; i++) {
    const [p0, p1, p2, p3] = [P[i - 1], P[i], P[i + 1], P[i + 2]];
    for (let s = 0; s < perSegment; s++) {
      const t = s / perSegment;
      const t2 = t * t;
      const t3 = t2 * t;
      out.push({
        x:
          0.5 *
          (2 * p1.x + (-p0.x + p2.x) * t + (2 * p0.x - 5 * p1.x + 4 * p2.x - p3.x) * t2 +
            (-p0.x + 3 * p1.x - 3 * p2.x + p3.x) * t3),
        y:
          0.5 *
          (2 * p1.y + (-p0.y + p2.y) * t + (2 * p0.y - 5 * p1.y + 4 * p2.y - p3.y) * t2 +
            (-p0.y + 3 * p1.y - 3 * p2.y + p3.y) * t3),
      });
    }
  }
  out.push(points[points.length - 1]);
  return out;
}

export interface MapPin extends Pt {
  /** Место на дороге долей её длины. */
  t: number;
  /** Подпись под знаком, а не над ним: дорога в этом месте идёт сверху. */
  labelBelow: boolean;
}

export interface Road {
  d: string;
  samples: Pt[];
  pins: MapPin[];
}

export function layRoad(count: number): Road {
  const control = ROAD_POINTS.map(([u, v]) => ({ x: u * MAP_W, y: v * MAP_H }));
  const samples = catmullRom(control, 48);
  const lengths = [0];
  for (let i = 1; i < samples.length; i++) {
    const a = samples[i - 1];
    const b = samples[i];
    lengths.push(lengths[i - 1] + Math.hypot(b.x - a.x, b.y - a.y));
  }
  const total = lengths[lengths.length - 1];
  const at = (t: number) => {
    const want = t * total;
    let i = 1;
    while (i < lengths.length - 1 && lengths[i] < want) i++;
    const span = lengths[i] - lengths[i - 1] || 1;
    const k = (want - lengths[i - 1]) / span;
    const a = samples[i - 1];
    const b = samples[i];
    return { x: a.x + (b.x - a.x) * k, y: a.y + (b.y - a.y) * k, dx: b.x - a.x, dy: b.y - a.y };
  };
  const pins: MapPin[] = [];
  for (let i = 0; i < count; i++) {
    const t = count === 1 ? 0.5 : 0.03 + (0.94 * i) / (count - 1);
    const p = at(t);
    // Дорога, идущая почти горизонтально, займёт место под знаком так же,
    // как над ним; тогда решает чередование, чтобы соседние подписи не
    // стояли в одну строку.
    const steep = Math.abs(p.dy) > Math.abs(p.dx) * 0.6;
    pins.push({ x: p.x, y: p.y, t, labelBelow: steep ? true : i % 2 === 0 });
  }
  const d = samples
    .map((p, i) => `${i ? 'L' : 'M'}${p.x.toFixed(1)} ${p.y.toFixed(1)}`)
    .join('');
  return { d, samples, pins };
}

// ── Поля расстояний ─────────────────────────────────────────────────────

/** Хребты в долях карты. Дорога проходит их перевалами (см. `valley`). */
const RANGES: [number, number][][] = [
  [
    [0.26, 0.15],
    [0.4, 0.21],
    [0.55, 0.17],
    [0.66, 0.24],
    [0.72, 0.16],
    [0.88, 0.14],
  ],
  [
    [0.4, 0.52],
    [0.45, 0.66],
    [0.53, 0.8],
  ],
  [
    [0.08, 0.36],
    [0.13, 0.27],
  ],
  [
    [0.9, 0.42],
    [0.88, 0.62],
  ],
];

/**
 * Озёра: центр и радиус в долях ширины. Рисуются в SVG поверх рельефа, а не
 * ямой в высотах: яма давала склон вокруг воды, и озеро читалось кратером.
 */
const LAKES: [number, number, number][] = [
  [0.635, 0.45, 0.04],
  [0.355, 0.61, 0.026],
];

/** Острова в море: центр, радиус, высота. */
const ISLANDS: [number, number, number, number][] = [
  [0.07, 0.1, 0.04, 0.55],
  [0.95, 0.88, 0.035, 0.5],
  [0.62, 0.95, 0.03, 0.4],
  [0.97, 0.3, 0.025, 0.35],
];

function segmentDistance(px: number, py: number, line: Pt[]) {
  let best = Infinity;
  for (let i = 1; i < line.length; i++) {
    const a = line[i - 1];
    const b = line[i];
    const vx = b.x - a.x;
    const vy = b.y - a.y;
    const len = vx * vx + vy * vy || 1;
    let k = ((px - a.x) * vx + (py - a.y) * vy) / len;
    k = k < 0 ? 0 : k > 1 ? 1 : k;
    const dx = px - (a.x + vx * k);
    const dy = py - (a.y + vy * k);
    const d = dx * dx + dy * dy;
    if (d < best) best = d;
  }
  return Math.sqrt(best);
}

/** Расстояние до ломаной в грубой сетке; читается билинейно. */
class DistanceField {
  private cols = 161;
  private rows = 101;
  private values: Float32Array;
  constructor(lines: Pt[][]) {
    this.values = new Float32Array(this.cols * this.rows);
    for (let j = 0; j < this.rows; j++) {
      for (let i = 0; i < this.cols; i++) {
        const x = (i / (this.cols - 1)) * MAP_W;
        const y = (j / (this.rows - 1)) * MAP_H;
        let d = Infinity;
        for (const line of lines) d = Math.min(d, segmentDistance(x, y, line));
        this.values[j * this.cols + i] = d;
      }
    }
  }
  at(x: number, y: number) {
    const fx = Math.min(Math.max((x / MAP_W) * (this.cols - 1), 0), this.cols - 1.001);
    const fy = Math.min(Math.max((y / MAP_H) * (this.rows - 1), 0), this.rows - 1.001);
    const i = Math.floor(fx);
    const j = Math.floor(fy);
    const u = fx - i;
    const v = fy - j;
    const c = this.cols;
    const a = this.values[j * c + i];
    const b = this.values[j * c + i + 1];
    const d = this.values[(j + 1) * c + i];
    const e = this.values[(j + 1) * c + i + 1];
    return (a * (1 - u) + b * u) * (1 - v) + (d * (1 - u) + e * u) * v;
  }
}

// ── Рельеф ──────────────────────────────────────────────────────────────

export interface Relief {
  /** Ширина и высота сетки; шаг сетки — `MAP_W / gw` единиц карты. */
  gw: number;
  gh: number;
  /** Высота: ниже нуля — вода. */
  height: Float32Array;
  /** Доля высоты, которую дали хребты: по ней ставятся горы. */
  ridge: Float32Array;
  /** Влажность: по ней ставятся леса. */
  wet: Float32Array;
  /** Расстояние до суши (в море) в клетках сетки. */
  seaDist: Float32Array;
  /** Расстояние до воды (на суше) в клетках сетки: по нему текут реки. */
  coastDist: Float32Array;
  /** Пятна старой бумаги. */
  stain: Float32Array;
}

const GW = 640;
const GH = 400;
const STEP = MAP_W / GW;

/** Отдать поток браузеру. Через MessageChannel, а не таймер: таймеры
 *  фоновой вкладки браузер растягивает до секунды. */
const pause = () =>
  new Promise<void>((done) => {
    if (typeof MessageChannel === 'undefined') {
      setTimeout(done, 0);
      return;
    }
    const channel = new MessageChannel();
    channel.port1.onmessage = () => {
      channel.port1.close();
      done();
    };
    channel.port2.postMessage(null);
  });

/**
 * Считает сетку высот. Работает кусками по строкам и отдаёт поток между
 * ними: полмиллиона точек шума одним куском заморозили бы страницу.
 */
export async function raiseRelief(road: Road): Promise<Relief> {
  const { fbm, ridged } = makeNoise(MAP_SEED);
  const roadField = new DistanceField([road.samples]);
  const rangeField = new DistanceField(
    RANGES.map((line) => catmullRom(line.map(([u, v]) => ({ x: u * MAP_W, y: v * MAP_H })), 12)),
  );
  const height = new Float32Array(GW * GH);
  const ridge = new Float32Array(GW * GH);
  const wet = new Float32Array(GW * GH);
  const stain = new Float32Array(GW * GH);

  for (let j = 0; j < GH; j++) {
    for (let i = 0; i < GW; i++) {
      const x = (i + 0.5) * STEP;
      const y = (j + 0.5) * STEP;
      // Шум считается в единицах высоты карты, чтобы был одинаков по осям.
      const px = x / MAP_H;
      const py = y / MAP_H;
      const wx = px + 0.09 * fbm(px * 2.2 + 11, py * 2.2, 4);
      const wy = py + 0.09 * fbm(px * 2.2, py * 2.2 + 7, 4);
      const dx = (wx - 0.8) / 0.7;
      const dy = (wy - 0.5) / 0.4;
      const reach = Math.sqrt(dx * dx + dy * dy);
      let h = (1 - reach) * 0.95 + 0.34 * fbm(px * 3.1, py * 3.1, 5) + 0.035 * fbm(px * 11, py * 11, 3);

      for (const [u, v, r, top] of ISLANDS) {
        const d = Math.hypot(x / MAP_W - u, (y - v * MAP_H) / MAP_W) / r;
        h = Math.max(h, top * Math.exp(-d * d) - 0.2 + 0.12 * fbm(px * 8 + u * 9, py * 8, 3));
      }

      const rd = roadField.at(x, y) / MAP_H;
      // Дорога всегда по суше.
      h = Math.max(h, 0.12 - rd * 1.6);

      // Горы: гребенчатый шум в полосе хребта, главный гребень — по линии
      // хребта. Дорога идёт перевалом: у неё горы опущены.
      const near = rangeField.at(x, y) / MAP_H;
      const band = Math.exp(-((near / 0.085) ** 2));
      const spine = Math.exp(-((near / 0.03) ** 2));
      const crest = ridged(wx * 4.2 + 3, wy * 4.2 - 5, 5);
      const valley = 1 - 0.9 * Math.exp(-((rd / 0.055) ** 2));
      const m = (band * crest * 1.25 + spine * 0.35) * valley * (h > 0 ? 1 : 0.4);
      h += m * 0.95;

      const k = j * GW + i;
      height[k] = h;
      ridge[k] = m;
      wet[k] = fbm(px * 4.2 + 40, py * 4.2 - 13, 4);
      stain[k] = fbm(px * 1.6 - 70, py * 1.6 + 30, 4);
    }
    if (j % 48 === 47) await pause();
  }

  const seaDist = chamfer(height, (v) => v > 0);
  const coastDist = chamfer(height, (v) => v <= 0);
  // Линии штриховки не идут ровно параллельно берегу: так рисует рука.
  for (let j = 0; j < GH; j++) {
    for (let i = 0; i < GW; i++) {
      const k = j * GW + i;
      if (seaDist[k] > 0) seaDist[k] += 1.4 * fbm(i * 0.03, j * 0.03, 3);
    }
  }

  return { gw: GW, gh: GH, height, ridge, wet, seaDist, coastDist, stain };
}

/** Расстояние до ближайшей клетки, где `seed` верно: двухпроходная фаска 1 / √2. */
function chamfer(height: Float32Array, seed: (v: number) => boolean) {
  const out = new Float32Array(GW * GH);
  const FAR = 1e6;
  for (let k = 0; k < GW * GH; k++) out[k] = seed(height[k]) ? 0 : FAR;
  const D = Math.SQRT2;
  for (let j = 0; j < GH; j++) {
    for (let i = 0; i < GW; i++) {
      const k = j * GW + i;
      let v = out[k];
      if (i > 0) v = Math.min(v, out[k - 1] + 1);
      if (j > 0) {
        v = Math.min(v, out[k - GW] + 1);
        if (i > 0) v = Math.min(v, out[k - GW - 1] + D);
        if (i < GW - 1) v = Math.min(v, out[k - GW + 1] + D);
      }
      out[k] = v;
    }
  }
  for (let j = GH - 1; j >= 0; j--) {
    for (let i = GW - 1; i >= 0; i--) {
      const k = j * GW + i;
      let v = out[k];
      if (i < GW - 1) v = Math.min(v, out[k + 1] + 1);
      if (j < GH - 1) {
        v = Math.min(v, out[k + GW] + 1);
        if (i < GW - 1) v = Math.min(v, out[k + GW + 1] + D);
        if (i > 0) v = Math.min(v, out[k + GW - 1] + D);
      }
      out[k] = v;
    }
  }
  return out;
}

function sample(grid: Float32Array, x: number, y: number) {
  const fx = Math.min(Math.max(x / STEP - 0.5, 0), GW - 1.001);
  const fy = Math.min(Math.max(y / STEP - 0.5, 0), GH - 1.001);
  const i = Math.floor(fx);
  const j = Math.floor(fy);
  const u = fx - i;
  const v = fy - j;
  const k = j * GW + i;
  return (
    (grid[k] * (1 - u) + grid[k + 1] * u) * (1 - v) +
    (grid[k + GW] * (1 - u) + grid[k + GW + 1] * u) * v
  );
}

// ── Отрисовка рельефа ───────────────────────────────────────────────────

const smooth = (a: number, b: number, x: number) => {
  const t = Math.min(Math.max((x - a) / (b - a), 0), 1);
  return t * t * (3 - 2 * t);
};

/** Высота, с которой лежит снег. */
const SNOW = 1.45;

/** Цвет суши по высоте: низина — светлая охра, нагорье — бурая. */
const LAND = [
  [0.0, 238, 226, 194],
  [0.25, 230, 215, 176],
  [0.6, 214, 192, 150],
  [1.1, 186, 160, 122],
];

function landColor(h: number): [number, number, number] {
  for (let i = 1; i < LAND.length; i++) {
    if (h <= LAND[i][0] || i === LAND.length - 1) {
      const [a0, ar, ag, ab] = LAND[i - 1];
      const [b0, br, bg, bb] = LAND[i];
      const t = Math.min(Math.max((h - a0) / (b0 - a0), 0), 1);
      return [ar + (br - ar) * t, ag + (bg - ag) * t, ab + (bb - ab) * t];
    }
  }
  return [238, 226, 194];
}

/** Штриховка моря: расстояния от берега в единицах карты и сила линии. */
const RIPPLES: [number, number][] = [
  [6, 0.42],
  [12, 0.33],
  [20, 0.25],
  [30, 0.17],
  [43, 0.11],
  [59, 0.07],
];

/**
 * Кладёт рельеф на canvas: бумага, цвет высоты, тени склонов, береговая
 * линия, штриховка моря, пятна и обожжённый край.
 */
type Canvas2D = CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D;

export async function paintRelief(canvas: HTMLCanvasElement | OffscreenCanvas, relief: Relief, scale = MAP_SCALE) {
  const cw = Math.round(MAP_W * scale);
  const ch = Math.round(MAP_H * scale);
  canvas.width = cw;
  canvas.height = ch;
  const ctx = canvas.getContext('2d') as Canvas2D | null;
  if (!ctx) return;
  const image = ctx.createImageData(cw, ch);
  const px = image.data;
  const { height, seaDist, stain } = relief;

  // Наклон в сетке — один раз, а не на каждый пиксель.
  const gx = new Float32Array(GW * GH);
  const gy = new Float32Array(GW * GH);
  for (let j = 0; j < GH; j++) {
    for (let i = 0; i < GW; i++) {
      const k = j * GW + i;
      const l = height[i > 0 ? k - 1 : k];
      const r = height[i < GW - 1 ? k + 1 : k];
      const u = height[j > 0 ? k - GW : k];
      const d = height[j < GH - 1 ? k + GW : k];
      gx[k] = (r - l) * 0.5;
      gy[k] = (d - u) * 0.5;
    }
  }

  const lx = -0.62;
  const ly = -0.62;
  const lz = 0.48;
  const flat = lz;
  const SLOPE = 30;
  const unit = 1 / scale;
  let seed = 0x9e3779b9;

  for (let y = 0; y < ch; y++) {
    const my = (y + 0.5) * unit;
    for (let x = 0; x < cw; x++) {
      const mx = (x + 0.5) * unit;
      const h = sample(height, mx, my);
      let r: number;
      let g: number;
      let b: number;

      if (h > 0) {
        [r, g, b] = landColor(h);
        const sx = sample(gx, mx, my) * SLOPE;
        const sy = sample(gy, mx, my) * SLOPE;
        const len = Math.sqrt(sx * sx + sy * sy + 1);
        const shade = (-sx * lx - sy * ly + lz) / len - flat;
        // Равнина у берега почти плоская, рельеф читается в нагорье.
        const strength = 0.2 + 0.8 * smooth(0.05, 0.6, h);
        const k = 1 + Math.max(-0.38, Math.min(0.28, shade * 1.1 * strength));
        r *= k;
        g *= k;
        b *= k * 0.98;
        // Тон под лесом: та же мерка, по которой стоят деревья.
        const woods = forestAt(relief, mx, my);
        if (woods > 0) {
          r += (196 - r) * woods * 0.22;
          g += (190 - g) * woods * 0.22;
          b += (150 - b) * woods * 0.22;
        }
        // Снег на вершинах — тоном бумаги, освещённая сторона светлее.
        const snow = smooth(SNOW - 0.2, SNOW + 0.25, h);
        if (snow > 0) {
          const lit = 0.75 + 0.25 * Math.max(0, Math.min(1, shade * 4 + 0.5));
          r += (246 * lit - r) * snow * 0.55;
          g += (240 * lit - g) * snow * 0.55;
          b += (226 * lit - b) * snow * 0.55;
        }
      } else {
        r = 226;
        g = 217;
        b = 192;
        const d = sample(seaDist, mx, my) * STEP;
        const shallow = Math.exp(-d / 22);
        r -= 16 * shallow;
        g -= 15 * shallow;
        b -= 10 * shallow;
        let line = 0;
        for (const [at, power] of RIPPLES) {
          const a = 1 - Math.abs(d - at) / 0.9;
          if (a > line / power) line = Math.max(line, a * power);
        }
        if (line > 0) {
          r += (70 - r) * line;
          g += (52 - g) * line;
          b += (40 - b) * line;
        }
      }

      // Береговая линия: высота, делённая на наклон, — расстояние до нуля.
      const slope = Math.hypot(sample(gx, mx, my), sample(gy, mx, my)) / STEP;
      if (slope > 1e-6) {
        const dist = Math.abs(h) / slope;
        const ink = Math.min(Math.max(1.35 - dist / 1.05, 0), 1);
        if (h > 0 && dist < 9) {
          const band = 0.14 * Math.exp(-dist / 3.2);
          r *= 1 - band;
          g *= 1 - band;
          b *= 1 - band;
        }
        if (ink > 0) {
          r += (62 - r) * ink * 0.9;
          g += (44 - g) * ink * 0.9;
          b += (32 - b) * ink * 0.9;
        }
      }

      // Бумага: зерно, пятна, обожжённый край.
      seed ^= seed << 13;
      seed ^= seed >>> 17;
      seed ^= seed << 5;
      const grain = 1 + (((seed >>> 0) / 4294967296) - 0.5) * 0.06;
      const spot = sample(stain, mx, my);
      const old = 1 - 0.1 * smooth(0.05, 0.4, spot);
      const ex = mx / MAP_W - 0.5;
      const ey = my / MAP_H - 0.5;
      const radial = Math.sqrt(ex * ex * 1.1 + ey * ey * 1.6) / 0.71;
      const edge = Math.min(mx, MAP_W - mx, my, MAP_H - my);
      const burn = 1 - 0.3 * smooth(0.62, 1.08, radial) - 0.25 * Math.exp(-edge / 14);
      const k = grain * old * burn;
      const o = (y * cw + x) * 4;
      px[o] = r * k;
      px[o + 1] = g * k * 0.985;
      px[o + 2] = b * k * 0.95;
      px[o + 3] = 255;
    }
    if (y % 120 === 119) await pause();
  }
  ctx.putImageData(image, 0, 0);
  await pause();
  drawHachures(ctx, relief, gx, gy, scale);
}

/** Крутизна, с которой начинается и на которой густеет штриховка. */
const HACHURE_FROM = 0.008;
const HACHURE_FULL = 0.035;

/**
 * Штриховка по склону, как на гравированных картах: короткий штрих идёт
 * вниз по склону, чем круче — тем гуще, длиннее и толще; на теневой стороне
 * темнее. Горы видны из самого рельефа, отдельных значков нет.
 */
function drawHachures(
  ctx: Canvas2D,
  relief: Relief,
  gx: Float32Array,
  gy: Float32Array,
  scale: number,
) {
  const { height } = relief;
  const r = rng(MAP_SEED + 11);
  const cw = ctx.canvas.width;
  const ch = ctx.canvas.height;
  const pitch = 3.1;
  // Штрихи собираются по корзинам (толщина × сила), чтобы рисовать одним
  // путём на корзину, а не десятками тысяч stroke().
  const BUCKETS = 4;
  const paths: Path2D[][] = Array.from({ length: BUCKETS }, () =>
    Array.from({ length: BUCKETS }, () => new Path2D()),
  );
  for (let y = pitch; y < ch - pitch; y += pitch) {
    for (let x = pitch; x < cw - pitch; x += pitch) {
      const sx = x + (r() - 0.5) * pitch * 0.9;
      const sy = y + (r() - 0.5) * pitch * 0.9;
      const mx = sx / scale;
      const my = sy / scale;
      const h = sample(height, mx, my);
      if (h < 0.04) continue;
      const ux = sample(gx, mx, my) / STEP;
      const uy = sample(gy, mx, my) / STEP;
      const slope = Math.hypot(ux, uy);
      const steep = smooth(HACHURE_FROM, HACHURE_FULL, slope);
      if (steep <= 0 || r() > steep * 0.9 + 0.05) continue;
      // Вниз по склону.
      const dx = -ux / slope;
      const dy = -uy / slope;
      // Склон смотрит на северо-запад — освещён.
      const lit = Math.max(0, -dx * 0.707 - dy * 0.707);
      const snowy = smooth(SNOW - 0.12, SNOW + 0.1, h);
      let power = steep * (1 - 0.6 * lit) * (1 - 0.55 * snowy);
      power = Math.min(0.999, power);
      if (power < 0.06) continue;
      const len = (2.2 + 4.2 * steep) * scale;
      const width = Math.min(0.999, steep * (1 - 0.4 * lit));
      const path = paths[Math.floor(width * BUCKETS)][Math.floor(power * BUCKETS)];
      path.moveTo(sx - dx * len * 0.3, sy - dy * len * 0.3);
      path.lineTo(sx + dx * len * 0.7, sy + dy * len * 0.7);
    }
  }
  ctx.save();
  ctx.lineCap = 'round';
  for (let w = 0; w < BUCKETS; w++) {
    for (let p = 0; p < BUCKETS; p++) {
      ctx.lineWidth = (0.55 + 0.35 * w) * scale;
      ctx.strokeStyle = `rgba(52, 36, 26, ${(0.2 + 0.2 * p).toFixed(2)})`;
      ctx.stroke(paths[w][p]);
    }
  }
  ctx.restore();
}

// ── Знаки на местности ──────────────────────────────────────────────────

export interface Lake {
  /** Берег. */
  d: string;
  /** Линия у берега — та же штриховка, что у моря. */
  shore: string[];
  /** Горизонтальная штриховка воды, три набора. */
  hatch: string[];
  reeds: string;
  /** Початки камыша. */
  heads: string;
  glints: (Pt & { len: number; delay: number; dur: number })[];
  ripples: (Pt & { delay: number })[];
  box: { x: number; y: number; w: number; h: number };
}

/** Замкнутая плавная линия через точки (Катмулл — Ром в кубические кривые). */
function closedCurve(pts: Pt[]): string {
  const n = pts.length;
  let d = `M${f1(pts[0].x)} ${f1(pts[0].y)}`;
  for (let i = 0; i < n; i++) {
    const p0 = pts[(i - 1 + n) % n];
    const p1 = pts[i];
    const p2 = pts[(i + 1) % n];
    const p3 = pts[(i + 2) % n];
    d +=
      `C${f1(p1.x + (p2.x - p0.x) / 6)} ${f1(p1.y + (p2.y - p0.y) / 6)} ` +
      `${f1(p2.x - (p3.x - p1.x) / 6)} ${f1(p2.y - (p3.y - p1.y) / 6)} ${f1(p2.x)} ${f1(p2.y)}`;
  }
  return d + 'Z';
}

export interface Decor {
  trees: Tree[];
  rivers: { d: string; width: number }[];
  lakes: Lake[];
  waves: Pt[];
  ship: string | null;
  serpent: Pt | null;
}

const f1 = (n: number) => n.toFixed(1);

export interface Tree {
  x: number;
  y: number;
  /** Силуэт кроны с заливкой бумагой: закрывает деревья сзади. */
  crown: string;
  /** Теневая сторона, к юго-востоку от света. */
  shade: string;
  /** Контур, ствол и штрихи — одной линией чернил. */
  ink: string;
}

/**
 * Насколько густ лес в точке, 0…1. Одна мерка на деревья и на тон под
 * лесом: тон без деревьев или деревья без тона читались бы двумя лесами.
 */
export function forestAt(relief: Relief, x: number, y: number): number {
  const h = sample(relief.height, x, y);
  if (h < 0.04 || h > 0.75) return 0;
  const e = 4;
  const tilt =
    Math.hypot(
      sample(relief.height, x + e, y) - sample(relief.height, x - e, y),
      sample(relief.height, x, y + e) - sample(relief.height, x, y - e),
    ) /
    (2 * e);
  // Лес не растёт там, где склон уже заштрихован как гора.
  const flat = 1 - smooth(HACHURE_FROM * 0.8, HACHURE_FROM * 1.5, tilt);
  return smooth(0.07, 0.19, sample(relief.wet, x, y)) * flat;
}

/** Ель: три яруса веток с опущенными концами, тень справа. */
function pineGlyph(x: number, y: number, s: number, r: () => number): Tree {
  const H = s * 1.55;
  const W = s * 0.5 * (0.85 + 0.3 * r());
  const j = () => (r() - 0.5) * s * 0.06;
  const base = y - H * 0.1;
  const tiers = 3;
  const left: string[] = [];
  const right: string[] = [];
  let prevL = { x: x, y: y - H };
  let prevR = { x: x, y: y - H };
  for (let t = 0; t < tiers; t++) {
    const bottom = y - H + (H * 0.9 * (t + 1)) / tiers + j();
    const top = y - H + (H * 0.9 * t) / tiers;
    const w = W * (0.5 + (0.5 * (t + 1)) / tiers);
    const tipL = { x: x - w + j(), y: bottom + s * 0.04 };
    const tipR = { x: x + w + j(), y: bottom + s * 0.04 };
    // Ветка провисает: вогнутая линия от выемки к концу.
    left.push(`Q${f1(x - w * 0.45)} ${f1((top + bottom) / 2 + s * 0.12)} ${f1(tipL.x)} ${f1(tipL.y)}`);
    right.push(`Q${f1(x + w * 0.45)} ${f1((top + bottom) / 2 + s * 0.12)} ${f1(tipR.x)} ${f1(tipR.y)}`);
    if (t < tiers - 1) {
      const notchY = bottom - s * 0.1;
      prevL = { x: x - w * 0.42, y: notchY };
      prevR = { x: x + w * 0.42, y: notchY };
      left.push(`L${f1(prevL.x)} ${f1(prevL.y)}`);
      right.push(`L${f1(prevR.x)} ${f1(prevR.y)}`);
    }
  }
  const apex = `M${f1(x)} ${f1(y - H)}`;
  const leftSide = apex + left.join('');
  // Правую сторону обходим снизу вверх, чтобы силуэт замкнулся.
  const rightUp = (() => {
    const pts: string[] = [];
    let t = tiers - 1;
    const tips: { x: number; y: number }[] = [];
    const notches: { x: number; y: number }[] = [];
    for (const seg of right) {
      const nums = seg.slice(1).split(' ').map(Number);
      if (seg[0] === 'Q') tips.push({ x: nums[2], y: nums[3] });
      else notches.push({ x: nums[0], y: nums[1] });
    }
    for (; t >= 0; t--) {
      pts.push(`L${f1(tips[t].x)} ${f1(tips[t].y)}`);
      const up = t > 0 ? notches[t - 1] : { x, y: y - H };
      pts.push(`Q${f1(x + (tips[t].x - x) * 0.45)} ${f1((up.y + tips[t].y) / 2 + s * 0.12)} ${f1(up.x)} ${f1(up.y)}`);
    }
    return pts.join('');
  })();
  const crown = `${leftSide}L${f1(x)} ${f1(base)}${rightUp}Z`;
  const shade = `M${f1(x)} ${f1(y - H)}L${f1(x)} ${f1(base)}${rightUp}Z`;
  let hatch = '';
  for (let k = 0; k < 3; k++) {
    const hy = y - H * (0.62 - k * 0.27);
    const hw = W * (0.35 + k * 0.18);
    hatch += `M${f1(x + hw * 0.2)} ${f1(hy)}l${f1(hw * 0.5)} ${f1(s * 0.12)}`;
  }
  const outline = `${leftSide}M${f1(x)} ${f1(y - H)}${right.join('')}`;
  const trunk = `M${f1(x)} ${f1(base - s * 0.05)}L${f1(x)} ${f1(y + s * 0.05)}`;
  return { x, y, crown, shade, ink: outline + trunk + hatch };
}

/** Лиственное дерево: неровная крона из выпуклостей, тень снизу справа. */
function leafGlyph(x: number, y: number, s: number, r: () => number): Tree {
  const rx = s * 0.58 * (0.9 + 0.25 * r());
  const ry = s * 0.5 * (0.9 + 0.2 * r());
  const cx = x + (r() - 0.5) * s * 0.1;
  const cy = y - s * 0.3 - ry;
  const lobes = 6 + Math.floor(r() * 3);
  const start = r() * Math.PI * 2;
  const pts: Pt[] = [];
  for (let i = 0; i <= lobes; i++) {
    const a = start + (i / lobes) * Math.PI * 2 + (i < lobes && i > 0 ? (r() - 0.5) * 0.35 : 0);
    const k = 0.9 + 0.12 * r();
    pts.push({ x: cx + Math.cos(a) * rx * k, y: cy + Math.sin(a) * ry * k });
  }
  pts[lobes] = pts[0];
  const lobe = (a: Pt, b: Pt) => {
    const chord = Math.hypot(b.x - a.x, b.y - a.y);
    return `A${f1(chord * 0.58)} ${f1(chord * 0.58)} 0 0 1 ${f1(b.x)} ${f1(b.y)}`;
  };
  let crown = `M${f1(pts[0].x)} ${f1(pts[0].y)}`;
  for (let i = 0; i < lobes; i++) crown += lobe(pts[i], pts[i + 1]);
  crown += 'Z';
  // Тень: дуга по краю от востока до юго-запада и назад через середину,
  // смещённую к свету.
  const arc: Pt[] = [];
  for (let i = 0; i <= 5; i++) {
    const a = -0.35 + (i / 5) * 2.4;
    arc.push({ x: cx + Math.cos(a) * rx * 0.97, y: cy + Math.sin(a) * ry * 0.97 });
  }
  let shade = `M${f1(arc[0].x)} ${f1(arc[0].y)}`;
  for (let i = 0; i < 5; i++) shade += lobe(arc[i], arc[i + 1]);
  shade += `Q${f1(cx + rx * 0.05)} ${f1(cy + ry * 0.05)} ${f1(arc[0].x)} ${f1(arc[0].y)}Z`;
  let hatch = '';
  for (let k = 0; k < 3; k++) {
    const hx = cx + rx * (0.15 + k * 0.22);
    const hy = cy + ry * (0.15 + (k % 2) * 0.2);
    hatch += `M${f1(hx)} ${f1(hy)}q${f1(rx * 0.1)} ${f1(ry * 0.2)} 0 ${f1(ry * 0.38)}`;
  }
  const trunkTop = cy + ry * 0.8;
  const trunk =
    `M${f1(x)} ${f1(trunkTop)}L${f1(x)} ${f1(y)}` +
    `M${f1(x)} ${f1(trunkTop + (y - trunkTop) * 0.3)}l${f1(-s * 0.12)} ${f1(-s * 0.12)}`;
  return { x, y, crown, shade, ink: crown + trunk + hatch };
}

/**
 * Расставляет леса, реки, озёра и волны по готовому рельефу. Обходит дорогу
 * и знаки этюдов: лес поверх крепости прятал бы её. Гор и холмов здесь нет —
 * их рисует сам рельеф штриховкой (`drawHachures`).
 */
export function dressRelief(relief: Relief, road: Road): Decor {
  const r = rng(MAP_SEED + 7);
  const { height, ridge, wet, seaDist, coastDist } = relief;
  const { noise } = makeNoise(MAP_SEED + 3);

  // Озёра: вытянутый плавный берег. Шум по углу низкой частоты — берег
  // должен изгибаться, а не дрожать.
  const SQUASH = 0.7;
  const lakeShapes = LAKES.map(([u, v, rad], n) => {
    const lr = rng(MAP_SEED + 50 + n);
    const cx = u * MAP_W;
    const cy = v * MAP_H;
    const R = rad * MAP_W * 1.12;
    const edge = (a: number) =>
      R *
      (1 +
        0.16 * noise(Math.cos(a) * 0.9 + n * 7, Math.sin(a) * 0.9) +
        0.05 * noise(Math.cos(a) * 2.2, Math.sin(a) * 2.2 + n));
    const at = (a: number, k = 1): Pt => ({
      x: cx + Math.cos(a) * edge(a) * k,
      y: cy + Math.sin(a) * edge(a) * k * SQUASH,
    });
    const ring = (k: number) => closedCurve(Array.from({ length: 28 }, (_, i) => at((i / 28) * Math.PI * 2, k)));
    // Камыш — несколько пучков по южному и западному берегу.
    let reeds = '';
    let heads = '';
    const clumps = 5 + Math.floor(lr() * 3);
    for (let c = 0; c < clumps; c++) {
      const a = 0.4 + lr() * 2.5; // от юго-востока до запада, ось y вниз
      const base = at(a, 0.97);
      const stalks = 6 + Math.floor(lr() * 5);
      for (let k = 0; k < stalks; k++) {
        const bx = base.x + (lr() - 0.5) * 18;
        const by = base.y + (lr() - 0.3) * 4;
        const tall = 10 + lr() * 11;
        const lean = (lr() - 0.5) * 5;
        reeds += `M${f1(bx)} ${f1(by)}q${f1(lean * 0.2)} ${f1(-tall * 0.6)} ${f1(lean)} ${f1(-tall)}`;
        if (lr() < 0.35) heads += `M${f1(bx + lean * 0.7)} ${f1(by - tall * 0.78)}l${f1(lean * 0.1)} ${f1(-tall * 0.2)}`;
      }
    }
    // Блики и рябь — внутри, подальше от берега.
    const inside = (k: number): Pt => {
      const a = lr() * Math.PI * 2;
      return at(a, Math.sqrt(lr()) * k);
    };
    const glints = Array.from({ length: 9 }, () => ({ ...inside(0.7), len: 5 + lr() * 9, delay: lr() * -8, dur: 4 + lr() * 4 }));
    const ripples = Array.from({ length: 8 }, () => ({ ...inside(0.75), delay: lr() * -10 }));
    // Штриховка воды, как на гравюре: горизонтальные прерывистые линии,
    // гуще у северного берега (там тень), три набора с разным шагом
    // пунктира — их сдвиг и есть течение воды.
    const hatch = ['', '', ''];
    const top = cy - R * 1.3 * SQUASH;
    const bottom = cy + R * 1.3 * SQUASH;
    let row = 0;
    for (let y = top; y < bottom; row++) {
      const k = (y - top) / (bottom - top);
      hatch[row % 3] += `M${f1(cx - R * 1.4)} ${f1(y)}H${f1(cx + R * 1.4)}`;
      y += 3 + k * 4.5;
    }
    return {
      cx,
      cy,
      edge,
      d: ring(1),
      shore: [ring(0.92)],
      hatch,
      reeds,
      heads,
      glints,
      ripples,
      box: { x: cx - R * 1.3, y: cy - R * 1.3 * SQUASH, w: R * 2.6, h: R * 2.6 * SQUASH },
    };
  });
  const inLake = (x: number, y: number, room = 0) =>
    lakeShapes.some((l) => {
      const dx = x - l.cx;
      const dy = (y - l.cy) / SQUASH;
      return Math.hypot(dx, dy) < l.edge(Math.atan2(dy, dx)) + room;
    });
  const h = (x: number, y: number) => (inLake(x, y) ? -0.3 : sample(height, x, y));
  const roadField = new DistanceField([road.samples]);
  const nearPin = (x: number, y: number, room: number) =>
    road.pins.some((p) => Math.hypot(p.x - x, p.y - y) < room);
  const offMap = (x: number, y: number, room: number) =>
    x < room || y < room || x > MAP_W - room || y > MAP_H - room;
  // Места картуша и розы ветров.
  const reserved = (x: number, y: number) =>
    (x < 470 && y < 180) || (x > 1330 && y > 740 && y < 990) || (x < 480 && y > 900);

  // Реки — вниз по склону от хребта до воды.
  const rivers: { d: string; width: number }[] = [];
  // Истоки — в горах, но подальше от моря: река от гребня прибрежного
  // хребта прячется под горами целиком.
  const landDist = (x: number, y: number) => {
    let d = Infinity;
    for (let a = 0; a < 16; a++) {
      const ca = Math.cos((a / 16) * Math.PI * 2);
      const sa = Math.sin((a / 16) * Math.PI * 2);
      for (let k = 20; k < 400; k += 20) {
        if (h(x + ca * k, y + sa * k) < 0) {
          d = Math.min(d, k);
          break;
        }
      }
    }
    return d;
  };
  const candidates: (Pt & { far: number })[] = [];
  for (let tries = 0; tries < 700; tries++) {
    const x = 80 + r() * (MAP_W - 160);
    const y = 80 + r() * (MAP_H - 160);
    if (sample(ridge, x, y) < 0.18 || h(x, y) < 0.25) continue;
    candidates.push({ x, y, far: landDist(x, y) });
  }
  candidates.sort((a, b) => b.far - a.far);
  const sources: Pt[] = [];
  for (const c of candidates) {
    if (sources.length >= 10) break;
    if (sources.some((s) => Math.hypot(s.x - c.x, s.y - c.y) < 140)) continue;
    sources.push(c);
  }
  const riverPoints: Pt[] = [];
  for (const source of sources) {
    const path: Pt[] = [source];
    let { x, y } = source;
    let vx = 0;
    let vy = 0;
    let reached = false;
    for (let step = 0; step < 500; step++) {
      // Вниз к воде по расстоянию до берега (у него нет ям, в которых
      // река застряла бы), с поправкой на склон — так русло петляет.
      const e = 4;
      const cdx = sample(coastDist, x + e, y) - sample(coastDist, x - e, y);
      const cdy = sample(coastDist, x, y + e) - sample(coastDist, x, y - e);
      const cl = Math.hypot(cdx, cdy) || 1e-6;
      const ddx = h(x + e, y) - h(x - e, y);
      const ddy = h(x, y + e) - h(x, y - e);
      const hl = Math.hypot(ddx, ddy) || 1e-6;
      const gxv = cdx / cl + (0.7 * ddx) / hl;
      const gyv = cdy / cl + (0.7 * ddy) / hl;
      const len = Math.hypot(gxv, gyv) || 1e-6;
      vx = vx * 0.7 - (gxv / len) * 0.3;
      vy = vy * 0.7 - (gyv / len) * 0.3;
      const vl = Math.hypot(vx, vy) || 1;
      const wob = (r() - 0.5) * 0.5;
      x += (vx / vl) * 5 + wob * (vy / vl) * 5;
      y += (vy / vl) * 5 - wob * (vx / vl) * 5;
      path.push({ x, y });
      if (h(x, y) < -0.01) {
        reached = true;
        break;
      }
      if (offMap(x, y, 10)) break;
    }
    if (!reached || path.length < 18 || rivers.length >= 30) continue;
    // Сглаживание Чайкина: шаг вниз по склону даёт угловатую линию.
    let line = path;
    for (let pass = 0; pass < 2; pass++) {
      const next: Pt[] = [line[0]];
      for (let i = 0; i < line.length - 1; i++) {
        const a = line[i];
        const b = line[i + 1];
        next.push({ x: a.x * 0.75 + b.x * 0.25, y: a.y * 0.75 + b.y * 0.25 });
        next.push({ x: a.x * 0.25 + b.x * 0.75, y: a.y * 0.25 + b.y * 0.75 });
      }
      next.push(line[line.length - 1]);
      line = next;
    }
    riverPoints.push(...line);
    // Река шире к устью: три отрезка разной толщины.
    const parts = 3;
    for (let p = 0; p < parts; p++) {
      const from = Math.floor((line.length * p) / parts);
      const to = Math.min(line.length, Math.floor((line.length * (p + 1)) / parts) + 1);
      const piece = line.slice(from, to);
      rivers.push({
        d: piece.map((q, i) => `${i ? 'L' : 'M'}${f1(q.x)} ${f1(q.y)}`).join(''),
        width: 1.3 + p * 0.9,
      });
    }
  }
  const nearRiver = (x: number, y: number, room: number) =>
    riverPoints.some((p) => Math.abs(p.x - x) < room && Math.abs(p.y - y) < room);

  // Леса — во влажных низинах, сплошной массой с рваным краем. Ряды
  // перекрываются: задние деревья видны только верхушками, как на гравюре.
  const trees: Tree[] = [];
  for (let y = 22; y < MAP_H - 20; y += 8) {
    for (let x = 20 + ((y / 8) % 2) * 5; x < MAP_W - 20; x += 10) {
      const jx = x + (r() - 0.5) * 6;
      const jy = y + (r() - 0.5) * 4;
      const hh = h(jx, jy);
      if (hh < 0.04 || hh > 0.75) continue;
      const thick = forestAt(relief, jx, jy);
      if (thick <= 0 || r() > thick) continue;
      if (roadField.at(jx, jy) < 14 || nearPin(jx, jy, 56) || reserved(jx, jy)) continue;
      if (nearRiver(jx, jy, 6) || offMap(jx, jy, 26) || inLake(jx, jy, 5)) continue;
      const moist = sample(wet, jx, jy);
      // Ели — выше и во влажном, лиственные — по краю леса и в низине.
      const pine = hh > 0.35 || moist > 0.22 || (moist > 0.15 && r() < 0.5);
      const size = (pine ? 9 : 8) + r() * 4.5;
      trees.push(pine ? pineGlyph(jx, jy, size, r) : leafGlyph(jx, jy, size, r));
    }
  }
  trees.sort((a, b) => a.y - b.y);

  const seaAt = (x: number, y: number) => h(x, y) < 0;
  const deep = (x: number, y: number) => sample(seaDist, x, y) * STEP;

  // Волны — в открытом море.
  const waves: Pt[] = [];
  for (let tries = 0; tries < 1200 && waves.length < 34; tries++) {
    const x = 40 + r() * (MAP_W - 80);
    const y = 40 + r() * (MAP_H - 80);
    if (!seaAt(x, y) || deep(x, y) < 70 || reserved(x, y)) continue;
    if (waves.some((w) => Math.hypot(w.x - x, w.y - y) < 60)) continue;
    waves.push({ x, y });
  }

  // Корабль идёт вдоль южного края, там, где дальше всего от берега.
  let ship: string | null = null;
  {
    const lane: Pt[] = [];
    let ok = 0;
    for (let x = -80; x <= MAP_W + 80; x += 40) {
      let best = { y: MAP_H * 0.95, d: -1 };
      for (let y = MAP_H * 0.84; y < MAP_H * 0.975; y += 6) {
        const cx = Math.min(Math.max(x, 0), MAP_W);
        const d = seaAt(cx, y) ? deep(cx, y) : -1;
        if (d > best.d) best = { y, d };
      }
      if (best.d > 18) ok++;
      lane.push({ x, y: best.y });
    }
    if (ok > lane.length * 0.75) {
      // Сгладить курс, иначе корабль дёргается от клетки к клетке.
      const calm = lane.map((p, i) => {
        const a = lane[Math.max(0, i - 1)];
        const b = lane[Math.min(lane.length - 1, i + 1)];
        return { x: p.x, y: (a.y + p.y * 2 + b.y) / 4 };
      });
      ship = catmullRom(calm, 6)
        .map((p, i) => `${i ? 'L' : 'M'}${f1(p.x)} ${f1(p.y)}`)
        .join('');
    }
  }

  // Морской змей — в самом глубоком месте восточного моря.
  let serpent: Pt | null = null;
  {
    let best = -1;
    for (let y = 200; y < 760; y += 10) {
      for (let x = 1200; x < MAP_W - 70; x += 10) {
        if (!seaAt(x, y)) continue;
        const d = deep(x, y);
        if (d > best) {
          best = d;
          serpent = { x, y };
        }
      }
    }
    if (best < 45) serpent = null;
  }

  return {
    trees,
    rivers,
    lakes: lakeShapes.map(({ d, shore, hatch, reeds, heads, glints, ripples, box }) => ({
      d,
      shore,
      hatch,
      reeds,
      heads,
      glints,
      ripples,
      box,
    })),
    waves,
    ship,
    serpent,
  };
}

// ── Облака ──────────────────────────────────────────────────────────────

/** Облако — картинка с прозрачностью, нарисованная шумом один раз. */
export async function cloudImage(seed: number, w = 560, h = 260): Promise<Blob | null> {
  const { fbm } = makeNoise(seed);
  const canvas = new OffscreenCanvas(w, h);
  const ctx = canvas.getContext('2d');
  if (!ctx) return null;
  const image = ctx.createImageData(w, h);
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const u = x / w - 0.5;
      const v = y / h - 0.5;
      const fall = 1 - Math.sqrt(u * u * 3.2 + v * v * 9);
      const n = fbm(x / 90, y / 90, 5);
      // У края картинки облако обязано сойти на нет, иначе виден прямоугольник.
      const border = smooth(0, 0.12, Math.min(x / w, 1 - x / w, y / h, 1 - y / h));
      const a = smooth(0.0, 0.45, fall * 0.9 + n * 0.75) * border;
      const o = (y * w + x) * 4;
      const lit = 1 - v * 0.25;
      image.data[o] = 252 * lit;
      image.data[o + 1] = 247 * lit;
      image.data[o + 2] = 236 * lit;
      image.data[o + 3] = a * 235;
    }
  }
  ctx.putImageData(image, 0, 0);
  return canvas.convertToBlob({ type: 'image/webp', quality: 0.85 });
}

// ── Неподвижное — на ту же картинку, что рельеф ─────────────────────────

/**
 * Деревья, реки и озёра рисуются на картинку рельефа, а не элементами SVG:
 * их две тысячи с лишним, и как элементы они делали страницу тяжёлой —
 * всякая анимация по соседству перерисовывала их все на каждом кадре.
 * Пути те же строки SVG (`Path2D` их понимает), поэтому рисунок не меняется.
 */
export function paintDecor(canvas: HTMLCanvasElement | OffscreenCanvas, decor: Decor, scale = MAP_SCALE) {
  const ctx = canvas.getContext('2d') as Canvas2D | null;
  if (!ctx) return;
  ctx.save();
  ctx.scale(scale, scale);
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';

  // Реки
  ctx.strokeStyle = 'rgba(79, 102, 112, 0.85)';
  for (const river of decor.rivers) {
    ctx.lineWidth = river.width;
    ctx.stroke(new Path2D(river.d));
  }

  // Озёра: вода, тень берега, линия у берега и сам берег. Движущееся —
  // штриховка, блики, камыш — лежит отдельным слоем поверх.
  for (const lake of decor.lakes) {
    const shape = new Path2D(lake.d);
    const water = ctx.createLinearGradient(0, lake.box.y, 0, lake.box.y + lake.box.h);
    water.addColorStop(0, '#cfc6aa');
    water.addColorStop(0.45, '#ddd4ba');
    water.addColorStop(1, '#e6dec6');
    ctx.fillStyle = water;
    ctx.fill(shape);
    ctx.save();
    ctx.clip(shape);
    for (const [width, alpha] of [
      [16, 0.06],
      [10, 0.07],
      [5, 0.09],
    ]) {
      ctx.lineWidth = width;
      ctx.strokeStyle = `rgba(40, 30, 20, ${alpha})`;
      ctx.stroke(shape);
    }
    ctx.lineWidth = 0.8;
    ctx.strokeStyle = 'rgba(62, 44, 32, 0.4)';
    for (const line of lake.shore) ctx.stroke(new Path2D(line));
    ctx.restore();
    ctx.lineWidth = 1.6;
    ctx.strokeStyle = '#3e2c20';
    ctx.stroke(shape);
  }

  // Деревья — по одному, сзади вперёд: крона закрывает стоящие за ней.
  for (const tree of decor.trees) {
    ctx.fillStyle = '#e8ddbb';
    ctx.fill(new Path2D(tree.crown));
    ctx.fillStyle = 'rgba(61, 46, 30, 0.26)';
    ctx.fill(new Path2D(tree.shade));
    ctx.lineWidth = 0.75;
    ctx.strokeStyle = '#3a2c1f';
    ctx.stroke(new Path2D(tree.ink));
  }
  ctx.restore();
}

/** То, что карта показывает движущимся, — всё, кроме рисунка. */
export interface MapMotion {
  lakes: Lake[];
  waves: Pt[];
  ship: string | null;
  serpent: Pt | null;
}

export interface BuiltMap {
  picture: Blob;
  clouds: Blob[];
  motion: MapMotion;
}

/**
 * Вся карта целиком: рельеф, знаки на нём и облака. Зовётся из отдельного
 * потока (`etudeMap.worker.ts`), а без него — здесь же, на странице.
 */
export async function buildMap(count: number): Promise<BuiltMap> {
  const road = layRoad(count);
  const relief = await raiseRelief(road);
  const decor = dressRelief(relief, road);
  const canvas = new OffscreenCanvas(1, 1);
  await paintRelief(canvas, relief);
  paintDecor(canvas, decor);
  const picture = await canvas.convertToBlob({ type: 'image/webp', quality: 0.9 });
  const clouds = (await Promise.all([41, 97, 203].map((seed) => cloudImage(seed)))).filter(
    (b): b is Blob => !!b,
  );
  const motion: MapMotion = {
    lakes: decor.lakes,
    waves: decor.waves,
    ship: decor.ship,
    serpent: decor.serpent,
  };
  return { picture, clouds, motion };
}

/** Римское число для номера этюда. */
export function roman(n: number): string {
  const table: [number, string][] = [
    [1000, 'M'],
    [900, 'CM'],
    [500, 'D'],
    [400, 'CD'],
    [100, 'C'],
    [90, 'XC'],
    [50, 'L'],
    [40, 'XL'],
    [10, 'X'],
    [9, 'IX'],
    [5, 'V'],
    [4, 'IV'],
    [1, 'I'],
  ];
  let out = '';
  for (const [v, s] of table) {
    while (n >= v) {
      out += s;
      n -= v;
    }
  }
  return out;
}
