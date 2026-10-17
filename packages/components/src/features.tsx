import { date, defineComponent, Editable, icon, link, z } from "@cairn/sdk";
import { useEffect, useId, useState } from "react";
import { pageHref, useRenderEnv } from "./env";
import { Icon, upgradeIconNames } from "./icons";

const feature = z
  .object({
    icon: icon().default("star"),
    title: z.string().default("New feature"),
    description: z.string().default(""),
    page: link(),
    /** A web address, for a card that leaves the site; a page link wins when both are set. */
    href: z.string().optional().meta({ title: "Web address", description: "Where it links when no page is chosen" }),
    newUntil: date().meta({ title: "New until", description: "A New tag shows on it until this day" }),
  })
  .meta({ title: "Feature" });

type Feature = z.infer<typeof feature>;

/** The card both feature components use: icon, title, description, and a link to a page when one is set. With `path` (where the card sits in the props) its title is written on the canvas. */
export function FeatureCard({ feature: f, path }: { feature: Feature; path?: string }) {
  const { urls, today } = useRenderEnv();
  const href = f.page ? pageHref(urls, f.page) : f.href;
  // A date in the past hides the tag at the next render; without a date the tag never shows.
  const isNew = Boolean(f.newUntil) && today !== "" && (f.newUntil as string) >= today;

  const body = (
    <>
      <span className="feature-icon">
        <Icon name={f.icon} />
      </span>
      <span className="feature-text">
        <span className="feature-title">
          {path ? <Editable path={`${path}.title`} /> : f.title}
          {isNew && (
            <>
              {"\u00a0"}
              <span className="feature-new">New</span>
            </>
          )}
        </span>
        {/* Not drawn between flex items, but keeps the title and description apart in search excerpts. */}{" "}
        {f.description && <span className="feature-description">{f.description}</span>}
      </span>
    </>
  );

  return href ? (
    <a className="feature-card" href={href}>
      {body}
    </a>
  ) : (
    <div className="feature-card">{body}</div>
  );
}

function FeatureGrid({ features }: { features: Feature[] }) {
  return (
    <div className="feature-grid">
      {features.map((f, i) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: cards are positional
        <FeatureCard key={i} feature={f} />
      ))}
    </div>
  );
}

const families = z
  .array(
    z.object({ label: z.string().default("Family"), features: z.array(feature).default([]) }).meta({ title: "Family" }),
  )
  .min(1)
  .default([{ label: "Family", features: [] }]);

export const featureTabs = defineComponent({
  name: "featureTabs",
  version: 2,
  // Version 1 had a dozen icons of its own; 2 uses Lucide, where three of them have other names.
  migrate: { 1: upgradeIconNames },
  title: "Feature tabs",
  description: "Feature cards grouped into tabs, one tab per family.",
  props: z.object({ families }),
  render: (props) => <FeatureTabs families={props.families} />,
});

/** Without JavaScript every family shows as a section under its heading; once running, the families become tabs. */
export function FeatureTabs({ families }: { families: { label: string; features: Feature[] }[] }) {
  const id = useId();
  const [active, setActive] = useState(0);
  const [ready, setReady] = useState(false);
  useEffect(() => setReady(true), []);

  return (
    <div className="feature-tabs" data-ready={ready || undefined}>
      <div role="tablist" className="feature-tablist">
        {families.map((family, i) => (
          <button
            // biome-ignore lint/suspicious/noArrayIndexKey: families are positional
            key={i}
            type="button"
            role="tab"
            id={`${id}-tab-${i}`}
            aria-selected={i === active}
            aria-controls={`${id}-panel-${i}`}
            className="feature-tab"
            onClick={() => setActive(i)}
          >
            {family.label}
          </button>
        ))}
      </div>
      {families.map((family, i) => (
        <section
          // biome-ignore lint/suspicious/noArrayIndexKey: families are positional
          key={i}
          role="tabpanel"
          id={`${id}-panel-${i}`}
          aria-labelledby={`${id}-tab-${i}`}
          hidden={ready && i !== active}
          className="feature-panel"
        >
          <h3 className="feature-family">{family.label}</h3>
          <FeatureGrid features={family.features} />
        </section>
      ))}
    </div>
  );
}

export const whatsNew = defineComponent({
  name: "whatsNew",
  version: 2,
  // Version 1 had a dozen icons of its own; 2 uses Lucide, where three of them have other names.
  migrate: { 1: upgradeIconNames },
  title: "What's new",
  description: "Recently added features, each with a New tag for a while.",
  props: z.object({
    title: z.string().default("What's new").meta({ title: "Heading" }),
    items: z.array(feature).default([]),
  }),
  render: (props) => (
    <section className="whats-new">
      <h3 className="feature-family">{props.title}</h3>
      <FeatureGrid features={props.items} />
    </section>
  ),
});
