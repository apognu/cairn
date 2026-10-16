import {
  cards,
  featureTabs,
  fileTree,
  linkCard,
  relatedPages,
  seeAlso,
  snippet,
  variable,
  whatsNew,
} from "@cairn/components";
import { customComponents } from "@cairn/components/custom";
import { type AnyComponent, propsSchema, VERSION_ATTR } from "@cairn/sdk";
import { Mark, mergeAttributes, Node } from "@tiptap/core";
import CodeBlock from "@tiptap/extension-code-block";
import { Table, TableCell, TableHeader, TableRow } from "@tiptap/extension-table";
import UniqueID from "@tiptap/extension-unique-id";
import { ReactNodeViewRenderer } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import { ComponentView } from "./ComponentView";
import { codeHighlight } from "./codeHighlight";
import { EmojiSuggestion } from "./emoji";
import { PageLink, PagePickerTriggers } from "./pageLink";
import { PhantomLine } from "./phantom";
import { SlashCommand } from "./slash";
import { StepsEditing } from "./steps";
import { TermSuggestion } from "./termSuggestion";

// Built-in nodes must produce the same tags and classes as `renderNode` in @cairn/renderer.

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
    return {
      assetId: { default: null },
      alt: { default: "" },
      caption: { default: "" },
      decorative: { default: false },
    };
  },
  parseHTML: () => [{ tag: "figure" }],
  renderHTML: ({ node }) => {
    const { assetId, alt, caption, decorative } = node.attrs;
    const image = ["img", { src: `/_a/${assetId}`, alt: decorative ? "" : (alt ?? ""), loading: "lazy" }];

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
  addProseMirrorPlugins() {
    return [...(this.parent?.() ?? []), codeHighlight()];
  },
});

/** A component without child content: its whole state is its props, edited in the settings panel. */
function atomComponent(definition: AnyComponent, inline = false) {
  const properties = propsSchema(definition).properties ?? {};

  return Node.create({
    name: definition.name,
    group: inline ? "inline" : "block",
    inline,
    atom: true,
    draggable: true,
    addAttributes() {
      return {
        ...Object.fromEntries(Object.keys(properties).map((key) => [key, { default: properties[key].default }])),
        // Records which version of the component wrote the props; `migrate` reads it when the component changes.
        [VERSION_ATTR]: { default: definition.version },
      };
    },
    parseHTML: () => [{ tag: `${inline ? "span" : "div"}[data-component="${definition.name}"]` }],
    renderHTML: () => [inline ? "span" : "div", { "data-component": definition.name }],
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
  // The tab group shows and hides its panels and links them to their tabs through attributes. Read back as an edit, each change
  // redrew the panel, which the group changed again: the editor froze on a page with tabs.
  addNodeView() {
    return () => {
      const dom = document.createElement("div");
      dom.className = "tabs-panel";
      dom.setAttribute("role", "tabpanel");

      return {
        dom,
        contentDOM: dom,
        ignoreMutation: (mutation) => mutation.type === "attributes" && mutation.target === dom,
      };
    };
  },
});

let tabGroups = 0;

const Tabs = Node.create({
  name: "tabs",
  group: "block",
  content: "tabPanel+",
  isolating: true,
  // "code" makes a code group: tabs around code blocks, drawn as one box.
  addAttributes() {
    return {
      variant: {
        default: null,
        parseHTML: (el) => (el.classList.contains("tabs-code") ? "code" : null),
        rendered: false,
      },
    };
  },
  parseHTML: () => [{ tag: "div.tabs" }],
  renderHTML: ({ node }) => ["div", { class: node.attrs.variant === "code" ? "tabs tabs-code" : "tabs" }, 0],
  // The tab strip is chrome around the panels, which are ordinary editable content.
  addNodeView() {
    return ({ node: initial, editor, getPos }) => {
      let node = initial;
      let active = 0;
      // Unique per tab group, so the ARIA links of two groups on one page never cross.
      const prefix = `tabs-${++tabGroups}`;

      const dom = document.createElement("div");
      const className = () => (node.attrs.variant === "code" ? "tabs tabs-code" : "tabs");
      dom.className = className();
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
          button.id = `${prefix}-tab-${i}`;
          button.setAttribute("aria-selected", String(i === active));
          button.setAttribute("aria-controls", `${prefix}-panel-${i}`);
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
          panel.id = `${prefix}-panel-${i}`;
          panel.setAttribute("aria-labelledby", `${prefix}-tab-${i}`);
        });
      };
      render();
      // ProseMirror fills the panels after the view is created, and again when they are redrawn.
      const observer = new MutationObserver(syncPanels);
      observer.observe(panels, { childList: true });

      // The cursor in a hidden panel (the page opens with it at the end, which can be the last tab) is moved out by the browser, read back
      // by ProseMirror, and the panels drawn again, without end: the tab holding the cursor is always the one shown.
      const followSelection = () => {
        const start = getPos();
        if (typeof start !== "number") return;
        const { from } = editor.state.selection;
        if (from <= start || from >= start + node.nodeSize) return;

        let offset = start + 1;
        node.forEach((panel, _offset, i) => {
          if (from >= offset && from < offset + panel.nodeSize && i !== active) {
            active = i;
            render();
          }
          offset += panel.nodeSize;
        });
      };
      editor.on("selectionUpdate", followSelection);
      followSelection();

      return {
        dom,
        contentDOM: panels,
        update(updated) {
          if (updated.type !== node.type) return false;
          node = updated;
          if (dom.className !== className()) dom.className = className();
          active = Math.min(active, node.childCount - 1);
          render();
          return true;
        },
        // The tab strip and every attribute (class, hidden, ARIA) are this view's own: reading them back as edits would update the
        // state, redraw the view, change them again, and never stop.
        ignoreMutation: (mutation) => list.contains(mutation.target) || mutation.type === "attributes",
        destroy: () => {
          observer.disconnect();
          editor.off("selectionUpdate", followSelection);
        },
      };
    };
  },
});

/** Words marked as a glossary term: shown dotted here, and a definition on hover with a link to the Glossary page once published. */
const Term = Mark.create({
  name: "term",
  inclusive: false,
  addAttributes() {
    return {
      term: {
        default: "",
        parseHTML: (el) => el.getAttribute("data-term") ?? "",
        renderHTML: (attrs) => ({ "data-term": attrs.term }),
      },
    };
  },
  parseHTML: () => [{ tag: "dfn[data-term]" }],
  renderHTML: ({ HTMLAttributes }) => ["dfn", mergeAttributes({ class: "term" }, HTMLAttributes), 0],
});

const Step = Node.create({
  name: "step",
  // The first paragraph is the step's title; whatever follows is its body.
  content: "paragraph block*",
  defining: true,
  isolating: true,
  parseHTML: () => [{ tag: "li.step" }],
  renderHTML: () => ["li", { class: "step" }, 0],
});

const Steps = Node.create({
  name: "steps",
  group: "block",
  content: "step+",
  isolating: true,
  parseHTML: () => [{ tag: "ol.steps" }],
  renderHTML: () => ["ol", { class: "steps" }, 0],
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
  "steps",
];

// While editing, a trailing paragraph keeps a place to click after a final block (a table, tabs…); read-only it would add a blank line the published page lacks.
export const buildExtensions = (editable: boolean) => [
  StarterKit.configure({ codeBlock: false, link: false, trailingNode: editable ? undefined : false }),
  PageLink,
  Term,
  CodeWithFileName,
  // Wrapped as the published page wraps it, so a wide table scrolls inside its own box in both.
  Table.configure({ renderWrapper: true }),
  TableRow,
  TableHeader,
  TableCell,
  Callout,
  Image,
  Tabs,
  TabPanel,
  Steps,
  Step,
  atomComponent(featureTabs as unknown as AnyComponent),
  atomComponent(whatsNew as unknown as AnyComponent),
  atomComponent(linkCard as unknown as AnyComponent),
  atomComponent(relatedPages as unknown as AnyComponent),
  atomComponent(seeAlso as unknown as AnyComponent),
  atomComponent(cards as unknown as AnyComponent),
  atomComponent(fileTree as unknown as AnyComponent),
  atomComponent(variable as unknown as AnyComponent, true),
  atomComponent(snippet as unknown as AnyComponent),
  ...customComponents.map((component) => atomComponent(component)),
  UniqueID.configure({ types: blockTypes }),
  ...(editable ? [SlashCommand, EmojiSuggestion, TermSuggestion, PagePickerTriggers, PhantomLine, StepsEditing] : []),
];
