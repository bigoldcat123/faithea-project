"use client";

import Link from "next/link";
import { useState } from "react";
import type { DocTreeNode } from "@/lib/docs";
import type { Locale } from "@/lib/i18n";
import { ui } from "@/lib/i18n";

function Tree({
  nodes,
  currentHref,
  locale,
}: {
  nodes: DocTreeNode[];
  currentHref: string;
  locale: Locale;
}) {
  return (
    <ul className="space-y-1">
      {nodes.map((node) =>
        node.type === "page" ? (
          <li key={node.href}>
            <Link
              className={`font-mono flex items-center justify-between border-l-2 px-3 py-2 text-[11px] leading-5 transition-colors ${
                node.href === currentHref
                  ? "border-ink bg-mint/30 font-black text-ink"
                  : "border-transparent text-[#59645d] hover:border-line hover:text-ink"
              }`}
              href={node.href}
            >
              <span>{node.title}</span>
              {node.missing && locale !== "en" ? (
                <span className="rounded border border-line px-1 text-[7px] font-black">
                  {ui[locale].docs.englishBadge}
                </span>
              ) : null}
            </Link>
          </li>
        ) : (
          <li className="pt-2" key={node.key}>
            <details open={node.defaultOpen}>
              <summary className="font-display cursor-pointer list-none px-3 py-2 text-sm font-black tracking-[-0.02em] uppercase">
                {node.title}
              </summary>
              <div className="ml-2 border-l border-line pl-2">
                <Tree currentHref={currentHref} locale={locale} nodes={node.children} />
              </div>
            </details>
          </li>
        ),
      )}
    </ul>
  );
}

export function DocsSidebar({
  tree,
  currentHref,
  locale,
}: {
  tree: DocTreeNode[];
  currentHref: string;
  locale: Locale;
}) {
  const [open, setOpen] = useState(false);
  const copy = ui[locale].docs;

  return (
    <>
      <button
        className="font-mono fixed right-4 bottom-4 z-40 border border-ink bg-ink px-4 py-3 text-[10px] font-black tracking-[0.08em] text-paper-light uppercase shadow-[4px_4px_0_var(--color-amber)] lg:hidden"
        onClick={() => setOpen(true)}
        type="button"
      >
        {copy.menu}
      </button>
      {open ? (
        <button
          aria-label={copy.close}
          className="fixed inset-0 z-40 bg-ink/40 lg:hidden"
          onClick={() => setOpen(false)}
          type="button"
        />
      ) : null}
      <aside
        className={`fixed top-0 bottom-0 left-0 z-50 w-[min(320px,88vw)] overflow-y-auto border-r border-line bg-paper-light p-5 transition-transform lg:sticky lg:top-[92px] lg:z-0 lg:block lg:h-[calc(100svh-116px)] lg:w-auto lg:translate-x-0 lg:border-r-0 lg:bg-transparent lg:p-0 ${
          open ? "translate-x-0" : "-translate-x-full"
        }`}
      >
        <div className="mb-5 flex items-center justify-between lg:hidden">
          <strong className="font-display text-xl uppercase">{copy.menu}</strong>
          <button className="font-mono text-xs" onClick={() => setOpen(false)} type="button">
            {copy.close}
          </button>
        </div>
        <nav aria-label={copy.menu} onClick={() => setOpen(false)}>
          <Tree currentHref={currentHref} locale={locale} nodes={tree} />
        </nav>
      </aside>
    </>
  );
}
