export type DocMark = { type: string; attrs?: Record<string, unknown> };

export type DocNode = {
  type: string;
  attrs?: Record<string, unknown>;
  content?: DocNode[];
  marks?: DocMark[];
  text?: string;
};

export type NavNode = { id: string; title: string; href: string; children: NavNode[] };

/** Everything outside the document that a page needs to render. Built by the server, never fetched during render. */
export type RenderContext = {
  site: { name: string };
  page: { id?: string; title: string };
  nav: NavNode[];
  /** Page ID to URL, for links stored by ID. */
  urls: Record<string, string>;
  /** `YYYY-MM-DD` of the render, for components that depend on the date. */
  today: string;
};

export type RenderInput = { doc: DocNode; context?: Partial<RenderContext> };

export const defaultContext: RenderContext = {
  site: { name: "yourdocs" },
  page: { title: "" },
  nav: [],
  urls: {},
  today: "",
};
