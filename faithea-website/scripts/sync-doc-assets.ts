import { cpSync, existsSync, mkdirSync, readdirSync, rmSync } from "node:fs";
import path from "node:path";

const source = path.join(process.cwd(), "content", "docs");
const destination = path.join(process.cwd(), "public", "docs-assets");
const markdownPattern = /\.(en|zh-CN)\.md$/;

function copyDirectory(from: string, to: string) {
  mkdirSync(to, { recursive: true });
  for (const entry of readdirSync(from, { withFileTypes: true })) {
    const sourcePath = path.join(from, entry.name);
    const destinationPath = path.join(to, entry.name);
    if (entry.isDirectory()) {
      copyDirectory(sourcePath, destinationPath);
    } else if (entry.name !== "_meta.json" && !markdownPattern.test(entry.name)) {
      cpSync(sourcePath, destinationPath);
    }
  }
}

rmSync(destination, { recursive: true, force: true });
if (existsSync(source)) copyDirectory(source, destination);
