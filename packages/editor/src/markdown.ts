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

// What code editors and terminals wrap copied text in to keep its colours and font: no structure of its own.
const styling = new Set(["html", "head", "body", "meta", "style", "div", "span", "font", "br", "p", "pre", "code"]);

/** Whether clipboard HTML only dresses up plain text (VS Code and terminals copy coloured source this way) rather than holding a document. */
export function isStyledText(html: string): boolean {
  const body = html.replace(/<!--[\s\S]*?-->|<style[\s\S]*?<\/style>/gi, "");
  return [...body.matchAll(/<\/?([a-zA-Z][\w-]*)/g)].every(([, tag]) => styling.has(tag.toLowerCase()));
}

/**
 * Whether a paste from VS Code or an editor built on Monaco holds prose rather than code: its `vscode-editor-data` names the language.
 * Without that entry, the clipboard did not come from such an editor.
 */
export function isProseFromCodeEditor(editorData: string): boolean {
  if (!editorData) return true;
  try {
    const { mode } = JSON.parse(editorData) as { mode?: unknown };
    return typeof mode !== "string" || ["markdown", "mdx", "plaintext"].includes(mode.toLowerCase());
  } catch {
    return true;
  }
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
