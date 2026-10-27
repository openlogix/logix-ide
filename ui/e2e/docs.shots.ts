// Screenshots for the user documentation (docs/images). Not a test: run
// with `npm run docs:screenshots` after UI changes, and check the images.
import { expect, test, type Page } from "@playwright/test";

// Relative to the repo root, where npm runs the tests.
const IMAGES = "docs/images";

const menu = (page: Page, id: string) =>
  page.evaluate(
    (id) => (window as unknown as { __fake: { menu: (id: string) => void } }).__fake.menu(id),
    id,
  );

/** Starts Logix with the Alhambra II blinky, as after "choose your board". */
async function startWithBoard(page: Page) {
  await page.goto("/");
  await expect(page.locator(".console")).toContainText("Ready.");
  // New projects start with the Alhambra II blinky.
  await expect(page.locator(".board-button")).toHaveText(/Alhambra II/);
  await expect(page.locator(".tab", { hasText: "pinout.pcf" })).toBeVisible();
  // The board is plugged in, as when following the quick start.
  await page.evaluate(() => {
    const w = window as unknown as {
      __fake: { connected: { board: string; exact: boolean }[] };
    };
    w.__fake.connected = [{ board: "alhambra-ii", exact: true }];
    window.dispatchEvent(new Event("focus"));
  });
  await expect(page.locator(".board-link")).toHaveText("Connected");
}

async function verify(page: Page, expected: RegExp) {
  await page.getByTitle(/^Verify/).click();
  await expect(page.locator(".status-text")).toHaveText(expected);
}

async function shot(page: Page, name: string) {
  // Let scrolling and fonts settle.
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${IMAGES}/${name}.png` });
}

test("the board picker", async ({ page }) => {
  await startWithBoard(page);
  await page.locator(".board-button").click();
  await expect(page.getByRole("listbox")).toBeVisible();
  await shot(page, "boards");
});

test("main window after Verify", async ({ page }) => {
  await startWithBoard(page);
  await verify(page, /^Build succeeded/);
  await shot(page, "main");
});

test("an error marked in the editor", async ({ page }) => {
  await startWithBoard(page);
  // Remove the ";" at the end of "assign led = counter[23];".
  await page.locator(".cm-line", { hasText: "assign led" }).click();
  await page.keyboard.press("End");
  await page.keyboard.press("Backspace");
  await verify(page, /^Build failed/);
  await page.locator(".cm-lintRange-error").hover();
  await expect(page.locator(".cm-tooltip-lint")).toBeVisible();
  await shot(page, "error");
});

test("examples", async ({ page }) => {
  await startWithBoard(page);
  await menu(page, "examples");
  const dialog = page.getByRole("dialog", { name: "Examples" });
  await dialog.getByText("blinky", { exact: true }).click();
  await shot(page, "examples");
});

test("run tests", async ({ page }) => {
  await startWithBoard(page);
  await page.getByTitle(/^Run tests/).click();
  await expect(page.locator(".status-text")).toContainText("Tests passed");
  await shot(page, "tests");
});

test("diagram", async ({ page }) => {
  await startWithBoard(page);
  await menu(page, "show-diagram");
  await expect(page.getByRole("dialog", { name: "Diagram" })).toBeVisible();
  await shot(page, "diagram");
});

test("settings", async ({ page }) => {
  await startWithBoard(page);
  await menu(page, "settings");
  await expect(page.getByRole("dialog", { name: "Settings" })).toBeVisible();
  await shot(page, "settings");
});

test("connected devices", async ({ page }) => {
  await startWithBoard(page);
  await menu(page, "devices");
  await expect(page.getByRole("dialog", { name: "Connected Devices" })).toContainText(
    "Alhambra II",
  );
  await shot(page, "devices");
});
