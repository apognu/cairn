import { customComponents } from "@yourdocs/components/custom";
import type { AnyComponent } from "@yourdocs/sdk";
import { cards, fileTree } from "./blocks";
import { callout } from "./Callout";
import { featureTabs, whatsNew } from "./features";
import { linkCard, relatedPages, seeAlso } from "./links";
import { snippet, variable } from "./reusable";

export { customSlots } from "@yourdocs/components/custom";
export { cards, fileTree, parseFileTree } from "./blocks";
export { Callout, type CalloutProps, callout } from "./Callout";
export { type RenderEnv, RenderEnvContext, useRenderEnv } from "./env";
export { FeatureCard, featureTabs, whatsNew } from "./features";
export { Icon, iconNames } from "./icons";
export { linkCard, relatedPages, seeAlso } from "./links";
export { snippet, variable } from "./reusable";
export { Tabs, type TabsProps } from "./Tabs";

/** Components whose nodes carry their props in attrs and get a generated settings panel. */
export const builtinComponents = [
  callout,
  featureTabs,
  whatsNew,
  linkCard,
  relatedPages,
  seeAlso,
  cards,
  fileTree,
  variable,
  snippet,
  ...customComponents,
] as unknown as AnyComponent[];

export const componentsByName = new Map(builtinComponents.map((c) => [c.name, c]));
