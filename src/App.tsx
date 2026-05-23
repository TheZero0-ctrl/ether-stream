import { useEffect, useState } from "react";
import { getBootstrapState } from "./lib/tauri";
import type { BootstrapState } from "./types/bootstrap";
import "./App.css";

function App() {
  const [state, setState] = useState<BootstrapState | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    getBootstrapState()
      .then((result) => {
        if (!cancelled) setState(result);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        setError(err instanceof Error ? err.message : "Failed to load bootstrap state.");
      });

    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <main className="app-shell">
      <section className="hero-panel">
        <div className="eyebrow">Tauri Rewrite Foundation</div>
        <h1>Ether</h1>
        <p className="hero-copy">
          Desktop-first foundation for rebuilding Streambert with a Rust core and a
          React frontend.
        </p>
        <div className="hero-meta">
          <span>Frontend: React + TypeScript</span>
          <span>Backend: Rust + Tauri</span>
        </div>
      </section>

      <section className="grid">
        <article className="card">
          <h2>Bootstrap State</h2>
          {!state && !error && <p className="muted">Loading data from Rust...</p>}
          {error && <p className="error">{error}</p>}
          {state && (
            <dl className="details-list">
              <div>
                <dt>App</dt>
                <dd>{state.app.name}</dd>
              </div>
              <div>
                <dt>Version</dt>
                <dd>{state.app.version}</dd>
              </div>
              <div>
                <dt>Frontend</dt>
                <dd>{state.app.frontend}</dd>
              </div>
              <div>
                <dt>Backend</dt>
                <dd>{state.app.backend}</dd>
              </div>
            </dl>
          )}
        </article>

        <article className="card">
          <h2>Native Capabilities</h2>
          <ul className="feature-list">
            {state?.capabilities.map((item) => <li key={item}>{item}</li>)}
          </ul>
        </article>

        <article className="card card-wide">
          <h2>Suggested Next Steps</h2>
          <ol className="step-list">
            {state?.nextSteps.map((item: string) => <li key={item}>{item}</li>)}
          </ol>
        </article>
      </section>
    </main>
  );
}

export default App;
