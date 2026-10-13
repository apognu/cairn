import { type NodeViewProps, NodeViewWrapper } from "@tiptap/react";
import { componentsByName } from "@yourdocs/components";

/** Draws an atom component with the very component the published page uses. */
export function ComponentView({ node }: NodeViewProps) {
  const definition = componentsByName.get(node.type.name);
  const parsed = definition?.props.safeParse(node.attrs);

  // Settings that do not fit the schema cannot render: an empty atom would be invisible and impossible to find, so the problem shows instead.
  const invalid = (problem: string) => (
    <span className="component-invalid" role="alert">
      {problem}
    </span>
  );
  const body = !definition
    ? invalid(`Unknown block “${node.type.name}”`)
    : parsed?.success
      ? definition.render(parsed.data, null)
      : invalid(`${definition.title}: ${parsed?.error.issues[0]?.message ?? "invalid settings"}`);

  return (
    <NodeViewWrapper as={node.isInline ? "span" : "div"} data-label={definition?.title}>
      {body}
    </NodeViewWrapper>
  );
}
