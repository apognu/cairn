import { defineComponent, link, z } from "@yourdocs/sdk";
import { useRenderEnv } from "./env";

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

/** A card with the target page's title and description, read from the page itself so a rename reaches every card at the next publish. */
function PageCard({ id }: { id: string | undefined }) {
  const { urls, pages, editing } = useRenderEnv();
  const info = id ? pages[id] : undefined;
  const href = id ? urls[id] : undefined;

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
      </span>
      {info.description && <span className="link-card-description">{info.description}</span>}
    </a>
  );
}

export const linkCard = defineComponent({
  name: "linkCard",
  version: 1,
  title: "Link card",
  props: z.object({ page: link() }),
  render: (props) => <PageCard id={props.page} />,
});

export const relatedPages = defineComponent({
  name: "relatedPages",
  version: 1,
  title: "Related pages",
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
  const { urls, pages } = useRenderEnv();
  const found = ids.filter((id) => urls[id] && pages[id]);

  return (
    <aside className="see-also">
      <strong>See also</strong>
      <ul>
        {found.map((id) => (
          <li key={id}>
            <a href={urls[id]}>{pages[id].title}</a>
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
  props: z.object({ pages: pageList }),
  render: (props) => <SeeAlsoLinks ids={props.pages} />,
});
