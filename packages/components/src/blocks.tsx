import { defineComponent, icon, link, z } from "@yourdocs/sdk";
import { FeatureCard } from "./features";
import { upgradeIconNames } from "./icons";

const card = z.object({
  icon: icon().default("star"),
  title: z.string().default("Card"),
  description: z.string().default(""),
  page: link(),
  href: z.string().optional().meta({ title: "Web address" }),
});

/** A grid of cards, each with an icon, a title, a description and an optional link to a page or a web address. */
export const cards = defineComponent({
  name: "cards",
  version: 2,
  // Version 1 had a dozen icons of its own; 2 uses Lucide, where three of them have other names.
  migrate: { 1: upgradeIconNames },
  title: "Cards",
  props: z.object({
    columns: z.enum(["2", "3"]).default("2"),
    cards: z
      .array(card)
      .default([{ icon: "star", title: "Card", description: "What it is about", page: undefined, href: undefined }]),
  }),
  render: (props) => (
    <div className="feature-grid" data-columns={props.columns}>
      {props.cards.map((c, i) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: cards are positional
        <FeatureCard key={i} feature={c} path={`cards.${i}`} />
      ))}
    </div>
  ),
});

type FileNode = { name: string; folder: boolean; children: FileNode[] };

/** Lines indented by two spaces per level; a trailing slash marks a folder. Anything indented under a name makes it a folder too. */
export function parseFileTree(text: string): FileNode[] {
  const roots: FileNode[] = [];
  const stack: { depth: number; node: FileNode }[] = [];

  for (const line of text.split("\n")) {
    if (!line.trim()) continue;
    const depth = Math.floor((line.length - line.trimStart().length) / 2);
    const name = line.trim();
    const node: FileNode = { name: name.replace(/\/$/, ""), folder: name.endsWith("/"), children: [] };

    while (stack.length > 0 && stack[stack.length - 1].depth >= depth) stack.pop();
    const parent = stack[stack.length - 1]?.node;
    if (parent) {
      parent.folder = true;
      parent.children.push(node);
    } else roots.push(node);
    stack.push({ depth, node });
  }

  return roots;
}

function FileList({ nodes }: { nodes: FileNode[] }) {
  return (
    <ul>
      {nodes.map((n, i) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: names can repeat in different folders
        <li key={i} className={n.folder ? "file-tree-folder" : "file-tree-file"}>
          <span>{n.name}</span>
          {n.children.length > 0 && <FileList nodes={n.children} />}
        </li>
      ))}
    </ul>
  );
}

export const fileTree = defineComponent({
  name: "fileTree",
  version: 1,
  title: "File tree",
  props: z.object({
    tree: z
      .string()
      .meta({
        title: "Files",
        "x-field": "text",
        description: "One name per line, indented two spaces per level; end a folder with /",
      })
      .default("src/\n  main.rs\n  lib.rs\nCargo.toml"),
  }),
  render: (props) => (
    <div className="file-tree">
      <FileList nodes={parseFileTree(props.tree)} />
    </div>
  ),
});
