import { mergeAttributes, Node } from "@tiptap/core";
import CodeBlock from "@tiptap/extension-code-block";
import { Table, TableCell, TableHeader, TableRow } from "@tiptap/extension-table";
import UniqueID from "@tiptap/extension-unique-id";
import { ReactNodeViewRenderer } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import { featureTabs, linkCard, relatedPages, seeAlso, whatsNew } from "@yourdocs/components";
import { type AnyComponent, propsSchema } from "@yourdocs/sdk";
import { ComponentView } from "./ComponentView";
import { PageLink, PagePickerTriggers } from "./pageLink";
import { SlashCommand } from "./slash";

// Built-in nodes must produce the same tags and classes as `renderNode` in @yourdocs/renderer.

const Callout = Node.create({
  name: "callout",
  group: "block",
  content: "block+",
  defining: true,
  addAttributes() {
    return {
      tone: {
        default: "note",
        parseHTML: (el) => el.getAttribute("data-tone") ?? "note",
        renderHTML: (attrs) => ({ "data-tone": attrs.tone }),
      },
    };
  },
  parseHTML: () => [{ tag: "aside.callout" }],
  renderHTML: ({ HTMLAttributes }) => ["aside", mergeAttributes({ class: "callout" }, HTMLAttributes), 0],
});

const Image = Node.create({
  name: "image",
  group: "block",
  atom: true,
  addAttributes() {
    return { assetId: { default: null }, alt: { default: "" }, caption: { default: "" } };
  },
  parseHTML: () => [{ tag: "figure" }],
  renderHTML: ({ node }) => {
    const { assetId, alt, caption } = node.attrs;
    const image = ["img", { src: `/_a/${assetId}`, alt: alt ?? "", loading: "lazy" }];

    return caption ? ["figure", {}, image, ["figcaption", {}, caption]] : ["figure", {}, image];
  },
});

// The file name is shown as a caption above the code (a CSS rule on `pre[data-file-name]`), so it travels as a data attribute.
const CodeWithFileName = CodeBlock.extend({
  addAttributes() {
    return {
      ...this.parent?.(),
      fileName: {
        default: null,
        parseHTML: (el) => el.getAttribute("data-file-name"),
        renderHTML: (attrs) => (attrs.fileName ? { "data-file-name": attrs.fileName } : {}),
      },
    };
  },
});

/** A component without child content: its whole state is its props, edited in the settings panel. */
function atomComponent(definition: AnyComponent) {
  const properties = propsSchema(definition).properties ?? {};

  return Node.create({
    name: definition.name,
    group: "block",
    atom: true,
    draggable: true,
    addAttributes() {
      return Object.fromEntries(Object.keys(properties).map((key) => [key, { default: properties[key].default }]));
    },
    parseHTML: () => [{ tag: `div[data-component="${definition.name}"]` }],
    renderHTML: () => ["div", { "data-component": definition.name }],
    addNodeView: () => ReactNodeViewRenderer(ComponentView),
  });
}

const TabPanel = Node.create({
  name: "tabPanel",
  content: "block+",
  isolating: true,
  addAttributes() {
    return { label: { default: "" } };
  },
  parseHTML: () => [{ tag: "div.tabs-panel" }],
  renderHTML: () => ["div", { role: "tabpanel", class: "tabs-panel" }, 0],
});

const Tabs = Node.create({
  name: "tabs",
  group: "block",
  content: "tabPanel+",
  isolating: true,
  parseHTML: () => [{ tag: "div.tabs" }],
  renderHTML: () => ["div", { class: "tabs" }, 0],
  // The tab strip is chrome around the panels, which are ordinary editable content.
  addNodeView() {
    return ({ node: initial }) => {
      let node = initial;
      let active = 0;

      const dom = document.createElement("div");
      dom.className = "tabs";
      const list = document.createElement("div");
      list.className = "tabs-list";
      list.setAttribute("role", "tablist");
      list.contentEditable = "false";
      const panels = document.createElement("div");
      // Not a box of its own, so the panels sit directly under `.tabs` as in the published markup.
      panels.style.display = "contents";
      dom.append(list, panels);

      const render = () => {
        const buttons: HTMLButtonElement[] = [];
        node.forEach((panel, _offset, i) => {
          const button = document.createElement("button");
          button.type = "button";
          button.className = "tabs-tab";
          button.setAttribute("role", "tab");
          button.id = `tab-${i}`;
          button.setAttribute("aria-selected", String(i === active));
          button.setAttribute("aria-controls", `panel-${i}`);
          button.textContent = String(panel.attrs.label ?? "");
          button.onclick = () => {
            active = i;
            render();
          };
          buttons.push(button);
        });
        list.replaceChildren(...buttons);
        syncPanels();
      };
      const syncPanels = () => {
        Array.from(panels.children).forEach((panel, i) => {
          panel.toggleAttribute("hidden", i !== active);
          panel.id = `panel-${i}`;
          panel.setAttribute("aria-labelledby", `tab-${i}`);
        });
      };
      render();
      // ProseMirror fills the panels after the view is created, and again when they are redrawn.
      new MutationObserver(syncPanels).observe(panels, { childList: true });

      return {
        dom,
        contentDOM: panels,
        update(updated) {
          if (updated.type !== node.type) return false;
          node = updated;
          active = Math.min(active, node.childCount - 1);
          render();
          return true;
        },
        ignoreMutation: (mutation) =>
          list.contains(mutation.target) || (mutation.type === "attributes" && mutation.target !== dom),
      };
    };
  },
});

// Block IDs are kept (and created for new blocks) so documents carry stable IDs.
const blockTypes = [
  "paragraph",
  "heading",
  "bulletList",
  "orderedList",
  "blockquote",
  "codeBlock",
  "table",
  "callout",
  "image",
  "tabs",
];

// While editing, a trailing paragraph keeps a place to click after a final block (a table, tabs…); read-only it would add a blank line the published page lacks.
export const buildExtensions = (editable: boolean) => [
  StarterKit.configure({ codeBlock: false, link: false, trailingNode: editable ? undefined : false }),
  PageLink,
  CodeWithFileName,
  Table,
  TableRow,
  TableHeader,
  TableCell,
  Callout,
  Image,
  Tabs,
  TabPanel,
  atomComponent(featureTabs as unknown as AnyComponent),
  atomComponent(whatsNew as unknown as AnyComponent),
  atomComponent(linkCard as unknown as AnyComponent),
  atomComponent(relatedPages as unknown as AnyComponent),
  atomComponent(seeAlso as unknown as AnyComponent),
  UniqueID.configure({ types: blockTypes }),
  ...(editable ? [SlashCommand, PagePickerTriggers] : []),
];
