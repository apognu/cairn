import { expect, test } from "bun:test";
import { existsSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { buildComponents } from "./build";

const fixture = (name: string) => resolve(import.meta.dir, "../fixtures", name);
const outDir = () => mkdtempSync(join(tmpdir(), "cairn-bundles-"));

test("a folder of components becomes bundles and a manifest with schemas and the SDK version", async () => {
  const out = outDir();
  await buildComponents(resolve(import.meta.dir, "../../../examples/components"), out);

  for (const file of ["ssr.js", "client.js", "site.css", "manifest.json", "editor/index.html"])
    expect(existsSync(join(out, file)), file).toBe(true);

  const manifest = JSON.parse(readFileSync(join(out, "manifest.json"), "utf8"));
  expect(manifest.sdk_version).toBe(1);
  expect(manifest.slots).toEqual(["Footer"]);
  expect(manifest.components).toHaveLength(1);
  expect(manifest.components[0]).toMatchObject({
    name: "acme/note",
    version: 1,
    schema: { properties: { title: { type: "string" } } },
  });
  // The component's stylesheet is part of the site's.
  expect(readFileSync(join(out, "site.css"), "utf8")).toContain("acme-note");
}, 120_000);

test("a component that cannot render fails the build and is named", async () => {
  await expect(buildComponents(fixture("broken"), outDir())).rejects.toThrow(
    /boom\.tsx: acme\/boom fails to render.*kaboom/s,
  );
}, 120_000);

test("a fixed color in a component's markup fails the build", async () => {
  await expect(buildComponents(fixture("colored"), outDir())).rejects.toThrow(
    /red\.tsx: acme\/red writes a fixed color/,
  );
}, 120_000);

test("a folder that holds anything but bundles is never emptied", async () => {
  const out = outDir();
  writeFileSync(join(out, "notes.txt"), "keep me");

  await expect(buildComponents(fixture("colored"), out)).rejects.toThrow(/not empty and holds no bundles/);
  expect(existsSync(join(out, "notes.txt"))).toBe(true);
});
