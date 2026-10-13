import { LibrarySettings } from "./LibrarySettings";
import { RedirectsSettings } from "./RedirectsSettings";
import { SiteSettings } from "./SiteSettings";
import { TrashSettings } from "./TrashSettings";

export type SettingsSection = "site" | "library" | "redirects" | "trash";

export const sectionTitle: Record<SettingsSection, string> = {
  site: "Site and theme",
  library: "Library",
  redirects: "Redirects",
  trash: "Trash",
};

/** The settings of the whole site, one section at a time. */
export function SettingsView({
  section,
  onSection,
  onClose,
}: {
  section: SettingsSection;
  onSection: (section: SettingsSection) => void;
  onClose: () => void;
}) {
  return (
    <>
      <nav className="sidebar" aria-label="Settings">
        <button type="button" className="back" onClick={onClose}>
          ← Back to pages
        </button>
        <ul>
          {(Object.keys(sectionTitle) as SettingsSection[]).map((key) => (
            <li key={key}>
              <div className="row" data-selected={section === key} data-depth="1">
                <button
                  type="button"
                  className="title"
                  aria-current={section === key ? "page" : undefined}
                  onClick={() => onSection(key)}
                >
                  {sectionTitle[key]}
                </button>
              </div>
            </li>
          ))}
        </ul>
      </nav>
      <main className="canvas settings-main">
        {section === "site" ? (
          <SiteSettings />
        ) : section === "library" ? (
          <LibrarySettings />
        ) : section === "trash" ? (
          <TrashSettings />
        ) : (
          <RedirectsSettings />
        )}
      </main>
    </>
  );
}
