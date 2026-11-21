import { useEffect, useRef } from "react";

interface Star {
  x: number;
  y: number;
  r: number;
  phase: number;
  speed: number;
  tint: number;
}

function hexToRgb(hex: string): [number, number, number] {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex);
  if (!m) return [168, 109, 255];
  const n = parseInt(m[1], 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

/** Animated night sky with a crescent moon, drawn for Luna. */
export function SkyCanvas({ accent, effect }: { accent: string; effect: number }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const opts = useRef({ accent, effect });
  opts.current = { accent, effect };

  useEffect(() => {
    const c = canvas.current!;
    const ctx = c.getContext("2d")!;
    const reduce = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
    let stars: Star[] = [];
    let w = 0;
    let h = 0;
    let raf = 0;
    let last = 0;
    let shoot: { x: number; y: number; vx: number; vy: number; life: number } | null = null;

    const resize = () => {
      const dpr = Math.min(window.devicePixelRatio || 1, 1.5);
      w = c.clientWidth;
      h = c.clientHeight;
      c.width = Math.max(1, Math.round(w * dpr));
      c.height = Math.max(1, Math.round(h * dpr));
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      const count = Math.round((w * h) / 5200);
      let seed = 7;
      const rnd = () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
      stars = Array.from({ length: count }, () => ({
        x: rnd() * w,
        y: rnd() * h,
        r: rnd() < 0.08 ? 1.1 + rnd() * 0.9 : 0.35 + rnd() * 0.7,
        phase: rnd() * Math.PI * 2,
        speed: 0.4 + rnd() * 1.4,
        tint: rnd(),
      }));
    };

    const drawMoon = (t: number) => {
      const mx = w * 0.82;
      const my = h * 0.2 + Math.sin(t * 0.00005) * 4;
      const r = Math.max(34, Math.min(w, h) * 0.075);
      const [ar, ag, ab] = hexToRgb(opts.current.accent);
      const glow = 0.25 + (opts.current.effect / 100) * 0.35;
      const halo = ctx.createRadialGradient(mx, my, r * 0.6, mx, my, r * 4.2);
      halo.addColorStop(0, `rgba(${ar},${ag},${ab},${glow})`);
      halo.addColorStop(1, "rgba(0,0,0,0)");
      ctx.fillStyle = halo;
      ctx.beginPath();
      ctx.arc(mx, my, r * 4.2, 0, Math.PI * 2);
      ctx.fill();

      ctx.save();
      ctx.beginPath();
      ctx.arc(mx, my, r, 0, Math.PI * 2);
      ctx.clip();
      const body = ctx.createRadialGradient(mx - r * 0.4, my - r * 0.4, r * 0.1, mx, my, r * 1.1);
      body.addColorStop(0, "#fbf7ff");
      body.addColorStop(0.6, "#e3d6f7");
      body.addColorStop(1, "#b9a6d8");
      ctx.fillStyle = body;
      ctx.fillRect(mx - r, my - r, r * 2, r * 2);
      ctx.fillStyle = "rgba(120,100,160,0.18)";
      for (const [dx, dy, cr] of [
        [-0.3, -0.1, 0.18],
        [0.15, 0.35, 0.12],
        [0.35, -0.3, 0.09],
        [-0.05, 0.05, 0.07],
      ]) {
        ctx.beginPath();
        ctx.arc(mx + dx * r, my + dy * r, cr * r, 0, Math.PI * 2);
        ctx.fill();
      }
      // The shadow that makes the crescent.
      ctx.fillStyle = "rgba(13,10,22,0.93)";
      ctx.beginPath();
      ctx.arc(mx + r * 0.55, my - r * 0.25, r * 0.98, 0, Math.PI * 2);
      ctx.fill();
      ctx.restore();
    };

    const frame = (t: number) => {
      raf = requestAnimationFrame(frame);
      if (document.hidden || t - last < 40) return;
      last = t;
      ctx.clearRect(0, 0, w, h);
      const [ar, ag, ab] = hexToRgb(opts.current.accent);
      const neb = ctx.createRadialGradient(w * 0.15, h * 0.95, 10, w * 0.15, h * 0.95, Math.max(w, h) * 0.7);
      neb.addColorStop(0, `rgba(${ar},${ag},${ab},${0.1 + opts.current.effect / 900})`);
      neb.addColorStop(1, "rgba(0,0,0,0)");
      ctx.fillStyle = neb;
      ctx.fillRect(0, 0, w, h);

      for (const s of stars) {
        const tw = reduce ? 0.8 : 0.55 + 0.45 * Math.sin(s.phase + t * 0.001 * s.speed);
        const x = reduce ? s.x : (s.x + t * 0.002 * s.r) % w;
        ctx.globalAlpha = tw;
        ctx.fillStyle = s.tint > 0.85 ? `rgb(${ar},${ag},${ab})` : s.tint > 0.7 ? "#dfe6ff" : "#f5f2ff";
        ctx.beginPath();
        ctx.arc(x, s.y, s.r, 0, Math.PI * 2);
        ctx.fill();
      }
      ctx.globalAlpha = 1;

      if (!reduce) {
        if (!shoot && Math.random() < 0.004) {
          shoot = { x: Math.random() * w * 0.7, y: Math.random() * h * 0.35, vx: 7 + Math.random() * 4, vy: 2.5 + Math.random() * 2, life: 1 };
        }
        if (shoot) {
          const g = ctx.createLinearGradient(shoot.x, shoot.y, shoot.x - shoot.vx * 10, shoot.y - shoot.vy * 10);
          g.addColorStop(0, `rgba(255,255,255,${shoot.life})`);
          g.addColorStop(1, "rgba(255,255,255,0)");
          ctx.strokeStyle = g;
          ctx.lineWidth = 1.4;
          ctx.beginPath();
          ctx.moveTo(shoot.x, shoot.y);
          ctx.lineTo(shoot.x - shoot.vx * 10, shoot.y - shoot.vy * 10);
          ctx.stroke();
          shoot.x += shoot.vx;
          shoot.y += shoot.vy;
          shoot.life -= 0.025;
          if (shoot.life <= 0) shoot = null;
        }
      }
      drawMoon(t);
    };

    resize();
    const ro = new ResizeObserver(resize);
    ro.observe(c);
    raf = requestAnimationFrame(frame);
    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
    };
  }, []);

  return <canvas ref={canvas} className="sky" aria-hidden="true" />;
}
