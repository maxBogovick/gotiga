<script lang="ts">
  /**
   * Raking Light — "examining the specimen under a conservator's grazing lamp".
   *
   * A museum technique: the restorer walks a light source at a shallow,
   * grazing angle across a surface, and the relief of the object emerges in
   * light and shadow — tool marks, seams, fingerprints, the topography paint
   * normally hides. Here the lamp is the pointer: a low torch whose planar
   * position tracks the cursor, raking a luminous pool across the plate while
   * the rest of the photograph stays its plain, untouched self.
   *
   * How the relief is recovered (the demonstrable part):
   *  - We treat a grayscale field as a height map h(uv). Surface normals come
   *    from its gradient by central differences: N = normalize(-∂h, +∂h, 1/k).
   *  - The lamp is a point light a short height `uGrazing` above the surface at
   *    the cursor's planar position. Low height ⇒ near-horizontal incidence ⇒
   *    long shadows off every slope — the defining property of raking light.
   *  - Shading is Lambertian diffuse minus the *flat-surface* response, so a
   *    perfectly flat region keeps the photo unchanged and only genuine relief
   *    (deviation of N from straight-up) brightens or darkens. A tight grazing
   *    specular adds the characteristic glint that catches raised edges.
   *  - Directly under the lamp the incidence is steep (light ≈ overhead) so
   *    relief flattens there, exactly as a real handheld lamp behaves; the
   *    revealing happens in the raked ring around the hotspot.
   *
   * Height source, in order of preference:
   *  1. `heightSrc` — a precomputed depth/height map (the same grayscale media
   *     variant the Living Daguerreotype uses). The correct, albedo-free path.
   *  2. fallback — luminance of the colour image, lightly blurred in-shader.
   *     Lets the whole archive participate today; note it conflates dark paint
   *     with depressions, so the depth map is always preferred when present.
   *
   * Design contract (it must never read as a "product 3D viewer"):
   *  - at rest (pointer away) the canvas output is byte-for-byte the original
   *    photograph — the effect eases to zero, nothing lingers;
   *  - no chrome of its own; it fills the stage exactly like the <img> it
   *    replaces, so the mat / vignette / grain overlays stay as siblings;
   *  - graceful by construction: a plain <img> sits underneath for SSR, the
   *    card→detail view-transition, reduced-motion and WebGL-less browsers.
   *
   * Lifecycle mirrors LivingDaguerreotype: the GL context, program and geometry
   * are built ONCE in onMount and persist; switching the gallery image reloads
   * only the textures (via the $effect on src/heightSrc) — no recompile, no
   * context-loss churn while paging ←/→.
   */
  import { onMount } from 'svelte';

  let {
    src,
    heightSrc = null,
    alt = '',
    intensity = 0.6,
    class: className = '',
    onActivate,
  }: {
    src?: string | null;
    /** Precomputed grayscale depth/height map; falls back to colour luminance. */
    heightSrc?: string | null;
    alt?: string;
    /** 0..1 — how strongly relief is exaggerated (scales the normal slope). */
    intensity?: number;
    class?: string;
    onActivate?: () => void;
  } = $props();

  // Height of the torch above the surface plane. Small ⇒ grazing incidence.
  const GRAZING = 0.2;
  const EASE = 0.12;          // pointer → eased lamp position
  const ACT_EASE = 0.08;      // effect engage / release

  let host = $state<HTMLDivElement>();
  let canvas = $state<HTMLCanvasElement>();
  let baseImg = $state<HTMLImageElement>(); // visible <img>; reused as the GL texture source
  let imageFailed = $state(false);
  let glReady = $state(false); // canvas takes over from the base <img> only once it has drawn

  const reducedMotion =
    typeof window !== 'undefined' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  // ── Persistent GL state (built once, reused across image switches) ──────────
  let gl: WebGLRenderingContext | null = null;
  let uColor: WebGLUniformLocation | null = null;
  let uHeight: WebGLUniformLocation | null = null;
  let uHasHeight: WebGLUniformLocation | null = null;
  let uImageAspect: WebGLUniformLocation | null = null;
  let uCanvasAspect: WebGLUniformLocation | null = null;
  let uMouse: WebGLUniformLocation | null = null;
  let uActivation: WebGLUniformLocation | null = null;
  let uTexel: WebGLUniformLocation | null = null;
  let uRelief: WebGLUniformLocation | null = null;
  let uGrazing: WebGLUniformLocation | null = null;

  let colorTex: WebGLTexture | null = null;
  let heightTex: WebGLTexture | null = null;
  let hasHeight = 0;
  let imageAspect = 1;
  let texelX = 1 / 1024, texelY = 1 / 1024; // updated from the loaded image

  // pointer → eased lamp position; activation eases the effect in/out
  let targetX = 0, targetY = 0, curX = 0, curY = 0;
  let targetAct = 0, curAct = 0;
  let pointerInside = false;
  let visible = true;
  let running = false;
  let raf = 0;
  let hostRect: DOMRect | null = null;
  let hostRectDirty = true;

  let destroyed = false;
  let initialized = false; // GL context + program ready
  let loadedKey = '';      // de-dupes the (src|heightSrc) currently loaded/loading
  let loadSeq = 0;         // supersedes in-flight loads when the image changes fast
  let isPointerFine = $state(true);

  function handleActivate() {
    if (isPointerFine) onActivate?.();
  }

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

  function reliefStrength() {
    // grad samples are in 0..1; this maps intensity → a slope multiplier that
    // reads as tactile relief without tipping into a plastic, embossed look.
    return 8 + Math.max(0, Math.min(1, intensity)) * 28;
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

  function isSameOrigin(url: string): boolean {
    try {
      return new URL(url, window.location.href).origin === window.location.origin;
    } catch {
      return false;
    }
  }

  // Await an existing <img> instead of refetching: only when same-origin, so
  // a prod-dump URL on ritunia.com can still paint the photograph (no CORS
  // on the visible <img>) while WebGL quietly sits out.
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
    gl.bindTexture(gl.TEXTURE_2D, heightTex ?? colorTex);
    gl.uniform1i(uHeight, 1);

    gl.uniform1f(uHasHeight, hasHeight);
    gl.uniform1f(uImageAspect, imageAspect);
    gl.uniform1f(uCanvasAspect, canvas.width / canvas.height);
    gl.uniform2f(uMouse, curX, curY);
    gl.uniform1f(uActivation, curAct);
    gl.uniform2f(uTexel, texelX, texelY);
    gl.uniform1f(uRelief, reliefStrength());
    gl.uniform1f(uGrazing, GRAZING);

    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    if ((host?.clientWidth ?? 0) > 8 && (host?.clientHeight ?? 0) > 8) glReady = true;
  }

  function frame() {
    const dx = targetX - curX;
    const dy = targetY - curY;
    const da = targetAct - curAct;
    curX += dx * EASE;
    curY += dy * EASE;
    curAct += da * ACT_EASE;
    draw();
    // settle: once the lamp is at rest and the effect has fully released, stop
    // burning frames. The final frame at curAct≈0 equals the untouched photo.
    if (!pointerInside && Math.hypot(dx, dy) < 0.0006 && Math.abs(da) < 0.0015) {
      curAct = targetAct; // snap to exact 0 so the last drawn frame is the plain image
      draw();
      running = false;
      raf = 0;
      return;
    }
    raf = requestAnimationFrame(frame);
  }

  function kick() {
    if (running || destroyed || !colorTex) return;
    if (!visible) {
      draw();
      return;
    }
    running = true;
    raf = requestAnimationFrame(frame);
  }

  function onMove(e: PointerEvent) {
    if (!host) return;
    if (!hostRect || hostRectDirty) updateHostRect();
    const r = hostRect;
    if (!r || !r.width || !r.height) return;
    targetX = ((e.clientX - r.left) / r.width) * 2 - 1;
    targetY = ((e.clientY - r.top) / r.height) * 2 - 1;
    pointerInside = true;
    targetAct = 1;
    kick();
  }
  function onEnter(e: PointerEvent) {
    updateHostRect();
    // seed the lamp at the entry point so it doesn't sweep in from the corner
    onMove(e);
  }
  function onLeave() {
    pointerInside = false;
    hostRect = null;
    hostRectDirty = true;
    targetAct = 0; // ease the relief away → back to the plain photograph
    kick();
  }

  // Load (or reload) the colour + optional height textures for a given pair of
  // sources. Idempotent per (src|heightSrc); a newer call supersedes an older
  // in-flight one via loadSeq. Reuses the persistent GL context — no teardown.
  async function loadTextures(colorSrc: string, heightSrc2: string | null) {
    if (!gl || destroyed) return;
    const key = `${colorSrc}|${heightSrc2 ?? ''}`;
    if (key === loadedKey) return;
    loadedKey = key;
    const seq = ++loadSeq;

    glReady = false;
    imageFailed = false;

    const colorImg = baseImg && isSameOrigin(colorSrc)
      ? (await awaitImg(baseImg, colorSrc)) ?? await loadImage(colorSrc)
      : await loadImage(colorSrc);
    if (destroyed || seq !== loadSeq) return;
    if (!colorImg) return;

    imageAspect = colorImg.naturalWidth / Math.max(1, colorImg.naturalHeight);
    texelX = 1 / Math.max(1, colorImg.naturalWidth);
    texelY = 1 / Math.max(1, colorImg.naturalHeight);
    if (!colorTex) colorTex = makeTexture();
    if (!uploadImage(colorTex, colorImg)) return; // tainted → base <img> stays

    hasHeight = 0;
    if (heightSrc2) {
      const heightImg = await loadImage(heightSrc2);
      if (destroyed || seq !== loadSeq) return;
      if (heightImg) {
        if (!heightTex) heightTex = makeTexture();
        if (uploadImage(heightTex, heightImg)) hasHeight = 1;
      }
    }

    draw();
    kick();
  }

  onMount(() => {
    isPointerFine = window.matchMedia('(pointer: fine)').matches;
    if (reducedMotion || !canvas || !host || !src) return;

    // preserveDrawingBuffer: like the daguerreotype, this canvas draws then
    // parks its rAF loop. Without buffer preservation the browser discards
    // those pixels on a composite not preceded by a redraw (view-transition
    // snapshot, the canvas's own opacity fade), leaving the stage blank until
    // the next pointermove. Cheap here; correctness over a micro-optimisation.
    gl =
      (canvas.getContext('webgl', { alpha: true, premultipliedAlpha: false, antialias: true, preserveDrawingBuffer: true }) as WebGLRenderingContext | null) ||
      (canvas.getContext('experimental-webgl', { alpha: true, preserveDrawingBuffer: true }) as WebGLRenderingContext | null);
    if (!gl) return; // base <img> stays visible — silent, correct fallback

    const vsrc = `
      attribute vec2 aPos;
      attribute vec2 aUv;
      varying vec2 vUv;
      void main() { vUv = aUv; gl_Position = vec4(aPos, 0.0, 1.0); }
    `;
    const fsrc = `
      precision highp float;
      varying vec2 vUv;
      uniform sampler2D uColor;
      uniform sampler2D uHeight;
      uniform float uHasHeight;
      uniform float uImageAspect;
      uniform float uCanvasAspect;
      uniform vec2  uMouse;      // -1..1 over the stage; eased lamp position
      uniform float uActivation; // 0..1 effect engagement (eased)
      uniform vec2  uTexel;      // 1/imgW, 1/imgH
      uniform float uRelief;     // normal slope multiplier
      uniform float uGrazing;    // lamp height above the plane (small = grazing)

      float luma(vec3 c) { return dot(c, vec3(0.299, 0.587, 0.114)); }

      // Height field h(uv). Depth map when present; else a lightly blurred
      // luminance so the relief survives but pixel noise doesn't.
      float heightAt(vec2 uv) {
        if (uHasHeight > 0.5) return texture2D(uHeight, uv).r;
        vec2 r = uTexel * 1.25;
        float s  = luma(texture2D(uColor, uv).rgb)              * 0.40;
        s += luma(texture2D(uColor, uv + vec2(r.x, 0.0)).rgb)   * 0.15;
        s += luma(texture2D(uColor, uv - vec2(r.x, 0.0)).rgb)   * 0.15;
        s += luma(texture2D(uColor, uv + vec2(0.0, r.y)).rgb)   * 0.15;
        s += luma(texture2D(uColor, uv - vec2(0.0, r.y)).rgb)   * 0.15;
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

        vec3 base = texture2D(uColor, imgUv).rgb;

        // ── surface normal from the height field (central differences) ──
        // Steps are a fixed texel count on each axis, so the gradient is
        // isotropic in pixel space; the per-axis constant folds into uRelief.
        vec2 e = uTexel * 1.5;
        float hL = heightAt(imgUv - vec2(e.x, 0.0));
        float hR = heightAt(imgUv + vec2(e.x, 0.0));
        float hD = heightAt(imgUv - vec2(0.0, e.y));
        float hU = heightAt(imgUv + vec2(0.0, e.y));
        vec2 grad = vec2(hR - hL, hU - hD);
        vec3 N = normalize(vec3(-grad * uRelief, 1.0));

        // ── lamp: a low point light at the cursor's planar position ──
        // aspect-correct image space so the luminous pool stays circular.
        vec2 q   = vec2(imgUv.x * uImageAspect, imgUv.y);
        vec2 lUv = uMouse * 0.5 / scale + 0.5;          // cursor → image uv
        vec2 lq  = vec2(lUv.x * uImageAspect, lUv.y);
        vec2 toL = lq - q;
        float dist = length(toL);
        vec3 L = normalize(vec3(toL, uGrazing));

        // Subtract the flat-surface response so flat regions keep the photo
        // unchanged and only true relief (N tilted off vertical) modulates it.
        float flatResp = L.z;                 // = dot(vec3(0,0,1), L)
        float lambert  = max(dot(N, L), 0.0);

        // Tight grazing specular — the glint that catches raised edges.
        vec3 V = vec3(0.0, 0.0, 1.0);
        vec3 H = normalize(L + V);
        float spec = pow(max(dot(N, H), 0.0), 32.0);

        // Luminous pool that follows the lamp; soft, never crushing the rest.
        float pool = exp(-dist * dist * 4.0);

        float relief = 1.0
          + (lambert - flatResp) * (2.6 * (0.32 + pool))
          + spec * pool * 1.35;

        vec3 lit = base * clamp(relief, 0.0, 4.0);

        // Engage smoothly; at activation 0 the output equals the plain photo.
        float k = uActivation * (0.42 + 0.58 * pool);
        gl_FragColor = vec4(mix(base, lit, k), 1.0);
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
    uHeight = gl.getUniformLocation(prog, 'uHeight');
    uHasHeight = gl.getUniformLocation(prog, 'uHasHeight');
    uImageAspect = gl.getUniformLocation(prog, 'uImageAspect');
    uCanvasAspect = gl.getUniformLocation(prog, 'uCanvasAspect');
    uMouse = gl.getUniformLocation(prog, 'uMouse');
    uActivation = gl.getUniformLocation(prog, 'uActivation');
    uTexel = gl.getUniformLocation(prog, 'uTexel');
    uRelief = gl.getUniformLocation(prog, 'uRelief');
    uGrazing = gl.getUniformLocation(prog, 'uGrazing');

    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

    function handleContextLost(e: Event) {
      e.preventDefault();
      stopAnimation();
      glReady = false;
      initialized = false;
      loadedKey = '';
      colorTex = null;
      heightTex = null;
      gl = null;
    }
    function handleContextRestored() {
      glReady = false; // a full remount recreates context + program
    }

    // After a view transition the live canvas can hold a stale/blank buffer
    // until something forces a fresh draw; the layout fires this once the
    // transition settles so the plate redraws without user input.
    function onExternalRedraw() {
      markHostRectDirty();
      draw();
      kick();
    }
    window.addEventListener('gotiga:redraw', onExternalRedraw);

    canvas.addEventListener('webglcontextlost', handleContextLost);
    canvas.addEventListener('webglcontextrestored', handleContextRestored);
    host.addEventListener('pointerenter', onEnter);
    host.addEventListener('pointermove', onMove);
    host.addEventListener('pointerleave', onLeave);
    const scrollOptions = { passive: true, capture: true } as const;
    window.addEventListener('scroll', markHostRectDirty, scrollOptions);
    window.addEventListener('resize', markHostRectDirty);

    const io = new IntersectionObserver(
      ([entry]) => {
        visible = entry.isIntersecting;
        if (visible) { markHostRectDirty(); kick(); }
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
    loadTextures(src, heightSrc);

    return () => {
      destroyed = true;
      stopAnimation();
      window.removeEventListener('gotiga:redraw', onExternalRedraw);
      canvas?.removeEventListener('webglcontextlost', handleContextLost);
      canvas?.removeEventListener('webglcontextrestored', handleContextRestored);
      host?.removeEventListener('pointerenter', onEnter);
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

  // Reload only the textures when the gallery image (or its height map) changes —
  // context, program and geometry are untouched. Guarded so the initial mount
  // (handled in onMount) isn't loaded twice.
  $effect(() => {
    const s = src;
    const h = heightSrc;
    if (initialized && s) loadTextures(s, h);
  });

  $effect(() => {
    intensity;
    if (initialized && colorTex) { draw(); kick(); }
  });
</script>

<div
  bind:this={host}
  class="raking {className}"
  class:raking--zoomable={isPointerFine && !!onActivate}
  role={isPointerFine && onActivate ? 'button' : 'img'}
  tabindex={isPointerFine && onActivate ? 0 : undefined}
  aria-label={alt}
  onclick={handleActivate}
  onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleActivate(); } }}
>
  <!-- Base photograph: always present. No crossorigin — a failed CORS check
       on the visible <img> is a broken-image icon, not a quiet GL fallback. -->
  {#if src && !imageFailed}
    <img bind:this={baseImg} class="raking-base" {src} {alt}
         draggable="false"
         onerror={() => (imageFailed = true)} />
  {:else}
    <div class="raking-fallback" aria-hidden="true"></div>
  {/if}
  <canvas bind:this={canvas} class="raking-canvas" class:is-ready={glReady} aria-hidden="true"></canvas>
</div>

<style>
  .raking {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
  }
  .raking--zoomable {
    cursor: zoom-in;
  }
  .raking-base,
  .raking-canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  .raking-base {
    object-fit: contain;
    user-select: none;
    -webkit-user-drag: none;
  }
  .raking-canvas {
    opacity: 0;
    transition: opacity 0.4s ease;
    pointer-events: none;
  }
  .raking-canvas.is-ready {
    opacity: 1;
  }
  .raking-fallback {
    width: 100%;
    height: 100%;
    background:
      radial-gradient(circle at 50% 28%, rgba(255, 255, 255, 0.5), transparent 48%),
      rgba(244, 236, 222, 0.75);
  }
</style>
