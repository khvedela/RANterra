import { readFile, readdir, stat, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import assert from "node:assert/strict";
async function files(dir) {
  return (
    await Promise.all(
      (await readdir(dir)).map(async (name) => {
        const path = `${dir}/${name}`;
        return (await stat(path)).isDirectory() ? files(path) : [path];
      }),
    )
  ).flat();
}
const paths = (await files("dist")).sort();
assert(paths.some((p) => /engine\.worker-.*\.js$/.test(p)));
assert(paths.some((p) => p.endsWith(".wasm")));
const hashes = {};
for (const path of paths) {
  const b = await readFile(path);
  if (path.endsWith(".wasm"))
    assert.deepEqual([...b.subarray(0, 4)], [0, 97, 115, 109]);
  hashes[path] = createHash("sha256").update(b).digest("hex");
}
const manifest = JSON.stringify(hashes, null, 2) + "\n";
if (process.argv.includes("--compare"))
  assert.equal(
    manifest,
    await readFile("build-manifest.json", "utf8"),
    "Production output changed between builds",
  );
else await writeFile("build-manifest.json", manifest);
console.log(`Verified ${paths.length} production artifacts and WASM magic.`);
