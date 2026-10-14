import type { Editor } from "@tiptap/core";

// Names the highlighter knows, as they are written in the document.
const languages: [value: string, label: string][] = [
  ["bash", "Bash"],
  ["c", "C"],
  ["cpp", "C++"],
  ["csharp", "C#"],
  ["css", "CSS"],
  ["diff", "Diff"],
  ["dockerfile", "Dockerfile"],
  ["go", "Go"],
  ["html", "HTML"],
  ["java", "Java"],
  ["javascript", "JavaScript"],
  ["json", "JSON"],
  ["kotlin", "Kotlin"],
  ["markdown", "Markdown"],
  ["php", "PHP"],
  ["python", "Python"],
  ["ruby", "Ruby"],
  ["rust", "Rust"],
  ["sql", "SQL"],
  ["swift", "Swift"],
  ["toml", "TOML"],
  ["typescript", "TypeScript"],
  ["xml", "XML"],
  ["yaml", "YAML"],
];

/** The settings of the code block the cursor is in. */
export function CodeBlockFields({ editor, attrs }: { editor: Editor; attrs: Record<string, unknown> }) {
  const language = String(attrs.language ?? "");
  const fileName = String(attrs.fileName ?? "");
  const set = (name: "language" | "fileName", value: string) =>
    editor
      .chain()
      .updateAttributes("codeBlock", { [name]: value || null })
      .run();
  // A language written some other way (in an imported page, say) stays selectable instead of silently turning to plain text.
  const known = languages.some(([value]) => value === language);

  return (
    <>
      <div className="props-head">
        <h2>Code block</h2>
      </div>
      <div className="form-field">
        <label>
          Language
          <select value={language} onChange={(e) => set("language", e.target.value)}>
            <option value="">Plain text</option>
            {!known && language && <option value={language}>{language}</option>}
            {languages.map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>
        </label>
      </div>
      <div className="form-field">
        <label>
          File name
          <input value={fileName} placeholder="Optional" onChange={(e) => set("fileName", e.target.value)} />
        </label>
      </div>
    </>
  );
}
