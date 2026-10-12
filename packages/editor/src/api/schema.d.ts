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
        /** Deletes a page and its descendants. */
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
    "/api/pages/{id}/draft": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** The latest saved edits of a page (an empty document if it has none yet). */
        get: operations["getDraft"];
        /** Saves a draft. Refused with 409 when `base_revision_id` is not the page's latest revision. */
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
        /** Publishes the latest saved draft of every page as a new release. */
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
}
export type webhooks = Record<string, never>;
export interface components {
    schemas: {
        Alias: {
            /** Format: date-time */
            created_at: string;
            origin: components["schemas"]["RedirectOrigin"];
            path: string;
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
            /** @description Hex SHA-256 of the file: two uploads of the same bytes are one asset. */
            sha256: string;
            uploaded_by: components["schemas"]["UserId"];
            /** Format: int32 */
            width?: number | null;
        };
        /** @description An uploaded file. */
        AssetId: string;
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
            /** @description The URL after publishing, for pages that still exist. */
            new_url?: string | null;
            /** @description The URL in the live release, for pages that were already published. */
            old_url?: string | null;
            page_id: components["schemas"]["PageId"];
            /** @description Whether the old URL keeps working through a redirect. */
            redirected: boolean;
            title: string;
        };
        /**
         * @description What changed about a page since the live release.
         * @enum {string}
         */
        ChangeKind: "new" | "edited" | "renamed" | "moved" | "header" | "deleted";
        CreatePage: {
            before?: components["schemas"]["PageId"] | null;
            /** @description Makes the new node an external link to this URL instead of a page. */
            external_url?: string | null;
            parent_id?: components["schemas"]["PageId"] | null;
            title: string;
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
        /** @description The body of every error response. */
        ErrorBody: {
            details: string[];
            message: string;
        };
        HeaderBody: {
            in_header: boolean;
        };
        Impact: {
            /** @description Pages outside the deleted ones that link to any of them. */
            linked_from: components["schemas"]["Backlink"][];
            /** @description How many pages go with it. */
            subpages: number;
        };
        /**
         * @description Why a redirect is not written, if it is not: a page now lives at its path, its page is gone, or it is part of a loop.
         * @enum {string}
         */
        Inactive: "page_at_path" | "target_gone" | "loop";
        /**
         * @description How a page refers to another.
         * @enum {string}
         */
        LinkKind: "inline" | "card" | "prop";
        /** @description A URL that works on the live site and will not after publishing. */
        LostUrl: {
            path: string;
            title: string;
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
        /** @description A node of the page tree. */
        Page: {
            /** Format: date-time */
            created_at: string;
            created_by: components["schemas"]["UserId"];
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
        PageKind: "page" | "link";
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
            /** Format: date-time */
            created_at: string;
            created_by: components["schemas"]["UserId"];
            id: components["schemas"]["ReleaseId"];
            /**
             * Format: int32
             * @description How many pages the release contains.
             */
            page_count: number;
            status: components["schemas"]["ReleaseStatus"];
        };
        /** @description An immutable published state of the whole site. */
        ReleaseId: string;
        /** @enum {string} */
        ReleaseStatus: "building" | "live" | "failed";
        RenamePage: {
            /** @description The new target of a link; refused for pages. */
            external_url?: string | null;
            /** @description Keep the old URL working with a redirect. Defaults to true. */
            redirect?: boolean | null;
            title: string;
        };
        /** @description An immutable snapshot of a page's document. */
        RevisionId: string;
        SaveDraft: {
            base_revision_id?: components["schemas"]["RevisionId"] | null;
            doc: components["schemas"]["Document"];
        };
        SetUrl: {
            /** @description The fixed URL, or null to use the one derived from the tree. */
            custom_url?: string | null;
            /** @description Keep the old URLs of the page and the pages below it as redirects. Defaults to true. */
            redirect?: boolean | null;
        };
        /** @description The site-wide settings of the theme basics. */
        Settings: {
            /** @description `#rrggbb`. */
            accent: string;
            logo_asset_id?: components["schemas"]["AssetId"] | null;
            site_name: string;
        };
        /** @description A user, including the built-in local one. */
        UserId: string;
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
                content?: never;
            };
            /** @description not a supported image */
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
            /** @description a page could not be rendered; nothing was published */
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
}
