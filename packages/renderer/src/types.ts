export type DocMark = { type: string; attrs?: Record<string, unknown> };

export type DocNode = {
  type: string;
  attrs?: Record<string, unknown>;
  content?: DocNode[];
  marks?: DocMark[];
  text?: string;
};

export type SiteImage = { src: string; width?: number; height?: number };
export type SiteLink = { label: string; url: string };

/** `external` marks a link that leaves the site. */
export type NavNode = { id: string; title: string; href: string; external?: boolean; children: NavNode[] };

/** Everything outside the document that a page needs to render. Built by the server, never fetched during render. */
export type RenderContext = {
  site: {
    name: string;
    logo?: SiteImage | null;
    /** Shown instead of `logo` in dark mode. */
    logoDark?: SiteImage | null;
    /** External links after the sections in the header. */
    headerLinks?: SiteLink[];
    footer?: { text: string; columns: { title: string; links: SiteLink[] }[] };
  };
  /** The description is shown as the page's lead, under its title. */
  page: { id?: string; title: string; description?: string | null };
  nav: NavNode[];
  /** The pages above this one, outermost first. */
  breadcrumbs: { title: string; href: string }[];
  /** The neighbours in the sidebar order, for the links at the end of the page. */
  previous?: { title: string; href: string } | null;
  next?: { title: string; href: string } | null;
  /** `YYYY-MM-DD` of the revision being published. */
  updated?: string | null;
  /** Where this page's Markdown version lives. */
  markdown?: string | null;
  /** The pages promoted to the site header, in order. */
  header: { id: string; title: string; href: string }[];
  /** The ID of the header entry the page belongs to, if any. */
  section?: string | null;
  /** Page ID to URL, for links stored by ID. */
  urls: Record<string, string>;
  /** Page ID to the title and description other pages show about it, for link cards. */
  pages: Record<string, { title: string; description?: string | null }>;
  /** Glossary term to its definition and the address of its entry, for the terms this page marks. */
  terms: Record<string, { definition: string; href: string }>;
  /** Asset ID to what the published page needs to show it: its URL and, when known, its size. */
  assets: Record<
    string,
    { src: string; width?: number; height?: number; srcset?: string; placeholder?: string | null }
  >;
  /** `YYYY-MM-DD` of the render, for components that depend on the date. */
  today: string;
};

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
