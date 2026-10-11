import { spawnSync } from "node:child_process";
import { homedir } from "node:os";
import { join, delimiter } from "node:path";
// Prefer rustup over a Homebrew rustc whose sysroot may not contain the WASM target.
const result = spawnSync(
  "wasm-pack",
  [
    "build",
    "engine",
    "--target",
    "web",
    "--out-dir",
    "../src/wasm",
    "--release",
  ],
  {
    stdio: "inherit",
    env: {
      ...process.env,
      PATH: join(homedir(), ".cargo", "bin") + delimiter + process.env.PATH,
    },
  },
);
process.exit(result.status ?? 1);
