# Static production deployment

No emulation backend is required. For the zero-server-budget recommendation and alternatives, see [free hosting options](hosting-options.md). Cloudflare deployment was authorized on 2026-10-11. The corrected candidate is live at https://ranterra.pages.dev, deployed from the verified CI artifact at `c9681f9`. Both CI jobs and all 32 hosted Chromium/Firefox browser checks passed. `ranterra.davidkh.com` is associated with Pages and awaits its CNAME/certificate activation. Release tagging/publication remains on hold. [Deployment evidence](hosting-evidence.json).

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

Before a hosted release verify the actual response headers, deep-link refresh, worker/WASM requests, configured map attribution/provider access, registration, session traffic, storage and browser recovery. Local Playwright tests exercise these application/static-server behaviors with external geography disabled; they do not prove the hosting provider configuration. The preceding Ant candidate `5f13d43` passed [both remote CI jobs](https://github.com/khvedela/RANterra/actions/runs/38156308147), including production browser tests under Xvfb. The earlier peer-lockfile and Firefox startup issues are documented in the release report. The Pages origin now passes actual HTTPS hosting verification; verify any new host or custom domain separately. Use the verified artifact rather than treating local header/route behavior as a complete host emulator.

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

## Deployed project and repeatable updates

Project `ranterra` uses Pages Direct Upload with production branch `main`. Deployment `5ecdd003-abd2-49d1-8391-c24f1ccb4c21` serves the exact CI artifact from [run 38160644943](https://github.com/khvedela/RANterra/actions/runs/38160644943). Its 14 manifest entries were independently hashed before upload. No Functions, Worker backend, databases, paid bindings or automatic branch deployment are configured. The existing portfolio and billing settings are unchanged.

For future updates, wait for both candidate CI jobs to pass, download `ranterra-static-candidate`, verify its manifest with `scripts/verify-build.mjs --compare` from the artifact's root, then deploy only its `dist/` directory. Record the tested commit in deployment metadata. For this source:

```sh
gh run download 38160644943 --name ranterra-static-candidate --dir /tmp/ranterra-candidate
# From /tmp/ranterra-candidate, run the repository's scripts/verify-build.mjs --compare.
CLOUDFLARE_SEND_METRICS=false npx --yes wrangler@4.149.0 pages deploy /tmp/ranterra-candidate/dist --project-name ranterra --branch main --commit-hash c9681f9f01b3871d1e170a0d26fe5e0849c909d8
RANTERRA_TEST_ORIGIN=https://ranterra.pages.dev npm run test:browser
```

The actual device authorization used account/user/zone read and Pages write scopes, with encrypted credentials backed by the macOS Keychain. It does not permit DNS edits: DNS API access returned 403. Do not broaden access or put credentials into repository files merely to update this static project. When creating a new project with Wrangler 4.149.0, `pages project create NAME --production-branch main --force` explicitly selects Pages instead of that version's default Workers delegation; existing Pages projects deploy normally with `pages deploy`.

The requested domain is already associated. Its remaining record in the existing `davidkh.com` zone is:

| Type  | Name     | Target             | Proxy | TTL  |
| ----- | -------- | ------------------ | ----- | ---- |
| CNAME | ranterra | ranterra.pages.dev | On    | Auto |

After DNS activation, wait for the Pages custom domain to become active and its certificate to validate. Then repeat the hosted browser suite with `RANTERRA_TEST_ORIGIN=https://ranterra.davidkh.com`, and independently verify redirect, TLS/certificate, headers, asset hashes and deep links there. Until then, only the Pages origin is verified. No apex record, nameserver change or rented server is needed.

Live checks confirm HTTP-to-HTTPS redirect, TLS 1.3 preference, TLS 1.2/1.3 support, server rejection of TLS 1.0/1.1, publicly trusted certificates, security headers, immutable hashed assets, HTML/config revalidation, supported route refreshes and no-store 404s. Known source-control paths also return 404. The new geographic-alignment check covers DPR 2, wide reload and resize during recentering. A manual live visit rendered OSM tiles with attribution and completed four registrations, one session and an echo. Automated tests block all third-party geographic traffic; public Overpass retrieval was not exercised as a hosted load test.
