import { useMemo, useState } from "react";
import type { ColorParams, Tone } from "../types";
import { useStore } from "../store";
import { api, errorText } from "../lib/backend";
import { NEUTRAL_COLOR, NEUTRAL_TONE, NVG_TONES } from "../lib/math";
import { applyColor, applyGamma, setColor, setTone, toneOf } from "../lib/presets";
import { Chip, Panel, Segmented, Select, Slider, Toggle } from "../components/ui";
import { CurveGraph } from "../components/CurveGraph";
import { Preview, Thumb } from "../components/Preview";
import { Icon } from "../components/Icon";
import { gpuName, vendorLabel, vibranceNote } from "../lib/gpu";
import { HELP } from "../lib/help";
import { HelpList } from "../components/Help";

const PIC_KEY = "luna-preview-picture";

export function DisplayPage() {
  const { settings, monitors, info, update, replace, toast } = useStore();
  const [tab, setTab] = useState<"tone" | "color">("tone");
  const [picture, setPictureState] = useState(() => localStorage.getItem(PIC_KEY) ?? "dark");
  const [strip, setStrip] = useState<"gamma" | "color">("gamma");
  const [nvgPreview, setNvgPreview] = useState(false);
  const setPicture = (id: string) => {
    setPictureState(id);
    localStorage.setItem(PIC_KEY, id);
  };

  const monId = settings.selectedMonitor || monitors[0]?.id || "";
  const mon = monitors.find((m) => m.id === monId);
  const tone = toneOf(settings, monId);
  const color = settings.color;
  const strength = info.gammaStrength[monId];
  const rust = settings.profile === "rust";

  const look = useMemo(() => {
    const holoOn = rust && settings.holo.mode !== "off";
    return {
      tone: nvgPreview && rust ? NVG_TONES[settings.nvg.mode] : tone,
      color: nvgPreview && rust ? NEUTRAL_COLOR : color,
      holo: holoOn ? settings.holo.mode : ("off" as const),
      holoStrength: settings.holo.strength / 100,
    };
  }, [tone, color, rust, settings.holo, settings.nvg.mode, nvgPreview]);

  const editTone = (patch: Partial<Tone>) => update((s) => setTone(s, monId, { ...toneOf(s, monId), ...patch }));
  const editColor = (patch: Partial<ColorParams>) => update((s) => setColor(s, { ...s.color, ...patch }));

  const reset = async () => {
    try {
      replace(await api.resetDisplay(settings));
      toast("Every screen is back to its original look.", "good");
    } catch (e) {
      toast(errorText(e), "bad");
    }
  };

  const monitorOptions = [
    { value: "", label: "Primary display" },
    ...monitors.map((m) => ({ value: m.id, label: `${m.label} (${m.width} x ${m.height}, ${vendorLabel(m.vendor)})` })),
  ];

  return (
    <div className="page display-page">
      <section className="hero panel">
        <div className="hero-text">
          <div className="hero-title">
            <h1>Display Tuning</h1>
            <Chip tone={settings.tuningOn ? "good" : "plain"}>{settings.tuningOn ? "ON" : "OFF"}</Chip>
          </div>
          <p className="muted">Shape gamma, contrast and colour for each screen. Changes show up live, in game and on the desktop.</p>
          <div className="pills">
            {monitors.map((m) => (
              <button
                key={m.id}
                type="button"
                className={`pill ${m.id === monId ? "active" : ""}`}
                onClick={() => update((s) => ({ ...s, selectedMonitor: m.id }))}
              >
                <Icon name="display" size={15} />
                {m.label}
                {m.primary && <span className="muted small"> main</span>}
                <span className="muted small">
                  {m.width}x{m.height}
                </span>
                <span className={`vendor-tag vendor-${m.vendor ?? "generic"}`} title={`${gpuName(m)}. Vibrance: ${m.vibrance}`}>
                  {vendorLabel(m.vendor)}
                </span>
                <span className="pill-gpu muted small">{gpuName(m)}</span>
              </button>
            ))}
            {monitors.length === 0 && <span className="muted small">No screen found.</span>}
          </div>
        </div>
        <div className="hero-toggle">
          <span className="muted small">Tuning</span>
          <Toggle big on={settings.tuningOn} label="Tuning on or off" onChange={(v) => update((s) => ({ ...s, tuningOn: v }))} />
        </div>
      </section>

      <div className="display-grid">
        <Panel
          className="adjust"
          title="Adjust"
          subtitle="Double-click a slider to put it back to normal."
          right={
            <Segmented
              value={tab}
              onChange={setTab}
              options={[
                { value: "tone", label: "Tone" },
                { value: "color", label: "Color" },
              ]}
            />
          }
        >
          {tab === "tone" ? (
            <div className="tone-tab">
              <CurveGraph tone={tone} />
              <Slider label="Gamma" value={tone.gamma} min={0.6} max={3} step={0.01} def={1} onChange={(v) => editTone({ gamma: v })} />
              <Slider label="Brightness" value={tone.brightness} min={-20} max={20} step={1} def={0} onChange={(v) => editTone({ brightness: v })} />
              <Slider label="Contrast" value={tone.contrast} min={70} max={130} step={1} def={100} unit="%" onChange={(v) => editTone({ contrast: v })} />
              <div className="rgb">
                <Slider label="Red" accent="#ff6b7d" value={tone.red} min={20} max={180} step={1} def={100} unit="%" onChange={(v) => editTone({ red: v })} />
                <Slider label="Green" accent="#6bff9c" value={tone.green} min={20} max={180} step={1} def={100} unit="%" onChange={(v) => editTone({ green: v })} />
                <Slider label="Blue" accent="#6bb5ff" value={tone.blue} min={20} max={180} step={1} def={100} unit="%" onChange={(v) => editTone({ blue: v })} />
              </div>
              {strength !== undefined && strength < 0.995 && (
                <p className="note warn">
                  Windows limits how far a screen curve can go without admin rights. Luna applies {Math.round(strength * 100)}% of this curve on{" "}
                  {mon?.label ?? "this screen"}.
                </p>
              )}
            </div>
          ) : (
            <div className="color-tab">
              {!info.magnifier && <p className="note warn">Windows colour effects are not available on this PC, so hue and saturation can't be applied.</p>}
              <Slider label="Hue" value={color.hue} min={-180} max={180} step={1} def={0} unit="°" onChange={(v) => editColor({ hue: v })} />
              <div className="hue-bar" />
              <Slider label="Saturation" value={color.saturation} min={0} max={350} step={1} def={100} unit="%" onChange={(v) => editColor({ saturation: v })} />
              <Slider label="Vibrance" value={color.vibrance} min={0} max={250} step={1} def={100} unit="%" onChange={(v) => editColor({ vibrance: v })} />
              <p className="muted small">
                {mon ? `${mon.label}: ` : ""}
                {vibranceNote(mon)} Colour applies to every screen.
              </p>
            </div>
          )}
          <div className="adjust-foot">
            <button
              type="button"
              className="btn ghost"
              onClick={() => (tab === "tone" ? update((s) => setTone(s, monId, NEUTRAL_TONE)) : update((s) => setColor(s, NEUTRAL_COLOR)))}
            >
              <Icon name="reset" size={15} /> Reset {tab === "tone" ? "tone" : "color"}
            </button>
            <button type="button" className="btn danger" onClick={reset} title="Neutral tone and colour on every screen, originals back">
              <Icon name="power" size={15} /> Reset all screens
            </button>
          </div>
        </Panel>

        <Panel
          className="preview-panel"
          title="Preview"
          subtitle="Drag the line to compare. The picture uses the same math as your screen."
          right={
            rust ? (
              <button type="button" className={`btn small ${nvgPreview ? "primary" : "ghost"}`} onClick={() => setNvgPreview((v) => !v)}>
                <Icon name="eye" size={14} /> NVG view
              </button>
            ) : undefined
          }
        >
          <Preview look={look} picture={picture} onPicture={setPicture} />
        </Panel>
      </div>

      <Panel
        title="Presets"
        subtitle={strip === "gamma" ? `Tone presets go on ${mon?.label ?? "the selected screen"}.` : "Colour presets go on every screen."}
        right={
          <Segmented
            small
            value={strip}
            onChange={setStrip}
            options={[
              { value: "gamma", label: "Gamma" },
              { value: "color", label: "Color" },
            ]}
          />
        }
      >
        <div className="strip">
          {strip === "gamma"
            ? settings.gammaPresets.map((p) => (
                <button
                  key={p.id}
                  type="button"
                  className={`strip-card ${settings.activeGamma[monId] === p.id ? "active" : ""}`}
                  onClick={() => update((s) => applyGamma(s, p, monId))}
                >
                  <Thumb look={{ tone: p.tone, color: NEUTRAL_COLOR }} picture="dark" width={150} height={84} />
                  <span>{p.name}</span>
                </button>
              ))
            : settings.colorPresets.map((p) => (
                <button
                  key={p.id}
                  type="button"
                  className={`strip-card ${settings.activeColor === p.id ? "active" : ""}`}
                  onClick={() => update((s) => applyColor(s, p))}
                >
                  <Thumb look={{ tone: NEUTRAL_TONE, color: p.color }} picture="bright" width={150} height={84} />
                  <span>{p.name}</span>
                </button>
              ))}
        </div>
      </Panel>

      <Panel
        title="Game tools"
        subtitle="Hold-to-use tools. Set their keys on the Binds page."
        right={
          <Segmented
            value={settings.profile}
            onChange={(v) => update((s) => ({ ...s, profile: v }))}
            options={[
              { value: "general", label: "General" },
              { value: "rust", label: "Rust" },
            ]}
          />
        }
      >
        <div className="tools-grid">
          <div className="tool">
            <div className="tool-head">
              <Icon name="zoom" />
              <h3>Ultra Zoom</h3>
              <Chip tone="accent">{settings.binds.zoom?.label ?? "No key"}</Chip>
            </div>
            <p className="muted small">Magnifies the middle of the screen while the key is held.</p>
            {!info.magnifier && <p className="note warn">Windows magnification is not available on this PC.</p>}
            <Slider
              label="Zoom"
              value={settings.zoom.factor}
              min={1.5}
              max={8}
              step={0.1}
              def={2.5}
              unit="x"
              onChange={(v) => update((s) => ({ ...s, zoom: { ...s.zoom, factor: v } }))}
            />
            <label className="field">
              <span className="muted small">Screen</span>
              <Select label="Zoom screen" value={settings.zoom.monitor} options={monitorOptions} onChange={(v) => update((s) => ({ ...s, zoom: { ...s.zoom, monitor: v } }))} />
            </label>
          </div>

          {rust ? (
            <>
              <div className="tool">
                <div className="tool-head">
                  <Icon name="eye" />
                  <h3>Night vision</h3>
                  <Chip tone="accent">{settings.binds.nvg?.label ?? "No key"}</Chip>
                </div>
                <p className="muted small">Gamma 3.00 with a tint while the key is held, for dark nights.</p>
                <Segmented
                  value={settings.nvg.mode}
                  onChange={(v) => update((s) => ({ ...s, nvg: { ...s.nvg, mode: v } }))}
                  options={[
                    { value: "green", label: "Green NVG" },
                    { value: "blue", label: "Blue NVG" },
                  ]}
                />
                <label className="field">
                  <span className="muted small">Screen</span>
                  <Select label="Night vision screen" value={settings.nvg.monitor} options={monitorOptions} onChange={(v) => update((s) => ({ ...s, nvg: { ...s.nvg, monitor: v } }))} />
                </label>
              </div>
              <div className="tool">
                <div className="tool-head">
                  <Icon name="crosshair" />
                  <h3>Sight colour</h3>
                </div>
                <p className="muted small">Pushes a sight's colour toward black so it stands out. Stays on until you turn it off.</p>
                <Segmented
                  value={settings.holo.mode}
                  onChange={(v) => update((s) => ({ ...s, holo: { ...s.holo, mode: v } }))}
                  options={[
                    { value: "off", label: "Off" },
                    { value: "greenTrueBlack", label: "Green True Black" },
                    { value: "redBlackHolo", label: "Red Black Holo" },
                  ]}
                />
                <Slider
                  label="Strength"
                  value={settings.holo.strength}
                  min={0}
                  max={100}
                  step={1}
                  def={85}
                  unit="%"
                  disabled={settings.holo.mode === "off"}
                  onChange={(v) => update((s) => ({ ...s, holo: { ...s.holo, strength: v } }))}
                />
              </div>
            </>
          ) : (
            <div className="tool tool-hint">
              <Icon name="sparkle" />
              <p className="muted small">Pick the Rust profile for night vision and sight colour tools.</p>
            </div>
          )}
        </div>
      </Panel>

      <Panel title="Questions" subtitle="Quick answers. More in Settings, under Help.">
        <HelpList items={HELP.slice(0, 4)} />
      </Panel>
    </div>
  );
}
