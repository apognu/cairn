import { Children, type ReactNode, useState } from "react";

/** `variant: "code"` is a code group: the tabs wrap code blocks and are drawn as one box with them. */
export type TabsProps = { labels: string[]; variant?: "code"; children: ReactNode };

export function Tabs({ labels, variant, children }: TabsProps) {
  const [active, setActive] = useState(0);
  const panels = Children.toArray(children);

  return (
    <div className={variant === "code" ? "tabs tabs-code" : "tabs"}>
      <div role="tablist" className="tabs-list">
        {labels.map((label, i) => (
          <button
            key={label}
            type="button"
            role="tab"
            id={`tab-${i}`}
            aria-selected={i === active}
            aria-controls={`panel-${i}`}
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
          id={`panel-${i}`}
          aria-labelledby={`tab-${i}`}
          hidden={i !== active}
          className="tabs-panel"
        >
          {panel}
        </div>
      ))}
    </div>
  );
}
