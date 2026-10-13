import { expect, test } from "bun:test";
import { defineComponent, z } from "@yourdocs/sdk";
import { migrateDoc } from "./migrate";
import type { DocNode } from "./types";

// Version 1 had a single `text`; version 2 splits it in a `title` and a `body`.
const note = defineComponent({
  name: "acme/note",
  version: 2,
  title: "Note",
  props: z.object({ title: z.string().default(""), body: z.string().default("") }),
  migrate: {
    1: ({ text, ...rest }) => ({ ...rest, title: String(text ?? "").split(". ")[0], body: String(text ?? "") }),
  },
  render: () => null,
});
const components = new Map([[note.name, note as never]]);

test("a version 1 document is migrated when it is opened with a version 2 component", () => {
  const old: DocNode = {
    type: "doc",
    content: [
      { type: "paragraph" },
      { type: "acme/note", attrs: { text: "First. Second", id: "b1" } },
      { type: "acme/note", attrs: { __v: 2, title: "Kept", body: "as is" } },
    ],
  };
  const migrated = migrateDoc(old, components);

  expect(migrated.content?.[1].attrs).toEqual({ title: "First", body: "First. Second", id: "b1", __v: 2 });
  // A node already at the current version, and nodes that are not components, are left alone.
  const [paragraph, , current] = old.content as DocNode[];
  expect(migrated.content?.[2]).toBe(current);
  expect(migrated.content?.[0]).toBe(paragraph);
  // Migrating twice changes nothing more.
  expect(migrateDoc(migrated, components)).toBe(migrated);
});
