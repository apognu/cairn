import { type NodeViewProps, NodeViewWrapper } from "@tiptap/react";
import { componentsByName } from "@yourdocs/components";

/** Draws an atom component with the very component the published page uses. */
export function ComponentView({ node }: NodeViewProps) {
  const definition = componentsByName.get(node.type.name);
  const parsed = definition?.props.safeParse(node.attrs);

  return (
    <NodeViewWrapper data-label={definition?.title}>
      {definition && parsed?.success ? definition.render(parsed.data, null) : null}
    </NodeViewWrapper>
  );
}
