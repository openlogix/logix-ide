// End-to-end tests of the UI against the fake backend (e2e/fakeBackend.ts).
import { expect, test, type Page } from "@playwright/test";

const status = (page: Page) => page.locator(".status-text");
const output = (page: Page) => page.locator(".console");
const tab = (page: Page, name: string) => page.locator(".tab", { hasText: name });

const boardButton = (page: Page) => page.locator(".board-button");

/** Opens the board picker and picks a board by its name. */
async function pickBoard(page: Page, name: string) {
  await boardButton(page).click();
  await page.getByRole("combobox").fill(name);
  await page.getByRole("option", { name, exact: true }).first().click();
}

/** Opens the app and waits until the (fake) FPGA tools are ready. */
async function start(page: Page) {
  await page.goto("/");
  await expect(output(page)).toContainText("Ready.");
}

/**
 * Picks a board and opens its blinky from the examples it offers. New
 * projects already start with the Alhambra II's blinky.
 */
async function chooseBoard(page: Page, name = "Alhambra II") {
  if ((await boardButton(page).textContent())?.includes(name)) {
    await expect(tab(page, "pinout.pcf")).toBeVisible();
    return;
  }
  await pickBoard(page, name);
  const dialog = page.getByRole("dialog");
  await dialog.getByRole("button", { name: /^blinky/ }).click();
  await dialog.getByRole("button", { name: "Open" }).click();
  await expect(tab(page, "pinout.pcf")).toBeVisible();
}

/** Types at the start of the open file. */
async function typeAtStart(page: Page, text: string) {
  await page.locator(".cm-content").click();
  await page.keyboard.press("Meta+ArrowUp");
  await page.keyboard.type(text);
}

test("starts up with the tools ready and the boards listed", async ({ page }) => {
  await start(page);
  await expect(output(page)).toContainText("146 examples available (18 hidden");
  // Apio's 109 boards, grouped by FPGA architecture. Only the current
  // board's group starts expanded; searching shows matches in all groups.
  await boardButton(page).click();
  await expect(page.locator(".group")).toHaveText([
    /Lattice iCE40\s*42/,
    /Lattice ECP5\s*36/,
    /Gowin\s*8/,
    /Xilinx\s*23/,
  ]);
  await expect(page.getByRole("option")).toHaveCount(42);
  await page.getByRole("button", { name: /Gowin/ }).click();
  await expect(page.getByRole("option")).toHaveCount(50);
  await page.getByRole("button", { name: /Lattice iCE40/ }).click();
  await expect(page.getByRole("option")).toHaveCount(8);
  await page.getByRole("combobox").fill("a");
  await expect(page.getByRole("option")).not.toHaveCount(8);
  await page.keyboard.press("Escape");
  await expect(page.getByRole("listbox")).toBeHidden();
  // A new project always has a board: the Alhambra II by default, with its
  // blinky, ready to build.
  await expect(boardButton(page)).toHaveText(/Alhambra II/);
  await expect(tab(page, "pinout.pcf")).toBeVisible();
  await expect(page.locator(".welcome")).toHaveCount(0);
  await expect(page.getByTitle(/^Verify/)).toBeEnabled();
  // Idle: no busy indicator, and the status bar says so.
  await expect(status(page)).toHaveText("Ready.");
  await expect(page.locator(".spinner")).toHaveCount(0);
});

test("a new project for a board builds right away", async ({ page }) => {
  await start(page);
  await chooseBoard(page);
  await expect(tab(page, "main_tb.v")).toBeVisible();
  await page.getByTitle(/^Verify/).click();
  await expect(status(page)).toContainText("Build succeeded");
  await expect(status(page)).toContainText("0.4% of logic cells");
});

test("a build error is marked in the editor and linked in the output", async ({ page }) => {
  await start(page);
  await chooseBoard(page);
  await typeAtStart(page, "oops\n");
  await expect(tab(page, "main.v").locator(".dot")).toBeVisible();

  await page.getByTitle(/^Verify/).click();
  await expect(status(page)).toContainText("Build failed: main.v:1:");
  await expect(page.locator(".cm-lintRange-error")).toBeVisible();
  await expect(tab(page, "main.v")).toHaveClass(/has-error/);

  // The "main.v:1" in the output jumps to the file.
  await tab(page, "pinout.pcf").click();
  await page.locator(".file-link", { hasText: "main.v:1" }).click();
  await expect(tab(page, "main.v")).toHaveClass(/active/);
});

test("adds a file as a new tab and checks names", async ({ page }) => {
  await start(page);
  await chooseBoard(page);
  await page.getByTitle(/^New file/).click();
  await page.keyboard.type("bad name.v");
  await page.keyboard.press("Enter");
  await expect(page.locator(".name-error")).toContainText("can't contain spaces");

  await page.locator(".tab-input").fill("counter.v");
  await page.keyboard.press("Enter");
  await expect(tab(page, "counter.v")).toHaveClass(/active/);
  await expect(page.locator(".cm-content")).toContainText("module counter");
});

test("Auto Format replaces the text and reports it", async ({ page }) => {
  await start(page);
  await chooseBoard(page);
  await typeAtStart(page, "// note   \n");
  await page.evaluate(() =>
    (window as unknown as { __fake: { menu: (id: string) => void } }).__fake.menu(
      "auto-format",
    ),
  );
  await expect(status(page)).toHaveText("Auto Format finished.");
  await expect(output(page)).toContainText("$ apio format main.v");
});

test("picking the current board offers its examples too", async ({ page }) => {
  await start(page);
  await boardButton(page).click();
  // The list opens at the current board, checked; arrows and Enter pick.
  await expect(page.getByRole("option", { selected: true }).last()).toHaveText(/Alhambra II/);
  await page.keyboard.press("Enter");
  const dialog = page.getByRole("dialog", { name: "Examples" });
  await expect(dialog.getByRole("heading")).toContainText("Examples for Alhambra II");
  await dialog.getByRole("button", { name: "Keep Current Files" }).click();
  await expect(boardButton(page)).toHaveText(/Alhambra II/);
});

test("changing the board offers its examples, or keeps the files", async ({ page }) => {
  await start(page);
  await chooseBoard(page);
  await typeAtStart(page, "// mine\n");
  await pickBoard(page, "iCEBreaker");
  const dialog = page.getByRole("dialog", { name: "Examples" });
  await expect(dialog.getByRole("heading")).toContainText("Examples for iCEBreaker");
  await expect(dialog.getByRole("list", { name: "Boards" })).toHaveCount(0);
  await dialog.getByRole("button", { name: "Keep Current Files" }).click();
  await expect(dialog).toBeHidden();
  await expect(page.locator(".cm-content")).toContainText("// mine");
  await expect(boardButton(page)).toHaveText(/iCEBreaker/);
});

test("lists plugged-in boards first and shows if the board is connected", async ({ page }) => {
  await start(page);
  await chooseBoard(page);
  const link = page.locator(".board-link");
  await expect(link).toHaveText("Not connected");
  await page.evaluate(() => {
    const w = window as unknown as {
      __fake: { connected: { board: string; exact: boolean }[] };
    };
    w.__fake.connected = [{ board: "alhambra-ii", exact: true }];
    window.dispatchEvent(new Event("focus"));
  });
  await expect(link).toHaveText("Connected");
  await boardButton(page).click();
  await expect(page.locator(".group").first()).toHaveText("Connected");
  await expect(page.getByRole("option").first()).toHaveText(/Alhambra II/);
});

test("opens a board's Bare Minimum from the examples", async ({ page }) => {
  await start(page);
  await chooseBoard(page);
  await page.evaluate(() =>
    (window as unknown as { __fake: { menu: (id: string) => void } }).__fake.menu(
      "examples",
    ),
  );
  const dialog = page.getByRole("dialog", { name: "Examples" });
  // Boards are grouped by architecture; the current board's group is open.
  const boards = dialog.getByRole("list", { name: "Boards" });
  await expect(boards.getByRole("button", { name: /Lattice iCE40/ })).toHaveAttribute(
    "aria-expanded",
    "true",
  );
  await expect(boards.getByRole("button", { name: /Xilinx/ })).toHaveAttribute(
    "aria-expanded",
    "false",
  );
  await expect(boards.getByRole("button", { name: /^Alchitry Au \(/ })).toHaveCount(0);
  await boards.getByRole("button", { name: /Xilinx/ }).click();
  await expect(boards.getByRole("button", { name: /^Alchitry Au \(/ })).toBeVisible();
  await dialog.getByRole("button", { name: /Bare Minimum/ }).click();
  await dialog.getByRole("button", { name: "Open" }).click();
  await expect(dialog).toBeHidden();
  await expect(tab(page, "pinout.pcf")).toBeVisible();
  await expect(tab(page, "main_tb.v")).toHaveCount(0);
  await expect(page.locator(".cm-content")).toContainText("assign led = 0;");
  await expect(status(page)).toHaveText("Opened example Bare Minimum.");
  await expect(page.locator(".spinner")).toHaveCount(0);
});

test("the editor's right-click menu comments lines", async ({ page }) => {
  await start(page);
  await chooseBoard(page);
  await page.locator(".cm-line").first().click({ button: "right" });
  type Fake = { editorMenu: unknown; menu: (id: string) => void };
  const editorMenu = () =>
    page.evaluate(() => (window as unknown as { __fake: Fake }).__fake.editorMenu);
  const menu = (id: string) =>
    page.evaluate((id) => (window as unknown as { __fake: Fake }).__fake.menu(id), id);
  // The fake has no live checker, so Go to Definition is off.
  expect(await editorMenu()).toEqual({ canGoToDefinition: false });
  const first = page.locator(".cm-line").first();
  const before = (await first.textContent()) ?? "";
  await menu("toggle-comment");
  await expect(first).toHaveText(`// ${before}`);
  await menu("toggle-comment");
  await expect(first).toHaveText(before);
});

test("the output can be selected, copied and cleared", async ({ page }) => {
  await start(page);
  type Fake = { outputMenu: number; menu: (id: string) => void };
  const selected = () => page.evaluate(() => window.getSelection()?.toString() ?? "");

  // Cmd+A in the output selects only the output, not the editor.
  await output(page).click();
  await page.keyboard.press("Meta+a");
  expect(await selected()).toContain("Ready.");
  expect(await selected()).not.toContain("module main");

  // Edit > Select All does the same while the output has the focus.
  await page.evaluate(() => window.getSelection()?.removeAllRanges());
  await page.evaluate(() =>
    (window as unknown as { __fake: Fake }).__fake.menu("select-all"),
  );
  expect(await selected()).toContain("Ready.");

  // Right-click shows the output's menu; Clear Output empties it.
  await output(page).click({ button: "right" });
  expect(
    await page.evaluate(() => (window as unknown as { __fake: Fake }).__fake.outputMenu),
  ).toBe(1);
  await page.evaluate(() =>
    (window as unknown as { __fake: Fake }).__fake.menu("clear-output"),
  );
  await expect(output(page)).toHaveText("");
});

test("the output can be hidden and comes back for a build", async ({ page }) => {
  await start(page);
  await chooseBoard(page);
  await page.getByRole("button", { name: /Hide/ }).click();
  await expect(output(page)).toBeHidden();
  // The one-line status bar stays.
  await expect(page.locator(".status-bar")).toBeVisible();

  await page.getByTitle(/^Verify/).click();
  await expect(output(page)).toBeVisible();
  await expect(status(page)).toContainText("Build succeeded");
});

test("shows the design diagram and switches modules", async ({ page }) => {
  await start(page);
  await chooseBoard(page);
  await page.evaluate(() =>
    (window as unknown as { __fake: { menu: (id: string) => void } }).__fake.menu(
      "show-diagram",
    ),
  );
  const dialog = page.getByRole("dialog", { name: "Diagram" });
  await expect(dialog).toBeVisible();
  await expect(dialog.getByRole("img", { name: "Diagram of module main" })).toBeVisible();
  await expect(status(page)).toContainText("Diagram ready");

  await dialog.getByTitle("Zoom in").click();
  await expect(dialog).toContainText("150%");

  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
});

test("lists the connected devices", async ({ page }) => {
  await start(page);
  await page.evaluate(() =>
    (window as unknown as { __fake: { menu: (id: string) => void } }).__fake.menu(
      "devices",
    ),
  );
  const dialog = page.getByRole("dialog", { name: "Connected Devices" });
  await expect(dialog).toContainText("Alhambra II v1.0A (AlhambraBits, 0403:6010)");
  await expect(dialog).toContainText("/dev/cu.usbserial-FTATANKC0");
  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
});

test("settings change the theme and font size", async ({ page }) => {
  await start(page);
  const menu = (id: string) =>
    page.evaluate(
      (id) =>
        (window as unknown as { __fake: { menu: (id: string) => void } }).__fake.menu(id),
      id,
    );
  await menu("settings");
  const dialog = page.getByRole("dialog", { name: "Settings" });
  await expect(dialog.getByLabel("Projects folder location:")).toHaveValue(
    "/Users/you/Documents/Logix",
  );

  // A bad value is reported and the dialog stays open.
  await dialog.getByLabel("Editor font size:").fill("99");
  await dialog.getByRole("button", { name: "OK" }).click();
  await expect(dialog).toContainText("between 8 and 32");

  await dialog.getByLabel("Editor font size:").fill("18");
  await dialog.getByLabel("Theme:").selectOption("dark");
  await dialog.getByRole("button", { name: "Browse" }).click();
  await expect(dialog.getByLabel("Projects folder location:")).toHaveValue("/Users/you/fpga");
  await dialog.getByRole("button", { name: "OK" }).click();
  await expect(dialog).toBeHidden();

  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  const fontSize = await page.evaluate(() =>
    getComputedStyle(document.documentElement).getPropertyValue("--editor-font-size"),
  );
  expect(fontSize.trim()).toBe("18px");
});
