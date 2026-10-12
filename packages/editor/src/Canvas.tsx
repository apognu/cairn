import { EditorContent, useEditor } from "@tiptap/react";
import { RenderEnvContext } from "@yourdocs/components";
import { collectHeadings, type DocNode, defaultContext, type RenderContext, SiteLayout } from "@yourdocs/renderer";
import { useEffect, useState } from "react";
import { BubbleMenus } from "./BubbleMenus";
import { buildExtensions } from "./extensions";
import { PropsPanel } from "./PropsPanel";

interface Props {
  doc: DocNode;
  context?: Partial<RenderContext>;
  editable?: boolean;
  /** Leave out the site header and navigation (the editor already has its own). */
  embedded?: boolean;
  onChange?: (doc: DocNode) => void;
  onBlur?: () => void;
  /** Where the settings panel of the component under the cursor is drawn. */
  panelHost?: HTMLElement | null;
}

/** The page body as ProseMirror, mounted where the published page puts its content. */
export function Canvas({ doc, context, editable = true, embedded = false, onChange, onBlur, panelHost }: Props) {
  const editor = useEditor({
    extensions: buildExtensions(editable),
    content: doc,
    editable,
    onUpdate: ({ editor: e }) => onChange?.(e.getJSON() as DocNode),
    onBlur: () => onBlur?.(),
  });
  const [headings, setHeadings] = useState(() => collectHeadings(doc).list);

  useEffect(() => {
    if (!editor) return;

    const update = () => setHeadings(collectHeadings(editor.getJSON() as DocNode).list);
    editor.on("update", update);

    return () => {
      editor.off("update", update);
    };
  }, [editor]);

  // The editor shows "new until" dates against today's date, as the publish render does with its own.
  const full = { ...defaultContext, today: new Date().toISOString().slice(0, 10), ...context };

  return (
    <RenderEnvContext.Provider value={{ urls: full.urls, today: full.today }}>
      <SiteLayout context={full} headings={headings} embedded={embedded}>
        <EditorContent editor={editor} />
        {editor && editable && <BubbleMenus editor={editor} />}
        {editor && editable && panelHost && <PropsPanel editor={editor} host={panelHost} />}
      </SiteLayout>
    </RenderEnvContext.Provider>
  );
}
