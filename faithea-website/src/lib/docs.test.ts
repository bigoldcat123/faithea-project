import { describe, expect, test } from "bun:test";
import { getDocPage, getDocStaticSlugs, getDocTree, rewriteDocUrl } from "./docs";

describe("documentation content pipeline", () => {
  test("generates canonical static slugs from English documents", () => {
    expect(getDocStaticSlugs()).toEqual([
      { slug: ["introduction", "welcome"] },
      { slug: ["getting-started", "installation"] },
      { slug: ["getting-started", "basic-usage"] },
      { slug: ["getting-started", "routing"] },
      { slug: ["getting-started", "request-data"] },
      { slug: ["getting-started", "responses"] },
      { slug: ["getting-started", "error-handling"] },
      { slug: ["advanced", "guards"] },
      { slug: ["advanced", "custom-extractors"] },
      { slug: ["advanced", "advanced-responses"] },
      { slug: ["advanced", "static-files"] },
      { slug: ["advanced", "streaming-sse"] },
      { slug: ["advanced", "websockets"] },
      { slug: ["advanced", "tls-http2"] },
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
        "routing",
        "request-data",
        "responses",
        "error-handling",
      ]);
    }
    const advanced = tree[3];
    expect(advanced.type).toBe("section");
    if (advanced.type === "section") {
      expect(advanced.children.map((node) => node.key)).toEqual([
        "guards",
        "custom-extractors",
        "advanced-responses",
        "static-files",
        "streaming-sse",
        "websockets",
        "tls-http2",
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

  test("renders all localized core getting started guides", async () => {
    const guides = await Promise.all(
      ["routing", "request-data", "responses", "error-handling"].map((slug) =>
        getDocPage(["getting-started", slug], "zh-CN"),
      ),
    );

    expect(guides.every((page) => page && !page.missing)).toBe(true);
    expect(guides.map((page) => page?.title)).toEqual([
      "路由",
      "请求数据",
      "响应",
      "错误处理",
    ]);
    expect(guides[0]?.headings.map((heading) => heading.text)).toContain(
      "通配符模式",
    );
    expect(guides[0]?.headings.map((heading) => heading.text)).toContain(
      "路由优先级",
    );
    expect(guides[1]?.headings.map((heading) => heading.text)).toContain(
      "Multipart 表单与文件",
    );
    expect(guides[1]?.headings.map((heading) => heading.text)).toContain(
      "自定义 Multipart 字段",
    );
    expect(guides[1]?.html).toContain("MultiPartFile");
  });

  test("renders all localized advanced guides", async () => {
    const slugs = [
      "guards",
      "custom-extractors",
      "advanced-responses",
      "static-files",
      "streaming-sse",
      "websockets",
      "tls-http2",
    ];
    const guides = await Promise.all(
      slugs.map((slug) => getDocPage(["advanced", slug], "zh-CN")),
    );

    expect(guides.every((page) => page && !page.missing)).toBe(true);
    expect(guides.map((page) => page?.title)).toEqual([
      "请求守卫",
      "自定义请求提取器",
      "高级响应",
      "静态文件",
      "流式响应与 SSE",
      "WebSockets",
      "TLS 与 HTTP/2",
    ]);
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
