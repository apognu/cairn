export interface paths {
    "/api/assets": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Uploads an image (the request body is the file). Uploading bytes that are already stored returns the existing asset. */
        post: operations["uploadAsset"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/events": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Server-sent events for open editors: lock changes and new releases. They carry IDs only; the editor reads what it needs again. A
         *     stream that falls behind skips the oldest events, so a reconnecting editor reloads its state.
         */
        get: operations["editorEvents"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/highlight": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Colors code the way publishing does, for the editor's canvas. */
        post: operations["highlightCode"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/jobs/run": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Runs the periodic jobs: deletes the uploads nothing uses any more, and reports the pages due for a review. Meant for a scheduler (cron, Cloud Scheduler) with an API token. */
        post: operations["runJobs"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/library": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Every variable, snippet and glossary term. */
        get: operations["listLibrary"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/library/snippets/{key}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** Creates or changes a snippet. One that would include itself is refused, with the chain in the message. */
        put: operations["saveSnippet"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/library/terms": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /**
         * Creates or changes a glossary term. A marked term shows its definition on hover and links to the generated Glossary page, which also
         *     lists its aliases and links the page that explains it in full.
         */
        put: operations["saveTerm"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/library/variables/{key}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** Creates or changes a variable. */
        put: operations["saveVariable"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/library/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** Deletes an entry nobody uses. */
        delete: operations["deleteLibraryEntry"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/locks": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Every page someone is editing now. */
        get: operations["listLocks"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/me": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** The signed-in user, with their role. */
        get: operations["getMe"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/pages": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Every live page, ordered by position. */
        get: operations["listPages"];
        put?: never;
        /** Creates a page, or an external link when `external_url` is set, under `parent_id` (or at the root). */
        post: operations["createPage"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/pages/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Changes the title (the URL segment follows it) and, for a link, its target. */
        post: operations["renamePage"];
        /** Deletes a page and its descendants. Refused with 423 while someone else edits one of them. */
        delete: operations["deletePage"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/pages/{id}/aliases": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Adds an alias: an old URL that redirects to the page. */
        post: operations["addAlias"];
        /** Removes an alias of the page. */
        delete: operations["removeAlias"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/pages/{id}/api-config": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** The sources an API reference page is built from. */
        get: operations["getApiConfig"];
        /**
         * Saves the sources, merge rules and display settings. They are a pending change of the page until the next publish; the specs themselves
         *     are fetched again at every publish and never are.
         */
        put: operations["saveApiConfig"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/pages/{id}/api-config/check": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Fetches and merges the given sources (the unsaved form as it stands) the way a publish would, without creating anything. */
        post: operations["checkApiConfig"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/pages/{id}/api-spec": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** The merged spec of the page's last successful check, or, before any, of the live release: what the settings screen previews. */
        get: operations["getApiSpec"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/pages/{id}/backlinks": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** The pages that link to this one. */
        get: operations["getBacklinks"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/pages/{id}/description": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** Sets the page's short description, shown on link cards and in its meta tags. */
        put: operations["setPageDescription"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/pages/{id}/draft": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** The latest saved edits of a page (an empty document if it has none yet). */
        get: operations["getDraft"];
        /**
         * Saves a draft, which also renews the lock. Refused with 409 when `base_revision_id` is not the page's latest revision, and with 423 or
         *     410 when the saver does not hold the page's lock.
         */
        put: operations["saveDraft"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/pages/{id}/header": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** Shows the page in the site header, or takes it out. A promoted page with subpages gets a sidebar of its own. */
        put: operations["setPageHeader"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/pages/{id}/history": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** The saved versions of a page, newest first. */
        get: operations["getHistory"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/pages/{id}/impact": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** What deleting a page would affect: its subpages and every page that links into them. */
        get: operations["getDeleteImpact"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/pages/{id}/lock": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Who is editing the page, if anyone. */
        get: operations["getLock"];
        /** Keeps the lock while its holder works. Refused with 423 when someone else took the page, and 410 when the lock was given up. */
        put: operations["renewLock"];
        /** Starts editing: takes the page's lock with a new token. Refused with 423 while someone else holds it. */
        post: operations["acquireLock"];
        /** Stops editing. The editor calls it when the page is left or after a long idle time; a token that lost the lock changes nothing. */
        delete: operations["releaseLock"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/pages/{id}/lock/take-over": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Takes the page from someone who left it idle (or, with `force`, from someone still at work: admins only). Their editor is told at
         *     once, stops, and keeps what it has not saved in their browser. Refused with 409 while the holder is active and `force` is not set.
         */
        post: operations["takeOverLock"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/pages/{id}/move": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Moves a page and its subtree under another parent. */
        post: operations["movePage"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/pages/{id}/operations": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * The operations of an API page that a link can point at: those of its last check, or of the live release before any. Empty when the
         *     page was never resolved.
         */
        get: operations["listApiOperations"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/pages/{id}/restore": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Makes an older version the page's draft, and optionally its title and description the page's. The draft it replaces stays in the
         *     history, so the restore can be undone.
         */
        post: operations["restoreRevision"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/pages/{id}/review": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** A page's owner and review date. */
        get: operations["getPageReview"];
        /** Sets a page's owner and review date. Saved at once: it is bookkeeping for the team, not content, and is never published. */
        put: operations["setPageReview"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/pages/{id}/url": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** The page's URL, custom URL and aliases. */
        get: operations["getPageUrl"];
        /** Sets or clears the page's custom URL. */
        put: operations["setPageUrl"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/publish": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Publishes the latest saved draft of every page as a new release, or of the pages picked for a hotfix. */
        post: operations["publish"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/publish/changes": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** What the next publish would change compared with the live site, and the live URLs it would break. */
        get: operations["pendingChanges"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/publish/checks": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** The problems that would stop a publish right now, each naming its page. An empty list means the site can be published. */
        get: operations["publishChecks"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/publish/plan": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** What a hotfix of these pages would also publish, and which of their links would show as plain text. Nothing is published. */
        post: operations["hotfixPlan"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/publish/preview": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Builds the latest saved drafts as a release that is not live: it can be looked at (`preview_url` when the target gives it an address of
         *     its own, `/_preview/<id>/` when it is self-hosted) and then promoted. Only for targets that can do it.
         */
        post: operations["publishPreview"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/publish/progress": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Server-sent events while a build is sent to the publish target, each a `PublishProgress` as JSON, about one per percent. Nothing comes
         *     while the pages render. Builds run one at a time, so the events are those of the build running.
         */
        get: operations["publishProgress"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/redirects": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Every redirect of the site, with where it leads and whether it is written. */
        get: operations["listRedirects"];
        put?: never;
        /** Adds a redirect by hand. */
        post: operations["addRedirect"];
        /** Deletes a redirect. */
        delete: operations["removeRedirect"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/releases": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** The latest releases that were the live site, newest first, for the Releases settings and rollback. */
        get: operations["listReleases"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/releases/{id}/promote": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Makes a preview the live site, exactly as it was built. */
        post: operations["promoteRelease"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/releases/{id}/rollback": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Makes an earlier release the live site again. Drafts are untouched, and what was published after it shows as pending. */
        post: operations["rollbackRelease"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/reports/stale": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** The pages whose review date has come, the most overdue first. */
        get: operations["stalePages"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/revisions/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** The content of one saved version. */
        get: operations["getRevision"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/scalar.js": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** The API reference script (Scalar), for the settings screen's preview. */
        get: operations["getScalarScript"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/settings": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** The site name, logo and accent color. */
        get: operations["getSettings"];
        /** Saves the settings. They reach the published site with the next publish. */
        put: operations["saveSettings"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/settings/check": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Checks unsaved settings: the contrast problems and the custom properties of each mode. */
        post: operations["checkSettings"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/site": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Where the published site lives, for the editor's "View as reader", and what the publish dialog says of its target. */
        get: operations["siteInfo"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/theme.css": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** The saved theme as a stylesheet, which the editor loads after the site's so authors write inside the site's colors. */
        get: operations["getThemeCss"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/tokens": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** The signed-in user's API tokens. Tokens can only be managed from a browser session, never with a token. */
        get: operations["listTokens"];
        put?: never;
        /** Makes a token for scripts and CI. It acts with its own role if it has one, and never above its owner's current role. */
        post: operations["createToken"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/tokens/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** Deletes a token; scripts using it stop working at once. */
        delete: operations["revokeToken"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/trash": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Deleted pages, most recent first. */
        get: operations["listTrash"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/trash/{id}/restore": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Brings a page back from the trash, with the pages deleted with it. */
        post: operations["restoreFromTrash"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/users": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Everyone who signed in, by name, with their role: the owners a page can have, and the people an admin gives roles to. */
        get: operations["listUsers"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/users/{id}/role": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** Gives someone a role. It applies to their next request, and to their API tokens. The site always keeps at least one admin. */
        put: operations["setUserRole"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
}
export type webhooks = Record<string, never>;
export interface components {
    schemas: {
        AcquireLock: {
            /**
             * Format: int64
             * @description How long ago, in milliseconds, the user last typed or clicked in the tab asking; absent when they have not yet.
             */
            idle_ms?: number | null;
        };
        AcquiredLock: {
            lock: components["schemas"]["LockView"];
            /** @description Sent with every save and renewal. */
            token: components["schemas"]["LockToken"];
        };
        Alias: {
            /** Format: date-time */
            created_at: string;
            origin: components["schemas"]["RedirectOrigin"];
            path: string;
        };
        /** @description What fetching and merging a page's sources gave. */
        ApiCheck: {
            /** @description Why the sources could not be merged, one sentence each. Empty when they merged. */
            errors: string[];
            /** @description The first operations, for a glance. */
            operations: components["schemas"]["Operation"][];
            /** @description All operations of the merged spec. */
            operations_count: number;
            /** @description Where each source was read from, and at which release. */
            sources: components["schemas"]["ResolvedSource"][];
            tags: string[];
        };
        /** @description What an API reference page is built from: public OpenAPI specs, merged into one document at every publish. */
        ApiConfig: {
            /**
             * @default {
             *       "beta": false,
             *       "hide_models": false,
             *       "request_client": true,
             *       "servers": []
             *     }
             */
            display: components["schemas"]["ApiDisplay"];
            /**
             * @default {
             *       "description": null,
             *       "title": null,
             *       "version": null
             *     }
             */
            merge: components["schemas"]["MergeRules"];
            /**
             * @description Merged in this order; the first one provides the document's `info` unless overridden.
             * @default []
             */
            sources: components["schemas"]["ApiSource"][];
        };
        ApiDisplay: {
            /**
             * @description Shows a "Beta" pill after the page's name in the site's sidebar.
             * @default false
             */
            beta: boolean;
            /** @default false */
            hide_models: boolean;
            /**
             * @description Lets readers try requests from the page.
             * @default true
             */
            request_client: boolean;
            /**
             * @description Server URLs shown in the reference; the specs' own are used when empty.
             * @default []
             */
            servers: string[];
        };
        /** @description Where one spec comes from. Everything is fetched anonymously over HTTPS. */
        ApiSource: {
            prefix?: string;
            /** @enum {string} */
            type: "url";
            url: string;
        } | {
            /** @description File name of a release asset, instead of a file in the repository. */
            asset?: string | null;
            git_ref?: components["schemas"]["GitRef"];
            /** @description Path of the spec inside the repository; ignored for release assets. */
            path?: string;
            prefix?: string;
            /** @description `owner/name`. */
            repo: string;
            /** @enum {string} */
            type: "github";
        };
        /** @description A token for scripts and CI, as listed: the secret itself is shown only when the token is created. */
        ApiToken: {
            /** Format: date-time */
            created_at: string;
            id: string;
            /** Format: date-time */
            last_used_at?: string | null;
            name: string;
            role?: components["schemas"]["Role"] | null;
        };
        /** @description An uploaded file. The bytes live in the `AssetStore` under the content hash; this is the row that describes them. */
        Asset: {
            /** Format: int64 */
            bytes: number;
            /** Format: date-time */
            created_at: string;
            /** Format: int32 */
            height?: number | null;
            id: components["schemas"]["AssetId"];
            mime: string;
            /** @description The image's average color, `#rrggbb`, shown while it loads. */
            placeholder?: string | null;
            /** @description Hex SHA-256 of the file: two uploads of the same bytes are one asset. */
            sha256: string;
            uploaded_by: components["schemas"]["UserId"];
            /** @description The WebP copies made at upload, narrowest first; empty for animated GIFs. */
            variants?: components["schemas"]["AssetVariant"][];
            /** Format: int32 */
            width?: number | null;
        };
        /** @description An uploaded file. */
        AssetId: string;
        /** @description One scaled copy of an image, a WebP file in the `AssetStore` (see `variant_key`). */
        AssetVariant: {
            /** Format: int64 */
            bytes: number;
            /** Format: int32 */
            height: number;
            /** Format: int32 */
            width: number;
        };
        /** @description A page that links to another. */
        Backlink: {
            kind: components["schemas"]["LinkKind"];
            source_id: components["schemas"]["PageId"];
            source_title: string;
            /** @description The page it links to (one of the pages below the one asked about, for a delete). */
            target_title: string;
        };
        Backlinks: {
            linked_from: components["schemas"]["Backlink"][];
            /** @description Glossary terms that send readers to the page for the full explanation. */
            terms: components["schemas"]["TermLink"][];
        };
        /** @description A message at the top of every page, which readers can dismiss. Off while `text` is empty. */
        Banner: {
            /** @default  */
            text: string;
            /**
             * @description Where the message leads: `https://…`, `mailto:…` or a path on the site; empty for none.
             * @default
             */
            url: string;
        };
        /** @description A link from a page that stays to a page that is going away. */
        BrokenLink: {
            source_id: components["schemas"]["PageId"];
            source_title: string;
            target_title: string;
        };
        /** @description One page with something to publish. */
        Change: {
            kinds: components["schemas"]["ChangeKind"][];
            live_revision_id?: components["schemas"]["RevisionId"] | null;
            /** @description The URL after publishing, for pages that still exist. */
            new_url?: string | null;
            /** @description The title in the live release, when it changed. */
            old_title?: string | null;
            /** @description The URL in the live release, for pages that were already published. */
            old_url?: string | null;
            page_id: components["schemas"]["PageId"];
            /** @description Whether the old URL keeps working through a redirect. */
            redirected: boolean;
            revision_id?: components["schemas"]["RevisionId"] | null;
            title: string;
        };
        /**
         * @description What changed about a page since the live release.
         * @enum {string}
         */
        ChangeKind: "new" | "edited" | "renamed" | "moved" | "header" | "deleted";
        CodeToken: {
            dark?: components["schemas"]["CodeTokenStyle"] | null;
            /** @description Offsets in UTF-16 code units, as JavaScript counts them. */
            from: number;
            light?: components["schemas"]["CodeTokenStyle"] | null;
            to: number;
        };
        CodeTokenStyle: {
            bold: boolean;
            color: string;
            italic: boolean;
        };
        CreatePage: {
            before?: components["schemas"]["PageId"] | null;
            /** @description Makes the new node an external link to this URL instead of a page. */
            external_url?: string | null;
            kind?: components["schemas"]["PageKind"] | null;
            parent_id?: components["schemas"]["PageId"] | null;
            title: string;
        };
        /** @description A new token with its secret, which is shown this once. */
        CreatedToken: {
            /** @description Sent as `Authorization: Bearer <secret>`. */
            secret: string;
            token: components["schemas"]["ApiToken"];
        };
        DescriptionBody: {
            /** @description Up to 300 characters; empty clears it. */
            description: string;
        };
        /**
         * @description A page body: the ProseMirror JSON of the editor, stored and rendered as is.
         *
         *     Block nodes carry a stable `attrs.id` (Tiptap UniqueID); internal links and components store page IDs and props in `attrs`.
         */
        Document: unknown;
        DraftResponse: {
            base_revision_id?: components["schemas"]["RevisionId"] | null;
            doc: components["schemas"]["Document"];
        };
        /** @description What open editors are told about, as server-sent events: one JSON object each, tagged by `type`. */
        EditorEvent: {
            page_id: components["schemas"]["PageId"];
            /** @enum {string} */
            type: "lock_changed";
        } | {
            from: components["schemas"]["UserId"];
            page_id: components["schemas"]["PageId"];
            /** @enum {string} */
            type: "lock_taken_over";
        } | {
            release_id: components["schemas"]["ReleaseId"];
            /** @enum {string} */
            type: "published";
        };
        /**
         * @description What kind of reusable content an entry is.
         * @enum {string}
         */
        EntryKind: "variable" | "snippet" | "term";
        /** @description The body of every error response. */
        ErrorBody: {
            details: string[];
            message: string;
        };
        Footer: {
            /** @default [] */
            columns: components["schemas"]["FooterColumn"][];
            /** @default  */
            text: string;
        };
        FooterColumn: {
            links: components["schemas"]["SiteLink"][];
            title: string;
        };
        /** @description Which version of a GitHub repository to read. */
        GitRef: {
            /** @enum {string} */
            type: "latest_release";
        } | {
            name: string;
            /** @enum {string} */
            type: "tag";
        } | {
            name: string;
            /** @enum {string} */
            type: "branch";
        };
        HeaderBody: {
            in_header: boolean;
        };
        HighlightRequest: {
            code: string;
            /** @description As written in the code block, like `rust` or `tsx`. */
            language: string;
        };
        Highlighted: {
            /** @description Runs of styled code, in order; text between them is plain. Empty for a language the highlighter does not know. */
            tokens: components["schemas"]["CodeToken"][];
        };
        HotfixPlan: {
            /** @description Links from the hotfix to pages it leaves out, which show as plain text. */
            plain_links: components["schemas"]["PlainLink"][];
            /** @description Pages the hotfix takes along: the new parent of a picked page, or a page left under one the hotfix deletes or moves. */
            pulled: components["schemas"]["Pulled"][];
        };
        Impact: {
            /** @description Pages outside the deleted ones that link to any of them. */
            linked_from: components["schemas"]["Backlink"][];
            /** @description How many pages go with it. */
            subpages: number;
            /** @description Glossary terms that lose their link to the full explanation. */
            terms: components["schemas"]["TermLink"][];
        };
        /**
         * @description Why a redirect is not written, if it is not: a page now lives at its path, its page is gone, or it is part of a loop.
         * @enum {string}
         */
        Inactive: "page_at_path" | "target_gone" | "loop";
        /** @description What one run did. */
        JobsReport: {
            /** @description Uploads that no draft, setting, published release or recent version used any more. */
            assets_deleted: number;
            /** @description Stored files deleted: those of the uploads above, and files no upload owns. */
            files_deleted: number;
            /** @description The pages whose review date has come, the most overdue first. */
            stale_pages: components["schemas"]["StalePage"][];
        };
        /** @description A variable, snippet or glossary term of the library, with the pages that use it. */
        LibraryEntry: {
            /** @description Other words for a term. */
            aliases: string[];
            doc?: components["schemas"]["Document"] | null;
            id: string;
            key: string;
            kind: components["schemas"]["EntryKind"];
            page_id?: components["schemas"]["PageId"] | null;
            /** @description The value of a variable, or the definition of a term as plain text. */
            text_value?: string | null;
            used_in: components["schemas"]["UsedBy"][];
        };
        /**
         * @description How a page refers to another.
         * @enum {string}
         */
        LinkKind: "inline" | "card" | "prop";
        LiveInfo: {
            published_by: string;
            release: components["schemas"]["Release"];
        };
        /** @description What a page's lock holder sends with each save and renewal; a new one is made whenever the lock changes hands. */
        LockToken: string;
        /** @description A lock as the editor shows it. */
        LockView: {
            /** Format: date-time */
            acquired_at: string;
            /**
             * Format: date-time
             * @description When the holder last typed or clicked in the page.
             */
            active_at: string;
            /** Format: date-time */
            expires_at: string;
            /** @description The holder has done nothing for a while: taking over does not need forcing. */
            idle: boolean;
            /**
             * Format: date-time
             * @description When the page's draft was last saved, by anyone.
             */
            last_saved_at?: string | null;
            /** @description Held by the user asking. */
            mine: boolean;
            page_id: components["schemas"]["PageId"];
            /** @description The page's title now; empty when the page is gone. */
            page_title: string;
            /**
             * Format: date-time
             * @description When the holder's editor last renewed the lock: it was open and in use then.
             */
            renewed_at: string;
            user_id: components["schemas"]["UserId"];
            user_name: string;
        };
        /** @description A URL that works on the live site and will not after publishing. */
        LostUrl: {
            path: string;
            title: string;
        };
        /** @description How the specs combine. Conflicts are build errors; a source's prefix is how they get resolved. */
        MergeRules: {
            /** @default null */
            description: string | null;
            /**
             * @description Replaces the `info` of the first source when set.
             * @default null
             */
            title: string | null;
            /** @default null */
            version: string | null;
        };
        /** @description Optional color overrides for one mode, each `#rrggbb`. What is not set keeps the built-in color. */
        ModeColors: {
            /** @default null */
            background: string | null;
            /** @default null */
            code_background: string | null;
            /** @default null */
            link: string | null;
            /** @default null */
            surface: string | null;
            /** @default null */
            text: string | null;
        };
        MovePage: {
            before?: components["schemas"]["PageId"] | null;
            parent_id?: components["schemas"]["PageId"] | null;
            /** @description Keep the old URLs of the page and its subtree working with redirects. Defaults to true. */
            redirect?: boolean | null;
        };
        NewAlias: {
            path: string;
        };
        NewRedirect: {
            from_path: string;
            to_page_id?: components["schemas"]["PageId"] | null;
            /** @description ...or to this URL or path. */
            to_url?: string | null;
        };
        NewToken: {
            name: string;
            role?: components["schemas"]["Role"] | null;
        };
        /** @description An operation of a merged spec, as the page's fallback, the search index and the pre-publish checks read it. */
        Operation: {
            description?: string | null;
            /** @description Upper case: `GET`. */
            method: string;
            operation_id?: string | null;
            path: string;
            summary?: string | null;
            tags: string[];
        };
        /** @description An operation a link can point at. */
        OperationLink: {
            /** @description What a link stores, and what follows `#` in its address. */
            anchor: string;
            method: string;
            path: string;
            summary?: string | null;
            tags: string[];
        };
        /** @description A node of the page tree. */
        Page: {
            /** Format: date-time */
            created_at: string;
            created_by: components["schemas"]["UserId"];
            /** @description A short summary, shown on link cards and in the page's meta description. */
            description?: string | null;
            /** @description Where a link goes; always `None` for pages. */
            external_url?: string | null;
            /** @description Orders the header entries (fractional index, like `position`). */
            header_position?: string | null;
            id: components["schemas"]["PageId"];
            /** @description Shown as an entry of the site header; a page with children also gets a sidebar of its own. */
            in_header?: boolean;
            kind: components["schemas"]["PageKind"];
            parent_id?: components["schemas"]["PageId"] | null;
            /** @description A fixed URL that replaces the one derived from the tree (`/webhooks/`); it stays put when the page moves. */
            path_override?: string | null;
            /** @description Fractional index; siblings sort by this string. */
            position: string;
            /** @description Lowercase, unique among siblings. */
            slug: string;
            title: string;
            /** Format: date-time */
            updated_at: string;
        };
        /** @description Stable identity of a page; URLs derive from the tree, never from this. */
        PageId: string;
        /**
         * @description What a tree node is: a page of the site or a link to somewhere else.
         * @enum {string}
         */
        PageKind: "page" | "link" | "api";
        /** @description Who looks after a page and when it should be checked again. Not content: it is saved at once and never published. */
        PageReview: {
            owner_id?: components["schemas"]["UserId"] | null;
            /**
             * Format: date
             * @description The day the page should be checked by, `YYYY-MM-DD`.
             */
            review_by?: string | null;
        };
        /** @description Where a page lives and which old URLs lead to it. */
        PageUrl: {
            aliases: components["schemas"]["Alias"][];
            custom_url?: string | null;
            /** @description The URL it would have without a custom one. */
            derived_url: string;
            /** @description The URL the page has now. */
            url: string;
        };
        Pending: {
            /** @description Links that would point at a deleted page: they render as plain text. */
            broken_links: components["schemas"]["BrokenLink"][];
            changes: components["schemas"]["Change"][];
            lost_urls: components["schemas"]["LostUrl"][];
            /** @description The site settings (name, logos, theme, header links, footer, custom CSS) differ from the live release's. */
            settings: boolean;
        };
        PlainLink: {
            source_title: string;
            target_id: components["schemas"]["PageId"];
            target_title: string;
        };
        PlanRequest: {
            pages: components["schemas"]["PageId"][];
        };
        /** @description A problem that stops a publish, with the page it is on. */
        Problem: {
            kind: components["schemas"]["ProblemKind"];
            message: string;
            page_id: components["schemas"]["PageId"];
            page_title: string;
        };
        /**
         * @description What a pre-publish check found wrong.
         * @enum {string}
         */
        ProblemKind: "broken_link" | "invalid_props" | "broken_operation_link" | "duplicate_url" | "missing_alt";
        PublishProgress: {
            /**
             * Format: int64
             * @description Files sent so far.
             */
            sent: number;
            /**
             * Format: int64
             * @description Files this build sends: those the target does not have yet. Unchanged files are not counted.
             */
            total: number;
        };
        PublishRequest: {
            /** @description Publish only these pages (a hotfix), with what they need; every other page keeps its published version. Everything when absent. */
            pages?: components["schemas"]["PageId"][] | null;
        };
        /**
         * @description Why a page that was not picked goes out with a hotfix.
         * @enum {string}
         */
        PullReason: "parent" | "orphan";
        /** @description A page the hotfix takes along without being asked. */
        Pulled: {
            page_id: components["schemas"]["PageId"];
            reason: components["schemas"]["PullReason"];
            title: string;
        };
        /** @description A redirect as the Settings page shows it. */
        RedirectEntry: {
            from_path: string;
            inactive?: components["schemas"]["Inactive"] | null;
            origin: components["schemas"]["RedirectOrigin"];
            /** @description Where it leads now, after following chains; absent when it is not written. */
            to?: string | null;
            /** @description Title of the page it leads to, when it leads to a page. */
            to_page_title?: string | null;
        };
        /**
         * @description Why a redirect exists.
         * @enum {string}
         */
        RedirectOrigin: "move" | "rename" | "manual";
        /** @description An immutable published state of the whole site. */
        Release: {
            base_release_id?: components["schemas"]["ReleaseId"] | null;
            /** Format: int64 */
            bytes?: number | null;
            /** Format: date-time */
            created_at: string;
            created_by: components["schemas"]["UserId"];
            /**
             * Format: int64
             * @description How many files the release sent to its target, and their total size; unknown for releases made before they were recorded.
             */
            file_count?: number | null;
            id: components["schemas"]["ReleaseId"];
            /**
             * Format: int32
             * @description How many pages the release contains.
             */
            page_count: number;
            /** @description Where a preview can be looked at, when its target gives it an address of its own. */
            preview_url?: string | null;
            status: components["schemas"]["ReleaseStatus"];
        };
        /** @description An immutable published state of the whole site. */
        ReleaseId: string;
        ReleaseRow: {
            /** @description Whether it can be rolled back to now: it was the site and is not any more. */
            can_roll_back: boolean;
            /** @description Titles of the pages that changed compared with the release it was built on: edited, new or removed. */
            changes: string[];
            /** @description Whether rolling back gives back exactly the files that were published, rather than rendering the release again from its content. */
            exact: boolean;
            /**
             * Format: int32
             * @description 1 for the first release that went live; stays with its release.
             */
            number: number;
            /** @description The name of whoever published it. */
            published_by: string;
            release: components["schemas"]["Release"];
        };
        /** @enum {string} */
        ReleaseStatus: "building" | "preview" | "live" | "superseded" | "failed";
        RenamePage: {
            /** @description The new target of a link; refused for pages. */
            external_url?: string | null;
            /** @description Keep the old URL working with a redirect. Defaults to true. */
            redirect?: boolean | null;
            title: string;
        };
        RenewLock: {
            /**
             * Format: int64
             * @description How long ago, in milliseconds, the holder last typed or clicked in the editor.
             */
            idle_ms?: number;
            token: components["schemas"]["LockToken"];
        };
        /** @description Where one spec was read from at a build, as recorded in the release. */
        ResolvedSource: {
            etag?: string | null;
            /** @description The source as written: a URL, or `owner/name` and the file or asset. */
            label: string;
            /** @description The release tag a GitHub source was read at. */
            tag?: string | null;
            /** @description The address the file came from. */
            url: string;
        };
        Restore: {
            /** @description Also bring back the description the page had then. */
            description?: boolean;
            revision_id: components["schemas"]["RevisionId"];
            /** @description Also bring back the title the page had then. Its URL stays. */
            title?: boolean;
        };
        Restored: {
            /** @description False when the old parent was gone or the old URL segment taken, so the page came back somewhere else or under another URL. */
            old_place_kept: boolean;
            page: components["schemas"]["Page"];
        };
        RestoredDraft: {
            base_revision_id?: components["schemas"]["RevisionId"] | null;
        };
        /** @description An immutable snapshot of a page's document. */
        RevisionId: string;
        /**
         * @description What a user may do. Each role can do everything the ones before it can, so roles compare in that order.
         * @enum {string}
         */
        Role: "viewer" | "editor" | "publisher" | "admin";
        SaveDraft: {
            base_revision_id?: components["schemas"]["RevisionId"] | null;
            doc: components["schemas"]["Document"];
            /** @description The token of the page's lock, which the saver must hold. */
            lock_token: components["schemas"]["LockToken"];
        };
        SetRole: {
            role: components["schemas"]["Role"];
        };
        SetUrl: {
            /** @description The fixed URL, or null to use the one derived from the tree. */
            custom_url?: string | null;
            /** @description Keep the old URLs of the page and the pages below it as redirects. Defaults to true. */
            redirect?: boolean | null;
        };
        /** @description The site-wide settings: name, logo, colors and the links around the page. */
        Settings: {
            /**
             * @description `#rrggbb`.
             * @default #0b6e4f
             */
            accent: string;
            /**
             * @default {
             *       "text": "",
             *       "url": ""
             *     }
             */
            banner: components["schemas"]["Banner"];
            /**
             * @description A stylesheet written after the theme, on the published site and in the editor.
             * @default
             */
            custom_css: string;
            /**
             * @default {
             *       "background": null,
             *       "code_background": null,
             *       "link": null,
             *       "surface": null,
             *       "text": null
             *     }
             */
            dark: components["schemas"]["ModeColors"];
            /**
             * @default {
             *       "columns": [],
             *       "text": ""
             *     }
             */
            footer: components["schemas"]["Footer"];
            /**
             * @description External links shown in the header, after the sections.
             * @default []
             */
            header_links: components["schemas"]["SiteLink"][];
            /**
             * @default {
             *       "background": null,
             *       "code_background": null,
             *       "link": null,
             *       "surface": null,
             *       "text": null
             *     }
             */
            light: components["schemas"]["ModeColors"];
            /** @default null */
            logo_asset_id: components["schemas"]["AssetId"] | null;
            /** @default null */
            logo_dark_asset_id: components["schemas"]["AssetId"] | null;
            /** @default Documentation */
            site_name: string;
        };
        SiteCapabilities: {
            /**
             * Format: int64
             * @description The most files a release may hold on this target.
             */
            max_files?: number | null;
            /** @description Whether a build can be viewed before it goes live. */
            preview: boolean;
            /** @description Whether that build can then be made live exactly as it is. */
            promote: boolean;
        };
        SiteInfo: {
            /** @description What the configured publish target can do; the editor hides what it cannot. */
            capabilities: components["schemas"]["SiteCapabilities"];
            live?: components["schemas"]["LiveInfo"] | null;
            /** @description What the editor calls the publish target ("Cloudflare Pages"). */
            target: string;
            /** @description Where readers find the published site, without a trailing slash. */
            url: string;
        };
        SiteLink: {
            label: string;
            /** @description `https://…`, `mailto:…` or a path on the site. */
            url: string;
        };
        SnippetBody: {
            doc: components["schemas"]["Document"];
        };
        /** @description A page whose review date has come. */
        StalePage: {
            owner?: components["schemas"]["User"] | null;
            page_id: components["schemas"]["PageId"];
            /** Format: date */
            review_by: string;
            title: string;
        };
        TakeOver: {
            /** @description Take the page even though its holder is still active. */
            force?: boolean;
        };
        TermBody: {
            aliases?: string[];
            /** @description Paragraphs of text with inline marks, at most 400 characters. */
            definition: components["schemas"]["Document"];
            page_id?: components["schemas"]["PageId"] | null;
            term: string;
        };
        /** @description A glossary term whose full explanation is the page. */
        TermLink: {
            /** @description The page it points to (one of the pages below the one asked about, for a delete). */
            target_title: string;
            term: string;
        };
        /** @description What the settings form shows while the palette is being edited. */
        ThemeCheck: {
            dark: {
                [key: string]: string;
            };
            /** @description The custom properties each mode gets, for the live preview. */
            light: {
                [key: string]: string;
            };
            /** @description Text pairs under WCAG AA; saving is refused while there are any. */
            problems: string[];
        };
        /** @description A deleted page waiting in the trash. */
        TrashedPage: {
            /** Format: date-time */
            deleted_at: string;
            id: components["schemas"]["PageId"];
            /** @description The URL it had. */
            old_url: string;
            /** @description How many pages were deleted with it. */
            subpages: number;
            title: string;
        };
        UsedBy: {
            page_id: components["schemas"]["PageId"];
            title: string;
        };
        /** @description Someone who signed in through an identity provider. */
        User: {
            email: string;
            id: components["schemas"]["UserId"];
            /**
             * Format: date-time
             * @description None for `dev`'s user, who never signs in.
             */
            last_login_at?: string | null;
            name: string;
            role: components["schemas"]["Role"];
        };
        /** @description A user, including the built-in local one. */
        UserId: string;
        VariableBody: {
            value: string;
        };
        /** @description One saved version of a page. */
        Version: {
            author_id: components["schemas"]["UserId"];
            /** Format: date-time */
            created_at: string;
            id: components["schemas"]["RevisionId"];
            /**
             * Format: date-time
             * @description When the live site last got this version, if it ever did.
             */
            published_at?: string | null;
            title: string;
        };
        VersionContent: {
            author_id: components["schemas"]["UserId"];
            author_name: string;
            /** Format: date-time */
            created_at: string;
            /** @description The page's description when this version was saved. */
            description?: string | null;
            /** @description False for versions saved before descriptions were recorded, which cannot restore one. */
            description_recorded: boolean;
            doc: components["schemas"]["Document"];
            title: string;
        };
    };
    responses: never;
    parameters: never;
    requestBodies: never;
    headers: never;
    pathItems: never;
}
export type $defs = Record<string, never>;
export interface operations {
    uploadAsset: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/octet-stream": string;
            };
        };
        responses: {
            /** @description the same file was already stored */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Asset"];
                };
            };
            /** @description stored as a new asset */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Asset"];
                };
            };
            /** @description the file is too large */
            413: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description not a supported image, too many pixels, or damaged */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    editorEvents: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/event-stream": components["schemas"]["EditorEvent"];
                };
            };
        };
    };
    highlightCode: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["HighlightRequest"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Highlighted"];
                };
            };
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    runJobs: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["JobsReport"];
                };
            };
            /** @description not signed in and no API token */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    listLibrary: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LibraryEntry"][];
                };
            };
        };
    };
    saveSnippet: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                key: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SnippetBody"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LibraryEntry"][];
                };
            };
            /** @description not a usable name, or the snippet would include itself */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    saveTerm: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["TermBody"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LibraryEntry"][];
                };
            };
            /** @description empty or too long, an alias another term has, or a linked page that does not exist */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    saveVariable: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                key: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["VariableBody"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LibraryEntry"][];
                };
            };
            /** @description not a usable name */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    deleteLibraryEntry: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LibraryEntry"][];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description pages or snippets still use it */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    listLocks: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LockView"][];
                };
            };
        };
    };
    getMe: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["User"];
                };
            };
            /** @description not signed in */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    listPages: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page"][];
                };
            };
        };
    };
    createPage: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreatePage"];
            };
        };
        responses: {
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page"];
                };
            };
            /** @description unknown parent */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description a sibling has the same URL segment */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    renamePage: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["PageId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RenamePage"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description a sibling has the same URL segment */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    deletePage: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["PageId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description someone else is editing the page or a page below it */
            423: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    addAlias: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["PageId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["NewAlias"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PageUrl"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description the path belongs to another page or alias; the message names it */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description not a usable path */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    removeAlias: {
        parameters: {
            query: {
                path: string;
            };
            header?: never;
            path: {
                id: components["schemas"]["PageId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PageUrl"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    getApiConfig: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["PageId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiConfig"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description not an API reference page */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    saveApiConfig: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["PageId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ApiConfig"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiConfig"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    checkApiConfig: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["PageId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ApiConfig"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiCheck"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description a check of this page is already running */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    getApiSpec: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["PageId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": string;
                };
            };
            /** @description nothing was merged yet */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    getBacklinks: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["PageId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Backlinks"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    setPageDescription: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["PageId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DescriptionBody"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description too long */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    getDraft: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["PageId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DraftResponse"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    saveDraft: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["PageId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SaveDraft"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DraftResponse"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description the page changed since the revision this save was based on */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description the lock was released */
            410: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description someone else is editing the page */
            423: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    setPageHeader: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["PageId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["HeaderBody"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description links cannot be shown in the header */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    getHistory: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["PageId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Version"][];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    getDeleteImpact: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["PageId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Impact"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    getLock: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["PageId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LockView"] | null;
                };
            };
        };
    };
    renewLock: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["PageId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RenewLock"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LockView"];
                };
            };
            /** @description the lock was released */
            410: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description someone else is editing the page */
            423: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    acquireLock: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["PageId"];
            };
            cookie?: never;
        };
        requestBody?: {
            content: {
                "application/json": components["schemas"]["AcquireLock"] | null;
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AcquiredLock"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description someone else is editing the page */
            423: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    releaseLock: {
        parameters: {
            query: {
                token: components["schemas"]["LockToken"];
            };
            header?: never;
            path: {
                id: components["schemas"]["PageId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    takeOverLock: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["PageId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["TakeOver"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AcquiredLock"];
                };
            };
            /** @description a viewer, or `force` without the admin role */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description the holder is still active */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    movePage: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["PageId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["MovePage"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Page"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description a sibling has the same URL segment */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description the target is inside the page's own subtree */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    listApiOperations: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["PageId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["OperationLink"][];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    restoreRevision: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["PageId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Restore"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RestoredDraft"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description the description was asked for, and the version was saved before descriptions were recorded */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description someone else is editing the page */
            423: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    getPageReview: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["PageId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PageReview"];
                };
            };
        };
    };
    setPageReview: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["PageId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["PageReview"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PageReview"];
                };
            };
            /** @description no such page, or it is in the trash */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    getPageUrl: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["PageId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PageUrl"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    setPageUrl: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["PageId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SetUrl"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PageUrl"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description the URL belongs to another page or alias; the message names it */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description not a usable path */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    publish: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: {
            content: {
                "application/json": components["schemas"]["PublishRequest"] | null;
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Release"];
                };
            };
            /** @description the checks found problems or a page could not be rendered; nothing was published; the details name each page */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    pendingChanges: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Pending"];
                };
            };
        };
    };
    publishChecks: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Problem"][];
                };
            };
        };
    };
    hotfixPlan: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["PlanRequest"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["HotfixPlan"];
                };
            };
        };
    };
    publishPreview: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Release"];
                };
            };
            /** @description the target has no previews, the checks found problems, or the target would refuse the site */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    publishProgress: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/event-stream": components["schemas"]["PublishProgress"];
                };
            };
        };
    };
    listRedirects: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RedirectEntry"][];
                };
            };
        };
    };
    addRedirect: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["NewRedirect"];
            };
        };
        responses: {
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RedirectEntry"][];
                };
            };
            /** @description the path belongs to a page or another redirect; the message names it */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description not a usable path or target, or it would make a loop */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    removeRedirect: {
        parameters: {
            query: {
                path: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["RedirectEntry"][];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    listReleases: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ReleaseRow"][];
                };
            };
        };
    };
    promoteRelease: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["ReleaseId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Release"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description the site was published after the preview was made */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description the release is not a preview */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    rollbackRelease: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["ReleaseId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description the release that is the site now: the one asked for, or a new one rendered from its content */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Release"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description the release is the live site already, or never was the site */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    stalePages: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["StalePage"][];
                };
            };
        };
    };
    getRevision: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["RevisionId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["VersionContent"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    getScalarScript: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/javascript": string;
                };
            };
        };
    };
    getSettings: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Settings"];
                };
            };
        };
    };
    saveSettings: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Settings"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Settings"];
                };
            };
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    checkSettings: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Settings"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ThemeCheck"];
                };
            };
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    siteInfo: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SiteInfo"];
                };
            };
        };
    };
    getThemeCss: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/css": string;
                };
            };
        };
    };
    listTokens: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiToken"][];
                };
            };
            /** @description called with an API token */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    createToken: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["NewToken"];
            };
        };
        responses: {
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CreatedToken"];
                };
            };
            /** @description called with an API token, in `dev`, or with a role above yours */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description no name */
            422: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    revokeToken: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description deleted */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    listTrash: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TrashedPage"][];
                };
            };
        };
    };
    restoreFromTrash: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["PageId"];
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Restored"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
    listUsers: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["User"][];
                };
            };
        };
    };
    setUserRole: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: components["schemas"]["UserId"];
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SetRole"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["User"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
            /** @description it would leave the site without an admin */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorBody"];
                };
            };
        };
    };
}
