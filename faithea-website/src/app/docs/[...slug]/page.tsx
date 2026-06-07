import { notFound } from "next/navigation";
import { DocsPageView } from "@/components/docs-page";
import { getDocPage, getDocStaticSlugs, getDocTree } from "@/lib/docs";

export const dynamicParams = false;

export function generateStaticParams() {
  return getDocStaticSlugs();
}

export default async function DocsArticlePage({
  params,
}: {
  params: Promise<{ slug: string[] }>;
}) {
  const { slug } = await params;
  const page = await getDocPage(slug, "en");
  if (!page) notFound();
  return <DocsPageView locale="en" page={page} tree={getDocTree("en")} />;
}
