import type { AnyComponent } from "@yourdocs/sdk";
import { callout } from "./Callout";

export { Callout, type CalloutProps, callout } from "./Callout";
export { Tabs, type TabsProps } from "./Tabs";

/** Components whose nodes carry their props in attrs and get a generated settings panel. */
export const builtinComponents: AnyComponent[] = [callout as unknown as AnyComponent];

export const componentsByName = new Map(builtinComponents.map((c) => [c.name, c]));
