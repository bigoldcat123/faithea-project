import type { Metadata } from "next";
import { SiteFooter, SiteHeader } from "@/components/site-chrome";
import "./globals.css";

export const metadata: Metadata = {
  title: "Faithea | Lightweight async HTTP for Rust",
  description:
    "Faithea is a lightweight, asynchronous HTTP framework built with Tokio.",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className="overflow-x-hidden bg-paper font-body text-ink">
        <SiteHeader />
        {children}
        <SiteFooter />
      </body>
    </html>
  );
}
