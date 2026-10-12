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
        /** Creates a page under `parent_id` (or at the root). */
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
        /** Changes the title; the URL segment follows it. */
        post: operations["renamePage"];
        /** Deletes a page and its descendants. */
        delete: operations["deletePage"];
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
}
export type webhooks = Record<string, never>;
export interface components {
    schemas: {
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
        CreatePage: {
            before?: components["schemas"]["PageId"] | null;
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
        MovePage: {
            before?: components["schemas"]["PageId"] | null;
            parent_id?: components["schemas"]["PageId"] | null;
        };
        /** @description A node of the page tree. */
        Page: {
            /** Format: date-time */
            created_at: string;
            created_by: components["schemas"]["UserId"];
            id: components["schemas"]["PageId"];
            parent_id?: components["schemas"]["PageId"] | null;
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
        RenamePage: {
            title: string;
        };
        /** @description An immutable snapshot of a page's document. */
        RevisionId: string;
        SaveDraft: {
            base_revision_id?: components["schemas"]["RevisionId"] | null;
            doc: components["schemas"]["Document"];
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
}
