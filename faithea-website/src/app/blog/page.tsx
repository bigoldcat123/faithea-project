import Link from "next/link";

export default function BlogPage() {
  return (
    <main className="placeholder-page placeholder-blog">
      <div className="placeholder-grid" aria-hidden="true" />
      <section>
        <span className="section-index">[ FAITHEA JOURNAL ]</span>
        <p className="placeholder-kicker">Notes from behind the framework.</p>
        <h1>
          Build logs,
          <br />
          <em>benchmarks & ideas.</em>
        </h1>
        <p className="placeholder-copy">
          Release notes, design decisions, and practical Rust HTTP articles
          will be published here.
        </p>
        <div className="placeholder-actions">
          <Link className="button button-primary" href="/">
            Back to home
          </Link>
          <span className="coming-soon">First dispatch coming soon</span>
        </div>
      </section>
    </main>
  );
}
