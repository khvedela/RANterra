# Static production deployment

No emulation backend is required. For the zero-server-budget recommendation and alternatives, see [free hosting options](hosting-options.md). Cloudflare deployment was authorized on 2026-10-11. The verified routing candidate is live at https://ranterra.pages.dev; all 30 existing hosted browser checks passed. A map-resize correction is being validated before replacing it. `ranterra.davidkh.com` is associated with Pages and awaits its CNAME and certificate activation. Release publication remains on hold.

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

Hashed `/assets/*` should be immutable for one year; `/index.html` and `/config.json` must revalidate. Serve `.wasm` as `application/wasm`, worker `.js` as JavaScript, and serve the workspace at `/` and `/workspace`, and rewrite `/experiment/*` to the root document. Missing assets must return errors rather than HTML. Do not cache an HTML error as a worker/WASM asset. HTTPS is required for a public deployment. A COOP/COEP shared-memory setup is not required for the single-threaded WASM engine.

Before a hosted release verify the actual response headers, deep-link refresh, worker/WASM requests, configured map attribution/provider access, registration, session traffic, storage and browser recovery. Local Playwright tests exercise these application/static-server behaviors with external geography disabled; they do not prove the hosting provider configuration. The preceding Ant candidate `5f13d43` passed [both remote CI jobs](https://github.com/khvedela/RANterra/actions/runs/38156308147), including production browser tests under Xvfb. The earlier peer-lockfile and Firefox startup issues are documented in the release report. Actual HTTPS hosting verification remains required. Use the verified artifact rather than treating local header/route behavior as a complete host emulator.

## Cloudflare routing correction and hosted validation

Wrangler 4.149.0 rejected the former catch-all `/* /index.html 200` as a redirect loop and returned HTML with HTTP 200 for a missing JavaScript asset. The prior local verification server did not reproduce that behavior. The candidate now includes `404.html` and two explicit root-document rewrites for `/workspace` and `/experiment/*`; unknown paths and missing assets return HTTP 404. No Pages Functions, Worker backend or paid bindings are needed. The local verifier reads the same rewrite file rather than inventing a general SPA fallback.

Use `RANTERRA_TEST_ORIGIN` to run the existing real-engine production browser suite against an already-running Pages runtime or a deployed site. External geography stays blocked by the tests, while same-origin HTTPS assets remain allowed. For example:

```sh
CLOUDFLARE_SEND_METRICS=false npx --yes wrangler@4.149.0 pages dev dist --port 8789 --compatibility-date 2026-10-11
# In another terminal:
RANTERRA_TEST_ORIGIN=http://127.0.0.1:8789 npm run test:browser
# After an authorized deployment:
RANTERRA_TEST_ORIGIN=https://ranterra.davidkh.com npm run test:browser
```

Phone-only authentication can use Wrangler's `login --device --browser=false` flow; approve the short-lived device code in Cloudflare on the phone. Do not paste API tokens/passwords into chat or export them into the repository. Deployment should reuse the verified static candidate, with no changes to the existing portfolio's project, apex DNS, billing or nameservers. Pages custom-domain association must be established before its CNAME is activated.

Local follow-up evidence: all 30 production browser tests passed against Wrangler Pages dev in 2.4 minutes. Strengthened worker/WASM MIME and magic checks passed in both browsers (2 tests, 9.6 seconds). The local static verifier also passed both routing tests. TypeScript/lint and a full Rust/WASM build pass; web/full builds reproduce 14 artifact hashes. These are local platform-runtime checks, not HTTPS/custom-domain verification.
