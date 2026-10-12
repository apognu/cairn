import type { ReactNode } from "react";
import { z } from "zod";

export { z };

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
  /** Pure: only props and children, no fetching and no `window`, so it renders the same in V8 and the browser. */
  render: (props: z.infer<z.ZodObject<Shape>>, children: ReactNode) => ReactNode;
}

export function defineComponent<Shape extends z.ZodRawShape>(
  definition: ComponentDefinition<Shape>,
): ComponentDefinition<Shape> {
  return definition;
}

/** A definition with its props type erased, for registries that hold components of different shapes. */
export type AnyComponent = ComponentDefinition<z.ZodRawShape>;

export type JsonSchema = {
  type?: string;
  title?: string;
  description?: string;
  default?: unknown;
  enum?: unknown[];
  properties?: Record<string, JsonSchema>;
  items?: JsonSchema;
};

/** The JSON Schema of a component's props, which the editor turns into a form. */
export function propsSchema(component: AnyComponent): JsonSchema {
  return z.toJSONSchema(component.props) as JsonSchema;
}
