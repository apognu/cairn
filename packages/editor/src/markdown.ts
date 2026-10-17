import { micromark } from "micromark";
import { gfm, gfmHtml } from "micromark-extension-gfm";

// Things only Markdown writes: a heading, a list or quote marker, a fence or a table at the start of a line; a link, bold or code span.
const markers = [
  /^#{1,6}\s+\S/m,
  /^\s*[-*+]\s+\S/m,
  /^\s*\d+[.)]\s+\S/m,
  /^\s*>\s/m,
  /^\s*```/m,
  /^\s*\|.*\|\s*$\n^\s*\|?\s*:?-{3,}/m,
  /\[[^\]\n]+\]\([^)\s]+\)/,
  /\*\*[^*\n]+\*\*/,
  /`[^`\n]+`/,
];

/** Whether pasted plain text reads as Markdown rather than as words. */
export function looksLikeMarkdown(text: string): boolean {
  return markers.some((marker) => marker.test(text));
}

/**
 * Markdown (with GitHub's tables and strikethrough) as HTML the editor parses into its own blocks. Raw HTML in it stays text. The page
 * title is the page's only first-level heading, so when the text has `#` headings every heading moves down a level; none goes below 4.
 */
export function markdownToHtml(text: string): string {
  const html = micromark(text, { extensions: [gfm()], htmlExtensions: [gfmHtml()] });
  const shift = /<h1>/.test(html) ? 1 : 0;

  return (
    html
      .replace(/<(\/?)h([1-6])>/g, (_, close: string, level: string) => {
        const next = Math.min(Math.max(Number(level) + shift, 2), 4);
        return `<${close}h${next}>`;
      })
      // A fence keeps the line break before its closing backticks, which would be an empty last line in the editor.
      .replace(/\n<\/code><\/pre>/g, "</code></pre>")
  );
}
