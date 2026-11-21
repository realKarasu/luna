import { useEffect, useState, type ReactNode } from "react";

export function Panel({
  title,
  subtitle,
  right,
  children,
  className = "",
}: {
  title?: ReactNode;
  subtitle?: ReactNode;
  right?: ReactNode;
  children?: ReactNode;
  className?: string;
}) {
  return (
    <section className={`panel ${className}`}>
      {(title || right) && (
        <header className="panel-head">
          <div>
            {title && <h2>{title}</h2>}
            {subtitle && <p className="muted small">{subtitle}</p>}
          </div>
          {right && <div className="panel-right">{right}</div>}
        </header>
      )}
      {children}
    </section>
  );
}

export function Toggle({
  on,
  onChange,
  label,
  big = false,
  disabled = false,
}: {
  on: boolean;
  onChange: (v: boolean) => void;
  label?: string;
  big?: boolean;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={on}
      aria-label={label}
      disabled={disabled}
      className={`toggle ${on ? "on" : ""} ${big ? "big" : ""}`}
      onClick={() => onChange(!on)}
    >
      <span className="knob" />
    </button>
  );
}

export function Segmented<T extends string>({
  value,
  options,
  onChange,
  small = false,
}: {
  value: T;
  options: { value: T; label: ReactNode }[];
  onChange: (v: T) => void;
  small?: boolean;
}) {
  return (
    <div className={`segmented ${small ? "small" : ""}`} role="tablist">
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          role="tab"
          aria-selected={o.value === value}
          className={o.value === value ? "active" : ""}
          onClick={() => onChange(o.value)}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

const fmt = (v: number, step: number) => {
  const decimals = step >= 1 ? 0 : step >= 0.1 ? 1 : 2;
  return v.toFixed(decimals);
};

export function Slider({
  label,
  value,
  min,
  max,
  step = 1,
  def,
  unit = "",
  onChange,
  accent,
  disabled = false,
  hint,
}: {
  label: ReactNode;
  value: number;
  min: number;
  max: number;
  step?: number;
  def: number;
  unit?: string;
  onChange: (v: number) => void;
  accent?: string;
  disabled?: boolean;
  hint?: string;
}) {
  const [text, setText] = useState(fmt(value, step));
  const [editing, setEditing] = useState(false);
  useEffect(() => {
    if (!editing) setText(fmt(value, step));
  }, [value, step, editing]);

  const clampSnap = (v: number) => {
    const snapped = Math.round(v / step) * step;
    return Math.min(max, Math.max(min, Number(snapped.toFixed(4))));
  };
  const commit = () => {
    setEditing(false);
    const v = parseFloat(text.replace(",", "."));
    if (Number.isFinite(v)) onChange(clampSnap(v));
    else setText(fmt(value, step));
  };
  const pct = ((value - min) / (max - min)) * 100;
  const defPct = ((def - min) / (max - min)) * 100;
  return (
    <div className={`slider ${disabled ? "disabled" : ""}`} title={hint}>
      <div className="slider-top">
        <span className="slider-label">{label}</span>
        <span className="slider-value">
          <input
            className="num"
            value={text}
            disabled={disabled}
            aria-label={typeof label === "string" ? `${label} value` : "value"}
            onFocus={() => setEditing(true)}
            onChange={(e) => setText(e.target.value)}
            onBlur={commit}
            onKeyDown={(e) => {
              if (e.key === "Enter") (e.target as HTMLInputElement).blur();
              if (e.key === "Escape") {
                setText(fmt(value, step));
                setEditing(false);
                (e.target as HTMLInputElement).blur();
              }
            }}
          />
          {unit && <span className="unit">{unit}</span>}
        </span>
      </div>
      <div className="range-wrap" style={{ ["--pct" as string]: `${pct}%`, ["--def" as string]: `${defPct}%`, ["--fill" as string]: accent }}>
        <span className="range-def" />
        <input
          type="range"
          min={min}
          max={max}
          step={step}
          value={value}
          disabled={disabled}
          aria-label={typeof label === "string" ? label : undefined}
          onChange={(e) => onChange(clampSnap(parseFloat(e.target.value)))}
          onDoubleClick={() => onChange(def)}
        />
      </div>
    </div>
  );
}

export function Chip({ children, tone = "plain" }: { children: ReactNode; tone?: "plain" | "good" | "bad" | "warn" | "accent" }) {
  return <span className={`chip ${tone}`}>{children}</span>;
}

export function Select<T extends string>({
  value,
  options,
  onChange,
  label,
}: {
  value: T;
  options: { value: T; label: string }[];
  onChange: (v: T) => void;
  label?: string;
}) {
  return (
    <select className="select" value={value} aria-label={label} onChange={(e) => onChange(e.target.value as T)}>
      {options.map((o) => (
        <option key={o.value} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  );
}

export function Row({ title, desc, children }: { title: ReactNode; desc?: ReactNode; children?: ReactNode }) {
  return (
    <div className="row">
      <div className="row-text">
        <div className="row-title">{title}</div>
        {desc && <div className="muted small">{desc}</div>}
      </div>
      <div className="row-control">{children}</div>
    </div>
  );
}
