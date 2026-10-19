import { LibrarySettings } from "./LibrarySettings";
import { MediaSettings } from "./MediaSettings";
import { useCan, usePending, useSignedIn } from "./queries";
import { RedirectsSettings } from "./RedirectsSettings";
import { ReleasesSettings } from "./ReleasesSettings";
import { ReviewSettings } from "./ReviewSettings";
import { sidebar } from "./Sidebar";
import { SiteSettings } from "./SiteSettings";
import { TokensSettings } from "./TokensSettings";
import { TrashSettings } from "./TrashSettings";
import { rowClass, titleClass } from "./TreeParts";
import { UsersSettings } from "./UsersSettings";
import * as ui from "./ui";

export type SettingsSection =
  | "site"
  | "library"
  | "media"
  | "redirects"
  | "releases"
  | "review"
  | "trash"
  | "users"
  | "tokens";

export const sectionTitle: Record<SettingsSection, string> = {
  site: "Site and theme",
  library: "Library",
  media: "Media",
  redirects: "Redirects",
  releases: "Releases",
  review: "Pages to review",
  trash: "Trash",
  users: "Users",
  tokens: "API tokens",
};

/** The settings of the whole site, one section at a time. */
export function SettingsView({
  section,
  onSection,
  onClose,
  onOpenPage,
}: {
  section: SettingsSection;
  onSection: (section: SettingsSection) => void;
  onClose: () => void;
  onOpenPage: (id: string) => void;
}) {
  const pending = usePending();
  // Tokens and users belong to signed-in people; `dev` has neither.
  const signedIn = useSignedIn();
  const admin = useCan("admin");
  const editor = useCan("editor");

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
            .filter((key) => key !== "users" || (signedIn && admin))
            .filter((key) => key !== "media" || editor)
            // Site settings are the admins'; the other sections can be read by everyone.
            .filter((key) => key !== "site" || admin)
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
      <main className="canvas relative overflow-y-auto col-[2/-1] py-6.5 px-8 mobile:col-[1/-1] mobile:px-4 text-ui leading-[1.4] has-[.sticky-actions]:pb-0 has-[.sticky-actions]:scroll-pb-20">
        {section === "site" ? (
          admin ? (
            <SiteSettings />
          ) : (
            <LibrarySettings />
          )
        ) : section === "library" ? (
          <LibrarySettings />
        ) : section === "media" && editor ? (
          <MediaSettings onOpenPage={onOpenPage} />
        ) : section === "releases" ? (
          <ReleasesSettings />
        ) : section === "review" ? (
          <ReviewSettings onOpenPage={onOpenPage} />
        ) : section === "trash" ? (
          <TrashSettings />
        ) : section === "tokens" ? (
          <TokensSettings />
        ) : section === "users" ? (
          <UsersSettings />
        ) : (
          <RedirectsSettings />
        )}
      </main>
    </>
  );
}
