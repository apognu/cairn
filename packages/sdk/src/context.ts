export type SiteImage = { src: string; width?: number; height?: number };
/** A header or footer link. `icon` (a Lucide name) is shown by header links only, alone on wide screens. */
export type SiteLink = { label: string; url: string; icon?: string };

/** `external` marks a link that leaves the site. */
export type NavNode = {
  id: string;
  title: string;
  href: string;
  external?: boolean;
  /** A short pill after the title, such as "Beta". */
  badge?: string;
  children: NavNode[];
};

/** An operation of an API reference page, as listed in the merged spec. Only the search index reads them. */
export type ApiOperation = {
  /** Upper case: `GET`. */
  method: string;
  path: string;
  summary?: string | null;
  description?: string | null;
  tags: string[];
  operation_id?: string | null;
};

/** What an API reference page shows instead of a document. */
export type ApiContext = {
  /** Where the merged OpenAPI document is served. */
  specUrl: string;
  /** The self-hosted Scalar script; absent where the page is only rendered on the server. */
  scalarUrl?: string;
  /** Written into the page, hidden, so the site search finds operations; readers see Scalar only. */
  operations: ApiOperation[];
  display: { servers: string[]; hideModels: boolean; requestClient: boolean };
};

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
    /** A message at the top of every page; `url` is empty when it leads nowhere. */
    banner?: { text: string; url: string };
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
  /** In a section's own sidebar: where the link at its top goes back to (the page above the section, or the main documentation). */
  sectionBack?: { title: string; href: string } | null;
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
  /** True for the page shown when no page matches the address. */
  notFound?: boolean;
  /** Present on an API reference page, which shows this instead of its (empty) document. */
  api?: ApiContext;
};
