export const locales = ["en", "zh-CN"] as const;
export type Locale = (typeof locales)[number];

export const defaultLocale: Locale = "en";

export function isLocale(value: string): value is Locale {
  return locales.includes(value as Locale);
}

export function localePrefix(locale: Locale) {
  return locale === defaultLocale ? "" : `/${locale}`;
}

export function localizedPath(locale: Locale, path: string) {
  const normalized = path === "/" ? "" : path.startsWith("/") ? path : `/${path}`;
  return `${localePrefix(locale)}${normalized}` || "/";
}

export const ui = {
  en: {
    nav: { home: "Home", docs: "Docs", blog: "Blog" },
    language: "Language",
    footer: "Lightweight async HTTP for Rust.",
    docs: {
      menu: "Browse docs",
      close: "Close",
      onThisPage: "On this page",
      previous: "Previous",
      next: "Next",
      missingTitle: "This page is not translated yet.",
      missingBody:
        "The current language version is not available. You can continue with the English page.",
      readEnglish: "Read the English version",
      englishBadge: "EN",
    },
  },
  "zh-CN": {
    nav: { home: "首页", docs: "文档", blog: "博客" },
    language: "语言",
    footer: "轻量、异步的 Rust HTTP 框架。",
    docs: {
      menu: "浏览文档",
      close: "关闭",
      onThisPage: "本页目录",
      previous: "上一篇",
      next: "下一篇",
      missingTitle: "当前语言版本尚未支持",
      missingBody: "这篇文档还没有简体中文版本，你可以继续阅读英文版本。",
      readEnglish: "阅读英文版本",
      englishBadge: "EN",
    },
  },
} as const;
