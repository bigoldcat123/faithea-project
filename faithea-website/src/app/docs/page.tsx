import Link from "next/link";

export default function DocsPage() {
  return (
    <main className="placeholder-page">
      <div className="placeholder-grid" aria-hidden="true" />
      <section>
        <span className="section-index">[ DOCUMENTATION ]</span>
        <p className="placeholder-kicker">The field guide is being written.</p>
        <h1>
          Clear docs for
          <br />
          <em>clear APIs.</em>
        </h1>
        <p className="placeholder-copy">
          Installation, routing, request extraction, response modifiers, and
          deployment guides will live here.
        </p>
        <div className="placeholder-actions">
          <Link className="button button-primary" href="/">
            Back to home
          </Link>
          <code>cargo add faithea</code>
        </div>
      </section>
    </main>
  );
}
