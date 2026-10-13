import { type AnyComponent, type JsonSchema, propsSchema } from "./index";

function sample(schema: JsonSchema): unknown {
  if (schema.default !== undefined) return schema.default;
  if (schema.enum) return schema.enum[0];

  switch (schema.type) {
    case "object":
      return Object.fromEntries(Object.entries(schema.properties ?? {}).map(([key, value]) => [key, sample(value)]));
    case "array":
      return Array.from({ length: Math.max(schema.minItems ?? 1, 1) }, () => sample(schema.items ?? {}));
    case "number":
    case "integer":
      return 1;
    case "boolean":
      return false;
    default:
      return schema["x-field"] === "date" ? "2026-01-01" : "Sample";
  }
}

/** Props that satisfy the component's schema, built from it: defaults where there are some, otherwise a plausible value per type. Used to test-render components. */
export function sampleProps(component: AnyComponent): Record<string, unknown> {
  const props = sample(propsSchema(component)) as Record<string, unknown>;

  // Page links are IDs of pages the sample site does not have; they are optional, so the sample leaves them out.
  const schema = propsSchema(component);
  const withoutPages = (value: unknown, node: JsonSchema): unknown => {
    if (node["x-field"] === "page") return undefined;
    if (node.type === "object" && value && typeof value === "object")
      return Object.fromEntries(
        Object.entries(value).map(([key, v]) => [key, withoutPages(v, node.properties?.[key] ?? {})]),
      );
    if (node.type === "array" && Array.isArray(value)) return value.map((v) => withoutPages(v, node.items ?? {}));

    return value;
  };

  return component.props.parse(withoutPages(props, schema)) as Record<string, unknown>;
}
