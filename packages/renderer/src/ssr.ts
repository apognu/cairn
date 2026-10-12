// V8 has no TextEncoder, which React's server build needs at load time.
import "./polyfills";
import { createElement } from "react";
import { renderToString } from "react-dom/server.browser";
import { Page } from "./page";
import { validateDoc } from "./render";
import type { RenderInput } from "./types";

// ssr_rs hands the argument over as a raw string and concatenates the result of every export, so this is the only export.
export function render(json: string): string {
  const input = JSON.parse(json) as RenderInput & { validate?: boolean };

  // The pre-publish check asks only whether the document would render; the answer is a JSON list of problems.
  if (input.validate) return JSON.stringify(validateDoc(input.doc));

  return renderToString(createElement(Page, input));
}
