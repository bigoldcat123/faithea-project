import Link from "next/link";

const features = [
  {
    number: "01",
    title: "Macro-first routing",
    description:
      "Declare intent where it belongs. Route macros keep handlers compact, readable, and close to the HTTP contract.",
    code: '#[get("/users/{id}")]',
  },
  {
    number: "02",
    title: "Async by default",
    description:
      "Built on Tokio for non-blocking I/O and predictable concurrency, without hiding the runtime behind heavy abstractions.",
    code: "async fn find_user(id: String)",
  },
  {
    number: "03",
    title: "Composable responses",
    description:
      "Return strings, JSON, files, or your own response modifiers. Faithea turns focused values into complete responses.",
    code: 'res_modifiers!("ready", CORS)',
  },
];

const capabilities = [
  "Path & query params",
  "JSON extraction",
  "Multipart forms",
  "WebSocket",
  "Server-sent events",
  "CORS",
  "TLS / HTTPS",
  "Custom extractors",
];

const codeLines = [
  <>
    <span className="code-keyword">use</span> faithea::&#123;get, HttpServer&#125;;
  </>,
  <></>,
  <>
    <span className="code-macro">#[get(&quot;/hello/&#123;name&#125;&quot;)]</span>
  </>,
  <>
    <span className="code-keyword">async fn</span>{" "}
    <span className="code-function">hello</span>(name: String) &#123;
  </>,
  <>
    {"  "}
    <span className="code-string">format!</span>(&quot;Hello, &#123;name&#125;!&quot;)
  </>,
  <> &#125;</>,
  <></>,
  <>
    <span className="code-macro">#[tokio::main]</span>
  </>,
  <>
    <span className="code-keyword">async fn</span>{" "}
    <span className="code-function">main</span>() &#123;
  </>,
  <>{"  "}HttpServer::builder()</>,
  <>
    {"    "}.mount(&quot;/&quot;, handlers!(hello))
  </>,
  <>{"    "}.build().run().await;</>,
  <> &#125;</>,
];

function ArrowIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 20 20">
      <path d="M4 10h11M11 5l5 5-5 5" />
    </svg>
  );
}

function Mark() {
  return (
    <svg aria-hidden="true" className="hero-mark" viewBox="0 0 96 96">
      <path d="M20 18h54L49 47h28L22 82l20-29H18z" />
    </svg>
  );
}

export default function Home() {
  return (
    <main>
      <section className="hero section-shell">
        <div className="hero-copy">
          <div className="eyebrow reveal reveal-1">
            <span className="status-dot" />
            Lightweight async HTTP for Rust
          </div>
          <h1 className="reveal reveal-2">
            Build fast.
            <br />
            <span>Stay close to Rust.</span>
          </h1>
          <p className="hero-lede reveal reveal-3">
            Faithea is a compact, Tokio-powered HTTP framework designed for
            developers who want expressive routing and control without the
            machinery.
          </p>
          <div className="hero-actions reveal reveal-4">
            <Link className="button button-primary" href="/docs">
              Read the docs <ArrowIcon />
            </Link>
            <a
              className="button button-secondary"
              href="https://crates.io/crates/faithea"
              rel="noreferrer"
              target="_blank"
            >
              View on crates.io
            </a>
          </div>
          <div className="install-command reveal reveal-4">
            <span>$</span>
            <code>cargo add faithea</code>
            <span className="command-note">one dependency, then build</span>
          </div>
        </div>

        <div className="hero-visual reveal reveal-3" aria-label="Faithea code example">
          <div className="orbit orbit-one" />
          <div className="orbit orbit-two" />
          <div className="mark-wrap">
            <Mark />
          </div>
          <div className="code-window">
            <div className="window-bar">
              <div className="window-dots">
                <span />
                <span />
                <span />
              </div>
              <span>src/main.rs</span>
              <span className="window-state">● compiled</span>
            </div>
            <pre>
              <code>
                {codeLines.map((line, index) => (
                  <span className="code-line" key={index}>
                    <span className="line-number">
                      {String(index + 1).padStart(2, "0")}
                    </span>
                    <span>{line}</span>
                  </span>
                ))}
              </code>
            </pre>
          </div>
          <div className="metric-card metric-top">
            <span>Runtime</span>
            <strong>Tokio</strong>
          </div>
          <div className="metric-card metric-bottom">
            <span>Philosophy</span>
            <strong>Less magic</strong>
          </div>
        </div>
      </section>

      <section className="marquee" aria-label="Faithea capabilities">
        <div>
          {[...capabilities, ...capabilities].map((capability, index) => (
            <span key={`${capability}-${index}`}>
              {capability}
              <i>+</i>
            </span>
          ))}
        </div>
      </section>

      <section className="feature-section section-shell" id="features">
        <div className="section-intro">
          <span className="section-index">[ 01 / WHY FAITHEA ]</span>
          <h2>
            Small surface.
            <br />
            <em>Serious capability.</em>
          </h2>
          <p>
            The essentials for modern HTTP services, shaped into APIs that
            remain understandable as your project grows.
          </p>
        </div>
        <div className="feature-list">
          {features.map((feature) => (
            <article className="feature-card" key={feature.number}>
              <span className="feature-number">{feature.number}</span>
              <div>
                <h3>{feature.title}</h3>
                <p>{feature.description}</p>
              </div>
              <code>{feature.code}</code>
            </article>
          ))}
        </div>
      </section>

      <section className="principles">
        <div className="section-shell principles-grid">
          <div>
            <span className="section-index section-index-light">
              [ 02 / THE APPROACH ]
            </span>
            <h2>Explicit where it matters. Effortless where it should be.</h2>
          </div>
          <div className="principle-list">
            <article>
              <span>01</span>
              <h3>Learn it quickly</h3>
              <p>
                A focused API and familiar Rust types keep the path from first
                route to working service short.
              </p>
            </article>
            <article>
              <span>02</span>
              <h3>Extend it naturally</h3>
              <p>
                Custom request extractors and response modifiers let your own
                types participate directly.
              </p>
            </article>
            <article>
              <span>03</span>
              <h3>Ship with confidence</h3>
              <p>
                Rust&apos;s type system, Tokio&apos;s proven runtime, and fewer
                hidden behaviors make services easier to reason about.
              </p>
            </article>
          </div>
        </div>
      </section>

      <section className="cta section-shell">
        <div className="cta-copy">
          <span className="section-index">[ READY WHEN YOU ARE ]</span>
          <h2>Your next service can be simpler.</h2>
          <p>
            Add Faithea, write a handler, and let Rust do what it does best.
          </p>
        </div>
        <div className="cta-actions">
          <div className="cta-command">
            <span>$</span>
            <code>cargo add faithea</code>
          </div>
          <Link className="button button-primary" href="/docs">
            Start building <ArrowIcon />
          </Link>
        </div>
      </section>
    </main>
  );
}
