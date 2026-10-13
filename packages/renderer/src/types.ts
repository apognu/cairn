export type DocMark = { type: string; attrs?: Record<string, unknown> };

export type DocNode = {
  type: string;
  attrs?: Record<string, unknown>;
  content?: DocNode[];
  marks?: DocMark[];
  text?: string;
};

export type { NavNode, RenderContext, SiteImage, SiteLink } from "@yourdocs/sdk";

import type { RenderContext } from "@yourdocs/sdk";

export type RenderInput = { doc: DocNode; context?: Partial<RenderContext> };

export const defaultContext: RenderContext = {
  site: { name: "yourdocs" },
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
