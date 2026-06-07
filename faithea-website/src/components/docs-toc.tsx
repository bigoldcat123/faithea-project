"use client";

import { useEffect, useState } from "react";
import type { DocHeading } from "@/lib/docs";
import type { Locale } from "@/lib/i18n";
import { ui } from "@/lib/i18n";

export function DocsToc({ headings, locale }: { headings: DocHeading[]; locale: Locale }) {
  const [active, setActive] = useState(headings[0]?.id ?? "");

  useEffect(() => {
    const elements = headings
      .map((heading) => document.getElementById(heading.id))
      .filter((element): element is HTMLElement => Boolean(element));
    const observer = new IntersectionObserver(
      (entries) => {
        const visible = entries.find((entry) => entry.isIntersecting);
        if (visible) setActive(visible.target.id);
      },
      { rootMargin: "-18% 0px -70% 0px" },
    );
    elements.forEach((element) => observer.observe(element));
    return () => observer.disconnect();
  }, [headings]);

  if (!headings.length) return null;

  return (
    <aside className="sticky top-[92px] hidden h-[calc(100svh-116px)] overflow-y-auto xl:block">
      <p className="font-mono mb-4 text-[9px] font-black tracking-[0.1em] text-[#69746d] uppercase">
        {ui[locale].docs.onThisPage}
      </p>
      <nav>
        {headings.map((heading) => (
          <a
            className={`font-mono block border-l py-1.5 text-[10px] leading-4 transition-colors ${
              heading.level === 3 ? "pl-5" : "pl-3"
            } ${active === heading.id ? "border-ink font-black text-ink" : "border-line text-[#778079]"}`}
            href={`#${heading.id}`}
            key={heading.id}
          >
            {heading.text}
          </a>
        ))}
      </nav>
    </aside>
  );
}
