// V8 has no TextEncoder, which React's server build needs at load time.
import "./polyfills";
import { createElement } from "react";
import { renderToString } from "react-dom/server.browser";
import { Doc, type DocNode } from "./doc";

// ssr_rs hands the argument over as a raw string and concatenates the result of every export, so this is the only export.
export function render(json: string): string {
  const doc = JSON.parse(json) as DocNode;
  return renderToString(createElement(Doc, { doc }));
}
