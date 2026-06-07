import type { Metadata } from "next";
import Link from "next/link";
import "./globals.css";

export const metadata: Metadata = {
  title: "Faithea | Lightweight async HTTP for Rust",
  description:
    "Faithea is a lightweight, asynchronous HTTP framework built with Tokio.",
};

function Logo() {
  return (
    <span className="font-display inline-flex items-center gap-2.5 text-[21px] font-black tracking-[-0.045em] uppercase">
      <span
        className="font-mono grid size-[29px] place-items-center rounded-[4px_11px_4px_4px] bg-ink text-[17px] italic text-mint shadow-[4px_4px_0_var(--color-amber)]"
        aria-hidden="true"
      >
        F
      </span>
      <span>faithea</span>
    </span>
  );
}

const navLink =
  "relative py-[27px] after:absolute after:right-0 after:bottom-5 after:left-0 after:h-0.5 after:origin-center after:scale-x-0 after:bg-ink after:transition-transform hover:after:scale-x-100 focus-visible:after:scale-x-100 max-sm:py-[13px]";

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className="overflow-x-hidden bg-paper font-body text-ink">
        <header className="sticky top-0 z-30 border-b border-line/70 bg-paper-light/90 backdrop-blur-lg max-sm:relative">
          <div className="mx-auto grid min-h-[72px] w-[min(1180px,calc(100%-48px))] grid-cols-[1fr_auto_1fr] items-center max-sm:w-[calc(100%-28px)] max-sm:grid-cols-[1fr_auto]">
            <Link aria-label="Faithea home" href="/">
              <Logo />
            </Link>
            <nav
              className="font-mono flex items-center gap-10 text-xs font-bold tracking-[0.08em] uppercase max-sm:order-3 max-sm:col-span-full max-sm:justify-center max-sm:gap-8 max-sm:border-t max-sm:border-line"
              aria-label="Primary navigation"
            >
              <Link className={navLink} href="/">
                Home
              </Link>
              <Link className={navLink} href="/docs">
                Docs
              </Link>
              <Link className={navLink} href="/blog">
                Blog
              </Link>
            </nav>
            <a
              className="font-mono justify-self-end rounded-full border border-ink px-3.5 py-2 text-[11px] font-extrabold tracking-[0.06em] uppercase transition-colors hover:bg-ink hover:text-paper-light max-sm:px-2.5 max-sm:py-1.5"
              href="https://crates.io/crates/faithea"
              rel="noreferrer"
              target="_blank"
            >
              crates.io <span aria-hidden="true">↗</span>
            </a>
          </div>
        </header>
        {children}
        <footer className="border-t border-ink bg-paper-light">
          <div className="mx-auto grid min-h-[110px] w-[min(1180px,calc(100%-48px))] grid-cols-[1fr_auto_1fr] items-center gap-7 max-sm:w-[calc(100%-28px)] max-sm:grid-cols-1 max-sm:py-8">
            <Logo />
            <p className="text-xs text-[#6c756f]">
              Lightweight async HTTP for Rust.
            </p>
            <div className="font-mono flex justify-self-end gap-5 text-[9px] font-extrabold tracking-[0.08em] uppercase max-sm:justify-self-start">
              <Link href="/docs">Docs</Link>
              <Link href="/blog">Blog</Link>
              <a
                href="https://crates.io/crates/faithea"
                rel="noreferrer"
                target="_blank"
              >
                crates.io
              </a>
            </div>
          </div>
        </footer>
      </body>
    </html>
  );
}
