import { componentsByName } from "@cairn/components";
import { EditableScopeContext, setPath } from "@cairn/sdk";
import { type NodeViewProps, NodeViewWrapper } from "@tiptap/react";

/** Draws an atom component with the very component the published page uses. */
export function ComponentView({ node, updateAttributes }: NodeViewProps) {
  const definition = componentsByName.get(node.type.name);
  const parsed = definition?.props.safeParse(node.attrs);

  // Settings that do not fit the schema cannot render: an empty atom would be invisible and impossible to find, so the problem shows instead.
  const invalid = (problem: string) => (
    <span
      className={`border border-dashed border-danger rounded-lg text-danger text-ui ${node.isInline ? "inline py-0 px-1" : "block py-2.5 px-3.5"}`}
      role="alert"
    >
      {problem}
    </span>
  );
  const body = !definition ? (
    invalid(`Unknown block “${node.type.name}”`)
  ) : parsed?.success ? (
    <EditableScopeContext.Provider
      value={{
        props: parsed.data,
        // Text written on the canvas goes back into the props, which the settings form shows as well.
        onChange: (path, value) => {
          const [name] = path.split(".");
          updateAttributes({ [name]: (setPath(parsed.data, path, value) as Record<string, unknown>)[name] });
        },
      }}
    >
      {definition.render(parsed.data, null)}
    </EditableScopeContext.Provider>
  ) : (
    invalid(`${definition.title}: ${parsed?.error.issues[0]?.message ?? "invalid settings"}`)
  );

  return (
    <NodeViewWrapper as={node.isInline ? "span" : "div"} data-label={definition?.title}>
      {body}
    </NodeViewWrapper>
  );
}
