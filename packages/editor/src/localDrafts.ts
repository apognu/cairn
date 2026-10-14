import type { DocNode } from "@cairn/renderer";

/** A copy of a page's document kept in the browser, so unsaved typing survives a reload or a dropped connection. */
export interface LocalDraft {
  pageId: string;
  doc: DocNode;
  /** The server revision the document was edited from. */
  base: string | null;
  /** True while the server has not accepted this document yet. */
  dirty: boolean;
}

const STORE = "drafts";

function open(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open("cairn", 1);
    request.onupgradeneeded = () => request.result.createObjectStore(STORE, { keyPath: "pageId" });
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

async function run<T>(mode: IDBTransactionMode, action: (store: IDBObjectStore) => IDBRequest<T>): Promise<T> {
  const db = await open();

  try {
    return await new Promise<T>((resolve, reject) => {
      const request = action(db.transaction(STORE, mode).objectStore(STORE));
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
  } finally {
    db.close();
  }
}

// IndexedDB can be missing or refuse writes (private windows, blocked storage): the server save still works, so failures are ignored.

export async function readLocal(pageId: string): Promise<LocalDraft | undefined> {
  try {
    return await run("readonly", (store) => store.get(pageId));
  } catch {
    return undefined;
  }
}

export async function writeLocal(draft: LocalDraft): Promise<void> {
  try {
    await run("readwrite", (store) => store.put(draft));
  } catch {
    // see above
  }
}

export async function discardLocal(pageId: string): Promise<void> {
  try {
    await run("readwrite", (store) => store.delete(pageId));
  } catch {
    // see above
  }
}
