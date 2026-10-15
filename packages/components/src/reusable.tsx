import { defineComponent, z } from "@cairn/sdk";
import { useRenderEnv } from "./env";

/** A library variable written inline. Published pages get its value written into the text before rendering; the editor shows it here. */
export const variable = defineComponent({
  name: "variable",
  version: 1,
  title: "Variable",
  description: "A value kept in the library, such as the latest version, shown wherever it is used.",
  props: z.object({ key: z.string().default("").meta({ title: "Variable", "x-field": "variable" }) }),
  render: (props) => <Variable name={props.key} />,
});

function Variable({ name }: { name: string }) {
  const { variables, editing } = useRenderEnv();
  const value = variables[name];

  if (value === undefined)
    return editing ? (
      <span className="variable" data-empty>
        {name ? `{{${name}}}` : "{{variable}}"}
      </span>
    ) : null;

  return <span className="variable">{value}</span>;
}

/** A library snippet included as a block. */
export const snippet = defineComponent({
  name: "snippet",
  version: 1,
  title: "Snippet",
  description: "A passage kept in the library, the same everywhere it is used.",
  props: z.object({ key: z.string().default("").meta({ title: "Snippet", "x-field": "snippet" }) }),
  render: (props) => <Snippet name={props.key} />,
});

function Snippet({ name }: { name: string }) {
  const { snippets, editing } = useRenderEnv();
  const content = snippets[name];

  if (content === undefined)
    return editing ? (
      <div className="snippet" data-empty>
        {name ? `Snippet “${name}” does not exist` : "Choose a snippet in the settings panel"}
      </div>
    ) : null;

  return <div className="snippet">{content}</div>;
}
