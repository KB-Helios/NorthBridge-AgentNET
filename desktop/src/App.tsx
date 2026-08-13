import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

type GatewayStatus = {
  state: string;
  adminUrl: string;
  llmBaseUrl: string;
  configPath: string;
  detail: string;
};

const idleStatus: GatewayStatus = {
  state: "checking",
  adminUrl: "http://127.0.0.1:15000",
  llmBaseUrl: "http://127.0.0.1:14000/v1",
  configPath: "",
  detail: "Checking local sidecar status.",
};

export default function App() {
  const [status, setStatus] = useState<GatewayStatus>(idleStatus);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      setError(null);
      setStatus(await invoke<GatewayStatus>("gateway_status"));
    } catch (reason) {
      setError(String(reason));
    }
  }, []);

  const start = useCallback(async () => {
    try {
      setBusy(true);
      setError(null);
      setStatus(await invoke<GatewayStatus>("start_gateway"));
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  }, []);

  const stop = useCallback(async () => {
    try {
      setBusy(true);
      setError(null);
      setStatus(await invoke<GatewayStatus>("stop_gateway"));
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
    const timer = window.setInterval(() => void refresh(), 2500);
    return () => window.clearInterval(timer);
  }, [refresh]);

  const running = status.state === "running";

  return (
    <main className="shell">
      <section className="hero">
        <div>
          <p className="eyebrow">NORTHBRIDGE AGENTNET</p>
          <h1>Local-first agent command center</h1>
          <p className="lede">The native host supervises a loopback-only AgentGateway now. The existing QM web interface and durable SQLite desktop profile are the next staged runtime integrations.</p>
        </div>
        <div className={`state ${status.state}`}>
          <span className="state-dot" />
          <span>{status.state}</span>
        </div>
      </section>

      <section className="grid">
        <article className="card primary">
          <p className="label">Gateway runtime</p>
          <h2>{running ? "AgentGateway is local and ready" : "AgentGateway is not running"}</h2>
          <p>{status.detail}</p>
          <div className="actions">
            <button type="button" onClick={() => void start()} disabled={busy || running}>Start gateway</button>
            <button type="button" className="secondary" onClick={() => void stop()} disabled={busy || !running}>Stop gateway</button>
            <button type="button" className="quiet" onClick={() => void refresh()} disabled={busy}>Refresh</button>
          </div>
          {error ? <p className="error">{error}</p> : null}
        </article>

        <article className="card">
          <p className="label">Loopback topology</p>
          <dl>
            <div><dt>Gateway administration</dt><dd>{status.adminUrl}</dd></div>
            <div><dt>OpenAI-compatible base URL</dt><dd>{status.llmBaseUrl}</dd></div>
            <div><dt>Managed configuration</dt><dd>{status.configPath || "Resolved on first start"}</dd></div>
          </dl>
        </article>

        <article className="card">
          <p className="label">Desktop runtime plan</p>
          <ol>
            <li>Bundle the verified AgentGateway binary.</li>
            <li>Stage Node 24 and existing QM resources.</li>
            <li>Reuse the QM Vite/Lit interface over the local core API.</li>
            <li>Replace Postgres and pg-boss with tested SQLite adapters.</li>
          </ol>
        </article>
      </section>
    </main>
  );
}
