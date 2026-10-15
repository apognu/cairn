import { LibrarySettings } from "./LibrarySettings";
import { usePending, useSignedIn } from "./queries";
import { RedirectsSettings } from "./RedirectsSettings";
import { ReleasesSettings } from "./ReleasesSettings";
import { sidebar } from "./Sidebar";
import { SiteSettings } from "./SiteSettings";
import { TokensSettings } from "./TokensSettings";
import { TrashSettings } from "./TrashSettings";
import { rowClass, titleClass } from "./TreeParts";
import * as ui from "./ui";

export type SettingsSection = "site" | "library" | "redirects" | "releases" | "trash" | "tokens";

export const sectionTitle: Record<SettingsSection, string> = {
  site: "Site and theme",
  library: "Library",
  redirects: "Redirects",
  releases: "Releases",
  trash: "Trash",
  tokens: "API tokens",
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
  const pending = usePending();
  // Tokens belong to signed-in users; `dev` has none.
  const signedIn = useSignedIn();

  return (
    <>
      <nav className={sidebar} aria-label="Settings">
        <button
          type="button"
          className="mx-0.5 mt-0 mb-2.5 py-1.5 px-2 border-0 bg-transparent text-muted cursor-pointer"
          onClick={onClose}
        >
          ← Back to pages
        </button>
        <ul className="list-none m-0 p-0">
          {(Object.keys(sectionTitle) as SettingsSection[])
            .filter((key) => key !== "tokens" || signedIn)
            .map((key) => (
              <li key={key}>
                <div className={`${rowClass({ selected: section === key, depth: 1 })} pl-2.5`}>
                  <button
                    type="button"
                    className={titleClass({ selected: section === key, depth: 1 })}
                    aria-current={section === key ? "page" : undefined}
                    onClick={() => onSection(key)}
                  >
                    {sectionTitle[key]}
                  </button>
                  {key === "site" && pending.data?.settings && (
                    <span className={ui.dot} role="img" aria-label="Unpublished changes" />
                  )}
                </div>
              </li>
            ))}
        </ul>
      </nav>
      <main className="canvas relative overflow-y-auto col-[2/-1] py-6.5 px-8 text-ui leading-[1.4] has-[.sticky-actions]:pb-0 has-[.sticky-actions]:scroll-pb-20">
        {section === "site" ? (
          <SiteSettings />
        ) : section === "library" ? (
          <LibrarySettings />
        ) : section === "releases" ? (
          <ReleasesSettings />
        ) : section === "trash" ? (
          <TrashSettings />
        ) : section === "tokens" ? (
          <TokensSettings />
        ) : (
          <RedirectsSettings />
        )}
      </main>
    </>
  );
}
