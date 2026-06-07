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

const principles = [
  {
    number: "01",
    title: "Learn it quickly",
    body: "A focused API and familiar Rust types keep the path from first route to working service short.",
  },
  {
    number: "02",
    title: "Extend it naturally",
    body: "Custom request extractors and response modifiers let your own types participate directly.",
  },
  {
    number: "03",
    title: "Ship with confidence",
    body: "Rust's type system, Tokio's proven runtime, and fewer hidden behaviors make services easier to reason about.",
  },
];

const codeLines = [
  <>
    <span className="text-[#fa8e76]">use</span> faithea::&#123;get, HttpServer&#125;;
  </>,
  <></>,
  <span className="text-amber" key="route">
    #[get(&quot;/hello/&#123;name&#125;&quot;)]
  </span>,
  <>
    <span className="text-[#fa8e76]">async fn</span>{" "}
    <span className="text-[#86d8ff]">hello</span>(name: String) &#123;
  </>,
  <>
    {"  "}
    <span className="text-mint">format!</span>(&quot;Hello, &#123;name&#125;!&quot;)
  </>,
  <> &#125;</>,
  <></>,
  <span className="text-amber" key="tokio">
    #[tokio::main]
  </span>,
  <>
    <span className="text-[#fa8e76]">async fn</span>{" "}
    <span className="text-[#86d8ff]">main</span>() &#123;
  </>,
  <>{"  "}HttpServer::builder()</>,
  <>
    {"    "}.mount(&quot;/&quot;, handlers!(hello))
  </>,
  <>{"    "}.build().run().await;</>,
  <> &#125;</>,
];

const shell = "mx-auto w-[min(1180px,calc(100%-48px))] max-sm:w-[calc(100%-28px)]";
const displayHeading =
  "font-display font-black leading-[0.86] tracking-[-0.075em] uppercase";
const label =
  "font-mono mb-6 block text-[9px] font-extrabold tracking-[0.13em] text-[#637068] uppercase";
const button =
  "font-mono inline-flex min-h-[50px] items-center justify-center gap-3.5 border border-ink px-5 text-[11px] font-extrabold tracking-[0.06em] uppercase transition-all duration-200 hover:-translate-x-0.5 hover:-translate-y-0.5 hover:shadow-[7px_7px_0_var(--color-mint)] [&_svg]:w-[17px] [&_svg]:fill-none [&_svg]:stroke-current [&_svg]:stroke-[1.8]";
const primaryButton = `${button} bg-ink text-paper-light shadow-[5px_5px_0_var(--color-amber)]`;

function ArrowIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 20 20">
      <path d="M4 10h11M11 5l5 5-5 5" />
    </svg>
  );
}

function Mark() {
  return (
    <svg aria-hidden="true" className="w-[70px] fill-ink max-sm:w-[50px]" viewBox="0 0 96 96">
      <path d="M20 18h54L49 47h28L22 82l20-29H18z" />
    </svg>
  );
}

function MetricCard({
  className,
  label,
  value,
}: {
  className: string;
  label: string;
  value: string;
}) {
  return (
    <div
      className={`font-mono absolute z-10 min-w-32 border border-ink bg-paper-light px-4 py-3 shadow-[5px_5px_0_var(--color-ink)] max-sm:min-w-[102px] max-sm:px-2.5 max-sm:py-2 ${className}`}
    >
      <span className="mb-1.5 block text-[8px] tracking-[0.1em] text-[#68736b] uppercase">
        {label}
      </span>
      <strong className="text-xs">{value}</strong>
    </div>
  );
}

export default function Home() {
  return (
    <main>
      <section
        className={`${shell} grid min-h-[calc(100svh-72px)] grid-cols-[minmax(0,0.88fr)_minmax(510px,1.12fr)] items-center gap-14 py-[84px_72px] max-lg:grid-cols-1 max-lg:pt-[90px] max-sm:min-h-0 max-sm:gap-7 max-sm:py-[64px_46px]`}
      >
        <div className="relative z-10">
          <div className="animate-reveal font-mono mb-6 flex items-center gap-2.5 text-[11px] font-extrabold tracking-[0.1em] uppercase [animation-delay:70ms]">
            <span className="size-[9px] rounded-full bg-[#62c24b] shadow-[0_0_0_4px_rgb(98_194_75/18%)]" />
            Lightweight async HTTP for Rust
          </div>
          <h1
            className={`${displayHeading} animate-reveal max-w-[660px] text-[clamp(68px,8.2vw,122px)] [animation-delay:140ms] [word-spacing:0.08em] max-sm:text-[clamp(61px,19vw,90px)]`}
          >
            Build fast.
            <br />
            <span className="outline-serif">Stay close to Rust.</span>
          </h1>
          <p className="animate-reveal mt-8 max-w-[580px] text-lg leading-[1.65] text-ink-soft [animation-delay:240ms] max-sm:text-base">
            Faithea is a compact, Tokio-powered HTTP framework designed for
            developers who want expressive routing and control without the
            machinery.
          </p>
          <div className="animate-reveal mt-8 flex flex-wrap gap-3 [animation-delay:340ms]">
            <Link className={primaryButton} href="/docs">
              Read the docs <ArrowIcon />
            </Link>
            <a
              className={`${button} bg-paper-light`}
              href="https://crates.io/crates/faithea"
              rel="noreferrer"
              target="_blank"
            >
              View on crates.io
            </a>
          </div>
          <div className="animate-reveal font-mono mt-10 grid max-w-[460px] grid-cols-[auto_auto_1fr] items-center gap-3 border-t border-line pt-4 text-xs [animation-delay:340ms] max-sm:grid-cols-[auto_1fr]">
            <span className="font-black text-[#629a35]">$</span>
            <code>cargo add faithea</code>
            <span className="justify-self-end text-[9px] tracking-[0.08em] text-[#778078] uppercase max-sm:hidden">
              one dependency, then build
            </span>
          </div>
        </div>

        <div
          className="animate-reveal relative isolate grid min-h-[610px] place-items-center overflow-hidden [animation-delay:240ms] max-sm:min-h-[430px]"
          aria-label="Faithea code example"
        >
          <div className="hero-rings absolute -z-30 aspect-square w-[82%] rounded-full max-sm:w-[105%]" />
          <div className="animate-orbit absolute -z-20 h-[360px] w-[500px] rotate-[-14deg] rounded-full border border-line before:absolute before:top-[-6px] before:left-1/2 before:size-[11px] before:rounded-full before:border-2 before:border-ink before:bg-mint max-sm:h-[300px] max-sm:w-[94%]" />
          <div className="animate-orbit-reverse absolute -z-20 h-[510px] w-[410px] rotate-[31deg] rounded-full border border-line before:absolute before:top-[-6px] before:left-1/2 before:size-[11px] before:rounded-full before:border-2 before:border-ink before:bg-mint max-sm:h-[390px] max-sm:w-[70%]" />
          <div className="absolute top-[9%] right-[6%] -z-10 grid size-[130px] rotate-[8deg] place-items-center rounded-full border border-ink bg-amber max-sm:top-[1%] max-sm:right-0 max-sm:size-[90px]">
            <Mark />
          </div>

          <div className="group w-[min(100%,580px)] rotate-[-1.2deg] overflow-hidden rounded-[5px] border border-[#344039] bg-[#101613] text-[#d9e2da] shadow-[16px_18px_0_rgb(17_23_20/9%)] transition-transform duration-300 hover:translate-y-[-5px] hover:rotate-0">
            <div className="font-mono grid min-h-[42px] grid-cols-[1fr_auto_1fr] items-center border-b border-[#344039] px-4 text-[9px] tracking-[0.06em] text-[#9aa49d]">
              <div className="flex gap-1.5 [&_span]:size-2 [&_span]:rounded-full">
                <span className="bg-[#ef765f]" />
                <span className="bg-amber" />
                <span className="bg-mint" />
              </div>
              <span>src/main.rs</span>
              <span className="justify-self-end text-mint">● compiled</span>
            </div>
            <pre className="font-mono m-0 px-[18px] py-[24px_26px] text-[clamp(10px,1.15vw,13px)] leading-[1.72] max-sm:px-2.5 max-sm:text-[8.5px]">
              <code className="block">
                {codeLines.map((line, index) => (
                  <span
                    className="grid grid-cols-[30px_1fr] max-sm:grid-cols-[24px_1fr]"
                    key={index}
                  >
                    <span className="select-none text-[#56635a]">
                      {String(index + 1).padStart(2, "0")}
                    </span>
                    <span>{line}</span>
                  </span>
                ))}
              </code>
            </pre>
          </div>
          <MetricCard
            className="top-[20%] left-0 rotate-[-4deg] max-sm:top-[12%]"
            label="Runtime"
            value="Tokio"
          />
          <MetricCard
            className="right-0 bottom-[19%] rotate-[3deg] max-sm:bottom-[8%]"
            label="Philosophy"
            value="Less magic"
          />
        </div>
      </section>

      <section
        className="font-mono overflow-hidden border-y border-ink bg-mint py-3 text-[10px] font-black tracking-[0.1em] uppercase"
        aria-label="Faithea capabilities"
      >
        <div className="animate-ticker flex w-max">
          {[...capabilities, ...capabilities].map((capability, index) => (
            <span
              className="inline-flex items-center gap-7 pr-7"
              key={`${capability}-${index}`}
            >
              {capability}
              <i className="text-lg font-normal not-italic">+</i>
            </span>
          ))}
        </div>
      </section>

      <section
        className={`${shell} grid grid-cols-[0.82fr_1.18fr] gap-[90px] py-[132px] max-lg:grid-cols-1 max-sm:gap-14 max-sm:py-[82px]`}
        id="features"
      >
        <div className="sticky top-[130px] self-start max-lg:static">
          <span className={label}>[ 01 / WHY FAITHEA ]</span>
          <h2
            className={`${displayHeading} text-[clamp(53px,6.5vw,88px)]`}
          >
            Small surface.
            <br />
            <em className="outline-serif">Serious capability.</em>
          </h2>
          <p className="mt-8 max-w-[410px] leading-[1.7] text-ink-soft">
            The essentials for modern HTTP services, shaped into APIs that
            remain understandable as your project grows.
          </p>
        </div>
        <div className="border-t border-ink">
          {features.map((feature) => (
            <article
              className="grid grid-cols-[50px_1fr] gap-[18px] border-b border-ink py-[34px] pr-1 transition-[padding] duration-200 hover:pl-3"
              key={feature.number}
            >
              <span className="font-mono pt-1 text-[9px] text-[#7a827d]">
                {feature.number}
              </span>
              <div>
                <h3 className="font-display m-0 text-3xl font-black tracking-[-0.045em] uppercase">
                  {feature.title}
                </h3>
                <p className="my-[12px_22px] max-w-[520px] leading-[1.65] text-ink-soft">
                  {feature.description}
                </p>
              </div>
              <code className="font-mono col-start-2 justify-self-start border border-line bg-paper-light px-2 py-1.5 text-[9px] text-[#56625a]">
                {feature.code}
              </code>
            </article>
          ))}
        </div>
      </section>

      <section className="technical-grid border-y border-[#465249] bg-ink text-paper-light">
        <div
          className={`${shell} grid grid-cols-[1fr_1.4fr] gap-[90px] py-[110px] max-lg:grid-cols-1 max-sm:gap-14 max-sm:py-[82px]`}
        >
          <div>
            <span className={`${label} text-mint`}>[ 02 / THE APPROACH ]</span>
            <h2
              className={`${displayHeading} max-w-[540px] text-[clamp(58px,7vw,96px)]`}
            >
              Explicit where it matters. Effortless where it should be.
            </h2>
          </div>
          <div className="grid grid-cols-3 self-end border-t border-[#59665d] max-sm:grid-cols-1">
            {principles.map((principle, index) => (
              <article
                className={`min-h-[270px] border-r border-[#59665d] px-[22px] py-[22px_16px] max-sm:min-h-0 max-sm:border-x-0 max-sm:border-b max-sm:px-0 max-sm:py-6 ${
                  index === 0 ? "border-l max-sm:border-l-0" : ""
                }`}
                key={principle.number}
              >
                <span className="font-mono mb-[72px] block text-[9px] text-amber max-sm:mb-6">
                  {principle.number}
                </span>
                <h3 className="font-display m-0 text-[22px] font-black tracking-[-0.045em] text-mint uppercase">
                  {principle.title}
                </h3>
                <p className="text-[13px] leading-[1.7] text-[#acb4ae]">
                  {principle.body}
                </p>
              </article>
            ))}
          </div>
        </div>
      </section>

      <section
        className={`${shell} grid grid-cols-[1.2fr_0.8fr] items-end gap-16 py-[126px] max-sm:grid-cols-1 max-sm:gap-14 max-sm:py-[82px]`}
      >
        <div>
          <span className={label}>[ READY WHEN YOU ARE ]</span>
          <h2
            className={`${displayHeading} max-w-[700px] text-[clamp(64px,8vw,112px)]`}
          >
            Your next service can be simpler.
          </h2>
          <p className="max-w-[470px] text-[17px] leading-[1.6] text-ink-soft">
            Add Faithea, write a handler, and let Rust do what it does best.
          </p>
        </div>
        <div className="grid gap-4">
          <div className="font-mono flex items-center gap-3 border border-ink bg-paper-light p-4 text-xs">
            <span className="font-black text-[#63923d]">$</span>
            <code>cargo add faithea</code>
          </div>
          <Link className={primaryButton} href="/docs">
            Start building <ArrowIcon />
          </Link>
        </div>
      </section>
    </main>
  );
}
