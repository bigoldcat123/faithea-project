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
    <span className="brand">
      <span className="brand-mark" aria-hidden="true">
        F
      </span>
      <span>faithea</span>
    </span>
  );
}

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body>
        <header className="site-header">
          <div className="nav-shell">
            <Link aria-label="Faithea home" href="/">
              <Logo />
            </Link>
            <nav aria-label="Primary navigation">
              <Link href="/">Home</Link>
              <Link href="/docs">Docs</Link>
              <Link href="/blog">Blog</Link>
            </nav>
            <a
              className="github-link"
              href="https://crates.io/crates/faithea"
              rel="noreferrer"
              target="_blank"
            >
              crates.io
              <span aria-hidden="true">↗</span>
            </a>
          </div>
        </header>
        {children}
        <footer className="site-footer">
          <div className="section-shell footer-inner">
            <Logo />
            <p>Lightweight async HTTP for Rust.</p>
            <div>
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
