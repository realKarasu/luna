import { useState } from "react";
import { useStore } from "../store";
import { api, errorText, isTauri } from "../lib/backend";
import { Panel, Row, Toggle } from "../components/ui";
import { Icon, MoonLogo } from "../components/Icon";
import { gpuName, usesDriver, vendorLabel, vibranceSummary } from "../lib/gpu";
import { HELP } from "../lib/help";
import { HelpList } from "../components/Help";
import { osName } from "../lib/platform";

export function SettingsPage() {
  const { settings, monitors, info, update, toast, setInfo } = useStore();
  const app = settings.app;
  const setApp = (patch: Partial<typeof app>) => update((s) => ({ ...s, app: { ...s.app, ...patch } }));

  const os = osName(info.platform);
  const win = info.platform === "windows";

  const autostart = async (on: boolean) => {
    try {
      const v = await api.setAutostart(on);
      setInfo({ autostart: v });
      setApp({ startWithWindows: v });
    } catch (e) {
      toast(errorText(e), "bad");
    }
  };

  return (
    <div className="page">
      <Panel title="Startup and tray">
        <Row title={`Start with ${os}`} desc="Luna opens when you sign in, so your screens are tuned right away.">
          <Toggle on={info.autostart} label={`Start with ${os}`} onChange={autostart} />
        </Row>
        <Row title="Start minimized" desc="When started at sign-in, stay in the tray instead of opening the window.">
          <Toggle on={app.startMinimized} label="Start minimized" onChange={(v) => setApp({ startMinimized: v })} />
        </Row>
        <Row title="Close to tray" desc="The close button hides Luna and keeps tuning running. Quit from the tray icon.">
          <Toggle on={app.closeToTray} label="Close to tray" onChange={(v) => setApp({ closeToTray: v })} />
        </Row>
      </Panel>

      <UpdatesPanel />

      <Panel title="Your data" subtitle="Everything stays on this computer. Luna has no account and sends nothing about you anywhere.">
        <Row title="Settings folder" desc={<span className="mono">{info.dataDir || "%APPDATA%\\Luna"}</span>}>
          <button type="button" className="btn" disabled={!isTauri} onClick={() => api.openDataDir()}>
            <Icon name="folder" size={15} /> Open
          </button>
        </Row>
        <Row title="Quit Luna" desc="Puts every screen back to how it was, then closes.">
          <button type="button" className="btn danger" disabled={!isTauri} onClick={() => api.quit()}>
            <Icon name="power" size={15} /> Quit
          </button>
        </Row>
      </Panel>

      <Panel title="Screens and graphics cards" subtitle="Found by Luna when it starts and again whenever the screens change.">
        {monitors.map((m) => (
          <Row
            key={m.id}
            title={
              <>
                {m.label}
                {m.primary && <span className="muted small"> (main)</span>}
              </>
            }
            desc={
              <>
                {gpuName(m)} <span className="muted">| Vibrance: {usesDriver(m) ? m.vibrance : "colour boost (Color matrix)"}</span>
              </>
            }
          >
            <span className={`vendor-tag vendor-${m.vendor ?? "generic"}`}>{vendorLabel(m.vendor)}</span>
          </Row>
        ))}
        {monitors.length === 0 && <p className="muted small">No screen found.</p>}
      </Panel>

      <Panel title="Help" subtitle="Short answers about how Luna works.">
        <HelpList items={HELP} />
      </Panel>

      <Panel title="About">
        <div className="about">
          <MoonLogo size={56} />
          <div>
            <h2 className="about-name">Luna</h2>
            <p className="muted">Version {info.version}</p>
            <p className="small">
              Free and open source under the MIT licence. Luna is an independent project and is not affiliated with any other display tuning product.
            </p>
            <p className="muted small">
              Graphics card features:{" "}
              {info.driverVibrance ? `${vibranceSummary(monitors)} from the driver.` : "no driver vibrance, colour boost used instead."} {win ? "Windows colour effects:" : "Hue, saturation and zoom:"}{" "}
              {info.magnifier ? "available." : win ? "not available." : "Windows only."}
            </p>
            <a className="btn small" href="./security.html">
              <Icon name="info" size={14} /> Is Luna safe?
            </a>
          </div>
        </div>
      </Panel>
    </div>
  );
}

function checkedText(at: number | null): string {
  if (!at) return "";
  const d = new Date(at * 1000);
  return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

function UpdatesPanel() {
  const { settings, info, update, toast, setInfo } = useStore();
  const [busy, setBusy] = useState(false);
  const u = info.update;
  const auto = settings.app.autoUpdate;
  const setAuto = (v: boolean) => update((s) => ({ ...s, app: { ...s.app, autoUpdate: v } }));

  const check = async () => {
    setBusy(true);
    try {
      const st = await api.updateCheck();
      setInfo({ update: st });
      if (st.state === "latest") toast("Luna is up to date", "good");
      if (st.state === "ready") toast(`Version ${st.version} is ready`, "good");
      if (st.state === "error") toast("Could not reach the update page", "bad");
    } catch (e) {
      toast(errorText(e), "bad");
    } finally {
      setBusy(false);
    }
  };
  const restart = () => api.updateInstall().catch((e) => toast(errorText(e), "bad"));

  let status: string;
  switch (u.state) {
    case "checking":
      status = "Looking for a new version";
      break;
    case "downloading":
      status = `Downloading version ${u.version} (${Math.round(u.progress * 100)} %)`;
      break;
    case "ready":
      status = `Version ${u.version} is downloaded. It installs when Luna quits, or now with Restart.`;
      break;
    case "latest":
      status = `Up to date, checked at ${checkedText(u.checkedAt)}`;
      break;
    case "error":
      status = "The update page could not be reached. Luna tries again later.";
      break;
    default:
      status = auto
        ? "Luna looks for a new version a few seconds after it starts, then every 6 hours."
        : "Automatic updates are off. Use Check now to look for a new version.";
  }
  const working = busy || u.state === "checking" || u.state === "downloading";

  return (
    <Panel title="Updates" subtitle="New versions come from the project's GitHub page and are checked against Luna's signature before they install.">
      <Row
        title="Automatic updates"
        desc="Installs new versions by itself. Your settings, presets and binds stay as they are."
      >
        <Toggle on={auto} label="Automatic updates" onChange={setAuto} />
      </Row>
      <Row title={`Version ${info.version}`} desc={status}>
        {u.state === "ready" ? (
          <button type="button" className="btn primary" disabled={!isTauri} onClick={restart}>
            <Icon name="download" size={15} /> Restart now
          </button>
        ) : (
          <button type="button" className="btn" disabled={working} onClick={check}>
            <Icon name="reset" size={15} /> {working ? "Checking" : "Check now"}
          </button>
        )}
      </Row>
      {u.state === "ready" && u.notes && <p className="muted small update-notes">{u.notes}</p>}
    </Panel>
  );
}
