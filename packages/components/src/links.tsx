import { defineComponent, link, operationLabel, pageTarget, z } from "@cairn/sdk";
import { pageHref, type RenderEnv, useRenderEnv } from "./env";

/** Fields named `page` and `pages` are how the server finds a component's links to other pages. */
const pageList = z.array(z.string().meta({ title: "Page", "x-field": "page" })).default([]);

function Arrow() {
  return (
    <svg
      width="15"
      height="15"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      aria-hidden="true"
    >
      <path d="M5 12h14M13 6l6 6-6 6" />
    </svg>
  );
}

/** What a link to a page or to an operation of an API page shows: the page's title and description, or the operation and the page's title. */
function target(value: string, pages: RenderEnv["pages"]): { title: string; description?: string | null } | undefined {
  const { id, anchor } = pageTarget(value);
  const page = pages[id];
  const operation = operationLabel(anchor);
  if (!page || !operation) return page;

  return { title: operation, description: page.title };
}

/** A card with the target page's title and description, read from the page itself so a rename reaches every card at the next publish. */
function PageCard({ id }: { id: string | undefined }) {
  const { urls, pages, editing } = useRenderEnv();
  const info = id ? target(id, pages) : undefined;
  const href = id ? pageHref(urls, id) : undefined;

  if (!info || !href) {
    return editing ? (
      <div className="link-card" data-empty>
        Choose a page in the settings panel
      </div>
    ) : null;
  }

  return (
    <a className="link-card" href={href}>
      <span className="link-card-title">
        {info.title}
        <Arrow />
      </span>{" "}
      {info.description && <span className="link-card-description">{info.description}</span>}
    </a>
  );
}

export const linkCard = defineComponent({
  name: "linkCard",
  version: 1,
  title: "Link card",
  description: "A card for one page, with its title and description.",
  props: z.object({ page: link() }),
  render: (props) => <PageCard id={props.page} />,
});

export const relatedPages = defineComponent({
  name: "relatedPages",
  version: 1,
  title: "Related pages",
  description: "Cards linking to other pages, at the end of this one.",
  props: z.object({ title: z.string().default("Related"), pages: pageList }),
  render: (props) => (
    <section className="related-pages">
      <h2 className="related-title">{props.title}</h2>
      <div className="card-grid">
        {props.pages.map((id, i) => (
          // biome-ignore lint/suspicious/noArrayIndexKey: the same page may be listed twice
          <PageCard key={`${id}-${i}`} id={id} />
        ))}
      </div>
    </section>
  ),
});

function SeeAlsoLinks({ ids }: { ids: string[] }) {
  const { urls, pages, editing } = useRenderEnv();
  const found = ids.flatMap((id) => {
    const href = pageHref(urls, id);
    const info = target(id, pages);
    return href && info ? [{ id, href, title: info.title }] : [];
  });

  if (found.length === 0)
    return editing ? (
      <aside className="see-also" data-empty>
        Choose pages in the settings panel
      </aside>
    ) : null;

  return (
    <aside className="see-also">
      <strong className="see-also-title">See also</strong>
      <ul className="see-also-links">
        {found.map(({ id, href, title }) => (
          <li key={id}>
            <a href={href}>{title}</a>
          </li>
        ))}
      </ul>
    </aside>
  );
}

export const seeAlso = defineComponent({
  name: "seeAlso",
  version: 1,
  title: "See also",
  description: "Links to related pages, set apart from the text.",
  props: z.object({ pages: pageList }),
  render: (props) => <SeeAlsoLinks ids={props.pages} />,
});
