import { useEffect, useState } from "react";
import type { CleanResult, TweakStatus } from "../types";
import { useStore } from "../store";
import { api, errorText } from "../lib/backend";
import { Chip, Panel, Toggle } from "../components/ui";
import { Icon } from "../components/Icon";

const TWEAKS: Record<string, { title: string; desc: string }> = {
  power: { title: "High performance power plan", desc: "Keeps the processor from slowing down to save power." },
  gameMode: { title: "Game Mode", desc: "Lets Windows put the game first while it runs." },
  gameDvr: { title: "Background recording off", desc: "Stops the Game Bar from recording clips in the background." },
  flipModel: { title: "Optimizations for windowed games", desc: "Lower latency for games in borderless or windowed mode." },
  mouseAccel: { title: "Mouse acceleration off", desc: "The pointer moves the same distance however fast you move the mouse." },
  stickyKeys: { title: "Sticky Keys popup off", desc: "No more popup after pressing Shift five times." },
  notifications: { title: "Notifications off", desc: "Hides toast notifications so nothing pops up mid game." },
  backgroundApps: { title: "Background apps off", desc: "Store apps stop running when you are not using them." },
  transparency: { title: "Transparency effects off", desc: "Plain taskbar and menus, a bit lighter on the graphics card." },
  animations: { title: "Window animations off", desc: "Windows open and close instantly." },
};

function size(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB"];
  let v = bytes / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v >= 100 ? 0 : 1)} ${units[i]}`;
}

export function OptimizerPage() {
  const { toast } = useStore();
  const [status, setStatus] = useState<TweakStatus[]>([]);
  const [busy, setBusy] = useState<string | null>(null);
  const [clean, setClean] = useState<CleanResult | null>(null);

  useEffect(() => {
    api.optimizerStatus().then(setStatus).catch((e) => toast(errorText(e), "bad"));
  }, [toast]);

  const run = async (key: string, fn: () => Promise<void>) => {
    setBusy(key);
    try {
      await fn();
    } catch (e) {
      toast(errorText(e), "bad");
    } finally {
      setBusy(null);
    }
  };

  const bulk = (undo: boolean) =>
    run(undo ? "undo" : "all", async () => {
      const r = undo ? await api.optimizerUndoAll() : await api.optimizerAll();
      setStatus(r.status);
      if (r.errors.length) toast(`Done, but ${r.errors.length} could not be changed: ${r.errors[0]}`, "bad");
      else toast(undo ? "Every setting is back to how it was." : "All tweaks applied.", "good");
    });

  const onCount = status.filter((s) => s.on).length;

  return (
    <div className="page">
      <section className="hero panel">
        <div className="hero-text">
          <div className="hero-title">
            <h1>Optimizer</h1>
            <Chip tone={onCount ? "good" : "plain"}>
              {onCount}/{status.length || 10} on
            </Chip>
          </div>
          <p className="muted">
            Windows settings that help games feel smoother. Luna remembers each setting before changing it, so Undo puts it back exactly. Only your own user settings
            are touched, no admin rights needed.
          </p>
        </div>
        <div className="hero-actions">
          <button type="button" className="btn primary" disabled={!!busy} onClick={() => bulk(false)}>
            <Icon name="optimizer" size={15} /> Optimize all
          </button>
          <button type="button" className="btn ghost" disabled={!!busy} onClick={() => bulk(true)}>
            <Icon name="reset" size={15} /> Undo all
          </button>
        </div>
      </section>

      <div className="tweak-grid">
        {Object.entries(TWEAKS).map(([id, t]) => {
          const st = status.find((s) => s.id === id);
          return (
            <div key={id} className={`tweak ${st?.on ? "on" : ""}`}>
              <div className="tweak-text">
                <div className="row-title">{t.title}</div>
                <div className="muted small">{t.desc}</div>
                {st && !st.available && <div className="small warn-text">Not available on this PC.</div>}
              </div>
              <Toggle
                on={!!st?.on}
                disabled={!st || !st.available || !!busy}
                label={t.title}
                onChange={(v) =>
                  run(id, async () => {
                    setStatus(await api.optimizerSet(id, v));
                  })
                }
              />
            </div>
          );
        })}
      </div>

      <Panel title="Clean temp files" subtitle="Deletes files older than one day from your temp folder. Files in use are skipped.">
        <div className="toolbar">
          <button
            type="button"
            className="btn"
            disabled={!!busy}
            onClick={() =>
              run("clean", async () => {
                const r = await api.cleanTemp();
                setClean(r);
                toast(`Freed ${size(r.freedBytes)}.`, "good");
              })
            }
          >
            <Icon name="broom" size={15} /> {busy === "clean" ? "Cleaning..." : "Clean now"}
          </button>
          {clean && (
            <span className="muted">
              Freed <strong className="accent-text">{size(clean.freedBytes)}</strong> from {clean.files} files
              {clean.skipped ? `, ${clean.skipped} skipped (in use or too new)` : ""}.
            </span>
          )}
        </div>
      </Panel>
    </div>
  );
}
