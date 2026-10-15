import { LibrarySettings } from "./LibrarySettings";
import { usePending, useSignedIn } from "./queries";
import { RedirectsSettings } from "./RedirectsSettings";
import { ReleasesSettings } from "./ReleasesSettings";
import { SiteSettings } from "./SiteSettings";
import { TokensSettings } from "./TokensSettings";
import { TrashSettings } from "./TrashSettings";

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
      <nav className="sidebar settings-nav" aria-label="Settings">
        <button type="button" className="back" onClick={onClose}>
          ← Back to pages
        </button>
        <ul>
          {(Object.keys(sectionTitle) as SettingsSection[])
            .filter((key) => key !== "tokens" || signedIn)
            .map((key) => (
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
                  {key === "site" && pending.data?.settings && (
                    <span className="dot" role="img" aria-label="Unpublished changes" />
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
