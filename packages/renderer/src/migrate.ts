import { componentsByName } from "@yourdocs/components";
import { type AnyComponent, upgradeProps } from "@yourdocs/sdk";
import type { DocNode } from "./types";

/** The document with every component node upgraded to its component's current version. Returns the same object when nothing changes. */
export function migrateDoc(doc: DocNode, components: ReadonlyMap<string, AnyComponent> = componentsByName): DocNode {
  const component = components.get(doc.type);
  const content = doc.content?.map((child) => migrateDoc(child, components));
  const contentChanged = content?.some((child, i) => child !== doc.content?.[i]);

  if (!component) return contentChanged ? { ...doc, content } : doc;

  const attrs = upgradeProps(component, doc.attrs ?? {});
  const attrsChanged = JSON.stringify(attrs) !== JSON.stringify(doc.attrs ?? {});

  return attrsChanged || contentChanged ? { ...doc, attrs, ...(content ? { content } : {}) } : doc;
}
