import { Children, type ReactNode, useId, useState } from "react";

/** `variant: "code"` is a code group: the tabs wrap code blocks and are drawn as one box with them. */
export type TabsProps = { labels: string[]; variant?: "code"; children: ReactNode };

export function Tabs({ labels, variant, children }: TabsProps) {
  const [active, setActive] = useState(0);
  const panels = Children.toArray(children);
  // Several tab groups can share a page: their ids must not point at each other's panels.
  const id = useId();

  return (
    <div className={variant === "code" ? "tabs tabs-code" : "tabs"}>
      <div role="tablist" className="tabs-list">
        {labels.map((label, i) => (
          <button
            // biome-ignore lint/suspicious/noArrayIndexKey: tabs are positional, and two can share a label (or have none yet)
            key={i}
            type="button"
            role="tab"
            id={`${id}-tab-${i}`}
            aria-selected={i === active}
            aria-controls={`${id}-panel-${i}`}
            className="tabs-tab"
            onClick={() => setActive(i)}
          >
            {label}
          </button>
        ))}
      </div>
      {panels.map((panel, i) => (
        <div
          // biome-ignore lint/suspicious/noArrayIndexKey: panels are positional and never reordered
          key={i}
          role="tabpanel"
          id={`${id}-panel-${i}`}
          aria-labelledby={`${id}-tab-${i}`}
          hidden={i !== active}
          className="tabs-panel"
        >
          {panel}
        </div>
      ))}
    </div>
  );
}
