import { notFound } from "next/navigation";
import { DocsPageView } from "@/components/docs-page";
import { getDocPage, getDocTree } from "@/lib/docs";

export default async function DocsPage() {
  const page = await getDocPage([], "en");
  if (!page) notFound();
  return <DocsPageView locale="en" page={page} tree={getDocTree("en")} />;
}
