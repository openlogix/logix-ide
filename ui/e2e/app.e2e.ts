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

/**
 * Opens the app, waits until the (fake) FPGA tools are ready, and starts
 * with the Alhambra II's blinky (the start screen's From Example…).
 */
async function start(page: Page) {
  await page.goto("/");
  await expect(output(page)).toContainText("Ready.");
  await page.getByRole("button", { name: "From Example…" }).click();
  const dialog = page.getByRole("dialog", { name: "Examples" });
  await dialog.getByRole("button", { name: /^blinky/ }).click();
  await dialog.getByRole("button", { name: "Open" }).click();
  await expect(tab(page, "pinout.pcf")).toBeVisible();
}

const menu = (page: Page, id: string) =>
  page.evaluate(
    (id) => (window as unknown as { __fake: { menu: (id: string) => void } }).__fake.menu(id),
    id,
  );

/** Picks a board and opens its blinky from the examples it offers. */
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

test("shows the engine's package install progress", async ({ page }) => {
  await page.goto("/?engineSetup");
  const bar = page.locator(".status-bar");
  await expect(status(page)).toContainText("Installing FPGA tools.");
  await expect(bar).toContainText("Installing oss-cad-suite 42%");
  await expect(bar.locator("progress")).toHaveJSProperty("value", 0.42);
  await expect(output(page)).toContainText("Installing oss-cad-suite...");
  await page.evaluate(() =>
    (
      window as unknown as { __fake: { finishSetup: () => void } }
    ).__fake.finishSetup(),
  );
  await expect(output(page)).toContainText("Ready.");
  await expect(bar.locator("progress")).toHaveCount(0);
});

test("starts with the start screen, and New / Close Project", async ({ page }) => {
  await page.goto("/");
  await expect(output(page)).toContainText("Ready.");
  // No project: the start screen, with the last board in the toolbar.
  const start = page.getByLabel("Start");
  await expect(start).toContainText("Welcome to Logix IDE");
  await expect(boardButton(page)).toHaveText(/Alhambra II/);
  await expect(page.getByTitle(/^Verify/)).toBeDisabled();
  // New Project: the board's pins and an empty design, not blinky.
  await start.getByRole("button", { name: "New Project" }).click();
  await expect(tab(page, "pinout.pcf")).toBeVisible();
  await expect(tab(page, "main_tb.v")).toHaveCount(0);
  await expect(page.locator(".cm-content")).toContainText("assign led = 0;");
  await expect(output(page)).toContainText(/New project\.\s*Ready\./);
  // Close Project: back to the start screen, with the recent projects.
  await page.evaluate(() => {
    (window as unknown as { __fake: { recents: string[] } }).__fake.recents = [
      "/Users/you/Documents/Logix/counter",
    ];
  });
  await menu(page, "close-project");
  await expect(start).toBeVisible();
  await expect(start.getByRole("button", { name: "counter" })).toBeVisible();
  await expect(output(page)).toContainText(/Closed the project\.\s*Ready\./);
  // File > Open Recent > Clear Menu empties the start screen's list too.
  await page.evaluate(() => {
    (window as unknown as { __fake: { recents: string[] } }).__fake.recents = [];
  });
  await menu(page, "recents-changed");
  await expect(start.getByText("Recent projects")).toHaveCount(0);
  await page.evaluate(() => {
    (window as unknown as { __fake: { recents: string[] } }).__fake.recents = [
      "/Users/you/Documents/Logix/counter",
    ];
  });
  await menu(page, "recents-changed");
  await expect(start.getByRole("button", { name: "counter" })).toBeVisible();
  // The board can still be changed; it's used for the next project.
  await pickBoard(page, "iCEBreaker");
  await expect(boardButton(page)).toHaveText(/iCEBreaker/);
  await expect(start).toBeVisible();
  // A recent project brings its own board to the toolbar.
  await start.getByRole("button", { name: "counter" }).click();
  await expect(tab(page, "main.v")).toBeVisible();
  await expect(boardButton(page)).toHaveText(/UPduino/i);
});

test("starts up with the tools ready and the boards listed", async ({ page }) => {
  await start(page);
  await expect(output(page)).toContainText("124 examples available.");
  // Logix Engine's 84 boards, grouped by FPGA architecture. Only the current
  // board's group starts expanded; searching shows matches in all groups.
  await boardButton(page).click();
  await expect(page.locator(".group")).toHaveText([
    /Lattice iCE40\s*31/,
    /Lattice ECP5\s*22/,
    /Gowin\s*8/,
    /Xilinx\s*23/,
  ]);
  await expect(page.getByRole("option")).toHaveCount(31);
  await page.getByRole("button", { name: /Gowin/ }).click();
  await expect(page.getByRole("option")).toHaveCount(39);
  await page.getByRole("button", { name: /Lattice iCE40/ }).click();
  await expect(page.getByRole("option")).toHaveCount(8);
  await page.getByRole("combobox").fill("a");
  await expect(page.getByRole("option")).not.toHaveCount(8);
  await page.keyboard.press("Escape");
  await expect(page.getByRole("listbox")).toBeHidden();
  // The Alhambra II by default, and its blinky (from the examples) is
  // ready to build.
  await expect(boardButton(page)).toHaveText(/Alhambra II/);
  await expect(tab(page, "pinout.pcf")).toBeVisible();
  await expect(page.locator(".welcome")).toHaveCount(0);
  await expect(page.getByTitle(/^Verify/)).toBeEnabled();
  // Idle: no busy indicator, and the status bar shows the last result.
  await expect(status(page)).toHaveText("Opened example alhambra-ii/blinky.");
  await expect(page.locator(".spinner")).toHaveCount(0);
});

test("a new project for a board builds right away", async ({ page }) => {
  await start(page);
  await chooseBoard(page);
  await expect(tab(page, "main_tb.v")).toBeVisible();
  await page.getByTitle(/^Verify/).click();
  await expect(status(page)).toContainText("Build succeeded");
  await expect(status(page)).toContainText("0.4% of logic cells");
  // The output ends with the result, then Ready.
  await expect(output(page)).toContainText(/Build succeeded in .*\s*Ready\.\s*$/);
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

  // The "main.v:1" in the output jumps to the file (the tool's line, and
  // the result line at the end).
  await tab(page, "pinout.pcf").click();
  await expect(page.locator(".file-link", { hasText: "main.v:1" })).toHaveCount(2);
  await page.locator(".file-link", { hasText: "main.v:1" }).first().click();
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

test("tab icons mark testbenches and the board's kind of pin file", async ({ page }) => {
  await start(page);
  await expect(tab(page, "main_tb.v").locator(".tab-icon")).toHaveCount(1);
  await expect(tab(page, "main.v").locator(".tab-icon")).toHaveCount(0);
  // The Alhambra II is iCE40: its pin file is .pcf.
  await expect(tab(page, "pinout.pcf").locator(".tab-icon")).toHaveCount(1);
  await expect(tab(page, "pinout.pcf")).toHaveAttribute("title", /^Pin file: /);
  await page.getByTitle(/^New file/).click();
  await page.locator(".tab-input").fill("extra.lpf");
  await page.keyboard.press("Enter");
  await expect(tab(page, "extra.lpf").locator(".tab-icon")).toHaveCount(0);
  await expect(tab(page, "extra.lpf")).toHaveAttribute(
    "title",
    /another kind of board: not used with this board, whose pin file is a \.pcf file/,
  );
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
  await expect(output(page)).toContainText("$ verible-verilog-format --nofailsafe_success --inplace main.v");
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

test("a board's variant: shown next to it, changed there or in Tools, saved and built with", async ({
  page,
}) => {
  const fake = () =>
    page.evaluate(
      () =>
        (window as unknown as { __fake: { menuState: unknown; taskVariant: unknown; saved: unknown } })
          .__fake,
    ) as Promise<{
      menuState: { variantMenus: { label: string; values: { label: string; checked: boolean }[] }[] };
      taskVariant: unknown;
      saved: { board: string; variant: unknown } | null;
    }>;
  const summary = page.locator(".variant .summary");
  await start(page);
  // Most boards have no choice, but Upload still says where the design went.
  await expect(summary).toHaveCount(0);
  await menu(page, "upload");
  await expect(output(page)).toContainText("Programming the board's flash through its USB port.");
  await expect(output(page)).toContainText(
    "Uploaded to flash: the design stays when the board is powered off.",
  );
  expect((await fake()).menuState.variantMenus).toEqual([]);

  // The iCESugar-Pro can be programmed to flash or to RAM.
  await pickBoard(page, "Icesugar Pro");
  await page.getByRole("dialog").getByRole("button", { name: "Keep Current Files" }).click();
  await expect(summary).toHaveText(/^\s*Flash/);
  const memory = (await fake()).menuState.variantMenus[0];
  expect(memory.label).toBe("Memory");
  expect(memory.values.map((v) => [v.label, v.checked])).toEqual([
    ["Flash", true],
    ["RAM", false],
  ]);

  // The summary opens a panel to change it.
  await summary.click();
  const panel = page.getByRole("dialog", { name: "Icesugar Pro options" });
  await panel.getByRole("radio", { name: "RAM" }).click();
  await expect(summary).toHaveText(/^\s*RAM/);
  await expect(status(page)).toContainText("Icesugar Pro: RAM.");
  await page.keyboard.press("Escape");
  await expect(panel).toBeHidden();

  // Builds use it, and saving stores it.
  await menu(page, "verify");
  await expect(output(page)).toContainText("for Icesugar Pro (RAM)");
  await expect(output(page)).toContainText("Ready.");
  expect((await fake()).taskVariant).toEqual({ connection: "usb", memory: "ram" });
  // After an upload to RAM, a reminder that it's lost at power-off.
  await menu(page, "upload");
  await expect(output(page)).toContainText(
    "Uploaded to RAM: the design is lost when the board is powered off.",
  );
  await menu(page, "save-as");
  await expect(status(page)).toContainText("Saved");
  expect((await fake()).saved).toEqual({
    board: "icesugar-pro",
    variant: { connection: "usb", memory: "ram" },
  });

  // Tools > Memory > Flash: back to the default, stored as nothing.
  await menu(page, "variant:memory:flash");
  await expect(summary).toHaveText(/^\s*Flash/);
  await menu(page, "save");
  await expect(status(page)).toContainText("Saved");
  expect((await fake()).saved).toEqual({ board: "icesugar-pro", variant: {} });
});

test("a value that doesn't go with the others is greyed out, and choosing it switches them", async ({
  page,
}) => {
  const summary = page.locator(".variant .summary");
  await start(page);
  // The ButterStick's USB bootloader writes flash, its JTAG adapters RAM.
  await pickBoard(page, "butterstick r1.0");
  await page.getByRole("dialog").getByRole("button", { name: "Keep Current Files" }).click();
  await expect(summary).toHaveText(/^\s*USB bootloader · Flash/);
  await summary.click();
  const panel = page.getByRole("dialog", { name: "butterstick r1.0 options" });
  const ram = panel.getByRole("radio", { name: "RAM" });
  await expect(ram).toHaveClass(/unavailable/);
  await ram.click();
  await expect(summary).toHaveText(/^\s*FT2232H adapter · RAM/);
  // The selected values explain themselves.
  await expect(panel).toContainText("lost when the board is powered off");
  await expect(panel).toContainText("FT2232H chip");
  await expect(status(page)).toContainText("Connection: FT2232H adapter changed with it");
  await expect(panel.getByRole("radio", { name: "USB bootloader" })).toHaveClass(/unavailable/);
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

test("changing the board offers your recent projects for it", async ({ page }) => {
  await start(page);
  await chooseBoard(page);
  await page.evaluate(() => {
    (window as unknown as { __fake: { recents: string[] } }).__fake.recents = [
      "/Users/you/Documents/Logix/counter",
    ];
  });
  // No recent projects for the iCEBreaker: only its examples.
  await pickBoard(page, "iCEBreaker");
  const dialog = page.getByRole("dialog", { name: "Examples" });
  await expect(dialog.getByRole("button", { name: /^blinky/ })).toBeVisible();
  await expect(dialog.getByText("Recent projects", { exact: true })).toHaveCount(0);
  await dialog.getByRole("button", { name: "Keep Current Files" }).click();
  // The fake's recent projects are UPduino v2 projects: listed first.
  await pickBoard(page, "UPDuino v2.0");
  await expect(dialog.getByText("Recent projects", { exact: true })).toBeVisible();
  await expect(dialog.getByRole("heading")).toHaveText("Projects and examples for UPDuino v2.0");
  await dialog.getByRole("button", { name: /counter/ }).click();
  await dialog.getByRole("button", { name: "Open" }).click();
  await expect(dialog).toBeHidden();
  await expect(status(page)).toHaveText("Opened /Users/you/Documents/Logix/counter.");
  await expect(boardButton(page)).toHaveText(/UPDuino/);
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
  await expect(status(page)).toHaveText("Opened Bare Minimum for Alhambra II.");
  // Like every command, its output ends with the result, then "Ready.".
  await expect(output(page)).toContainText(
    /Opened Bare Minimum for Alhambra II\.\s*Ready\./,
  );
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
  // In main.v; the fake has no live checker, so Go to Definition is off.
  expect(await editorMenu()).toEqual({
    project: true,
    verilog: true,
    comment: true,
    definition: false,
    testbenches: true,
    simulate: true,
    testbenchOpen: false,
    variantMenus: [],
  });
  const first = page.locator(".cm-line").first();
  const before = (await first.textContent()) ?? "";
  await menu("toggle-comment");
  await expect(first).toHaveText(`// ${before}`);
  await menu("toggle-comment");
  await expect(first).toHaveText(before);
});

test("file commands are enabled only where they work", async ({ page }) => {
  await start(page);
  type Fake = { menuState: Record<string, boolean> | null };
  const menuState = () =>
    page.evaluate(() => (window as unknown as { __fake: Fake }).__fake.menuState);
  const runTests = page.getByTitle(/^Run tests/);
  const simulate = page.getByTitle(/^Simulate/);
  // Blinky: main.v open, one testbench.
  await expect.poll(menuState).toMatchObject({ verilog: true, testbenches: true, simulate: true });
  await expect(runTests).toBeEnabled();
  await expect(simulate).toBeEnabled();
  // A testbench: its own Run This Test in the editor's right-click menu.
  await tab(page, "main_tb.v").click();
  await expect.poll(menuState).toMatchObject({ testbenchOpen: true });
  await menu(page, "run-this-test");
  await expect(page.locator(".status-text")).toContainText("Tests passed");
  // A pin file: Comment/Uncomment with its own line comment, but no Auto
  // Format.
  await tab(page, "pinout.pcf").click();
  await expect.poll(menuState).toMatchObject({
    verilog: false,
    comment: true,
    definition: false,
  });
  const first = page.locator(".cm-line").first();
  const before = (await first.textContent()) ?? "";
  await first.click();
  await menu(page, "toggle-comment");
  await expect(first).toHaveText(`# ${before}`);
  await menu(page, "toggle-comment");
  await expect(first).toHaveText(before);
  // A new project has no testbench: no Run Tests or Simulate.
  await menu(page, "new");
  await expect(tab(page, "main_tb.v")).toHaveCount(0);
  await expect.poll(menuState).toMatchObject({
    verilog: true,
    testbenches: false,
    simulate: false,
  });
  await expect(runTests).toBeDisabled();
  await expect(simulate).toBeDisabled();
  // No project: nothing that needs one.
  await menu(page, "close-project");
  await expect.poll(menuState).toEqual({
    project: false,
    verilog: false,
    comment: false,
    definition: false,
    testbenches: false,
    simulate: false,
    testbenchOpen: false,
    variantMenus: [],
  });
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

test("the output comes back only on errors if the setting says so", async ({ page }) => {
  await start(page);
  await menu(page, "settings");
  const dialog = page.getByRole("dialog", { name: "Settings" });
  await expect(dialog.getByLabel("Show the output:")).toHaveValue("onCommand");
  await dialog.getByLabel("Show the output:").selectOption("onError");
  await dialog.getByRole("button", { name: "OK" }).click();
  await expect(dialog).toBeHidden();

  await page.getByRole("button", { name: /Hide/ }).click();
  await expect(output(page)).toBeHidden();
  // A build that succeeds leaves it hidden...
  await page.getByTitle(/^Verify/).click();
  await expect(status(page)).toContainText("Build succeeded");
  await expect(output(page)).toBeHidden();
  // ...and one that fails brings it back.
  await typeAtStart(page, "oops\n");
  await page.getByTitle(/^Verify/).click();
  await expect(status(page)).toContainText("Build failed");
  await expect(output(page)).toBeVisible();
});

test("dragging the divider to the bottom hides the output", async ({ page }) => {
  await start(page);
  const divider = page.locator(".divider");
  const before = await page.locator(".console-area").boundingBox();
  const box = (await divider.boundingBox())!;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width / 2, page.viewportSize()!.height - 2, {
    steps: 5,
  });
  await page.mouse.up();
  await expect(output(page)).toBeHidden();
  await expect(page.locator(".status-bar")).toBeVisible();
  // Show brings it back at its earlier height.
  await page.getByRole("button", { name: /Show/ }).click();
  await expect(output(page)).toBeVisible();
  const after = await page.locator(".console-area").boundingBox();
  expect(Math.round(after!.height)).toBe(Math.round(before!.height));
  // Hidden again, the divider can be dragged back up to show it.
  await page.getByRole("button", { name: /Hide/ }).click();
  await expect(output(page)).toBeHidden();
  const low = (await divider.boundingBox())!;
  await page.mouse.move(low.x + low.width / 2, low.y + low.height / 2);
  await page.mouse.down();
  await page.mouse.move(low.x + low.width / 2, low.y - 200, { steps: 5 });
  await page.mouse.up();
  await expect(output(page)).toBeVisible();
  await expect(page.getByRole("button", { name: /Hide/ })).toBeVisible();
  const dragged = await page.locator(".console-area").boundingBox();
  expect(dragged!.height).toBeGreaterThan(150);
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
