import { useStore } from "../store";
import { refById } from "../lib/refs";
import { vendorLabel } from "../lib/gpu";
import { ZOOM_HELP } from "../lib/help";
import { HelpList } from "../components/Help";
import { BindButton, useBindRecorder } from "../components/BindButton";
import { Chip, Panel, Row, Select, Slider } from "../components/ui";

const STEPS = [2, 3, 4, 6];

export function ZoomPage() {
  const { settings, monitors, info, update } = useStore();
  const rec = useBindRecorder();
  const factor = settings.zoom.factor;
  const key = settings.binds.zoom;
  const pic = refById("dark")?.url;
  const setFactor = (v: number) => update((s) => ({ ...s, zoom: { ...s.zoom, factor: v } }));

  const monitorOptions = [
    { value: "", label: "Primary display" },
    ...monitors.map((m) => ({ value: m.id, label: `${m.label} (${m.width} x ${m.height}, ${vendorLabel(m.vendor)})` })),
  ];
  // Share of the screen that ends up filling it, for the frame on the small picture.
  const area = `${(100 / factor).toFixed(1)}%`;

  return (
    <div className="page zoom-page">
      <div className="xh-grid">
        <Panel
          title="Ultra Zoom"
          subtitle="Hold a key to magnify the middle of the screen. Let go and the view snaps back."
          right={<Chip tone={key ? "accent" : "warn"}>{key ? key.label : "No key yet"}</Chip>}
        >
          <div
            className="zoom-stage"
            style={pic ? { backgroundImage: `url(${pic})`, backgroundSize: `${factor * 100}%` } : undefined}
            aria-label={`What the middle of the screen looks like at ${factor.toFixed(1)}x`}
          >
            <div className="zoom-mini" style={pic ? { backgroundImage: `url(${pic})` } : undefined}>
              <span style={{ width: area, height: area }} />
            </div>
            <span className="zoom-factor">{factor.toFixed(1)}x</span>
          </div>
          {!info.magnifier && <p className="note warn">Windows magnification is not available on this PC, so the zoom cannot start.</p>}
          <Row title="Key" desc="Hold it in game. Works while Luna is in the background.">
            <BindButton slot={{ key: "zoom" }} rec={rec} />
          </Row>
          <Row title="Screen" desc="The zoom stays centred on this screen.">
            <Select label="Zoom screen" value={settings.zoom.monitor} options={monitorOptions} onChange={(v) => update((s) => ({ ...s, zoom: { ...s.zoom, monitor: v } }))} />
          </Row>
        </Panel>

        <div className="zoom-side">
          <Panel title="Strength" subtitle="How much bigger the middle of the screen gets while the key is held.">
            <Slider label="Zoom" value={factor} min={1.5} max={8} step={0.1} def={2.5} unit="x" onChange={setFactor} />
            <div className="zoom-steps">
              {STEPS.map((n) => (
                <button key={n} type="button" className={`pill ${Math.abs(factor - n) < 0.05 ? "active" : ""}`} onClick={() => setFactor(n)}>
                  {n}x
                </button>
              ))}
            </div>
            <p className="muted small">
              At {factor.toFixed(1)}x, the middle {area} of the width fills the whole screen. Small steps like 2x keep enough around the target to track it, big steps help read far away detail.
            </p>
          </Panel>

          <Panel title="Questions">
            <HelpList items={ZOOM_HELP} />
          </Panel>
        </div>
      </div>
    </div>
  );
}
