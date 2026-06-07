"use client";

import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import { useEffect } from "react";
import { localizedPath, type Locale, ui } from "@/lib/i18n";

function currentLocale(pathname: string): Locale {
  return pathname === "/zh-CN" || pathname.startsWith("/zh-CN/") ? "zh-CN" : "en";
}

function pathForLocale(pathname: string, locale: Locale) {
  const withoutLocale =
    pathname === "/zh-CN" ? "/" : pathname.replace(/^\/zh-CN(?=\/)/, "");
  return localizedPath(locale, withoutLocale);
}

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

export function SiteHeader() {
  const pathname = usePathname();
  const router = useRouter();
  const locale = currentLocale(pathname);
  const copy = ui[locale];

  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);

  return (
    <header className="sticky top-0 z-40 border-b border-line/70 bg-paper-light/90 backdrop-blur-lg max-sm:relative">
      <div className="mx-auto grid min-h-[72px] w-[min(1180px,calc(100%-48px))] grid-cols-[1fr_auto_1fr] items-center max-sm:w-[calc(100%-28px)] max-sm:grid-cols-[1fr_auto]">
        <Link aria-label="Faithea home" href={localizedPath(locale, "/")}>
          <Logo />
        </Link>
        <nav
          className="font-mono flex items-center gap-10 text-xs font-bold tracking-[0.08em] uppercase max-sm:order-3 max-sm:col-span-full max-sm:justify-center max-sm:gap-8 max-sm:border-t max-sm:border-line"
          aria-label="Primary navigation"
        >
          <Link className={navLink} href={localizedPath(locale, "/")}>
            {copy.nav.home}
          </Link>
          <Link className={navLink} href={localizedPath(locale, "/docs")}>
            {copy.nav.docs}
          </Link>
          <Link className={navLink} href={localizedPath(locale, "/blog")}>
            {copy.nav.blog}
          </Link>
        </nav>
        <div className="font-mono flex items-center justify-self-end gap-2">
          <label className="sr-only" htmlFor="site-language">
            {copy.language}
          </label>
          <select
            className="cursor-pointer rounded-full border border-ink bg-transparent px-3 py-2 text-[10px] font-extrabold tracking-[0.06em] uppercase max-sm:px-2 max-sm:py-1.5"
            id="site-language"
            onChange={(event) => router.push(pathForLocale(pathname, event.target.value as Locale))}
            value={locale}
          >
            <option value="en">EN</option>
            <option value="zh-CN">中文</option>
          </select>
          <a
            className="rounded-full border border-ink px-3.5 py-2 text-[11px] font-extrabold tracking-[0.06em] uppercase transition-colors hover:bg-ink hover:text-paper-light max-md:hidden"
            href="https://crates.io/crates/faithea"
            rel="noreferrer"
            target="_blank"
          >
            crates.io ↗
          </a>
        </div>
      </div>
    </header>
  );
}

export function SiteFooter() {
  const pathname = usePathname();
  const locale = currentLocale(pathname);
  const copy = ui[locale];

  return (
    <footer className="border-t border-ink bg-paper-light">
      <div className="mx-auto grid min-h-[110px] w-[min(1180px,calc(100%-48px))] grid-cols-[1fr_auto_1fr] items-center gap-7 max-sm:w-[calc(100%-28px)] max-sm:grid-cols-1 max-sm:py-8">
        <Logo />
        <p className="text-xs text-[#6c756f]">{copy.footer}</p>
        <div className="font-mono flex justify-self-end gap-5 text-[9px] font-extrabold tracking-[0.08em] uppercase max-sm:justify-self-start">
          <Link href={localizedPath(locale, "/docs")}>{copy.nav.docs}</Link>
          <Link href={localizedPath(locale, "/blog")}>{copy.nav.blog}</Link>
          <a href="https://crates.io/crates/faithea" rel="noreferrer" target="_blank">
            crates.io
          </a>
        </div>
      </div>
    </footer>
  );
}
