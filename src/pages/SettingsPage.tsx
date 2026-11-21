import { useStore } from "../store";
import { api, errorText, isTauri } from "../lib/backend";
import { Panel, Row, Toggle } from "../components/ui";
import { Icon, MoonLogo } from "../components/Icon";
import { gpuName, usesDriver, vendorLabel, vibranceSummary } from "../lib/gpu";
import { HELP } from "../lib/help";
import { HelpList } from "../components/Help";

export function SettingsPage() {
  const { settings, monitors, info, update, toast, setInfo } = useStore();
  const app = settings.app;
  const setApp = (patch: Partial<typeof app>) => update((s) => ({ ...s, app: { ...s.app, ...patch } }));

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
        <Row title="Start with Windows" desc="Luna opens when you sign in, so your screens are tuned right away.">
          <Toggle on={info.autostart} label="Start with Windows" onChange={autostart} />
        </Row>
        <Row title="Start minimized" desc="When started with Windows, stay in the tray instead of opening the window.">
          <Toggle on={app.startMinimized} label="Start minimized" onChange={(v) => setApp({ startMinimized: v })} />
        </Row>
        <Row title="Close to tray" desc="The close button hides Luna and keeps tuning running. Quit from the tray icon.">
          <Toggle on={app.closeToTray} label="Close to tray" onChange={(v) => setApp({ closeToTray: v })} />
        </Row>
      </Panel>

      <Panel title="Your data" subtitle="Everything stays on this PC. Luna has no account and sends nothing anywhere.">
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

      <Panel title="Screens and graphics cards" subtitle="Found by Luna when it starts and again whenever Windows changes the screens.">
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
              {info.driverVibrance ? `${vibranceSummary(monitors)} from the driver.` : "no driver vibrance, colour boost used instead."} Windows colour effects:{" "}
              {info.magnifier ? "available." : "not available."}
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
