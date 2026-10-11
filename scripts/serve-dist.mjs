// Local verification server for static production output, including _headers.
import { createServer } from "node:http";
import { readFile, stat } from "node:fs/promises";
import { resolve, extname, sep } from "node:path";
const root = resolve("dist");
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
};
const server = createServer(async (req, res) => {
  try {
    const path = decodeURIComponent(
      new URL(req.url, "http://localhost").pathname,
    );
    let file = resolve(root, "." + path);
    if (!file.startsWith(root + sep) && file !== root) {
      res.writeHead(403).end();
      return;
    }
    try {
      if ((await stat(file)).isDirectory()) file = resolve(file, "index.html");
    } catch {
      if (extname(path)) {
        res.writeHead(404).end();
        return;
      }
      file = resolve(root, "index.html");
    }
    const headers = {
      "Content-Type": mime[extname(file)] ?? "application/octet-stream",
    };
    const custom = {};
    for (const r of rules) {
      const pattern = new RegExp(
        "^" +
          r.path.replace(/[.+?^${}()|[\]\\]/g, "\\$&").replaceAll("*", ".*") +
          "$",
      );
      // Pages combines matching _headers rules rather than using last-write-wins.
      // Keep these separate from the server's MIME defaults, which are overridden.
      if (pattern.test(path)) {
        for (const [name, value] of Object.entries(r.headers)) {
          custom[name] = custom[name] ? `${custom[name]}, ${value}` : value;
        }
      }
    }
    Object.assign(headers, custom);
    const data = await readFile(file);
    res.writeHead(200, headers);
    res.end(req.method === "HEAD" ? undefined : data);
  } catch {
    res.writeHead(500).end("Static asset error");
  }
});
server.listen(4173, "127.0.0.1", () =>
  console.log("Production static output at http://127.0.0.1:4173"),
);
