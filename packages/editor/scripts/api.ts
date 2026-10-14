// Keeps `src/api/openapi.json` and `src/api/schema.d.ts` in step with the routes of the Rust server.
// `write` regenerates them; `check` (run by `typecheck`) fails when they are stale.
import { spawnSync } from "node:child_process";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import openapiTS, { astToString } from "openapi-typescript";

const root = resolve(import.meta.dir, "../../..");
const specPath = resolve(import.meta.dir, "../src/api/openapi.json");
const typesPath = resolve(import.meta.dir, "../src/api/schema.d.ts");

const cargo = spawnSync("cargo", ["run", "--quiet", "-p", "cairn-cli", "--", "openapi"], {
  cwd: root,
  encoding: "utf8",
});
if (cargo.status !== 0) {
  console.error(cargo.stderr);
  process.exit(1);
}

const spec = cargo.stdout;
const types = astToString(await openapiTS(JSON.parse(spec)));
const read = (path: string) => (existsSync(path) ? readFileSync(path, "utf8") : "");

if (process.argv[2] === "write") {
  writeFileSync(specPath, spec);
  writeFileSync(typesPath, types);
} else if (read(specPath) !== spec || read(typesPath) !== types) {
  console.error(
    "The API client is out of date with the server routes. Run `bun run gen:api` in packages/editor and commit the result.",
  );
  process.exit(1);
}
