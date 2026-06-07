import { existsSync, readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import matter from "gray-matter";
import rehypeAutolinkHeadings from "rehype-autolink-headings";
import rehypePrettyCode from "rehype-pretty-code";
import rehypeSlug from "rehype-slug";
import rehypeStringify from "rehype-stringify";
import remarkGfm from "remark-gfm";
import remarkParse from "remark-parse";
import remarkRehype from "remark-rehype";
import { unified } from "unified";
import { visit } from "unist-util-visit";
import { defaultLocale, localePrefix, type Locale } from "./i18n";

const DOCS_ROOT = path.join(process.cwd(), "content", "docs");
const FILE_PATTERN = /^(.*)\.(en|zh-CN)\.md$/;

type MetaFile = {
  title?: Partial<Record<Locale, string>>;
  order?: string[];
  defaultOpen?: boolean;
};

type MarkdownVersion = {
  filePath: string;
  title: string;
  description: string;
};

type CanonicalPage = {
  key: string;
  slug: string[];
  versions: Partial<Record<Locale, MarkdownVersion>>;
};

export type DocTreePage = {
  type: "page";
  key: string;
  title: string;
  href: string;
  missing: boolean;
};

export type DocTreeSection = {
  type: "section";
  key: string;
  title: string;
  defaultOpen: boolean;
  children: DocTreeNode[];
};

export type DocTreeNode = DocTreePage | DocTreeSection;

export type DocHeading = {
  id: string;
  text: string;
  level: 2 | 3;
};

export type DocPageData = {
  slug: string[];
  title: string;
  description: string;
  html: string | null;
  headings: DocHeading[];
  missing: boolean;
  englishHref: string;
  previous: { title: string; href: string } | null;
  next: { title: string; href: string } | null;
};

type AstNode = {
  type: string;
  value?: string;
  url?: string;
  depth?: number;
  children?: AstNode[];
};

function readMeta(directory: string): MetaFile {
  try {
    return JSON.parse(readFileSync(path.join(directory, "_meta.json"), "utf8")) as MetaFile;
  } catch {
    return {};
  }
}

function readVersion(filePath: string, fallbackTitle: string): MarkdownVersion {
  const parsed = matter(readFileSync(filePath, "utf8"));
  return {
    filePath,
    title: String(parsed.data.title || fallbackTitle),
    description: String(parsed.data.description || ""),
  };
}

function humanize(value: string) {
  return value
    .replace(/[-_]+/g, " ")
    .replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function orderItems<T extends { key: string }>(items: T[], order: string[] = []) {
  const positions = new Map(order.map((key, index) => [key, index]));
  return items.sort((a, b) => {
    const aIndex = positions.get(a.key) ?? Number.MAX_SAFE_INTEGER;
    const bIndex = positions.get(b.key) ?? Number.MAX_SAFE_INTEGER;
    return aIndex - bIndex || a.key.localeCompare(b.key);
  });
}

function scanPages(directory = DOCS_ROOT, segments: string[] = []): CanonicalPage[] {
  const entries = readdirSync(directory, { withFileTypes: true });
  const grouped = new Map<string, CanonicalPage>();

  for (const entry of entries) {
    if (entry.isDirectory()) {
      grouped.set(
        `__dir__${entry.name}`,
        {
          key: `__dir__${entry.name}`,
          slug: [],
          versions: {},
        },
      );
      continue;
    }

    const match = entry.name.match(FILE_PATTERN);
    if (!match) continue;
    const [, key, locale] = match;
    const page = grouped.get(key) ?? {
      key,
      slug: key === "index" ? segments : [...segments, key],
      versions: {},
    };
    page.versions[locale as Locale] = readVersion(
      path.join(directory, entry.name),
      humanize(key),
    );
    grouped.set(key, page);
  }

  const pages = [...grouped.values()].filter(
    (page) => !page.key.startsWith("__dir__") && page.versions.en,
  );
  const nested = entries
    .filter((entry) => entry.isDirectory())
    .flatMap((entry) => scanPages(path.join(directory, entry.name), [...segments, entry.name]));

  return [...pages, ...nested];
}

function pageHref(locale: Locale, slug: string[]) {
  const suffix = slug.length ? `/${slug.join("/")}` : "";
  return `${localePrefix(locale)}/docs${suffix}` || "/docs";
}

function buildTree(directory: string, segments: string[], locale: Locale): DocTreeNode[] {
  const entries = readdirSync(directory, { withFileTypes: true });
  const meta = readMeta(directory);
  const pages = scanPages();

  const directPages: DocTreePage[] = [];
  for (const entry of entries) {
    const match = entry.name.match(FILE_PATTERN);
    if (!match || match[2] !== defaultLocale) continue;
    const expectedSlug = match[1] === "index" ? segments : [...segments, match[1]];
    const page = pages.find((candidate) => candidate.slug.join("/") === expectedSlug.join("/"));
    if (!page) continue;
    const version = page.versions[locale] ?? page.versions.en!;
    directPages.push({
      type: "page",
      key: page.key,
      title: version.title,
      href: pageHref(locale, page.slug),
      missing: !page.versions[locale],
    });
  }

  const sections: DocTreeSection[] = entries
    .filter((entry) => entry.isDirectory())
    .map((entry): DocTreeSection => {
      const childPath = path.join(directory, entry.name);
      const childMeta = readMeta(childPath);
      return {
        type: "section",
        key: entry.name,
        title: childMeta.title?.[locale] ?? childMeta.title?.en ?? humanize(entry.name),
        defaultOpen: childMeta.defaultOpen ?? true,
        children: buildTree(childPath, [...segments, entry.name], locale),
      };
    })
    .filter(
      (section) =>
        section.children.length > 0 ||
        existsSync(path.join(directory, section.key, "_meta.json")),
    );

  return orderItems([...directPages, ...sections], meta.order);
}

function flattenTree(tree: DocTreeNode[]): DocTreePage[] {
  return tree.flatMap((node) => (node.type === "page" ? [node] : flattenTree(node.children)));
}

function nodeText(node: AstNode): string {
  if (node.value) return node.value;
  return node.children?.map(nodeText).join("") ?? "";
}

function slugify(text: string, seen: Map<string, number>) {
  const base =
    text
      .toLowerCase()
      .trim()
      .replace(/[^\p{Letter}\p{Number}\s-]/gu, "")
      .replace(/\s+/g, "-") || "section";
  const count = seen.get(base) ?? 0;
  seen.set(base, count + 1);
  return count ? `${base}-${count}` : base;
}

function splitUrl(url: string) {
  const hashIndex = url.indexOf("#");
  return hashIndex === -1
    ? { pathname: url, hash: "" }
    : { pathname: url.slice(0, hashIndex), hash: url.slice(hashIndex) };
}

export function rewriteDocUrl(url: string, currentDirectory: string[], locale: Locale) {
  if (
    !url ||
    url.startsWith("#") ||
    url.startsWith("/") ||
    /^[a-z][a-z\d+.-]*:/i.test(url)
  ) {
    return url;
  }

  const { pathname, hash } = splitUrl(url);
  const resolved = path.posix.normalize(path.posix.join("/", ...currentDirectory, pathname));

  if (pathname.endsWith(".md")) {
    const withoutExtension = resolved
      .replace(/\.md$/, "")
      .replace(/\.(en|zh-CN)$/, "")
      .replace(/\/index$/, "");
    return `${localePrefix(locale)}/docs${withoutExtension === "/" ? "" : withoutExtension}${hash}`;
  }

  return `/docs-assets${resolved}${hash}`;
}

function createMarkdownPlugin(currentDirectory: string[], locale: Locale, headings: DocHeading[]) {
  return () => (tree: AstNode) => {
    const seen = new Map<string, number>();
    visit(tree, (node: AstNode) => {
      if ((node.type === "link" || node.type === "image") && node.url) {
        node.url = rewriteDocUrl(node.url, currentDirectory, locale);
      }
      if (node.type === "heading" && (node.depth === 2 || node.depth === 3)) {
        const text = nodeText(node);
        headings.push({
          id: slugify(text, seen),
          text,
          level: node.depth,
        });
      }
    });
  };
}

async function renderMarkdown(page: CanonicalPage, locale: Locale) {
  const version = page.versions[locale];
  if (!version) return { html: null, headings: [] };
  const parsed = matter(readFileSync(version.filePath, "utf8"));
  const headings: DocHeading[] = [];
  const currentDirectory = page.key === "index" ? page.slug : page.slug.slice(0, -1);
  const result = await unified()
    .use(remarkParse)
    .use(remarkGfm)
    .use(createMarkdownPlugin(currentDirectory, locale, headings))
    .use(remarkRehype)
    .use(rehypeSlug)
    .use(rehypeAutolinkHeadings, { behavior: "wrap" })
    .use(rehypePrettyCode, { theme: "github-dark-dimmed" })
    .use(rehypeStringify)
    .process(parsed.content);
  return { html: String(result), headings };
}

export function getDocTree(locale: Locale) {
  return buildTree(DOCS_ROOT, [], locale);
}

export function getDocStaticSlugs() {
  return flattenTree(getDocTree("en"))
    .filter((page) => page.href !== "/docs")
    .map((page) => ({ slug: page.href.replace(/^\/docs\//, "").split("/") }));
}

export async function getDocPage(slug: string[], locale: Locale): Promise<DocPageData | null> {
  const pages = scanPages();
  const page = pages.find((candidate) => candidate.slug.join("/") === slug.join("/"));
  if (!page) return null;
  const english = page.versions.en!;
  const version = page.versions[locale] ?? english;
  const rendered = await renderMarkdown(page, locale);
  const navigation = flattenTree(getDocTree(locale));
  const currentIndex = navigation.findIndex((item) => item.key === page.key && item.href === pageHref(locale, slug));
  const previous = navigation[currentIndex - 1] ?? null;
  const next = navigation[currentIndex + 1] ?? null;

  return {
    slug,
    title: version.title,
    description: version.description,
    html: rendered.html,
    headings: rendered.headings,
    missing: !page.versions[locale],
    englishHref: pageHref("en", slug),
    previous: previous ? { title: previous.title, href: previous.href } : null,
    next: next ? { title: next.title, href: next.href } : null,
  };
}
