import type { ReactNode } from "react";
import type { RenderContext } from "./context";

/** The parts of the site layout a deployer can replace. Each one is an API to maintain, so the list stays short. */
export const SLOT_NAMES = ["Header", "Footer", "SidebarFooter", "PageFooter", "NotFound"] as const;

export type SlotName = (typeof SLOT_NAMES)[number];

export interface SlotProps {
  context: RenderContext;
  /** What the layout draws without the slot: return it to keep it, wrap it, or leave it out to replace it. */
  children?: ReactNode;
}

export interface SlotDefinition {
  kind: "slot";
  name: SlotName;
  /** Pure like components: only the context and children, no fetching and no `window`. It may use hooks. */
  render: (props: SlotProps) => ReactNode;
}

export function defineSlot(name: SlotName, render: (props: SlotProps) => ReactNode): SlotDefinition {
  return { kind: "slot", name, render };
}
