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

/** Starts Logix IDE with the Alhambra II blinky, as after "choose your board". */
async function startWithBoard(page: Page) {
  await page.goto("/");
  await expect(page.locator(".console")).toContainText("Ready.");
  // Open the iCESugar v1.5 blinky from the start screen's Examples….
  await expect(page.locator(".board-button")).toHaveText(/iCESugar v1\.5/);
  await page.getByRole("button", { name: "Examples…" }).click();
  const examples = page.getByRole("dialog", { name: "Examples" });
  await examples.getByRole("button", { name: /^blinky/ }).click();
  await examples.getByRole("button", { name: "New Project" }).click();
  await expect(page.locator(".tab", { hasText: "pinout.pcf" })).toBeVisible();
  // The board is plugged in, as when following the tutorial.
  await page.evaluate(() => {
    const w = window as unknown as { __fake: { connected: unknown[] } };
    w.__fake.connected = [
      {
        id: "usb:1-1.4.2:1d50:602b:07000001004300413100001056313032a5a5a5a500000000",
        name: "DAPLink CMSIS-DAP",
        serial: "07000001004300413100001056313032a5a5a5a500000000",
        boards: [{ board: "icesugar-1-5", level: "strong", variants: [{}] }],
      },
    ];
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

test("the start screen", async ({ page }) => {
  await page.goto("/?recents=/Users/you/Documents/OpenLogix/IDE/counter,/Users/you/Documents/OpenLogix/IDE/uart");
  await expect(page.locator(".console")).toContainText("Ready.");
  await expect(page.getByLabel("Start")).toContainText("counter");
  await shot(page, "start");
});

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

test("the build report", async ({ page }) => {
  await startWithBoard(page);
  await verify(page, /^Build succeeded/);
  await menu(page, "build-report");
  await expect(page.getByRole("dialog", { name: "Board Utilization" })).toBeVisible();
  await shot(page, "board-utilization");
});

test("an error marked in the editor", async ({ page }) => {
  await startWithBoard(page);
  // Remove the ";" at the end of "reg [N-1:0] counter = 0;".
  await page.locator(".cm-line", { hasText: "reg [N-1:0] counter = 0;" }).click();
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
  // The Diagram window shows Logix Engine's page, captured for the blinky
  // design (fixtures/diagram.html), at the window's size.
  await page.setViewportSize({ width: 1000, height: 640 });
  await page.goto("/e2e/fixtures/diagram.html");
  await expect(page.locator("svg.logix-diagram")).toBeVisible();
  await shot(page, "diagram");
});

test("waveforms", async ({ page }) => {
  // The Waveforms window shows Logix Engine's waveform viewer, captured for
  // the blinky testbench (fixtures/waveforms.html), at the window's size:
  // the testbench's signals, the design's counter and green_on added, and
  // the cursor on one of the LED's changes.
  await page.setViewportSize({ width: 1100, height: 640 });
  await page.goto("/e2e/fixtures/waveforms.html?theme=light");
  await expect(page.locator(".nrow")).toHaveCount(2);
  await page.locator(".scope-h", { hasText: "dut" }).click();
  await page.locator('.var[data-path="main_tb.dut.counter"] .chk').click();
  await page.locator('.var[data-path="main_tb.dut.green_on"] .chk').click();
  const box = (await page.locator("#waves").boundingBox())!;
  await page.mouse.click(box.x + box.width * 0.62, box.y + 45);
  await page.mouse.move(box.x + box.width * 0.62, box.y + 200);
  await shot(page, "waveforms");
});

test("settings", async ({ page }) => {
  await startWithBoard(page);
  await menu(page, "settings");
  await expect(page.getByRole("dialog", { name: "Settings" })).toBeVisible();
  await shot(page, "settings");
});

// The canonical board, an iCESugar v1.5, with the USB names captured from a
// real one.
test("connected devices", async ({ page }) => {
  await startWithBoard(page);
  await menu(page, "devices");
  await expect(page.getByRole("dialog", { name: "Connected Devices" })).toContainText(
    "DAPLink CMSIS-DAP (MuseLab, 1d50:602b)",
  );
  await shot(page, "devices");
});

test("serial monitor", async ({ page }) => {
  await startWithBoard(page);
  await menu(page, "serial-monitor");
  const monitor = page.getByRole("log", { name: "Serial Monitor output" });
  await expect(monitor).toContainText("Opened");
  // What the serial example sends, and an echo of what was typed.
  await page.evaluate(() => {
    const f = (window as unknown as { __fake: { serialReceive: (t: string) => void } }).__fake;
    f.serialReceive("Hello from your FPGA board\r\nHello from your FPGA board\r\nhi\r\nHello from your FPGA board\r\n");
  });
  await expect(monitor.locator(".line").nth(-2)).toHaveText("Hello from your FPGA board");
  await page.getByRole("textbox", { name: "Message to send" }).fill("hello");
  await shot(page, "serial-monitor");
});

test("project options after a board change", async ({ page }) => {
  await startWithBoard(page);
  await page.locator(".board-button").click();
  await page.getByRole("combobox").fill("iCEBreaker");
  await page.getByRole("option", { name: "iCEBreaker", exact: true }).first().click();
  await expect(page.getByRole("dialog", { name: "Examples" })).toContainText("Project options for board “iCEBreaker”");
  await shot(page, "project-options");
});

test("board explorer", async ({ page }) => {
  await startWithBoard(page);
  await menu(page, "board-explorer");
  const dialog = page.getByRole("dialog", { name: "Board Explorer" });
  await expect(dialog.getByLabel("Details")).toContainText("ICE40UP5K-SG48");
  await shot(page, "board-explorer");
});

test("toolbar icons", async ({ page }) => {
  // The toolbar buttons as small images for the docs text (icon-<name>.png),
  // taken from the real UI so they always match it: no toolbar color around
  // the round buttons, which get the toolbar's regular teal (the darker
  // one is for contrast on the teal toolbar; the docs page is white), and
  // Stop shown as when something runs.
  await startWithBoard(page);
  await page.addStyleTag({
    content:
      ".toolbar { background: transparent !important; }" +
      ".toolbar button { background: var(--toolbar-bg) !important; }",
  });
  const buttons: [string, RegExp][] = [
    ["verify", /^Verify/],
    ["upload", /^Upload/],
    ["test", /^Run tests/],
    ["simulate", /^Simulate/],
    ["stop", /^Stop/],
  ];
  for (const [name, title] of buttons) {
    const button = page.getByTitle(title);
    await button.evaluate((b) => b.removeAttribute("disabled"));
    await page.mouse.move(0, 0);
    await button.screenshot({ path: `${IMAGES}/icon-${name}.png`, omitBackground: true });
  }
});
