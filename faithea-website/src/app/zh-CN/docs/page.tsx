import { notFound } from "next/navigation";
import { DocsPageView } from "@/components/docs-page";
import { getDocPage, getDocTree } from "@/lib/docs";

export default async function DocsPage() {
  const page = await getDocPage([], "zh-CN");
  if (!page) notFound();
  return <DocsPageView locale="zh-CN" page={page} tree={getDocTree("zh-CN")} />;
}
