import type { Editor } from "@tiptap/core";
import { LanguagePicker } from "./LanguagePicker";
import { PropsHead } from "./PropsHead";
import * as ui from "./ui";

/** The settings of the code block the cursor is in. */
export function CodeBlockFields({ editor, attrs }: { editor: Editor; attrs: Record<string, unknown> }) {
  const language = String(attrs.language ?? "");
  const fileName = String(attrs.fileName ?? "");
  const set = (name: "language" | "fileName", value: string) =>
    editor
      .chain()
      .updateAttributes("codeBlock", { [name]: value || null })
      .run();

  return (
    <>
      <PropsHead title="Code block" description="Code shown as written, with colors for its language when published." />
      <div className={ui.field}>
        <label className={ui.fieldLabel} htmlFor="code-language">
          Language
        </label>
        <LanguagePicker id="code-language" value={language} onChange={(value) => set("language", value)} />
      </div>
      <div className={ui.field}>
        <label className={ui.fieldLabel}>
          File name
          <input
            className={ui.input}
            value={fileName}
            placeholder="Optional"
            onChange={(e) => set("fileName", e.target.value)}
          />
        </label>
        <small className="text-muted">Shown above the code, like a tab: config.toml, src/main.rs.</small>
      </div>
    </>
  );
}
