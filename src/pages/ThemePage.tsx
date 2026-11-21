import { useRef } from "react";
import type { Theme } from "../types";
import { useStore } from "../store";
import { api, errorText } from "../lib/backend";
import { ACCENTS } from "../lib/defaults";
import { Panel, Row, Segmented, Select, Slider } from "../components/ui";
import { Icon } from "../components/Icon";

const MAX_BYTES = 15 * 1024 * 1024;

export function ThemePage({ background, onBackground }: { background: string | null; onBackground: (d: string | null) => void }) {
  const { settings, update, toast } = useStore();
  const t = settings.theme;
  const file = useRef<HTMLInputElement>(null);
  const edit = (patch: Partial<Theme>) => update((s) => ({ ...s, theme: { ...s.theme, ...patch } }));

  const pick = (f: File | undefined) => {
    if (!f) return;
    if (!f.type.startsWith("image/")) return toast("Pick a picture or a GIF.", "bad");
    if (f.size > MAX_BYTES) return toast("That file is over 15 MB. Pick a smaller one.", "bad");
    const reader = new FileReader();
    reader.onload = async () => {
      const data = String(reader.result);
      try {
        await api.setBackground(data);
        onBackground(data);
        edit({ background: "custom" });
        toast("Background set.", "good");
      } catch (e) {
        toast(errorText(e), "bad");
      }
    };
    reader.readAsDataURL(f);
  };

  const removeCustom = async () => {
    try {
      await api.setBackground(null);
    } catch {
      /* nothing to remove */
    }
    onBackground(null);
    edit({ background: "sky" });
  };

  return (
    <div className="page">
      <Panel title="Background" subtitle="The animated night sky, or a picture or GIF of your own.">
        <div className="bg-choices">
          <button type="button" className={`bg-choice ${t.background === "sky" ? "active" : ""}`} onClick={() => edit({ background: "sky" })}>
            <span className="bg-sample sky-sample" />
            <span>Night sky</span>
          </button>
          <button
            type="button"
            className={`bg-choice ${t.background === "custom" ? "active" : ""}`}
            onClick={() => (background ? edit({ background: "custom" }) : file.current?.click())}
          >
            <span className="bg-sample" style={background ? { backgroundImage: `url(${background})` } : undefined}>
              {!background && <Icon name="image" size={28} />}
            </span>
            <span>Your picture</span>
          </button>
        </div>
        <div className="toolbar">
          <button type="button" className="btn" onClick={() => file.current?.click()}>
            <Icon name="image" size={15} /> Choose picture or GIF
          </button>
          {background && (
            <button type="button" className="btn ghost" onClick={removeCustom}>
              <Icon name="trash" size={15} /> Remove
            </button>
          )}
          <input ref={file} type="file" accept="image/*" hidden onChange={(e) => pick(e.target.files?.[0])} />
        </div>
      </Panel>

      <Panel title="Look">
        <Row title="Accent colour">
          <div className="swatches">
            {ACCENTS.map((a) => (
              <button
                key={a}
                type="button"
                className={`swatch ${t.accent.toLowerCase() === a ? "active" : ""}`}
                style={{ background: a }}
                aria-label={`Accent ${a}`}
                onClick={() => edit({ accent: a })}
              />
            ))}
            <label className="swatch custom" title="Any colour">
              <input type="color" value={t.accent} onChange={(e) => edit({ accent: e.target.value })} aria-label="Any accent colour" />
            </label>
          </div>
        </Row>
        <div className="row-slider">
          <Slider label="Glass and glow" value={t.effect} min={0} max={100} step={1} def={60} unit="%" onChange={(v) => edit({ effect: v })} />
        </div>
        <Row title="Font">
          <Select
            label="Font"
            value={t.font}
            onChange={(v) => edit({ font: v })}
            options={[
              { value: "hanken", label: "Hanken Grotesk" },
              { value: "segoe", label: "Segoe UI" },
              { value: "bahnschrift", label: "Bahnschrift" },
              { value: "mono", label: "JetBrains Mono" },
            ]}
          />
        </Row>
        <Row title="Corners">
          <Segmented
            value={t.radius}
            onChange={(v) => edit({ radius: v })}
            options={[
              { value: "sharp", label: "Sharp" },
              { value: "soft", label: "Soft" },
              { value: "round", label: "Round" },
            ]}
          />
        </Row>
        <div className="toolbar">
          <button
            type="button"
            className="btn ghost"
            onClick={() => edit({ accent: "#a86dff", effect: 60, font: "hanken", radius: "soft" })}
          >
            <Icon name="reset" size={15} /> Default look
          </button>
        </div>
      </Panel>
    </div>
  );
}
