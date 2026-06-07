import { describe, expect, test } from "bun:test";
import { getDocPage, getDocStaticSlugs, getDocTree, rewriteDocUrl } from "./docs";

describe("documentation content pipeline", () => {
  test("generates canonical static slugs from English documents", () => {
    expect(getDocStaticSlugs()).toEqual([
      { slug: ["examples", "markdown"] },
      { slug: ["examples", "english-only"] },
    ]);
  });

  test("uses _meta.json ordering for the navigation tree", () => {
    const tree = getDocTree("en");
    expect(tree.map((node) => node.key)).toEqual(["index", "examples"]);
    const examples = tree[1];
    expect(examples.type).toBe("section");
    if (examples.type === "section") {
      expect(examples.children.map((node) => node.key)).toEqual(["markdown", "english-only"]);
    }
  });

  test("creates a localized fallback for missing translations", async () => {
    const page = await getDocPage(["examples", "english-only"], "zh-CN");
    expect(page?.missing).toBe(true);
    expect(page?.html).toBeNull();
    expect(page?.englishHref).toBe("/docs/examples/english-only");
  });

  test("renders translated Markdown and extracts its outline", async () => {
    const page = await getDocPage(["examples", "markdown"], "zh-CN");
    expect(page?.missing).toBe(false);
    expect(page?.html).toContain("/docs-assets/assets/example.svg");
    expect(page?.headings.map((heading) => heading.text)).toContain("代码与命令");
  });

  test("rewrites document and asset links relative to the Markdown directory", () => {
    expect(rewriteDocUrl("../index.md#top", ["examples"], "zh-CN")).toBe(
      "/zh-CN/docs#top",
    );
    expect(rewriteDocUrl("../assets/example.svg", ["examples"], "en")).toBe(
      "/docs-assets/assets/example.svg",
    );
  });
});
