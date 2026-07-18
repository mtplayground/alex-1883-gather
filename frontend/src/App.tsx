export function App() {
  return (
    <main className="app-shell">
      <section className="workspace-panel" aria-labelledby="workspace-title">
        <p className="eyebrow">Issue #1</p>
        <h1 id="workspace-title">Full-stack workspace initialized</h1>
        <p>
          The React client and Rust API are ready for focused feature work in
          later issues.
        </p>
        <dl className="status-grid" aria-label="Project entry points">
          <div>
            <dt>Frontend</dt>
            <dd>Vite React SPA</dd>
          </div>
          <div>
            <dt>Backend</dt>
            <dd>Axum on port 8080</dd>
          </div>
          <div>
            <dt>API health</dt>
            <dd>/health</dd>
          </div>
        </dl>
      </section>
    </main>
  );
}
