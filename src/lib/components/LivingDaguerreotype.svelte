<script lang="ts">
  /**
   * Living Daguerreotype — monocular-depth 2.5D parallax for a single still.
   *
   * The work sits very slightly *behind* its plate: as the pointer drifts, the
   * subject shifts against its ground by a few pixels, depth-weighted, so the
   * photograph seems faintly alive — a portrait that breathes when you look away.
   *
   * Design contract (so it never reads as a "store viewer"):
   *  - extremely low displacement (a few % of the frame), eased, never snappy;
   *  - no chrome of its own — fills the stage exactly like the <img> it replaces,
   *    the mat / vignette / grain stay as sibling overlays;
   *  - graceful by construction: a plain <img> is always present underneath, so
   *    SSR, the card→detail view-transition, reduced-motion and WebGL-less
   *    browsers all show the real photograph with zero jank.
   *
   * Lifecycle: the WebGL context, shader program and geometry are built ONCE in
   * onMount and persist for the component's life. Switching the gallery image
   * only reloads the textures (via the $effect on src/depthSrc) — no context
   * teardown, no shader recompile, no context-loss churn while paging ←/→.
   *
   * Depth source, in order of preference:
   *  1. `depthSrc` — a precomputed monocular depth map (Depth-Anything-class),
   *     a grayscale image served as a media variant. The headline path.
   *  2. fallback — luminance of the colour image, blurred in-shader. Lets the
   *     whole archive participate today with no per-image ML pass.
   */
  import { onMount } from 'svelte';

  let {
    src,
    depthSrc = null,
    alt = '',
    intensity = 0.6,
    class: className = '',
    onActivate,
  }: {
    src?: string | null;
    depthSrc?: string | null;
    alt?: string;
    /** 0..1 — multiplies the (already subtle) maximum displacement. */
    intensity?: number;
    class?: string;
    onActivate?: () => void;
  } = $props();

  // Max texture-space shift at intensity 1 and full depth. ~3.4% of the frame —
  // perceptible as presence, never as a gimmick.
  const MAX_SHIFT = 0.034;
  const EASE = 0.09;

  let host = $state<HTMLDivElement>();
  let canvas = $state<HTMLCanvasElement>();
  let baseImg = $state<HTMLImageElement>(); // the visible <img>; reused as the GL texture source
  let imageFailed = $state(false);
  let glReady = $state(false); // canvas takes over from the base <img> only once it has drawn

  const reducedMotion =
    typeof window !== 'undefined' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  // ── Persistent GL state (built once, reused across image switches) ──────────
  let gl: WebGLRenderingContext | null = null;
  let uColor: WebGLUniformLocation | null = null;
  let uDepth: WebGLUniformLocation | null = null;
  let uHasDepth: WebGLUniformLocation | null = null;
  let uImageAspect: WebGLUniformLocation | null = null;
  let uCanvasAspect: WebGLUniformLocation | null = null;
  let uMouse: WebGLUniformLocation | null = null;
  let uIntensity: WebGLUniformLocation | null = null;

  let colorTex: WebGLTexture | null = null;
  let depthTex: WebGLTexture | null = null;
  let hasDepth = 0;
  let imageAspect = 1;

  // pointer → eased camera offset
  let targetX = 0, targetY = 0, curX = 0, curY = 0;
  let pointerInside = false;
  let visible = true;
  let running = false;
  let raf = 0;
  let hostRect: DOMRect | null = null;
  let hostRectDirty = true;

  let destroyed = false;
  let initialized = false; // GL context + program ready
  let loadedKey = '';      // de-dupes the (src|depthSrc) currently loaded/loading
  let loadSeq = 0;         // supersedes in-flight loads when the image changes fast

  function updateHostRect() {
    hostRect = host?.getBoundingClientRect() ?? null;
    hostRectDirty = false;
  }

  function markHostRectDirty() {
    hostRectDirty = true;
  }

  function stopAnimation() {
    running = false;
    if (raf) cancelAnimationFrame(raf);
    raf = 0;
  }

  function makeTexture(): WebGLTexture | null {
    if (!gl) return null;
    const tex = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    return tex;
  }

  function uploadImage(tex: WebGLTexture | null, img: HTMLImageElement): boolean {
    if (!gl) return false;
    try {
      gl.bindTexture(gl.TEXTURE_2D, tex);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, img);
      return true;
    } catch {
      return false; // cross-origin taint etc. → caller falls back
    }
  }

  function loadImage(url: string): Promise<HTMLImageElement | null> {
    return new Promise((resolve) => {
      const img = new Image();
      img.crossOrigin = 'anonymous';
      img.onload = () => resolve(img);
      img.onerror = () => resolve(null);
      img.src = url;
    });
  }

  // Await an existing <img> instead of fetching the URL again. The browser is
  // already loading the visible base image, so the colour texture costs zero
  // extra network requests — we just upload that same element once it's ready.
  function imageMatches(img: HTMLImageElement, url: string): boolean {
    try {
      return img.currentSrc === url || img.src === new URL(url, window.location.href).href;
    } catch {
      return img.currentSrc === url || img.src === url;
    }
  }

  function awaitImg(img: HTMLImageElement, expectedUrl: string): Promise<HTMLImageElement | null> {
    if (!imageMatches(img, expectedUrl)) return Promise.resolve(null);
    if (img.complete) return Promise.resolve(img.naturalWidth > 0 ? img : null);
    return new Promise((resolve) => {
      img.addEventListener('load', () => resolve(imageMatches(img, expectedUrl) && img.naturalWidth > 0 ? img : null), { once: true });
      img.addEventListener('error', () => resolve(null), { once: true });
    });
  }

  function resize() {
    if (!host || !canvas) return;
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const w = Math.max(1, Math.round(host.clientWidth * dpr));
    const h = Math.max(1, Math.round(host.clientHeight * dpr));
    if (canvas.width !== w || canvas.height !== h) {
      canvas.width = w;
      canvas.height = h;
    }
  }

  function draw() {
    if (!gl || !canvas || !colorTex) return;
    resize();
    gl.viewport(0, 0, canvas.width, canvas.height);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);

    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, colorTex);
    gl.uniform1i(uColor, 0);
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_2D, depthTex ?? colorTex);
    gl.uniform1i(uDepth, 1);

    gl.uniform1f(uHasDepth, hasDepth);
    gl.uniform1f(uImageAspect, imageAspect);
    gl.uniform1f(uCanvasAspect, canvas.width / canvas.height);
    gl.uniform2f(uMouse, curX, curY);
    gl.uniform1f(uIntensity, MAX_SHIFT * Math.max(0, Math.min(1, intensity)));

    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
  }

  function frame() {
    const dx = targetX - curX;
    const dy = targetY - curY;
    curX += dx * EASE;
    curY += dy * EASE;
    draw();
    // settle: once at rest and the pointer has left, stop burning frames.
    if (!pointerInside && Math.hypot(dx, dy) < 0.0006) {
      running = false;
      raf = 0;
      return;
    }
    raf = requestAnimationFrame(frame);
  }

  function kick() {
    if (running || destroyed || !visible || !colorTex) return;
    running = true;
    raf = requestAnimationFrame(frame);
  }

  function onMove(e: PointerEvent) {
    if (!host) return;
    if (!hostRect || hostRectDirty) updateHostRect();
    const r = hostRect;
    if (!r) return;
    if (!r.width || !r.height) return;
    targetX = ((e.clientX - r.left) / r.width) * 2 - 1;
    targetY = ((e.clientY - r.top) / r.height) * 2 - 1;
    pointerInside = true;
    kick();
  }
  function onLeave() {
    pointerInside = false;
    hostRect = null;
    hostRectDirty = true;
    targetX = 0;
    targetY = 0;
    kick(); // ease back to the resting frame
  }

  // Load (or reload) the colour + optional depth textures for a given pair of
  // sources. Idempotent per (src|depthSrc); a newer call supersedes an older
  // in-flight one via loadSeq. Reuses the persistent GL context — no teardown.
  async function loadTextures(colorSrc: string, depthSrc2: string | null) {
    if (!gl || destroyed) return;
    const key = `${colorSrc}|${depthSrc2 ?? ''}`;
    if (key === loadedKey) return;
    loadedKey = key;
    const seq = ++loadSeq;

    // Fade the canvas out while the new plate loads; the base <img> (its src is
    // bound reactively) shows through and fades the new photograph in.
    glReady = false;
    imageFailed = false;

    // Colour: reuse the visible base <img> when it's the same element/source,
    // else a fresh load (covers the no-DOM-yet edge).
    const colorImg = baseImg
      ? (await awaitImg(baseImg, colorSrc)) ?? await loadImage(colorSrc)
      : await loadImage(colorSrc);
    if (destroyed || seq !== loadSeq) return;
    if (!colorImg) { imageFailed = true; return; }

    imageAspect = colorImg.naturalWidth / Math.max(1, colorImg.naturalHeight);
    if (!colorTex) colorTex = makeTexture();
    if (!uploadImage(colorTex, colorImg)) return; // tainted → base <img> stays

    // Depth: optional, fetched separately (it's small and only present sometimes).
    hasDepth = 0;
    if (depthSrc2) {
      const depthImg = await loadImage(depthSrc2);
      if (destroyed || seq !== loadSeq) return;
      if (depthImg) {
        if (!depthTex) depthTex = makeTexture();
        if (uploadImage(depthTex, depthImg)) hasDepth = 1;
      }
    }

    draw();
    glReady = true; // fade the canvas in over the base <img>
    kick();
  }

  onMount(() => {
    if (reducedMotion || !canvas || !host || !src) return;

    gl =
      (canvas.getContext('webgl', { alpha: true, premultipliedAlpha: false, antialias: true }) as WebGLRenderingContext | null) ||
      (canvas.getContext('experimental-webgl', { alpha: true }) as WebGLRenderingContext | null);
    if (!gl) return; // base <img> stays visible — silent, correct fallback

    // ── program ────────────────────────────────────────────────────────────
    const vsrc = `
      attribute vec2 aPos;
      attribute vec2 aUv;
      varying vec2 vUv;
      void main() { vUv = aUv; gl_Position = vec4(aPos, 0.0, 1.0); }
    `;
    const fsrc = `
      precision mediump float;
      varying vec2 vUv;
      uniform sampler2D uColor;
      uniform sampler2D uDepth;
      uniform float uHasDepth;
      uniform float uImageAspect;
      uniform float uCanvasAspect;
      uniform vec2  uMouse;     // -1..1
      uniform float uIntensity; // max texture-space shift

      float luma(vec3 c) { return dot(c, vec3(0.299, 0.587, 0.114)); }

      float depthAt(vec2 uv) {
        if (uHasDepth > 0.5) return texture2D(uDepth, uv).r;
        // cheap separable-ish blur of luminance — depth is low-frequency, so a
        // 5-tap cross at this radius is enough to kill texture shimmer.
        float r = 0.012;
        float s = luma(texture2D(uColor, uv).rgb) * 0.36;
        s += luma(texture2D(uColor, uv + vec2(r, 0.0)).rgb) * 0.16;
        s += luma(texture2D(uColor, uv - vec2(r, 0.0)).rgb) * 0.16;
        s += luma(texture2D(uColor, uv + vec2(0.0, r)).rgb) * 0.16;
        s += luma(texture2D(uColor, uv - vec2(0.0, r)).rgb) * 0.16;
        return s;
      }

      void main() {
        // object-fit: contain — letterbox bands stay transparent so the
        // parchment mat shows through, exactly like the <img> did.
        vec2 scale = vec2(1.0);
        if (uImageAspect > uCanvasAspect) scale.y = uCanvasAspect / uImageAspect;
        else                              scale.x = uImageAspect / uCanvasAspect;

        vec2 imgUv = (vUv - 0.5) / scale + 0.5;
        if (imgUv.x < 0.0 || imgUv.x > 1.0 || imgUv.y < 0.0 || imgUv.y > 1.0) discard;

        float d = depthAt(imgUv);
        vec2 disp = uMouse * d * uIntensity;
        vec2 s = clamp(imgUv - disp, 0.0, 1.0); // clamp, not wrap → no ground bleed at edges
        gl_FragColor = texture2D(uColor, s);
      }
    `;

    function compile(type: number, source: string): WebGLShader | null {
      const sh = gl!.createShader(type);
      if (!sh) return null;
      gl!.shaderSource(sh, source);
      gl!.compileShader(sh);
      if (!gl!.getShaderParameter(sh, gl!.COMPILE_STATUS)) {
        gl!.deleteShader(sh);
        return null;
      }
      return sh;
    }

    const vs = compile(gl.VERTEX_SHADER, vsrc);
    const fs = compile(gl.FRAGMENT_SHADER, fsrc);
    if (!vs || !fs) { gl = null; return; }
    const prog = gl.createProgram();
    if (!prog) { gl = null; return; }
    gl.attachShader(prog, vs);
    gl.attachShader(prog, fs);
    gl.linkProgram(prog);
    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) { gl = null; return; }
    gl.useProgram(prog);

    // ── geometry: full-frame quad. uv (0,0) = top-left, matching image rows. ──
    const quad = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, quad);
    gl.bufferData(
      gl.ARRAY_BUFFER,
      new Float32Array([
        // aPos      aUv
        -1, -1, 0, 1,
         1, -1, 1, 1,
        -1,  1, 0, 0,
         1,  1, 1, 0,
      ]),
      gl.STATIC_DRAW,
    );
    const aPos = gl.getAttribLocation(prog, 'aPos');
    const aUv = gl.getAttribLocation(prog, 'aUv');
    gl.enableVertexAttribArray(aPos);
    gl.vertexAttribPointer(aPos, 2, gl.FLOAT, false, 16, 0);
    gl.enableVertexAttribArray(aUv);
    gl.vertexAttribPointer(aUv, 2, gl.FLOAT, false, 16, 8);

    uColor = gl.getUniformLocation(prog, 'uColor');
    uDepth = gl.getUniformLocation(prog, 'uDepth');
    uHasDepth = gl.getUniformLocation(prog, 'uHasDepth');
    uImageAspect = gl.getUniformLocation(prog, 'uImageAspect');
    uCanvasAspect = gl.getUniformLocation(prog, 'uCanvasAspect');
    uMouse = gl.getUniformLocation(prog, 'uMouse');
    uIntensity = gl.getUniformLocation(prog, 'uIntensity');

    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

    function handleContextLost(e: Event) {
      e.preventDefault();
      stopAnimation();
      glReady = false;
      initialized = false;
      loadedKey = '';
      colorTex = null;
      depthTex = null;
      gl = null;
    }

    function handleContextRestored() {
      // The base <img> remains visible. A full remount recreates the context and
      // shader program; avoid presenting a stale canvas after browser recovery.
      glReady = false;
    }

    canvas.addEventListener('webglcontextlost', handleContextLost);
    canvas.addEventListener('webglcontextrestored', handleContextRestored);
    host.addEventListener('pointerenter', updateHostRect);
    host.addEventListener('pointermove', onMove);
    host.addEventListener('pointerleave', onLeave);
    const scrollOptions = { passive: true, capture: true };
    window.addEventListener('scroll', markHostRectDirty, scrollOptions);
    window.addEventListener('resize', markHostRectDirty);

    const io = new IntersectionObserver(
      ([entry]) => {
        visible = entry.isIntersecting;
        if (visible) markHostRectDirty();
        if (visible) kick();
      },
      { threshold: 0 },
    );
    io.observe(host);

    const ro = new ResizeObserver(() => {
      markHostRectDirty();
      kick();
    });
    ro.observe(host);

    initialized = true;
    // Initial textures. Later src/depthSrc changes are driven by the $effect
    // below, which reuses this very context (no remount, no recompile).
    loadTextures(src, depthSrc);

    return () => {
      destroyed = true;
      stopAnimation();
      canvas?.removeEventListener('webglcontextlost', handleContextLost);
      canvas?.removeEventListener('webglcontextrestored', handleContextRestored);
      host?.removeEventListener('pointerenter', updateHostRect);
      host?.removeEventListener('pointermove', onMove);
      host?.removeEventListener('pointerleave', onLeave);
      window.removeEventListener('scroll', markHostRectDirty, scrollOptions);
      window.removeEventListener('resize', markHostRectDirty);
      io.disconnect();
      ro.disconnect();
      const ext = gl?.getExtension('WEBGL_lose_context');
      ext?.loseContext();
      gl = null;
    };
  });

  // Reload only the textures when the gallery image (or its depth map) changes —
  // the GL context, program and geometry above are untouched. Guarded so the
  // initial mount (handled in onMount) isn't loaded twice.
  $effect(() => {
    const s = src;
    const d = depthSrc;
    if (initialized && s) loadTextures(s, d);
  });

  $effect(() => {
    intensity;
    if (initialized && colorTex) {
      draw();
      kick();
    }
  });
</script>

<div
  bind:this={host}
  class="daguerreotype {className}"
  role="button"
  tabindex="0"
  aria-label={alt}
  onclick={() => onActivate?.()}
  onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onActivate?.(); } }}
>
  <!-- Base photograph: always present (SSR, view-transition, reduced-motion,
       no-WebGL fallback). The canvas fades in on top once it has drawn.
       crossorigin keeps the WebGL upload from this element CORS-clean (so the
       effect also works on a cross-origin media host) AND makes the texture
       reuse the same cached request as the visible image — one fetch, not two. -->
  {#if src && !imageFailed}
    <img bind:this={baseImg} class="daguerreotype-base" {src} {alt} class:is-hidden={glReady}
         crossorigin="anonymous" draggable="false" />
  {:else}
    <div class="daguerreotype-fallback" aria-hidden="true"></div>
  {/if}
  <canvas bind:this={canvas} class="daguerreotype-canvas" class:is-ready={glReady} aria-hidden="true"></canvas>
</div>

<style>
  .daguerreotype {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    cursor: zoom-in;
  }
  .daguerreotype-base,
  .daguerreotype-canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  .daguerreotype-base {
    object-fit: contain;
    user-select: none;
    -webkit-user-drag: none;
    transition: opacity 0.4s ease;
  }
  .daguerreotype-base.is-hidden {
    opacity: 0;
  }
  .daguerreotype-canvas {
    opacity: 0;
    transition: opacity 0.4s ease;
    pointer-events: none;
  }
  .daguerreotype-canvas.is-ready {
    opacity: 1;
  }
  .daguerreotype-fallback {
    width: 100%;
    height: 100%;
    background:
      radial-gradient(circle at 50% 28%, rgba(255, 255, 255, 0.5), transparent 48%),
      rgba(244, 236, 222, 0.75);
  }
</style>
