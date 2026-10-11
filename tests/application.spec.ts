import { test, expect } from "@playwright/test";
import { readFileSync } from "node:fs";
const minimal = JSON.parse(readFileSync("examples/minimal.json", "utf8"));
test.beforeEach(async ({ page }) => {
  // Disable geographic network assets; never automate public OSM tile traffic.
  await page.route("**/config.json", (route) =>
    route.fulfill({
      json: { tiles: "", search: "", overpass: "", osmMap: "" },
    }),
  );
  await page.route(/^https:\/\//, (route) => route.abort());
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Run simulation", exact: true }),
  ).toBeEnabled();
  await page
    .locator("input[type=file]")
    .setInputFiles({
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
  await page.getByLabel("Simulation speed").selectOption("10");
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
  await page.getByLabel("Laboratory fault").selectOption("InvalidCredentials");
  await expect(
    page.getByRole("button", { name: "Run simulation", exact: true }),
  ).toBeEnabled();
  await page.getByLabel("Simulation speed").selectOption("10");
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
  await page
    .locator("input[type=file]")
    .setInputFiles({
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
  await expect(page.getByLabel("Laboratory fault")).toHaveValue(
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
  expect(asset.headers()["cache-control"]).toContain("immutable");
  expect((await request.get("/missing.js")).status()).toBe(404);
});
