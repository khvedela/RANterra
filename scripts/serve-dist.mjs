// Local verification server for static production output, including _headers.
import { createServer } from "node:http";
import { readFile, stat } from "node:fs/promises";
import { resolve, extname, sep } from "node:path";
const root = resolve("dist");
function pattern(path) {
  return new RegExp(
    "^" +
      path.replace(/[.+?^${}()|[\]\\]/g, "\\$&").replaceAll("*", ".*") +
      "$",
  );
}
const rewrites = (await readFile(resolve(root, "_redirects"), "utf8"))
  .split("\n")
  .filter((line) => line.trim() && !line.startsWith("#"))
  .map((line) => {
    const [from, to, status] = line.trim().split(/\s+/);
    if (status !== "200" || !to.startsWith("/"))
      throw Error(
        "The local verification server supports origin-relative 200 rewrites only.",
      );
    return { from: pattern(from), to };
  });
const rules = [];
let current;
for (const line of (await readFile(resolve(root, "_headers"), "utf8")).split(
  "\n",
)) {
  if (line.startsWith("/")) {
    current = { path: line, headers: {} };
    rules.push(current);
  } else if (current && line.startsWith("  ")) {
    const i = line.indexOf(":");
    current.headers[line.slice(0, i).trim()] = line.slice(i + 1).trim();
  }
}
const mime = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript",
  ".css": "text/css",
  ".json": "application/json",
  ".wasm": "application/wasm",
  ".woff": "font/woff",
  ".woff2": "font/woff2",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".svg": "image/svg+xml",
  ".txt": "text/plain; charset=utf-8",
};
const server = createServer(async (req, res) => {
  try {
    const path = decodeURIComponent(
      new URL(req.url, "http://localhost").pathname,
    );
    const rewrite = rewrites.find((rule) => rule.from.test(path));
    let file = resolve(root, "." + (rewrite?.to ?? path));
    let status = 200;
    if (!file.startsWith(root + sep) && file !== root) {
      res.writeHead(403).end();
      return;
    }
    try {
      if ((await stat(file)).isDirectory()) file = resolve(file, "index.html");
    } catch {
      file = resolve(root, "404.html");
      status = 404;
    }
    const headers = {
      "Content-Type": mime[extname(file)] ?? "application/octet-stream",
    };
    const custom = {};
    for (const r of rules) {
      // Pages combines matching _headers rules rather than using last-write-wins.
      // Keep these separate from the server's MIME defaults, which are overridden.
      if (pattern(r.path).test(path)) {
        for (const [name, value] of Object.entries(r.headers)) {
          custom[name] = custom[name] ? `${custom[name]}, ${value}` : value;
        }
      }
    }
    Object.assign(headers, custom);
    const data = await readFile(file);
    res.writeHead(status, headers);
    res.end(req.method === "HEAD" ? undefined : data);
  } catch {
    res.writeHead(500).end("Static asset error");
  }
});
server.listen(4173, "127.0.0.1", () =>
  console.log("Production static output at http://127.0.0.1:4173"),
);
