import { execFileSync } from "node:child_process";
import { readFile, writeFile } from "node:fs/promises";
// Exact SPDX expressions reviewed for the locked graph. An unreviewed expression fails CI.
// Select MIT/Apache alternatives, including for r-efi's optional LGPL alternative.
const approved = new Set([
  "(MIT OR Apache-2.0)",
  "(MIT OR Apache-2.0) AND NCSA",
  "BlueOak-1.0.0",
  "MIT",
  "ISC",
  "Apache-2.0",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "0BSD",
  "CC0-1.0",
  "CC-BY-4.0",
  "OFL-1.1",
  "Unicode-3.0",
  "Unlicense",
  "MIT OR Apache-2.0",
  "Apache-2.0 OR MIT",
  "MIT/Apache-2.0",
  "MIT OR Apache-2.0 OR BSD-1-Clause",
  "MIT OR Apache-2.0 OR LGPL-2.1-or-later",
  "Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT",
  "(MIT OR Apache-2.0) AND Unicode-3.0",
  "Unlicense OR MIT",
  "Zlib OR Apache-2.0 OR MIT",
  "(MIT OR CC0-1.0)",
]);
const cargo = JSON.parse(
  execFileSync("cargo", ["metadata", "--locked", "--format-version", "1"], {
    encoding: "utf8",
    maxBuffer: 10000000,
  }),
);
const fuzz = JSON.parse(
  execFileSync(
    "cargo",
    [
      "metadata",
      "--manifest-path",
      "validation/fuzz/Cargo.toml",
      "--locked",
      "--format-version",
      "1",
    ],
    { encoding: "utf8", maxBuffer: 10000000 },
  ),
);
const base = new Set(cargo.packages.map((p) => `${p.name}@${p.version}`));
const additions = fuzz.packages.filter(
  (p) => !base.has(`${p.name}@${p.version}`),
);
const lock = JSON.parse(await readFile("package-lock.json", "utf8"));
const dependencies = [
  ...additions.map((p) => ({
    ecosystem: "cargo-validation",
    name: p.name,
    version: p.version,
    license: p.license,
  })),
  ...cargo.packages.map((p) => ({
    ecosystem: "cargo",
    name: p.name,
    version: p.version,
    license: p.license,
  })),
  ...Object.entries(lock.packages)
    .filter(([p]) => p)
    .map(([p, v]) => ({
      ecosystem: "npm",
      name: p.replace(/^.*node_modules\//, ""),
      version: v.version,
      license: v.license,
    })),
];
const unknown = dependencies.filter((p) => !approved.has(p.license));
await writeFile(
  "docs/DEPENDENCIES.json",
  JSON.stringify(dependencies, null, 2) + "\n",
);
if (unknown.length) {
  console.error("License review required:", unknown);
  process.exit(1);
}
console.log(
  `Reviewed license expressions for ${dependencies.length} locked packages. No AGPL dependency integrated.`,
);
