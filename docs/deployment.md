# Static production deployment

No emulation backend is required. This candidate has been tested from local production static output; no hosted deployment has been performed or authorized.

## Build prerequisites

Use Node.js 24.18.0, npm, Rust 1.97.1 (pinned in `rust-toolchain.toml`), the wasm32-unknown-unknown target and wasm-pack 0.15.0. Python 3 is used only to verify independent test fixtures.

```sh
rustup toolchain install 1.97.1 --component rustfmt --component clippy --target wasm32-unknown-unknown
cargo install wasm-pack --version 0.15.0 --locked
npm ci
npm run build
node scripts/verify-build.mjs
node scripts/serve-dist.mjs
```

Open http://127.0.0.1:4173 for the local production server. It serves the static `dist` directory with the included headers and MIME types. For development run `npm run build:wasm` then `npm run dev`. Never serve the source WASM module with a file:// URL.

## Provider configuration

Edit `public/config.json` before building, or the public `/config.json` on the host. Example with self-hosted assets and external geography disabled:

```json
{
  "tiles": "/tiles/{z}/{x}/{y}.png",
  "attribution": "Your tile attribution and applicable data license",
  "search": "",
  "overpass": "",
  "osmMap": ""
}
```

Empty `tiles` produces a usable map canvas with application overlays and no remote base map. Search uses a configured compatible geocoding endpoint; it is disabled by default. Coordinates can always be entered directly. Overpass is requested only through explicit building retrieval and bounded/cacheable requests. The OSM map API fallback is opt-in. No bulk tile download or PWA tile caching is implemented.

The demonstration configuration uses OSM Standard tiles for light interactive use and public Overpass for manual retrieval. Before public/high-volume use, choose a suitable paid/hosted or self-hosted provider. Follow the current [OSM tile policy](https://operations.osmfoundation.org/policies/tiles/), [Nominatim policy](https://operations.osmfoundation.org/policies/nominatim/), provider terms and [OSM attribution requirements](https://www.openstreetmap.org/copyright). Keep visible attribution; bundled OSM geometry remains ODbL data, separate from MIT code. Public infrastructure has no application SLA. Do not enable Nominatim autocomplete: this application uses submitted searches only.

Provider settings accept HTTPS or origin-relative URLs. Add any new external origins to the CSP in `public/_headers`. Browser tokens in public configuration are visible to everyone; restrict them by origin and never use secret server credentials. Geography failure must leave emulation usable.

## Cloudflare Pages / Netlify / equivalent

Set build command `npm run build` and output `dist`. Ensure the build image actually installs the pinned Rust target and wasm-pack (or build in CI and upload the resulting static artifact after approval). Include `_headers` and `_redirects`; equivalent hosts must translate those files into their own configuration. Host at the origin root unless Vite's base path and public configuration are adjusted and retested.

Hashed `/assets/*` should be immutable for one year; `/index.html` and `/config.json` must revalidate. Serve `.wasm` as `application/wasm`, worker `.js` as JavaScript, and route extensionless application paths to index.html. Missing assets must return errors rather than HTML. Do not cache an HTML error as a worker/WASM asset. HTTPS is required for a public deployment. A COOP/COEP shared-memory setup is not required for the single-threaded WASM engine.

Before a hosted release verify the actual response headers, deep-link refresh, worker/WASM requests, configured map attribution/provider access, registration, session traffic, storage and browser recovery. Local Playwright tests exercise these application/static-server behaviors with external geography disabled; they do not prove the hosting provider configuration. The previous remote GitHub Actions run failed at `npm ci` because its lockfile lacked peer dependencies. This tree fixes that defect and passes clean local installation, but its revised pipeline has not yet run remotely. A green run for the exact release commit and actual HTTPS hosting verification are still required.
