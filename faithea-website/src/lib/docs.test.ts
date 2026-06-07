import { describe, expect, test } from "bun:test";
import { getDocPage, getDocStaticSlugs, getDocTree, rewriteDocUrl } from "./docs";

describe("documentation content pipeline", () => {
  test("generates canonical static slugs from English documents", () => {
    expect(getDocStaticSlugs()).toEqual([
      { slug: ["introduction", "welcome"] },
      { slug: ["getting-started", "installation"] },
      { slug: ["getting-started", "basic-usage"] },
    ]);
  });

  test("uses _meta.json ordering for the navigation tree", () => {
    const tree = getDocTree("en");
    expect(tree.map((node) => node.key)).toEqual([
      "index",
      "introduction",
      "getting-started",
      "advanced",
    ]);
    const introduction = tree[1];
    expect(introduction.type).toBe("section");
    if (introduction.type === "section") {
      expect(introduction.children.map((node) => node.key)).toEqual(["welcome"]);
    }
    const gettingStarted = tree[2];
    expect(gettingStarted.type).toBe("section");
    if (gettingStarted.type === "section") {
      expect(gettingStarted.children.map((node) => node.key)).toEqual([
        "installation",
        "basic-usage",
      ]);
    }
  });

  test("renders translated Markdown and extracts its outline", async () => {
    const page = await getDocPage(["introduction", "welcome"], "zh-CN");
    expect(page?.missing).toBe(false);
    expect(page?.html).toContain("Faithea");
    expect(page?.headings.map((heading) => heading.text)).toContain("为什么选择 Faithea");
  });

  test("renders the localized installation guide", async () => {
    const page = await getDocPage(
      ["getting-started", "installation"],
      "zh-CN",
    );
    expect(page?.missing).toBe(false);
    expect(page?.title).toBe("安装");
    expect(page?.html).toContain("cargo add faithea");
  });

  test("renders the localized basic usage guide", async () => {
    const page = await getDocPage(["getting-started", "basic-usage"], "zh-CN");
    expect(page?.missing).toBe(false);
    expect(page?.title).toBe("基本用法");
    expect(page?.html).toContain("handlers!");
    expect(page?.headings.map((heading) => heading.text)).toContain("发送请求");
  });

  test("rewrites document and asset links relative to the Markdown directory", () => {
    expect(rewriteDocUrl("../index.md#top", ["introduction"], "zh-CN")).toBe(
      "/zh-CN/docs#top",
    );
    expect(rewriteDocUrl("../assets/example.svg", ["introduction"], "en")).toBe(
      "/docs-assets/assets/example.svg",
    );
  });
});
