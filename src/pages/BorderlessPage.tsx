import { useCallback, useEffect, useState } from "react";
import type { BorderlessGame, Fit, MonitorInfo, WindowInfo } from "../types";
import { useStore } from "../store";
import { api, errorText } from "../lib/backend";
import { BORDERLESS_HELP } from "../lib/help";
import { HelpList } from "../components/Help";
import { BindButton, useBindRecorder } from "../components/BindButton";
import { Icon } from "../components/Icon";
import { Chip, Panel, Row, Select } from "../components/ui";

type Box = { x: number; y: number; w: number; h: number };

/** Same maths as `fit.rs`: where the window lands on a screen of `sw` x `sh`. */
function place(fit: Fit, sw: number, sh: number, client: [number, number]): Box {
  if (fit.mode === "stay") {
    const [cw, ch] = [Math.min(client[0], sw), Math.min(client[1], sh)];
    return { x: Math.floor((sw - cw) / 2), y: Math.floor((sh - ch) / 2), w: cw, h: ch };
  }
  let [w, h] = [sw, sh];
  if (fit.mode === "keep") [w, h] = client;
  else if (fit.mode === "custom") [w, h] = [fit.width, fit.height];
  else {
    const m = /^(\d+):(\d+)$/.exec(fit.mode);
    if (m && +m[1] > 0 && +m[2] > 0) {
      const a = +m[1];
      const b = +m[2];
      const wide = Math.floor((sh * a) / b);
      [w, h] = wide <= sw ? [wide, sh] : [sw, Math.floor((sw * b) / a)];
    }
  }
  w = Math.min(Math.max(1, w), sw);
  h = Math.min(Math.max(1, h), sh);
  return { x: Math.floor((sw - w) / 2), y: Math.floor((sh - h) / 2), w, h };
}

function modeOptions(screen: MonitorInfo | undefined, fit: Fit) {
  const size = (mode: string) => {
    if (!screen) return "";
    const b = place({ ...fit, mode }, screen.width, screen.height, [1920, 1080]);
    return ` (${b.w} x ${b.h})`;
  };
  return [
    { value: "stay", label: "Frame off only, same size and place" },
    { value: "fill", label: `Fill the screen${size("fill")}` },
    { value: "16:9", label: `16:9, centred${size("16:9")}` },
    { value: "43:18", label: `21:9, centred${size("43:18")}` },
    { value: "4:3", label: `4:3, centred${size("4:3")}` },
    { value: "keep", label: "Keep the game's size, centred" },
    { value: "custom", label: `Custom size (${fit.width} x ${fit.height})` },
  ];
}

const sameExe = (a: string, b: string) => a !== "" && a.toLowerCase() === b.toLowerCase();

export function BorderlessPage() {
  const { settings, monitors, update, toast } = useStore();
  const rec = useBindRecorder();
  const b = settings.borderless;
  const fit = b.fit;
  const key = settings.binds.borderless;
  const [windows, setWindows] = useState<WindowInfo[] | null>(null);
  const [busy, setBusy] = useState<number | null>(null);

  const refresh = useCallback(() => {
    api.borderlessWindows().then(setWindows, () => setWindows([]));
  }, []);

  useEffect(() => {
    refresh();
    const t = window.setInterval(refresh, 3000);
    const off = api.onBorderlessChanged((err) => {
      if (err) toast(err, "bad");
      window.setTimeout(refresh, 400);
    });
    return () => {
      window.clearInterval(t);
      off();
    };
  }, [refresh, toast]);

  const editFit = (patch: Partial<Fit>) =>
    update((s) => ({ ...s, borderless: { ...s.borderless, fit: { ...s.borderless.fit, ...patch } } }));
  const setGames = (fn: (g: BorderlessGame[]) => BorderlessGame[]) =>
    update((s) => ({ ...s, borderless: { ...s.borderless, games: fn(s.borderless.games) } }));

  const run = async (w: WindowInfo, job: () => Promise<WindowInfo[]>) => {
    setBusy(w.id);
    try {
      setWindows(await job());
      window.setTimeout(refresh, 500);
    } catch (e) {
      toast(errorText(e), "bad");
      refresh();
    } finally {
      setBusy(null);
    }
  };

  const remembered = (w: WindowInfo) => b.games.find((g) => sameExe(g.exe, w.exe));
  const fitFor = (w: WindowInfo) => remembered(w)?.fit ?? fit;
  const remember = (w: WindowInfo) => {
    setGames((g) => [...g.filter((x) => !sameExe(x.exe, w.exe)), { exe: w.exe, name: w.title, fit: { ...fit } }]);
    toast(`Luna will make ${w.title} borderless every time it opens.`, "good");
  };

  // The small picture shows the chosen screen, or the primary one.
  const screen = monitors.find((m) => m.id === fit.monitor) ?? monitors.find((m) => m.primary) ?? monitors[0];
  const sample = windows?.find((w) => w.framed && w.width > 0);
  const box = screen ? place(fit, screen.width, screen.height, sample ? [sample.width, sample.height] : [1920, 1080]) : null;
  const pct = (v: number, of: number) => `${(v / of) * 100}%`;

  const screenOptions = [
    { value: "", label: "The screen the game is on" },
    ...monitors.map((m) => ({ value: m.id, label: `${m.label} (${m.width} x ${m.height})` })),
  ];
  const options = modeOptions(screen, fit);

  return (
    <div className="page borderless-page">
      <div className="xh-grid">
        <div className="zoom-side">
          <Panel
            title="Borderless window"
            subtitle="Takes the title bar and frame off a game in windowed mode. By default the game keeps its size and place."
            right={<Chip tone={key ? "accent" : "warn"}>{key ? key.label : "No key yet"}</Chip>}
          >
            {screen && box && (
              <div className="fit-stage" aria-label={`The game covers ${box.w} x ${box.h} of a ${screen.width} x ${screen.height} screen`}>
                <div className="fit-screen" style={{ aspectRatio: `${screen.width} / ${screen.height}` }}>
                  <div
                    className="fit-game"
                    style={{
                      left: pct(box.x, screen.width),
                      top: pct(box.y, screen.height),
                      width: pct(box.w, screen.width),
                      height: pct(box.h, screen.height),
                    }}
                  >
                    <span className="mono">
                      {box.w} x {box.h}
                    </span>
                  </div>
                </div>
                <span className="muted small">
                  {screen.label}, {screen.width} x {screen.height}
                </span>
              </div>
            )}
            <Row title="Size">
              <Select label="Borderless size" value={fit.mode} options={options} onChange={(v) => editFit({ mode: v })} />
            </Row>
            {fit.mode === "custom" && (
              <Row title="Width and height" desc="In pixels. Kept inside the screen.">
                <div className="fit-size">
                  <input
                    className="input small"
                    type="number"
                    min={320}
                    max={15360}
                    value={fit.width}
                    aria-label="Width"
                    onChange={(e) => editFit({ width: Math.round(+e.target.value) || 0 })}
                  />
                  <span className="muted">x</span>
                  <input
                    className="input small"
                    type="number"
                    min={240}
                    max={8640}
                    value={fit.height}
                    aria-label="Height"
                    onChange={(e) => editFit({ height: Math.round(+e.target.value) || 0 })}
                  />
                </div>
              </Row>
            )}
            <Row title="Screen">
              <Select label="Borderless screen" value={fit.monitor} options={screenOptions} onChange={(v) => editFit({ monitor: v })} />
            </Row>
            <Row title="Key" desc="Press it in game to switch the window in front.">
              <BindButton slot={{ key: "borderless" }} rec={rec} />
            </Row>
          </Panel>

          <Panel title="Questions">
            <HelpList items={BORDERLESS_HELP} />
          </Panel>
        </div>

        <div className="zoom-side">
          <Panel
            title="Open windows"
            subtitle="Start the game in windowed mode, then pick it here."
            right={
              <button type="button" className="icon-btn" aria-label="Refresh the list" title="Refresh" onClick={refresh}>
                <Icon name="reset" size={16} />
              </button>
            }
          >
            {windows === null ? (
              <p className="muted small">Looking for windows...</p>
            ) : windows.length === 0 ? (
              <p className="muted small">No windows found. Start the game in windowed mode, then refresh.</p>
            ) : (
              <div className="bind-list">
                {windows.map((w) => (
                  <div className="bind-row win-row" key={w.id}>
                    <div className="row-text">
                      <div className="row-title">
                        <span className="win-title">{w.title}</span>
                        {w.borderless && <Chip tone="good">Borderless</Chip>}
                      </div>
                      <div className="muted small mono">
                        {w.exe || "Unknown program"}
                        {w.width > 0 ? `  ${w.width} x ${w.height}` : "  minimised"}
                      </div>
                    </div>
                    <div className="win-actions">
                      {w.exe &&
                        (remembered(w) ? (
                          <span className="muted small">Remembered</span>
                        ) : (
                          <button type="button" className="btn small ghost" onClick={() => remember(w)}>
                            Remember
                          </button>
                        ))}
                      {w.borderless ? (
                        <button type="button" className="btn small" disabled={busy === w.id} onClick={() => run(w, () => api.borderlessRestore(w.id))}>
                          Restore
                        </button>
                      ) : (
                        <button
                          type="button"
                          className="btn small primary"
                          disabled={busy === w.id}
                          onClick={() => run(w, () => api.borderlessApply(w.id, fitFor(w)))}
                        >
                          Make borderless
                        </button>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </Panel>

          {b.games.length > 0 && (
            <Panel title="Remembered games" subtitle="Luna takes their frame off by itself, a couple of seconds after they open.">
              <div className="bind-list">
                {b.games.map((g) => (
                  <div className="bind-row win-row" key={g.exe}>
                    <div className="row-text">
                      <div className="row-title">
                        <span className="win-title">{g.name || g.exe}</span>
                      </div>
                      <div className="muted small mono">{g.exe}</div>
                    </div>
                    <div className="win-actions">
                      <Select
                        label={`Size for ${g.name || g.exe}`}
                        value={g.fit.mode}
                        options={modeOptions(screen, g.fit)}
                        onChange={(v) => setGames((all) => all.map((x) => (x.exe === g.exe ? { ...x, fit: { ...x.fit, mode: v } } : x)))}
                      />
                      <button
                        type="button"
                        className="icon-btn danger"
                        aria-label={`Forget ${g.name || g.exe}`}
                        title="Forget"
                        onClick={() => setGames((all) => all.filter((x) => x.exe !== g.exe))}
                      >
                        <Icon name="trash" size={15} />
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            </Panel>
          )}
        </div>
      </div>
    </div>
  );
}
