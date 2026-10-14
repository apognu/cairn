import type { ReactNode } from "react";
import { z } from "zod";

export { z };

export * from "./context";
export * from "./editable";
export { sampleProps } from "./sample";
export * from "./slots";

export const SDK_VERSION = 1;

export interface ComponentDefinition<Shape extends z.ZodRawShape = z.ZodRawShape> {
  /** The node type in documents. Namespaced for custom components (`acme/link-tabs`); built-ins have no prefix. */
  name: string;
  /** Stored with nodes so `migrate` can upgrade old props. */
  version: number;
  /** Shown in the editor: the panel title and the slash menu entry. */
  title: string;
  /** Props live in the node's attrs. The editor generates the settings form from this schema. */
  props: z.ZodObject<Shape>;
  /** Shown under the title in the slash menu. */
  description?: string;
  /** Extra words the slash menu matches, separated by spaces. */
  keywords?: string;
  /** Hydrates as an island on the published site; static components ship no client script of their own. */
  interactive?: boolean;
  /**
   * Upgrades the props of nodes saved with an older `version`: the function under key `n` turns version `n` props into version `n + 1`.
   * A version without a function keeps its props, which the schema then completes with defaults.
   */
  migrate?: Record<number, (props: Record<string, unknown>) => Record<string, unknown>>;
  /** Pure: only props and children, no fetching and no `window`, so it renders the same in V8 and the browser. */
  render: (props: z.infer<z.ZodObject<Shape>>, children: ReactNode) => ReactNode;
}

export function defineComponent<Shape extends z.ZodRawShape>(
  definition: ComponentDefinition<Shape>,
): ComponentDefinition<Shape> {
  return definition;
}

/** The node attribute that records which version of the component wrote the props. Nodes without it were written by version 1. */
export const VERSION_ATTR = "__v";

/** The node's attributes brought up to the component's current version, with the version recorded. */
export function upgradeProps(component: AnyComponent, attrs: Record<string, unknown>): Record<string, unknown> {
  const stored = attrs[VERSION_ATTR];
  const from = typeof stored === "number" && stored >= 1 ? stored : 1;
  // Nothing to upgrade: the attributes stay as saved, so opening a page never rewrites its untouched components.
  if (from >= component.version) return attrs;

  let props = attrs;
  for (let version = from; version < component.version; version++) {
    const step = component.migrate?.[version];
    if (step) props = step(props);
  }

  return { ...props, [VERSION_ATTR]: component.version };
}

/** A definition with its props type erased, for registries that hold components of different shapes. */
export type AnyComponent = ComponentDefinition<z.ZodRawShape>;

export type JsonSchema = {
  type?: string;
  title?: string;
  description?: string;
  default?: unknown;
  enum?: unknown[];
  /** How the editor draws the field: a page picker, a date, an icon… (see `link()`, `icon()`). */
  "x-field"?: string;
  properties?: Record<string, JsonSchema>;
  items?: JsonSchema;
  minItems?: number;
};

/** The JSON Schema of a component's props, which the editor turns into a form. */
export function propsSchema(component: AnyComponent): JsonSchema {
  return z.toJSONSchema(component.props) as JsonSchema;
}

/** An optional link to another page, stored as the page's ID so moves and renames never break it. */
export function link() {
  return z.string().optional().meta({ title: "Page", "x-field": "page" });
}

/** The name of an icon of the Lucide collection (lucide.dev), chosen in the editor from a searchable grid. */
export function icon() {
  return z.string().meta({ title: "Icon", "x-field": "icon" });
}

/** An optional calendar date, `YYYY-MM-DD`. */
export function date() {
  return z.string().optional().meta({ "x-field": "date" });
}
