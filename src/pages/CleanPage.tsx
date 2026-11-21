import { useCallback, useEffect, useState } from "react";
import type { CleanGroup, CleanItem, CleanReport } from "../types";
import { useStore } from "../store";
import { api, errorText } from "../lib/backend";
import { CLEAN_HELP } from "../lib/help";
import { count, fileSize } from "../lib/format";
import { HelpList } from "../components/Help";
import { Icon } from "../components/Icon";
import { Chip, Panel, Toggle } from "../components/ui";

const GROUPS: { id: CleanGroup; title: string; subtitle: string }[] = [
  { id: "system", title: "Windows", subtitle: "Leftovers from Windows, updates and crashed programs." },
  { id: "browsers", title: "Browsers", subtitle: "Cached pages and pictures. You stay signed in everywhere." },
  { id: "apps", title: "Apps and games", subtitle: "Launcher, chat and music caches. Games and saves stay." },
  { id: "dev", title: "Developer tools", subtitle: "Download caches of package managers." },
];

const cleanable = (i: CleanItem) => i.bytes > 0 && !i.needsAdmin;

export function CleanPage() {
  const { toast } = useStore();
  const [report, setReport] = useState<CleanReport | null>(null);
  const [picked, setPicked] = useState<Set<string> | null>(null);
  const [busy, setBusy] = useState<"scan" | "clean" | "admin" | null>("scan");

  const scan = useCallback(async () => {
    setBusy("scan");
    try {
      const r = await api.cleanScan();
      setReport(r);
      // First scan: tick what is suggested. Later scans keep the user's choice.
      setPicked((old) => {
        const ids = new Set(r.items.filter(cleanable).map((i) => i.id));
        if (!old) return new Set(r.items.filter((i) => i.suggested && cleanable(i)).map((i) => i.id));
        return new Set([...old].filter((id) => ids.has(id)));
      });
    } catch (e) {
      toast(errorText(e), "bad");
    } finally {
      setBusy(null);
    }
  }, [toast]);

  useEffect(() => {
    scan();
  }, [scan]);

  const items = report?.items ?? [];
  const sel = picked ?? new Set<string>();
  const chosen = items.filter((i) => sel.has(i.id) && cleanable(i));
  const chosenBytes = chosen.reduce((a, i) => a + i.bytes, 0);
  const found = items.reduce((a, i) => a + i.bytes, 0);
  const locked = items.filter((i) => i.needsAdmin);

  const pick = (id: string, on: boolean) =>
    setPicked((old) => {
      const next = new Set(old ?? []);
      if (on) next.add(id);
      else next.delete(id);
      return next;
    });

  const clean = async () => {
    setBusy("clean");
    try {
      const r = await api.cleanRun(chosen.map((i) => i.id));
      const stayed = r.skipped ? ` ${count(r.skipped, "file")} in use stayed.` : "";
      toast(`Freed ${fileSize(r.freedBytes)} from ${count(r.files, "file")}.${stayed}`, "good");
    } catch (e) {
      toast(errorText(e), "bad");
    }
    await scan();
  };

  const restartAdmin = async () => {
    setBusy("admin");
    try {
      await api.restartAdmin("clean");
    } catch (e) {
      toast(errorText(e), "bad");
      setBusy(null);
    }
  };

  const scanning = busy === "scan" && !report;
  const share = found > 0 ? Math.min(100, (chosenBytes / found) * 100) : 0;

  return (
    <div className="page clean-page">
      <section className="hero panel">
        <div className="hero-text">
          <div className="hero-title">
            <h1>Clean</h1>
            {report && <Chip tone={found > 0 ? "accent" : "good"}>{found > 0 ? `${fileSize(found)} found` : "Nothing to clean"}</Chip>}
          </div>
          <p className="muted">
            Caches, temporary files, old logs and crash reports that programs make again by themselves. Documents, saves, passwords and history are never touched.
          </p>
          <div className="clean-meter" aria-hidden={scanning}>
            <div className="clean-bar">
              <div className={`clean-fill ${scanning ? "scanning" : ""}`} style={{ width: scanning ? "30%" : `${share}%` }} />
            </div>
            <span className="muted small mono">
              {scanning
                ? "Looking through your folders..."
                : `${fileSize(chosenBytes)} selected of ${fileSize(found)}, ${count(chosen.reduce((a, i) => a + i.files, 0), "file")}`}
            </span>
          </div>
        </div>
        <div className="hero-actions">
          <button type="button" className="btn primary" disabled={!!busy || chosen.length === 0} onClick={clean}>
            <Icon name="broom" size={15} /> {busy === "clean" ? "Cleaning..." : chosen.length ? `Clean ${fileSize(chosenBytes)}` : "Clean"}
          </button>
          <button type="button" className="btn ghost" disabled={!!busy} onClick={scan}>
            <Icon name="reset" size={15} /> {busy === "scan" ? "Scanning..." : "Scan again"}
          </button>
        </div>
      </section>

      {report && !report.admin && locked.length > 0 && (
        <div className="clean-admin panel">
          <div className="row-text">
            <div className="row-title">{locked.length === 1 ? "One item needs administrator" : `${locked.length} items need administrator`}</div>
            <div className="muted small">{locked.map((i) => i.title).join(", ")}. Windows asks for permission, then Luna opens again on this page.</div>
          </div>
          <button type="button" className="btn" disabled={!!busy} onClick={restartAdmin}>
            Restart as administrator
          </button>
        </div>
      )}

      <div className="xh-grid">
        {[GROUPS.slice(0, 2), GROUPS.slice(2)].map((column, c) => (
          <div className="zoom-side" key={c}>
            {column.map((g) => {
              const rows = items.filter((i) => i.group === g.id).sort((a, b) => b.bytes - a.bytes);
              if (report && rows.length === 0) return null;
              const total = rows.reduce((a, i) => a + i.bytes, 0);
              return (
                <Panel key={g.id} title={g.title} subtitle={g.subtitle} right={report && <span className="muted small mono">{fileSize(total)}</span>}>
                  {!report ? (
                    <p className="muted small">Scanning...</p>
                  ) : (
                    <div className="bind-list">
                      {rows.map((i) => (
                        <CleanRow key={i.id} item={i} on={sel.has(i.id)} disabled={!!busy} onChange={(v) => pick(i.id, v)} />
                      ))}
                    </div>
                  )}
                </Panel>
              );
            })}
            {c === 1 && (
              <Panel title="Questions">
                <HelpList items={CLEAN_HELP} />
              </Panel>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

function CleanRow({ item, on, disabled, onChange }: { item: CleanItem; on: boolean; disabled: boolean; onChange: (v: boolean) => void }) {
  const can = cleanable(item);
  return (
    <div className={`bind-row clean-row ${can ? "" : "locked"}`}>
      <div className="row-text">
        <div className="row-title">
          {item.title}
          {item.needsAdmin && <Chip tone="warn">Administrator</Chip>}
          {item.open && can && <Chip>{item.open} is open</Chip>}
        </div>
        <div className="muted small">{item.desc}</div>
      </div>
      <div className="clean-side">
        <span className="mono clean-size">{item.needsAdmin && item.bytes === 0 ? "?" : fileSize(item.bytes)}</span>
        <Toggle on={on && can} disabled={disabled || !can} label={`Clean ${item.title}`} onChange={onChange} />
      </div>
    </div>
  );
}
