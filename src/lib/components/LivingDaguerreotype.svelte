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

  onMount(() => {
    if (reducedMotion || !canvas || !host || !src) return;

    const gl =
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
    if (!vs || !fs) return;
    const prog = gl.createProgram();
    if (!prog) return;
    gl.attachShader(prog, vs);
    gl.attachShader(prog, fs);
    gl.linkProgram(prog);
    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) return;
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

    const uColor = gl.getUniformLocation(prog, 'uColor');
    const uDepth = gl.getUniformLocation(prog, 'uDepth');
    const uHasDepth = gl.getUniformLocation(prog, 'uHasDepth');
    const uImageAspect = gl.getUniformLocation(prog, 'uImageAspect');
    const uCanvasAspect = gl.getUniformLocation(prog, 'uCanvasAspect');
    const uMouse = gl.getUniformLocation(prog, 'uMouse');
    const uIntensity = gl.getUniformLocation(prog, 'uIntensity');

    function makeTexture(): WebGLTexture | null {
      const tex = gl!.createTexture();
      gl!.bindTexture(gl!.TEXTURE_2D, tex);
      gl!.texParameteri(gl!.TEXTURE_2D, gl!.TEXTURE_WRAP_S, gl!.CLAMP_TO_EDGE);
      gl!.texParameteri(gl!.TEXTURE_2D, gl!.TEXTURE_WRAP_T, gl!.CLAMP_TO_EDGE);
      gl!.texParameteri(gl!.TEXTURE_2D, gl!.TEXTURE_MIN_FILTER, gl!.LINEAR);
      gl!.texParameteri(gl!.TEXTURE_2D, gl!.TEXTURE_MAG_FILTER, gl!.LINEAR);
      return tex;
    }

    let imageAspect = 1;
    let destroyed = false;
    let colorTex: WebGLTexture | null = null;
    let depthTex: WebGLTexture | null = null;
    let hasDepth = 0;

    function uploadImage(tex: WebGLTexture | null, img: HTMLImageElement): boolean {
      try {
        gl!.bindTexture(gl!.TEXTURE_2D, tex);
        gl!.texImage2D(gl!.TEXTURE_2D, 0, gl!.RGBA, gl!.RGBA, gl!.UNSIGNED_BYTE, img);
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
    function awaitImg(img: HTMLImageElement): Promise<HTMLImageElement | null> {
      if (img.complete) return Promise.resolve(img.naturalWidth > 0 ? img : null);
      return new Promise((resolve) => {
        img.addEventListener('load', () => resolve(img.naturalWidth > 0 ? img : null), { once: true });
        img.addEventListener('error', () => resolve(null), { once: true });
      });
    }

    // ── pointer → eased camera offset ────────────────────────────────────────
    let targetX = 0, targetY = 0, curX = 0, curY = 0;
    let pointerInside = false;
    let visible = true;
    let running = false;
    let raf = 0;

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
      if (!canvas) return;
      resize();
      gl!.viewport(0, 0, canvas.width, canvas.height);
      gl!.clearColor(0, 0, 0, 0);
      gl!.clear(gl!.COLOR_BUFFER_BIT);

      gl!.activeTexture(gl!.TEXTURE0);
      gl!.bindTexture(gl!.TEXTURE_2D, colorTex);
      gl!.uniform1i(uColor, 0);
      gl!.activeTexture(gl!.TEXTURE1);
      gl!.bindTexture(gl!.TEXTURE_2D, depthTex ?? colorTex);
      gl!.uniform1i(uDepth, 1);

      gl!.uniform1f(uHasDepth, hasDepth);
      gl!.uniform1f(uImageAspect, imageAspect);
      gl!.uniform1f(uCanvasAspect, canvas.width / canvas.height);
      gl!.uniform2f(uMouse, curX, curY);
      gl!.uniform1f(uIntensity, MAX_SHIFT * Math.max(0, Math.min(1, intensity)));

      gl!.drawArrays(gl!.TRIANGLE_STRIP, 0, 4);
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
      if (running || destroyed || !visible) return;
      running = true;
      raf = requestAnimationFrame(frame);
    }

    function onMove(e: PointerEvent) {
      if (!host) return;
      const r = host.getBoundingClientRect();
      targetX = ((e.clientX - r.left) / r.width) * 2 - 1;
      targetY = ((e.clientY - r.top) / r.height) * 2 - 1;
      pointerInside = true;
      kick();
    }
    function onLeave() {
      pointerInside = false;
      targetX = 0;
      targetY = 0;
      kick(); // ease back to the resting frame
    }

    host.addEventListener('pointermove', onMove);
    host.addEventListener('pointerleave', onLeave);

    const io = new IntersectionObserver(
      ([entry]) => {
        visible = entry.isIntersecting;
        if (visible) kick();
      },
      { threshold: 0 },
    );
    io.observe(host);

    const ro = new ResizeObserver(() => kick());
    ro.observe(host);

    // ── boot: load colour (required) + optional depth, then reveal canvas ────
    (async () => {
      // Reuse the visible base <img> as the texture source (no second fetch of
      // the same photo). Fall back to a fresh load only if it isn't bound yet.
      const colorImg = baseImg ? await awaitImg(baseImg) : await loadImage(src!);
      if (destroyed) return;
      if (!colorImg) { imageFailed = true; return; }
      imageAspect = colorImg.naturalWidth / Math.max(1, colorImg.naturalHeight);
      colorTex = makeTexture();
      if (!uploadImage(colorTex, colorImg)) return; // tainted → base <img> stays

      if (depthSrc) {
        const depthImg = await loadImage(depthSrc);
        if (destroyed) return;
        if (depthImg) {
          depthTex = makeTexture();
          if (uploadImage(depthTex, depthImg)) hasDepth = 1;
        }
      }

      gl!.enable(gl!.BLEND);
      gl!.blendFunc(gl!.SRC_ALPHA, gl!.ONE_MINUS_SRC_ALPHA);
      draw();
      glReady = true; // fade the canvas in over the base <img>
      kick();
    })();

    return () => {
      destroyed = true;
      if (raf) cancelAnimationFrame(raf);
      host?.removeEventListener('pointermove', onMove);
      host?.removeEventListener('pointerleave', onLeave);
      io.disconnect();
      ro.disconnect();
      const ext = gl.getExtension('WEBGL_lose_context');
      ext?.loseContext();
    };
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
       no-WebGL fallback). The canvas fades in on top once it has drawn. -->
  {#if src && !imageFailed}
    <!-- crossorigin keeps the WebGL upload from this element CORS-clean (so the
         effect also works on a cross-origin media host) AND makes the texture
         reuse the same cached request as the visible image — one fetch, not two. -->
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
