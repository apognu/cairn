export type DocMark = { type: string; attrs?: Record<string, unknown> };

export type DocNode = {
  type: string;
  attrs?: Record<string, unknown>;
  content?: DocNode[];
  marks?: DocMark[];
  text?: string;
};

export type { NavNode, RenderContext, SiteImage, SiteLink } from "@cairn/sdk";

import type { RenderContext } from "@cairn/sdk";

export type RenderInput = { doc: DocNode; context?: Partial<RenderContext> };

export const defaultContext: RenderContext = {
  site: { name: "cairn" },
  page: { title: "" },
  nav: [],
  breadcrumbs: [],
  header: [],
  urls: {},
  pages: {},
  terms: {},
  assets: {},
  today: "",
};
