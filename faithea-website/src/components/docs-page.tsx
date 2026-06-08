import Link from "next/link";
import { DocsProse } from "./docs-prose";
import { DocsSidebar } from "./docs-sidebar";
import { DocsToc } from "./docs-toc";
import type { DocPageData, DocTreeNode } from "@/lib/docs";
import { localePrefix, type Locale, ui } from "@/lib/i18n";

export function DocsPageView({
  page,
  tree,
  locale,
}: {
  page: DocPageData;
  tree: DocTreeNode[];
  locale: Locale;
}) {
  const currentHref = `${localePrefix(locale)}/docs${page.slug.length ? `/${page.slug.join("/")}` : ""}`;
  const copy = ui[locale].docs;

  return (
    <main className="mx-auto grid w-[min(1480px,calc(100%-48px))] grid-cols-[230px_minmax(0,1fr)_190px] gap-12 py-12 max-xl:grid-cols-[230px_minmax(0,1fr)] max-lg:block max-sm:w-[calc(100%-28px)] max-sm:py-8">
      <DocsSidebar currentHref={currentHref} locale={locale} tree={tree} />
      <article className="min-w-0 max-w-[820px]">
        <header className="mb-10 border-b border-line pb-8">
          <span className="font-mono text-[9px] font-black tracking-[0.12em] text-[#69746d] uppercase">
            [ Documentation / {locale} ]
          </span>
          <h1 className="font-display mt-4 text-[clamp(48px,7vw,82px)] font-black leading-[0.92] tracking-[-0.065em] uppercase">
            {page.title}
          </h1>
          {page.description ? (
            <p className="mt-5 max-w-[650px] text-[17px] leading-7 text-ink-soft">
              {page.description}
            </p>
          ) : null}
        </header>

        {page.missing ? (
          <section className="border border-ink bg-paper-light p-8 shadow-[8px_8px_0_var(--color-amber)]">
            <span className="font-mono text-[9px] font-black tracking-[0.12em] uppercase">
              [ Missing translation ]
            </span>
            <h2 className="font-display mt-5 text-4xl font-black tracking-[-0.04em] uppercase">
              {copy.missingTitle}
            </h2>
            <p className="my-5 max-w-xl leading-7 text-ink-soft">{copy.missingBody}</p>
            <Link
              className="font-mono inline-flex border border-ink bg-ink px-4 py-3 text-[10px] font-black tracking-[0.08em] text-paper-light uppercase"
              href={page.englishHref}
            >
              {copy.readEnglish} →
            </Link>
          </section>
        ) : (
          <DocsProse html={page.html ?? ""} />
        )}

        <nav className="mt-16 grid grid-cols-2 gap-4 border-t border-line pt-8 max-sm:grid-cols-1">
          {page.previous ? (
            <Link className="border border-line bg-paper-light p-4" href={page.previous.href}>
              <span className="font-mono block text-[8px] font-black tracking-[0.1em] text-[#748078] uppercase">
                ← {copy.previous}
              </span>
              <strong className="font-display mt-2 block text-lg uppercase">
                {page.previous.title}
              </strong>
            </Link>
          ) : (
            <span />
          )}
          {page.next ? (
            <Link className="border border-line bg-paper-light p-4 text-right" href={page.next.href}>
              <span className="font-mono block text-[8px] font-black tracking-[0.1em] text-[#748078] uppercase">
                {copy.next} →
              </span>
              <strong className="font-display mt-2 block text-lg uppercase">
                {page.next.title}
              </strong>
            </Link>
          ) : null}
        </nav>
      </article>
      <DocsToc headings={page.headings} locale={locale} />
    </main>
  );
}
