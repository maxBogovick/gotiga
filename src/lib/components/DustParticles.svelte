<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D | null;
  let animationFrameId: number;
  let particles: Particle[] = [];
  let mouse = { x: -1000, y: -1000 };
  
  const PARTICLE_COUNT = 80;
  const CONNECTION_DISTANCE = 100;
  const MOUSE_RADIUS = 150;

  class Particle {
    x: number;
    y: number;
    vx: number;
    vy: number;
    size: number;
    baseX: number;
    baseY: number;
    density: number;
    alpha: number;

    constructor(w: number, h: number) {
      this.x = Math.random() * w;
      this.y = Math.random() * h;
      this.vx = (Math.random() - 0.5) * 0.5; // Slow drift velocity
      this.vy = (Math.random() - 0.5) * 0.5;
      this.size = Math.random() * 2 + 0.5;
      this.baseX = this.x;
      this.baseY = this.y;
      this.density = (Math.random() * 30) + 1;
      this.alpha = Math.random() * 0.5 + 0.1;
    }

    update(w: number, h: number) {
      // Mouse interaction
      let dx = mouse.x - this.x;
      let dy = mouse.y - this.y;
      let distance = Math.sqrt(dx*dx + dy*dy);
      
      // Repulsion force
      let forceDirectionX = dx / distance;
      let forceDirectionY = dy / distance;
      let maxDistance = MOUSE_RADIUS;
      let force = (maxDistance - distance) / maxDistance;
      let directionX = forceDirectionX * force * this.density;
      let directionY = forceDirectionY * force * this.density;

      if (distance < MOUSE_RADIUS) {
        this.x -= directionX;
        this.y -= directionY;
      } else {
          // Return to drift
          if (this.x !== this.baseX) {
              let dx = this.x - this.baseX;
              this.x -= dx/20; // slow return? No, let's just drift.
          }
          if (this.y !== this.baseY) {
               let dy = this.y - this.baseY;
               this.y -= dy/20;
          }
          
          // Constant drift
          this.x += this.vx;
          this.y += this.vy;
      }
      
      // Wrap around screen
      if (this.x < 0) this.x = w;
      else if (this.x > w) this.x = 0;
      
      if (this.y < 0) this.y = h;
      else if (this.y > h) this.y = 0;
    }

    draw(ctx: CanvasRenderingContext2D) {
      ctx.fillStyle = `rgba(198, 95, 60, ${this.alpha})`; // Cabinet bone color
      ctx.beginPath();
      ctx.arc(this.x, this.y, this.size, 0, Math.PI * 2);
      ctx.closePath();
      ctx.fill();
    }
  }

  function init() {
    if (!canvas) return;
    particles = [];
    for (let i = 0; i < PARTICLE_COUNT; i++) {
      particles.push(new Particle(canvas.width, canvas.height));
    }
  }

  function animate() {
    if (!ctx || !canvas) return;
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    
    for (let i = 0; i < particles.length; i++) {
      particles[i].update(canvas.width, canvas.height);
      particles[i].draw(ctx);
    }
    
    // Optional: Draw subtle connections for "web" feel, but very faint
    // Keeping it simple "dust" for now is better for performance and atmosphere.
    
    animationFrameId = requestAnimationFrame(animate);
  }

  function handleResize() {
    if (!canvas) return;
    canvas.width = window.innerWidth;
    canvas.height = window.innerHeight;
    init();
  }
  
  function handleMouseMove(e: MouseEvent) {
      mouse.x = e.x;
      mouse.y = e.y;
  }

  function handleTouchMove(e: TouchEvent) {
      if (e.touches.length > 0) {
          mouse.x = e.touches[0].clientX;
          mouse.y = e.touches[0].clientY;
      }
  }

  function handleTouchEnd() {
      mouse.x = -1000;
      mouse.y = -1000;
  }

  onMount(() => {
    ctx = canvas.getContext('2d');
    canvas.width = window.innerWidth;
    canvas.height = window.innerHeight;

    window.addEventListener('resize', handleResize);
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('touchmove', handleTouchMove, { passive: true });
    window.addEventListener('touchend', handleTouchEnd);

    init();
    animate();
  });

  onDestroy(() => {
    if (typeof window !== 'undefined') {
        window.removeEventListener('resize', handleResize);
        window.removeEventListener('mousemove', handleMouseMove);
        window.removeEventListener('touchmove', handleTouchMove);
        window.removeEventListener('touchend', handleTouchEnd);
        cancelAnimationFrame(animationFrameId);
    }
  });
</script>

<canvas
  bind:this={canvas}
  class="fixed inset-0 pointer-events-none z-50 mix-blend-screen opacity-60"
></canvas>
