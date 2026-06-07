import Link from "next/link";
import { localizedPath, type Locale } from "@/lib/i18n";

const button =
  "font-mono inline-flex min-h-[50px] items-center justify-center border border-ink bg-ink px-5 text-[11px] font-extrabold tracking-[0.06em] text-paper-light uppercase shadow-[5px_5px_0_var(--color-amber)] transition-all duration-200 hover:-translate-x-0.5 hover:-translate-y-0.5 hover:shadow-[7px_7px_0_var(--color-mint)]";

const content = {
  en: {
    label: "[ FAITHEA JOURNAL ]",
    kicker: "Notes from behind the framework.",
    title: ["Build logs,", "benchmarks & ideas."],
    body: "Release notes, design decisions, and practical Rust HTTP articles will be published here.",
    back: "Back to home",
    soon: "First dispatch coming soon",
  },
  "zh-CN": {
    label: "[ FAITHEA 日志 ]",
    kicker: "来自框架背后的笔记。",
    title: ["构建记录、", "基准测试与想法。"],
    body: "发布说明、设计决策和实用的 Rust HTTP 文章将在这里发布。",
    back: "返回首页",
    soon: "第一篇文章即将到来",
  },
} as const;

export function BlogPageView({ locale }: { locale: Locale }) {
  const copy = content[locale];
  return (
    <main className="relative isolate grid min-h-[calc(100svh-72px)] place-items-center overflow-hidden px-6 py-[90px] max-sm:min-h-[calc(100svh-112px)]">
      <div className="placeholder-grid absolute inset-0 -z-20" aria-hidden="true" />
      <div
        className="absolute top-[-30vw] left-[-12vw] -z-10 aspect-square w-[70vw] rounded-full border border-line shadow-[0_0_0_60px_transparent,0_0_0_61px_var(--color-line),0_0_0_130px_transparent,0_0_0_131px_var(--color-line)]"
        aria-hidden="true"
      />
      <section className="w-[min(900px,100%)]">
        <span className="font-mono mb-6 block text-[9px] font-extrabold tracking-[0.13em] text-[#637068] uppercase">
          {copy.label}
        </span>
        <p className="font-mono mb-7 text-xs font-extrabold tracking-[0.08em] uppercase">
          {copy.kicker}
        </p>
        <h1 className="font-display text-[clamp(72px,12vw,154px)] font-black leading-[0.86] tracking-[-0.075em] uppercase max-sm:text-[clamp(62px,19vw,102px)]">
          {copy.title[0]}
          <br />
          <em className="outline-serif">{copy.title[1]}</em>
        </h1>
        <p className="my-8 max-w-[570px] text-[17px] leading-[1.7] text-ink-soft">
          {copy.body}
        </p>
        <div className="flex flex-wrap items-center gap-5">
          <Link className={button} href={localizedPath(locale, "/")}>
            {copy.back}
          </Link>
          <span className="font-mono text-[10px] tracking-[0.06em] text-[#657069] uppercase">
            {copy.soon}
          </span>
        </div>
      </section>
    </main>
  );
}

export default function BlogPage() {
  return <BlogPageView locale="en" />;
}
