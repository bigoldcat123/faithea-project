"use client";

import { useEffect, useRef } from "react";

const copiedClassName = "is-copied";

async function copyText(value: string) {
  if (navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(value);
      return;
    } catch {
      // Fall back for browsers or permissions that reject Clipboard API writes.
    }
  }

  const textarea = document.createElement("textarea");
  textarea.value = value;
  textarea.setAttribute("readonly", "");
  textarea.style.position = "fixed";
  textarea.style.top = "-9999px";
  document.body.append(textarea);
  textarea.select();
  const copied = document.execCommand("copy");
  textarea.remove();

  if (!copied) {
    throw new Error("Unable to copy code block");
  }
}

export function DocsProse({ html }: { html: string }) {
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const root = rootRef.current;
    if (!root) return;

    const cleanups: Array<() => void> = [];
    const figures = root.querySelectorAll<HTMLElement>(
      "figure[data-rehype-pretty-code-figure]",
    );

    figures.forEach((figure) => {
      if (figure.querySelector(":scope > .code-copy-button")) return;

      const pre = figure.querySelector("pre");
      if (!pre) return;

      const button = document.createElement("button");
      button.type = "button";
      button.className = "code-copy-button";
      button.textContent = "COPY";
      button.setAttribute("aria-label", "Copy code block");

      let resetTimer: number | undefined;
      const reset = () => {
        button.classList.remove(copiedClassName);
        button.textContent = "COPY";
      };

      const handleClick = async () => {
        try {
          await copyText(pre.textContent ?? "");
          button.classList.add(copiedClassName);
          button.textContent = "COPIED";
          if (resetTimer) window.clearTimeout(resetTimer);
          resetTimer = window.setTimeout(reset, 1400);
        } catch {
          button.textContent = "FAILED";
          if (resetTimer) window.clearTimeout(resetTimer);
          resetTimer = window.setTimeout(reset, 1400);
        }
      };

      button.addEventListener("click", handleClick);
      figure.append(button);
      cleanups.push(() => {
        if (resetTimer) window.clearTimeout(resetTimer);
        button.removeEventListener("click", handleClick);
        button.remove();
      });
    });

    return () => {
      cleanups.forEach((cleanup) => cleanup());
    };
  }, [html]);

  return (
    <div
      ref={rootRef}
      className="docs-prose prose max-w-none prose-headings:font-display prose-headings:font-black prose-headings:tracking-[-0.035em] prose-headings:text-ink prose-headings:uppercase prose-p:leading-7 prose-p:text-ink-soft prose-a:font-bold prose-a:text-ink prose-strong:text-ink prose-code:font-mono prose-code:text-ink prose-img:border prose-img:border-ink prose-img:shadow-[8px_8px_0_var(--color-amber)]"
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}
