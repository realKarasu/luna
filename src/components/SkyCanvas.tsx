import { useEffect, useRef } from "react";
import crescent from "../assets/sky-crescent.jpg";
import nebula from "../assets/sky-nebula.jpg";

/** The painted skies Nebula can show, by the `theme.background` value that picks them. */
export const SKY_ART = { sky: crescent, nebula } as const;
export type SkyArt = keyof typeof SKY_ART;

interface Star {
  x: number;
  y: number;
  r: number;
  phase: number;
  speed: number;
}

/**
 * Night sky behind the Nebula skin: a painted picture that drifts very slowly,
 * tinted by the accent, with a thin layer of twinkling stars and the odd shooting star.
 */
export function SkyCanvas({ art }: { art: SkyArt }) {
  const canvas = useRef<HTMLCanvasElement>(null);

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
      // The picture already holds the dense star field; these only add a little life.
      const count = Math.round((w * h) / 26000);
      let seed = 7;
      const rnd = () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
      stars = Array.from({ length: count }, () => ({
        x: rnd() * w,
        y: rnd() * h * 0.8,
        r: rnd() < 0.15 ? 1 + rnd() * 0.6 : 0.4 + rnd() * 0.5,
        phase: rnd() * Math.PI * 2,
        speed: 0.3 + rnd() * 1.1,
      }));
    };

    const frame = (t: number) => {
      raf = requestAnimationFrame(frame);
      if (document.hidden || t - last < 50) return;
      last = t;
      ctx.clearRect(0, 0, w, h);
      for (const s of stars) {
        const tw = reduce ? 0.6 : Math.max(0, Math.sin(s.phase + t * 0.0009 * s.speed));
        ctx.globalAlpha = tw * 0.85;
        ctx.fillStyle = "#f3f0ff";
        ctx.beginPath();
        ctx.arc(s.x, s.y, s.r, 0, Math.PI * 2);
        ctx.fill();
      }
      ctx.globalAlpha = 1;

      if (!reduce) {
        if (!shoot && Math.random() < 0.002) {
          shoot = { x: Math.random() * w * 0.7, y: Math.random() * h * 0.35, vx: 7 + Math.random() * 4, vy: 2.5 + Math.random() * 2, life: 1 };
        }
        if (shoot) {
          const g = ctx.createLinearGradient(shoot.x, shoot.y, shoot.x - shoot.vx * 10, shoot.y - shoot.vy * 10);
          g.addColorStop(0, `rgba(255,255,255,${shoot.life * 0.8})`);
          g.addColorStop(1, "rgba(255,255,255,0)");
          ctx.strokeStyle = g;
          ctx.lineWidth = 1.2;
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

  return (
    <>
      <div className="sky-art" style={{ backgroundImage: `url(${SKY_ART[art]})` }} aria-hidden="true" />
      <div className="sky-tint" aria-hidden="true" />
      <canvas ref={canvas} className="sky" aria-hidden="true" />
    </>
  );
}
