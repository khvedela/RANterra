import { test, expect, type Page } from "@playwright/test";
import { readFileSync } from "node:fs";
// Ant Select uses an accessible popup rather than a native select element.
async function choose(page: Page, label: string, value: string) {
  await page.getByRole("combobox", { name: label, exact: true }).click();
  await page
    .getByRole("option", {
      name: label === "Simulation speed" ? `${value}×` : value,
      exact: true,
    })
    .click();
}
const minimal = JSON.parse(readFileSync("examples/minimal.json", "utf8"));
async function blockGeographicRequests(page: Page) {
  const origin = new URL(
    process.env.RANTERRA_TEST_ORIGIN ?? "http://127.0.0.1:4173",
  ).origin;
  await page.route(/^https:\/\//, (route) =>
    new URL(route.request().url()).origin === origin
      ? route.fallback()
      : route.abort(),
  );
}
test.beforeEach(async ({ page }) => {
  // Disable geographic network assets; never automate public OSM tile traffic.
  await page.route("**/config.json", (route) =>
    route.fulfill({
      json: { tiles: "", search: "", overpass: "", osmMap: "" },
    }),
  );
  await blockGeographicRequests(page);
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Run simulation", exact: true }),
  ).toBeEnabled();
  await page.locator("input[type=file]").setInputFiles({
    name: "minimal.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(minimal)),
  });
  await expect(page.getByLabel("Scenario name")).toHaveValue(minimal.name);
});
test("production boot, registration, secured session, offline echo and teardown", async ({
  page,
  context,
}) => {
  await expect(page.locator(".maplibregl-canvas")).toBeVisible();
  await expect(page.locator(".map-warning")).toHaveCount(0);
  await expect(page.locator(".maplibregl-ctrl-attrib")).toContainText(
    "OpenStreetMap contributors",
  );
  await choose(page, "Simulation speed", "10");
  await page
    .getByRole("button", { name: "Run simulation", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Exchange complete", exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: `Select ${minimal.ues[0].name}`, exact: true })
    .click();
  await expect(
    page.getByText("REGISTERED · NIA2 / NEA2", { exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Establish PDU session", exact: true })
    .click();
  await expect(page.getByText("10.45.0.1", { exact: true })).toBeVisible();
  await context.setOffline(true);
  await page.getByRole("button", { name: "Send echo", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Release session", exact: true }),
  ).toBeEnabled();
  await expect(
    page.locator(".event-row").filter({ hasText: "Echo delivered" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "GTP-U", exact: true }).click();
  await expect(
    page.locator(".event-row").filter({ hasText: "GTP-U G-PDU" }),
  ).toHaveCount(2);
  await page
    .getByRole("button", { name: "Release session", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Send echo", exact: true }),
  ).toBeDisabled();
  await context.setOffline(false);
  await page.screenshot({
    path: "test-results/production-workspace.png",
    fullPage: true,
  });
  await page.getByRole("button", { name: "Results", exact: true }).click();
  await expect(
    page.getByRole("dialog", { name: "Experiment results" }),
  ).toBeVisible();
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "Export metrics CSV" }).click();
  expect((await download).suggestedFilename()).toBe("ranterra-metrics.csv");
});
test("invalid subscriber, import rejection and saved scenario persistence", async ({
  page,
}) => {
  await page
    .getByRole("button", { name: `Select ${minimal.ues[0].name}`, exact: true })
    .click();
  await choose(page, "Laboratory fault", "InvalidCredentials");
  await expect(
    page.getByRole("button", { name: "Run simulation", exact: true }),
  ).toBeEnabled();
  await choose(page, "Simulation speed", "10");
  await page
    .getByRole("button", { name: "Run simulation", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Exchange complete", exact: true }),
  ).toBeVisible();
  await expect(page.getByText(/UE rejected AUTN/)).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Establish PDU session", exact: true }),
  ).toBeDisabled();
  await page.locator("input[type=file]").setInputFiles({
    name: "bad.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify({ ...minimal, version: 99 })),
  });
  await expect(page.getByRole("alert")).toContainText(
    "Unsupported scenario version",
  );
  await expect(page.getByLabel("Scenario name")).toHaveValue(minimal.name);
  await page.reload();
  await expect(page.getByLabel("Scenario name")).toHaveValue(minimal.name);
  await page
    .getByRole("button", { name: `Select ${minimal.ues[0].name}`, exact: true })
    .click();
  await expect(page.getByLabel("Laboratory fault").locator("..")).toContainText(
    "InvalidCredentials",
  );
});
test("entity creation, edit, scenario export and worker crash recovery", async ({
  page,
}) => {
  // Worker wrapper only exposes a test handle; all execution is the real production worker.
  await page.addInitScript(() => {
    const Original = window.Worker;
    class Observed extends Original {
      constructor(url: string | URL, options?: WorkerOptions) {
        super(url, options);
        if (String(url).includes("engine.worker"))
          (window as unknown as { engineWorker: Worker }).engineWorker = this;
      }
    }
    window.Worker = Observed;
  });
  await page.reload();
  await expect(
    page.getByRole("button", { name: "Run simulation", exact: true }),
  ).toBeEnabled();
  await page
    .getByRole("button", { name: "UE User equipment", exact: true })
    .click();
  await page.getByTestId("network-map").click({ position: { x: 400, y: 280 } });
  await expect(page.getByLabel("Entity name")).toBeVisible();
  await page.getByLabel("Entity name").fill("Mobile laboratory UE");
  await page.getByLabel("Entity name").press("Tab");
  await expect(
    page.getByRole("heading", { name: "Mobile laboratory UE" }),
  ).toBeVisible();
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "Export", exact: true }).click();
  expect((await download).suggestedFilename()).toBe("ranterra-scenario.json");
  await page.evaluate(() => {
    const w = (window as unknown as { engineWorker: Worker }).engineWorker;
    w.terminate();
    w.dispatchEvent(
      new ErrorEvent("error", { message: "Injected worker crash" }),
    );
  });
  await expect(page.getByRole("alert")).toContainText("Injected worker crash");
  await page
    .getByRole("button", { name: "Recover worker", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Run simulation", exact: true }),
  ).toBeEnabled();
});

test("static hosting MIME, headers and route refresh", async ({ request }) => {
  const page = await request.get("/experiment/local");
  expect(page.status()).toBe(200);
  expect(page.headers()["content-security-policy"]).toContain(
    "'wasm-unsafe-eval'",
  );
  const html = await page.text();
  const js = html.match(/src="([^"]+\.js)"/)?.[1];
  expect(js).toBeTruthy();
  const asset = await request.get(js!);
  expect(asset.headers()["cache-control"]).toBe(
    "public, max-age=31536000, immutable",
  );
  const workerPath = (await asset.text()).match(
    /\/assets\/engine\.worker-[\w-]+\.js/,
  )?.[0];
  expect(workerPath).toBeTruthy();
  const worker = await request.get(workerPath!);
  expect(worker.status()).toBe(200);
  expect(worker.headers()["content-type"]).toContain("javascript");
  const wasmName = (await worker.text()).match(
    /ranterra_engine_bg-[\w-]+\.wasm/,
  )?.[0];
  expect(wasmName).toBeTruthy();
  const wasm = await request.get(`/assets/${wasmName}`);
  expect(wasm.status()).toBe(200);
  expect(wasm.headers()["content-type"]).toBe("application/wasm");
  expect([...(await wasm.body()).subarray(0, 4)]).toEqual([0, 97, 115, 109]);
  for (const path of ["/", "/index.html", "/config.json"]) {
    expect((await request.get(path)).headers()["cache-control"]).toBe(
      "no-cache",
    );
  }
  expect((await request.get("/missing.js")).status()).toBe(404);
  expect((await request.get("/assets/missing.wasm")).status()).toBe(404);
  expect((await request.get("/unknown-route")).status()).toBe(404);
  expect((await request.get("/workspace")).status()).toBe(200);
});

test("configurable service, checkpoint persistence, import replay and comparison", async ({
  page,
}) => {
  await page
    .getByRole("button", { name: "Session policies", exact: true })
    .click();
  await page.screenshot({ path: "test-results/session-policies.png" });
  await page.getByLabel("Policy 1 DNN").fill("research.example");
  await page.getByLabel("Policy 1 SST").fill("2");
  await page.getByLabel("Policy 1 SD").fill("010203");
  await page
    .getByRole("button", { name: "Apply policies", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Apply policies", exact: true }),
  ).toBeEnabled();
  await page
    .getByRole("button", { name: "Close policies", exact: true })
    .click();
  await page
    .getByRole("button", { name: `Select ${minimal.ues[0].name}`, exact: true })
    .click();
  await page.getByLabel("UE DNN").fill("research.example");
  await page.getByLabel("UE DNN").press("Tab");
  await expect(
    page.getByRole("button", { name: "Run simulation", exact: true }),
  ).toBeEnabled();
  await page.getByLabel("UE SST").fill("2");
  await page.getByLabel("UE SST").press("Tab");
  await expect(
    page.getByRole("button", { name: "Run simulation", exact: true }),
  ).toBeEnabled();
  await page.getByLabel("UE SD").fill("010203");
  await page.getByLabel("UE SD").press("Tab");
  await expect(
    page.getByRole("button", { name: "Run simulation", exact: true }),
  ).toBeEnabled();
  await choose(page, "Simulation speed", "10");
  await page
    .getByRole("button", { name: "Run simulation", exact: true })
    .click();
  await expect(
    page.getByText("REGISTERED · NIA2 / NEA2", { exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Establish PDU session", exact: true })
    .click();
  await expect(
    page.getByText("Active: research.example · SST 2 · SD 010203", {
      exact: true,
    }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Send echo", exact: true }).click();
  await expect(
    page.locator(".event-row").filter({ hasText: "Echo delivered" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Results", exact: true }).click();
  await page.getByRole("button", { name: "Save run", exact: true }).click();
  await expect(
    page.getByText("Experiment checkpoint saved with command journal.", {
      exact: true,
    }),
  ).toBeVisible();
  const download = page.waitForEvent("download");
  await page
    .getByRole("button", { name: "Export experiment", exact: true })
    .click();
  const artifact = JSON.parse(
    readFileSync((await (await download).path())!, "utf8"),
  );
  expect(
    artifact.commands.map((c: { action: { type: string } }) => c.action.type),
  ).toEqual(["establish", "echo"]);
  expect(artifact.scenario.version).toBe(2);
  await page.reload();
  await page.getByRole("button", { name: "Results", exact: true }).click();
  await expect(
    page.getByRole("dialog", { name: "Experiment results", exact: true }),
  ).toHaveCSS("transform", "none");
  await page
    .getByRole("button", { name: "Restore saved run", exact: true })
    .click();
  const download2 = page.waitForEvent("download");
  await page
    .getByRole("button", { name: "Export experiment", exact: true })
    .click();
  const restored = JSON.parse(
    readFileSync((await (await download2).path())!, "utf8"),
  );
  expect(restored).toEqual(artifact);
  const chooserPromise = page.waitForEvent("filechooser");
  await page
    .getByRole("button", { name: "Compare experiment", exact: true })
    .click();
  await (
    await chooserPromise
  ).setFiles({
    name: "experiment.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(artifact)),
  });
  await expect(
    page.getByRole("table", { name: "Experiment comparison" }),
  ).toBeVisible();
  // Corrupted replay must preserve the active experiment.
  await page.locator("input[type=file]").setInputFiles({
    name: "corrupt.json",
    mimeType: "application/json",
    buffer: Buffer.from(
      JSON.stringify({ ...artifact, state_digest: "0".repeat(64) }),
    ),
  });
  await expect(page.getByText(/Replay state digest mismatch/)).toBeVisible();
  const download3 = page.waitForEvent("download");
  await page
    .getByRole("button", { name: "Export experiment", exact: true })
    .click();
  expect(
    JSON.parse(readFileSync((await (await download3).path())!, "utf8")),
  ).toEqual(artifact);
});

test("policy denial exposes genuine failure and prevents session traffic", async ({
  page,
}) => {
  const denied = structuredClone(minimal);
  denied.session_policies = [
    { request: { dnn: "lab", snssai: { sst: 1, sd: null } }, max_sessions: 0 },
  ];
  await page.locator("input[type=file]").setInputFiles({
    name: "denied.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(denied)),
  });
  await choose(page, "Simulation speed", "10");
  await page
    .getByRole("button", { name: "Run simulation", exact: true })
    .click();
  await page
    .getByRole("button", { name: `Select ${minimal.ues[0].name}`, exact: true })
    .click();
  await expect(
    page.getByText("REGISTERED · NIA2 / NEA2", { exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Establish PDU session", exact: true })
    .click();
  await expect(
    page.getByText(/5GSM cause 26: Insufficient resources/),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Send echo", exact: true }),
  ).toBeDisabled();
  await page.getByRole("button", { name: "5GSM", exact: true }).click();
  await expect(
    page
      .locator(".event-row")
      .filter({ hasText: "PDU Session Establishment Reject" }),
  ).toBeVisible();
});

test("IndexedDB v1 migration preserves scenarios and quota failure remains recoverable", async ({
  browser,
}) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await page.route("**/migration-setup", (route) =>
    route.fulfill({
      contentType: "text/html",
      body: "<!doctype html><title>Storage migration setup</title>",
    }),
  );
  await page.route("**/config.json", (route) =>
    route.fulfill({
      json: { tiles: "", search: "", overpass: "", osmMap: "" },
    }),
  );
  await blockGeographicRequests(page);
  await page.goto("/migration-setup");
  await page.evaluate(
    async (scenario) => {
      await new Promise<void>((resolve, reject) => {
        const request = indexedDB.open("ranterra", 1);
        request.onupgradeneeded = () => {
          request.result.createObjectStore("scenarios");
          request.result.createObjectStore("assets");
        };
        request.onerror = () => reject(request.error);
        request.onsuccess = () => {
          const db = request.result;
          const tx = db.transaction("scenarios", "readwrite");
          tx.objectStore("scenarios").put(scenario, "current");
          tx.oncomplete = () => {
            db.close();
            resolve();
          };
          tx.onerror = () => reject(tx.error);
        };
      });
    },
    { ...minimal, name: "Migrated legacy scenario" },
  );
  await page.goto("/");
  await expect(page.getByLabel("Scenario name")).toHaveValue(
    "Migrated legacy scenario",
  );
  await page.getByRole("button", { name: "Results", exact: true }).click();
  await page.evaluate(() => {
    const original = IDBObjectStore.prototype.put;
    IDBObjectStore.prototype.put = function (
      ...args: Parameters<IDBObjectStore["put"]>
    ) {
      if (this.name === "runs")
        throw new DOMException(
          "Injected quota exhaustion",
          "QuotaExceededError",
        );
      return original.apply(this, args);
    };
  });
  await page.getByRole("button", { name: "Save run", exact: true }).click();
  await expect(
    page.getByText(/Save run failed:.*QuotaExceededError/),
  ).toBeVisible();
  const download = page.waitForEvent("download");
  await page
    .getByRole("button", { name: "Export experiment", exact: true })
    .click();
  const artifact = JSON.parse(
    readFileSync((await (await download).path())!, "utf8"),
  );
  expect(artifact.scenario.name).toBe("Migrated legacy scenario");
  expect(artifact.started).toBe(false);
  await context.close();
});

test("AUTS synchronization failure recovers to registered and usable session", async ({
  page,
}) => {
  await page
    .getByRole("button", { name: `Select ${minimal.ues[0].name}`, exact: true })
    .click();
  await choose(page, "Laboratory fault", "SqnOutOfSync");
  await expect(
    page.getByRole("button", { name: "Run simulation", exact: true }),
  ).toBeEnabled();
  await choose(page, "Simulation speed", "10");
  await page
    .getByRole("button", { name: "Run simulation", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Exchange complete", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("REGISTERED · NIA2 / NEA2", { exact: true }),
  ).toBeVisible();
  await expect(
    page
      .locator(".event-row")
      .filter({ hasText: "Authentication Failure" })
      .first(),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Establish PDU session", exact: true })
    .click();
  await expect(page.getByText("10.45.0.1", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Send echo", exact: true }).click();
  await expect(
    page.locator(".event-row").filter({ hasText: "Echo delivered" }),
  ).toBeVisible();
});

test("timed signaling recovers registration loss and releases failed N2 resources", async ({
  page,
}) => {
  await page
    .getByRole("button", { name: `Select ${minimal.ues[0].name}`, exact: true })
    .click();
  await choose(page, "Signaling loss", "SecurityModeComplete");
  await page.getByLabel("Signaling drop count").fill("1");
  await choose(page, "Simulation speed", "10");
  await page
    .getByRole("button", { name: "Run simulation", exact: true })
    .click();
  await expect(
    page.getByText("REGISTERED · NIA2 / NEA2", { exact: true }),
  ).toBeVisible();
  await expect(
    page.locator(".event-row").filter({ hasText: "T3560 retransmission" }),
  ).toBeVisible();
  await page.getByLabel("Reject N2 radio resources").check();
  await page
    .getByRole("button", { name: "Run simulation", exact: true })
    .click();
  await expect(
    page.getByText("REGISTERED · NIA2 / NEA2", { exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Establish PDU session", exact: true })
    .click();
  await expect(page.getByText(/NGAP radioNetwork cause 22/)).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Send echo", exact: true }),
  ).toBeDisabled();
  await expect(
    page.getByText("Session signaling pending · virtual time"),
  ).toHaveCount(0);
  await expect(
    page
      .locator(".event-row")
      .filter({ hasText: "PDU Session Release Complete" }),
  ).toBeVisible();
});

test("exhausted Registration Complete delivery never reports verified registration", async ({
  page,
}) => {
  await page
    .getByRole("button", { name: `Select ${minimal.ues[0].name}`, exact: true })
    .click();
  await choose(page, "Signaling loss", "RegistrationComplete");
  await page.getByLabel("Signaling drop count").fill("5");
  await choose(page, "Simulation speed", "10");
  await page
    .getByRole("button", { name: "Run simulation", exact: true })
    .click();
  await expect(
    page.getByText(
      /T3550 exhausted: AMF registered, UE completion unconfirmed/,
    ),
  ).toBeVisible();
  await expect(
    page.getByText("REGISTERED · NIA2 / NEA2", { exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Establish PDU session", exact: true }),
  ).toBeDisabled();
});

test("untrusted labels remain text and blocked storage leaves computation usable", async ({
  page,
}) => {
  await page.addInitScript(() => {
    indexedDB.open = () => {
      throw new DOMException("Storage disabled for test", "SecurityError");
    };
  });
  await page.reload();
  const s = structuredClone(minimal);
  s.name = '<img src=x onerror="globalThis.injected=true">';
  s.ues[0].name = '<svg onload="globalThis.injected=true">';
  await page.locator("input[type=file]").setInputFiles({
    name: "untrusted.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(s)),
  });
  await expect(page.getByLabel("Scenario name")).toHaveValue(s.name);
  await expect(
    page.getByRole("button", { name: `Select ${s.ues[0].name}`, exact: true }),
  ).toBeVisible();
  expect(await page.evaluate(() => Object.hasOwn(globalThis, "injected"))).toBe(
    false,
  );
  await choose(page, "Simulation speed", "10");
  await page
    .getByRole("button", { name: "Run simulation", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Exchange complete", exact: true }),
  ).toBeVisible();
});

test("unavailable WebGL leaves a visible map error and usable simulation", async ({
  page,
}) => {
  await page.addInitScript(() => {
    const original = HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext = function (
      kind: string,
      ...args: unknown[]
    ) {
      if (/webgl|experimental-webgl/.test(kind)) return null;
      return Reflect.apply(original, this, [kind, ...args]);
    } as typeof original;
  });
  await page.reload();
  await expect(page.locator(".map-warning")).toContainText(
    "Map initialization failed",
  );
  await expect(
    page.getByRole("button", { name: "Run simulation", exact: true }),
  ).toBeEnabled();
  await choose(page, "Simulation speed", "10");
  await page
    .getByRole("button", { name: "Run simulation", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Exchange complete", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Results", exact: true }).click();
  await expect(
    page.getByRole("dialog", { name: "Experiment results" }),
  ).toBeVisible();
});

test("Ant dialogs trap focus, close with Escape and return focus to the trigger", async ({
  page,
}) => {
  const trigger = page.getByRole("button", { name: "Results", exact: true });
  await trigger.focus();
  await page.keyboard.press("Enter");
  const dialog = page.getByRole("dialog", {
    name: "Experiment results",
    exact: true,
  });
  await expect(dialog).toBeVisible();
  await expect(dialog).toBeInViewport();
  await expect(dialog).toHaveCSS("opacity", "1");
  for (let i = 0; i < 20; i++) {
    await page.keyboard.press("Tab");
    await expect
      .poll(() => dialog.evaluate((el) => el.contains(document.activeElement)))
      .toBe(true);
  }
  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
  await expect(trigger).toBeFocused();
});

test("mobile workspace keeps the map usable and opens editable inspectors in drawers", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(page.locator(".maplibregl-canvas")).toBeVisible();
  await expect
    .poll(() =>
      page.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth,
      ),
    )
    .toBe(true);
  await page
    .getByRole("button", { name: "Network setup", exact: true })
    .click();
  const setup = page.getByRole("dialog", {
    name: "Network setup",
    exact: true,
  });
  await expect(setup.getByRole("combobox", { name: "RF model" })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(setup).toBeHidden();
  // An inline Ant drawer otherwise follows document scroll and can open offscreen.
  await page.evaluate(() => window.scrollTo(0, 350));
  await page
    .getByRole("button", { name: `Select ${minimal.ues[0].name}`, exact: true })
    .click();
  const inspector = page.getByRole("dialog", {
    name: "Entity inspector",
    exact: true,
  });
  await expect(inspector).toBeInViewport({ ratio: 0.99 });
  await expect
    .poll(async () => Math.round((await inspector.boundingBox())!.y))
    .toBe(0);
  await expect(inspector.getByLabel("Entity name")).toBeVisible();
  await inspector.getByLabel("Entity name").fill("Mobile research UE");
  await inspector.getByLabel("Entity name").press("Tab");
  await expect(
    inspector.getByRole("heading", { name: "Mobile research UE", exact: true }),
  ).toBeVisible();
  await inspector.getByRole("button", { name: "Close", exact: true }).click();
  await expect(inspector).toBeHidden();
  await page.screenshot({
    path: `test-results/mobile-workspace-${test.info().project.name}.png`,
    fullPage: true,
  });
});

test("workspace and results have no serious or critical automated accessibility violations", async ({
  page,
}) => {
  const { default: AxeBuilder } = await import("@axe-core/playwright");
  const check = async () => {
    const audit = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
      .analyze();
    expect(
      audit.violations.filter(
        (v) => v.impact === "serious" || v.impact === "critical",
      ),
    ).toEqual([]);
  };
  await check();
  await page.getByRole("button", { name: "Results", exact: true }).click();
  const dialog = page.getByRole("dialog", {
    name: "Experiment results",
    exact: true,
  });
  await expect(dialog).toBeInViewport();
  await expect(dialog).toHaveCSS("opacity", "1");
  await check();
});
