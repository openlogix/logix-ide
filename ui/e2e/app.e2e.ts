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
  await dialog.getByRole("button", { name: "New Project" }).click();
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
  await dialog.getByRole("button", { name: "New Project" }).click();
  await expect(tab(page, "pinout.pcf")).toBeVisible();
}

/**
 * Project options after a board change: choose the current project and
 * Change the Board (nothing is selected at first).
 */
async function changeTheBoard(page: Page) {
  const dialog = page.getByRole("dialog");
  await dialog.getByRole("button", { name: /^Change the board of/ }).click();
  await dialog.getByRole("button", { name: "Change the Board", exact: true }).click();
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
  // The start screen's output: Logix's own first lines (opening a project
  // clears them).
  await page.goto("/");
  await expect(output(page)).toContainText("119 examples available.");
  await start(page);
  await expect(output(page)).not.toContainText("119 examples available.");
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

test("Show Project Folder on a project never saved offers Save As…, then shows the folder", async ({
  page,
}) => {
  type Fake = {
    shownFolder: string | null;
    confirms: { title: string; okLabel: string; kind: string }[];
    confirmAnswer: boolean;
  };
  const fake = () => page.evaluate(() => (window as unknown as { __fake: Fake }).__fake);
  const answer = (yes: boolean) =>
    page.evaluate((yes) => {
      (window as unknown as { __fake: Fake }).__fake.confirmAnswer = yes;
    }, yes);
  await start(page);

  // Cancel: nothing saved, no folder shown.
  await answer(false);
  await menu(page, "show-folder");
  await expect.poll(async () => (await fake()).confirms.length).toBe(1);
  expect((await fake()).confirms[0]).toEqual({
    title: "This project hasn't been saved yet",
    okLabel: "Save As…",
    kind: "info",
  });
  expect((await fake()).shownFolder).toBeNull();

  // Save As…: saved, then its new folder is shown.
  await answer(true);
  await menu(page, "show-folder");
  await expect.poll(async () => (await fake()).shownFolder).toMatch(/^\/fake\/projects\//);
  await expect(status(page)).toContainText("Saved");

  // Once saved, it shows the folder without asking.
  await menu(page, "show-folder");
  await expect.poll(async () => (await fake()).confirms.length).toBe(2);
});

test("Copy Project Folder Path offers Save As… first, then copies the path and says so", async ({
  page,
}) => {
  type Fake = { copiedPath: string | null; shownFolder: string | null; confirms: unknown[] };
  const fake = () => page.evaluate(() => (window as unknown as { __fake: Fake }).__fake);
  await start(page);
  await menu(page, "copy-path");
  await expect.poll(async () => (await fake()).copiedPath).toMatch(/^\/fake\/projects\//);
  expect((await fake()).confirms.length).toBe(1);
  expect((await fake()).shownFolder).toBeNull();
  // The status line shows the text copied, as quoted for the shell.
  await expect(status(page)).toContainText(
    `Copied the project folder's path: '${(await fake()).copiedPath}'`,
  );
});

test("switching projects clears the output", async ({ page }) => {
  await start(page);
  await menu(page, "verify");
  await expect(status(page)).toContainText("Build succeeded");
  await expect(output(page)).toContainText("Build succeeded");
  // Another project: only its own lines.
  await menu(page, "new");
  await expect(output(page)).toContainText(/^\s*New project\.\s*Ready\.\s*$/);
  // Closing it, too.
  await menu(page, "verify");
  await expect(status(page)).toContainText("Build succeeded");
  await menu(page, "close-project");
  await expect(output(page)).toContainText(/^\s*Closed the project\.\s*Ready\.\s*$/);
});

test("a programmer's progress stays on one line, as in a terminal", async ({ page }) => {
  await start(page);
  await menu(page, "upload");
  // While it runs: its latest state, shown at once.
  await expect(output(page)).toContainText("addr 0x000100  50%");
  await expect(output(page)).not.toContainText("addr 0x000000");
  // Then printed over by the programmer's next line.
  await expect(output(page)).toContainText("programming..");
  await expect(output(page)).toContainText(/programming\.\.\s*done\./);
  await expect(output(page)).not.toContainText("addr 0x");
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

  // Editing the file clears what Verify found in it, marker and tab mark.
  await page.keyboard.press("End");
  await page.keyboard.type(" ");
  await expect(page.locator(".cm-lintRange-error")).toHaveCount(0);
  await expect(tab(page, "main.v")).not.toHaveClass(/has-error/);
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
  await expect(dialog.getByRole("heading")).toHaveText("Project options for board “Alhambra II”");
  // The same board: keep the project, not "change" it.
  await expect(dialog.getByRole("button", { name: /^Change the board of/ })).toHaveCount(0);
  await dialog.getByRole("button", { name: /^Keep the current project, with board “Alhambra II”/ }).click();
  await dialog.getByRole("button", { name: "Keep the Project", exact: true }).click();
  await expect(dialog).toBeHidden();
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
  // Every board shows how it's programmed, even with nothing to choose.
  await expect(summary).toHaveText(/^\s*USB · Flash/);
  const plain = (await fake()).menuState.variantMenus;
  expect(plain.map((m) => m.values.map((v) => [v.label, v.checked]))).toEqual([
    [["USB", true]],
    [["Flash", true]],
  ]);
  await menu(page, "upload");
  // The engine says it when programming starts, after the build.
  await expect(output(page)).toContainText(
    /yosys[\s\S]*Programming the flash of the Alhambra II through its USB port\.[\s\S]*Programmed/,
  );
  await expect(output(page)).toContainText(
    "Programmed the flash of the Alhambra II: the design stays when the board is powered off.",
  );

  // The iCESugar-Pro can be programmed to flash or to RAM.
  await pickBoard(page, "Icesugar Pro");
  await changeTheBoard(page);
  await expect(summary).toHaveText(/^\s*USB · Flash/);
  const memory = (await fake()).menuState.variantMenus[1];
  expect(memory.label).toBe("Memory");
  expect(memory.values.map((v) => [v.label, v.checked])).toEqual([
    ["Flash", true],
    ["RAM", false],
  ]);

  // The summary opens a panel to change it.
  await summary.click();
  const panel = page.getByRole("dialog", { name: "Icesugar Pro options" });
  await panel.getByRole("radio", { name: "RAM" }).click();
  await expect(summary).toHaveText(/^\s*USB · RAM/);
  await expect(status(page)).toContainText("Icesugar Pro: USB · RAM.");
  await page.keyboard.press("Escape");
  await expect(panel).toBeHidden();

  // Builds use it, and saving stores it.
  await menu(page, "verify");
  await expect(output(page)).toContainText("for Icesugar Pro (USB · RAM)");
  await expect(output(page)).toContainText("Ready.");
  expect((await fake()).taskVariant).toEqual({ connection: "usb", memory: "ram" });
  // After an upload to RAM, a reminder that it's lost at power-off.
  await menu(page, "upload");
  await expect(output(page)).toContainText(
    "Programmed the RAM of the Icesugar Pro: the design is lost when the board is powered off.",
  );
  await menu(page, "save-as");
  await expect(status(page)).toContainText("Saved");
  expect((await fake()).saved).toEqual({
    board: "icesugar-pro",
    variant: { connection: "usb", memory: "ram" },
  });

  // Tools > Memory > Flash: back to the default, stored as nothing.
  await menu(page, "variant:memory:flash");
  await expect(summary).toHaveText(/^\s*USB · Flash/);
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
  await changeTheBoard(page);
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

test("changing the board offers project options: the current project first", async ({ page }) => {
  await start(page);
  await chooseBoard(page);
  await typeAtStart(page, "// mine\n");
  // Recent projects aren't offered, whatever their board.
  await page.evaluate(() => {
    (window as unknown as { __fake: { recents: string[] } }).__fake.recents = [
      "/Users/you/Documents/Logix/counter",
    ];
  });
  await pickBoard(page, "iCEBreaker");
  const dialog = page.getByRole("dialog", { name: "Examples" });
  await expect(dialog.getByRole("heading")).toHaveText("Project options for board “iCEBreaker”");
  await expect(dialog.getByText("Recent projects", { exact: true })).toHaveCount(0);
  await expect(dialog.getByRole("button", { name: /counter/ })).toHaveCount(0);
  // Until a choice is made, the toolbar keeps the current board.
  await expect(boardButton(page)).toHaveText(/Alhambra II/);
  await expect(dialog.getByRole("list", { name: "Boards" })).toHaveCount(0);
  // Few choices: no search.
  await expect(dialog.getByRole("searchbox")).toHaveCount(0);
  // The current project, with what the change means; nothing is selected
  // until the user chooses.
  const current = dialog.getByRole("button", { name: /^Change the board of the current project to “iCEBreaker”/ });
  await expect(current).not.toHaveClass(/selected/);
  await expect(dialog.locator("button.selected")).toHaveCount(0);
  // The change, then its implications, the risky ones red.
  await expect(current.locator(".note .point")).toHaveText([
    "FPGA: ICE40HX4K-TQ144 → ICE40UP5K-SG48.",
    /^FPGA-specific parts/,
    /^The design may not fit/,
    /^The pins, clock frequency and LED and button wiring may differ/,
  ]);
  // Every implication is red.
  await expect(current.locator(".note .point.risky")).toHaveCount(3);
  await expect(dialog.getByText("New project", { exact: true })).toBeVisible();
  // The main button waits for a choice, then follows it; Enter does
  // nothing before.
  const main = dialog.locator("footer .primary");
  await expect(main).toHaveText("Choose an Option");
  await expect(main).toBeDisabled();
  await page.keyboard.press("Enter");
  await expect(dialog).toBeVisible();
  await current.click();
  await expect(main).toHaveText("Change the Board");
  await dialog.getByRole("button", { name: /^blinky/ }).click();
  await expect(main).toHaveText("New Project");
  await expect(current).not.toHaveClass(/selected/);
  // Cancel keeps the board as it was.
  await dialog.getByRole("button", { name: "Cancel" }).click();
  await expect(dialog).toBeHidden();
  await expect(boardButton(page)).toHaveText(/Alhambra II/);
  // Another family: another kind of pin file, and probably more.
  await pickBoard(page, "ColorLight-5A-75B-V8");
  const other = dialog.getByRole("button", { name: /^Change the board of/ });
  await expect(other.locator(".note .point").first()).toHaveText(
    "Architecture: Lattice iCE40 → Lattice ECP5.",
  );
  await expect(other.locator(".note .point.risky")).toHaveText([
    "Needs a .lpf pin file instead of pinout.pcf, with this board's pins.",
    /^Probably significant changes/,
    /^The clock frequency/,
  ]);
  await page.keyboard.press("Escape");
  await expect(boardButton(page)).toHaveText(/Alhambra II/);
  // Change the Board: the files stay, the board changes.
  await pickBoard(page, "iCEBreaker");
  await changeTheBoard(page);
  await expect(dialog).toBeHidden();
  await expect(page.locator(".cm-content")).toContainText("// mine");
  await expect(boardButton(page)).toHaveText(/iCEBreaker/);
});

test("shows if the board is connected, and lists recently used boards first", async ({ page }) => {
  await start(page);
  await chooseBoard(page);
  const link = page.locator(".board-link");
  await expect(link).toHaveText("Not connected");
  await page.evaluate(() => {
    const w = window as unknown as { __fake: { connected: unknown[] } };
    w.__fake.connected = [
      {
        id: "usb:1-2:0403:6010:FTATANKC",
        name: "Alhambra II v1.0A",
        serial: "FTATANKC",
        boards: [{ board: "alhambra-ii", level: "strong", variants: [{}] }],
      },
    ];
    window.dispatchEvent(new Event("focus"));
  });
  await expect(link).toHaveText("Connected");
  await expect(link).toHaveAttribute("title", /may be the Alhambra II/);
  await boardButton(page).click();
  // No groups from connected devices: only the architectures.
  await expect(page.locator(".group", { hasText: "Connected" })).toHaveCount(0);
  await expect(page.locator(".group").first()).toHaveText(/Lattice iCE40/);
  await page.keyboard.press("Escape");

  // The device is a strong match for the Basys3 and a weak one for the
  // Arty: the status still says Connected (erring to that side: Upload
  // checks), and the selected Arty is listed under the device, after the
  // Basys3.
  await page.evaluate(() => {
    const w = window as unknown as { __fake: { connected: unknown[] } };
    w.__fake.connected = [
      {
        id: "usb:1-3:0403:6010:210319AFEF8F",
        name: "Digilent USB Device",
        serial: "210319AFEF8F",
        boards: [
          {
            board: "basys3",
            level: "strong",
            variants: [
              { connection: "usb", memory: "flash" },
              { connection: "usb", memory: "ram" },
            ],
          },
          {
            board: "arty-a7-35t",
            level: "weak",
            // Both of its configurations use the same USB port.
            variants: [
              { connection: "usb", memory: "flash" },
              { connection: "usb", memory: "ram" },
            ],
          },
        ],
      },
    ];
    window.dispatchEvent(new Event("focus"));
  });
  await pickBoard(page, "Arty A7-35T (Xilinx, Artix7)");
  await changeTheBoard(page);
  await expect(link).toHaveText("Connected");
  await expect(link).toHaveAttribute("title", /may be the Arty A7-35T/);
  await page.keyboard.press("Escape");

  // The boards picked come first, under "Recently used", newest first.
  await pickBoard(page, "iCEBreaker");
  await changeTheBoard(page);
  await boardButton(page).click();
  const recent = page.locator(".group").first();
  await expect(recent).toHaveText(/^\s*Recently used\s*2\s*$/);
  await expect(page.getByRole("option").nth(0)).toHaveText(/iCEBreaker/);
  await expect(page.getByRole("option").nth(1)).toHaveText(/Arty A7-35T/);
  // It collapses like an architecture's group.
  const toggle = recent.getByRole("button");
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  await expect(page.getByRole("option").first()).not.toHaveText(/iCEBreaker/);
  await toggle.click();
  await expect(page.getByRole("option").first()).toHaveText(/iCEBreaker/);
});

test("opens a board's bare-minimum from the examples", async ({ page }) => {
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
  await dialog.getByRole("button", { name: /^bare-minimum/ }).click();
  await dialog.getByRole("button", { name: "New Project" }).click();
  await expect(dialog).toBeHidden();
  await expect(tab(page, "pinout.pcf")).toBeVisible();
  await expect(tab(page, "main_tb.v")).toHaveCount(0);
  await expect(page.locator(".cm-content")).toContainText("assign led = 0;");
  await expect(status(page)).toHaveText("Opened bare-minimum for Alhambra II.");
  // Like every command, its output ends with the result, then "Ready.".
  await expect(output(page)).toContainText(
    /Opened bare-minimum for Alhambra II\.\s*Ready\./,
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
  expect(await editorMenu()).toMatchObject({
    project: true,
    formattable: true,
    comment: true,
    definition: false,
    testbenches: true,
    simulate: true,
    testbenchOpen: false,
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
  await expect.poll(menuState).toMatchObject({ formattable: true, testbenches: true, simulate: true });
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
    formattable: false,
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
    formattable: true,
    testbenches: false,
    simulate: false,
  });
  await expect(runTests).toBeDisabled();
  await expect(simulate).toBeDisabled();
  // No project: nothing that needs one.
  await menu(page, "close-project");
  await expect.poll(menuState).toMatchObject({
    project: false,
    formattable: false,
    comment: false,
    definition: false,
    testbenches: false,
    simulate: false,
    testbenchOpen: false,
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

test("Clear empties the output and the status, not the problem marks", async ({ page }) => {
  await start(page);
  await chooseBoard(page);
  await typeAtStart(page, "oops\n");
  await page.getByTitle(/^Verify/).click();
  await expect(status(page)).toContainText("Build failed");
  await expect(tab(page, "main.v").locator(".problem-icon.error")).toBeVisible();
  await expect(tab(page, "main.v")).toHaveAttribute("title", /^1 error\n/);
  await page.getByRole("button", { name: "Clear", exact: true }).click();
  await expect(output(page)).toHaveText("");
  await expect(status(page)).toHaveText("Ready.");
  await expect(tab(page, "main.v").locator(".problem-icon.error")).toBeVisible();
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

test("a local engine is shown for the whole session", async ({ page }) => {
  await page.goto("/?devEngine");
  const banner = page.getByRole("status").filter({ hasText: "local Logix Engine" });
  await expect(banner).toContainText(
    "Using a local Logix Engine, for diagnosis: /Users/you/logix-engine/target/debug/logix-engine",
  );
  // Still there once setup is done.
  await expect(output(page)).toContainText("Ready.");
  await expect(banner).toBeVisible();
});

test("Upload asks which device when several fit the board", async ({ page }) => {
  await start(page);
  await page.evaluate(() => {
    const f = (window as unknown as { __fake: { uploadDevices: unknown[] } }).__fake;
    const device = (serial: string, level: string) => ({
      id: `usb:1-${serial}:0403:6010:${serial}`,
      vid: "0403", pid: "6010", manufacturer: "AlhambraBits",
      product: "Alhambra II v1.0A", serialNumber: serial,
      boards: [{ board: "alhambra-ii", level }], buildMatch: level,
    });
    f.uploadDevices = [device("A1", "strong"), device("B2", "strong")];
  });
  await menu(page, "upload");
  const picker = page.getByRole("dialog", { name: "Choose a Device" });
  await expect(picker).toContainText("2 connected devices could be the Alhambra II");
  await expect(picker).toContainText("serial B2 · Strong match");
  await expect(status(page)).toContainText("Choose the device to upload to.");
  await picker.getByRole("listitem").filter({ hasText: "serial B2" }).getByRole("button", { name: "Upload" }).click();
  await expect(picker).toBeHidden();
  await expect(output(page)).toContainText("Programmed the flash of the Alhambra II");
  expect(
    await page.evaluate(
      () => (window as unknown as { __fake: { uploadedDevice: string } }).__fake.uploadedDevice,
    ),
  ).toBe("usb:1-B2:0403:6010:B2");
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
  // The device is a group, open, with its matches as a bullet list.
  const device = dialog.getByRole("button", { name: /Alhambra II v1.0A/ });
  await expect(device).toHaveAttribute("aria-expanded", "true");
  const matches = dialog.getByRole("button", { name: /^Matches/ });
  await expect(matches).toHaveAttribute("aria-expanded", "true");
  await expect(dialog.locator("ul.boards li")).toHaveText(["Alhambra II (selected)"]);
  // A strong match: the weaker ones aren't listed.
  await expect(dialog.getByRole("button", { name: /^Other possible matches/ })).toHaveCount(0);
  // The device collapses, to skip it.
  await device.click();
  await expect(device).toHaveAttribute("aria-expanded", "false");
  await expect(matches).toHaveCount(0);
  await page.keyboard.press("Escape");

  // A device with no strong match: its boards under Other possible
  // matches, closed until clicked.
  await page.evaluate(() => {
    (window as unknown as { __fake: { usb: unknown[] } }).__fake.usb = [
      {
        id: "usb:1-4:0403:6010:", vid: "0403", pid: "6010", manufacturer: "FTDI",
        product: "Dual RS232-HS", serialNumber: "", buildMatch: null,
        boards: [{ board: "alchitry-au", level: "weak" }, { board: "icestick", level: "weak" }],
      },
    ];
    (window as unknown as { __fake: { menu: (id: string) => void } }).__fake.menu("devices");
  });
  const others = dialog.getByRole("button", { name: /^Other possible matches\s*2$/ });
  await expect(others).toHaveAttribute("aria-expanded", "false");
  await others.click();
  await expect(dialog.getByRole("button", { name: /^Alchitry Au/ })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
  // The toolbar's Connected / Not connected status opens it too.
  await page.getByRole("button", { name: /^(Connected|Not connected)$/ }).click();
  await expect(dialog).toBeVisible();
});

test("a project's custom-boards.json, edited outside Logix, adds its own boards", async ({ page }) => {
  type Fake = {
    customBoardsOnDisk: Record<string, string>;
    taskBoardsFolder: unknown;
    scannedBoardsFolder: unknown;
  };
  const folder = "/Users/you/Documents/Logix/mine";
  const myBoards = {
    "my-upduino": {
      description: "My UPduino",
      fpga_id: "ice40up5k-sg48",
      variants: [
        {
          selector: { connection: "usb", memory: "flash" }, programmer: "iceprog",
          args: "-d d:${BUS}/${DEV} ${BITSTREAM}",
          usb: { vid: "0403", pid: "6014" }, tested: false,
        },
      ],
    },
  };
  /** Writes (or, with null, deletes) the file on the fake disk, as another editor would. */
  const writeFile = async (text: string | null) => {
    await page.evaluate(
      ([dir, t]) => {
        const disk = (window as unknown as { __fake: Fake }).__fake.customBoardsOnDisk;
        if (t === null) delete disk[dir as string];
        else disk[dir as string] = t;
        // The IDE checks the file when the window gets the focus back.
        window.dispatchEvent(new Event("focus"));
      },
      [folder, text],
    );
  };
  await page.goto(`/?recents=${folder}`);
  await writeFile(JSON.stringify(myBoards, null, 2));
  await expect(output(page)).toContainText("Ready.");
  await page.getByRole("button", { name: /mine/ }).click();
  // Not a tab: it's edited outside Logix.
  await expect(tab(page, "main.v")).toBeVisible();
  await expect(tab(page, "custom-boards.json")).toHaveCount(0);

  // Listed first in "Custom boards" (expanded), and first in its
  // architecture's group too, with the board icon and a tooltip saying
  // where it comes from.
  await boardButton(page).click();
  await expect(page.locator(".group").first()).toContainText("Custom boards");
  await expect(
    page.locator(".group").first().locator("xpath=following-sibling::li[1]"),
  ).toHaveText(/My UPduino/);
  // (The current board's group, iCE40, opens by itself.)
  const first = page
    .locator(".group", { hasText: "Lattice iCE40" })
    .locator("xpath=following-sibling::li[1]");
  await expect(first).toHaveText(/My UPduino/);
  await expect(first.locator(".custom")).toHaveCount(1);
  await expect(first).toHaveAttribute("title", /custom-boards\.json/);
  await page.getByRole("combobox").fill("My UPduino");
  await page.getByRole("option", { name: "My UPduino", exact: true }).first().click();
  // Its project options: the current project, with the change note, and
  // no examples or new projects, saying why.
  const options = page.getByRole("dialog", { name: "Examples" });
  await expect(options.getByRole("heading")).toHaveText("Project options for custom board “My UPduino”");
  await expect(options.locator(".current .note .point").first()).toHaveText(/^Board: UPDuino v2\.0 → My UPduino\. Same FPGA: /);
  await expect(options.locator(".custom-note")).toContainText("Custom boards have no examples");
  await expect(options.getByRole("button", { name: /^bare-minimum/ })).toHaveCount(0);
  await expect(options.getByRole("button", { name: /^blinky/ })).toHaveCount(0);
  // Not applied until chosen.
  await expect(boardButton(page)).toHaveText(/UPDuino v2/);
  await changeTheBoard(page);
  await expect(options).toBeHidden();
  await expect(boardButton(page)).toHaveText(/My UPduino/);
  await expect(boardButton(page).locator(".custom")).toHaveCount(1);
  await expect(boardButton(page)).toHaveAttribute(
    "title",
    "Custom board: My UPduino\nFrom this project's custom-boards.json",
  );

  // The project folder goes with every task and device scan (the engine
  // reads the file there), and the output says where the board is from.
  await menu(page, "verify");
  await expect(status(page)).toContainText("Build succeeded");
  await expect(output(page)).toContainText("The board is from this project's custom-boards.json.");
  const fake = () => page.evaluate(() => (window as unknown as { __fake: Fake }).__fake);
  expect((await fake()).taskBoardsFolder).toBe(folder);
  await expect.poll(async () => (await fake()).scannedBoardsFolder).toBe(folder);

  // Broken in another editor: its problems go to the output at line and
  // column, the board list says so in one line and leaves its boards out,
  // and there's no banner.
  await writeFile("oops");
  // At the file's full path: it's edited outside Logix. Noticed in the
  // background, so "Ready." comes again after it: it's always last when idle.
  await expect(output(page)).toContainText(`${folder}/custom-boards.json:1:1: error: Expected value.`);
  await expect(output(page)).toContainText(/Expected value\.\s*Ready\.\s*$/);
  await expect(page.locator(".banner")).toHaveCount(0);
  await boardButton(page).click();
  // The note is in the Custom boards group, which stays, empty.
  await expect(page.locator(".problem-note")).toContainText("custom-boards.json has problems");
  await expect(page.locator(".group").first()).toContainText("Custom boards");
  await expect(page.locator(".group").first().locator(".count")).toHaveText("0");
  await page.getByRole("combobox").fill("My UPduino");
  await expect(page.getByRole("option", { name: "My UPduino", exact: true })).toHaveCount(0);
  await page.keyboard.press("Escape");
  // A task stops with the same problems.
  await menu(page, "verify");
  await expect(output(page)).toContainText(`${folder}/custom-boards.json:1:1: error: Expected value.`);
  // The status line counts them rather than repeating them.
  await expect(status(page)).toHaveText(
    "Build failed: the project's custom-boards.json has problems.",
  );

  // Fixed: said once, and the boards are back.
  await writeFile(JSON.stringify(myBoards));
  await expect(output(page)).toContainText("custom-boards.json has no problems now.");
  await expect(output(page)).toContainText(/has no problems now\.\s*Ready\.\s*$/);
  await boardButton(page).click();
  await expect(page.locator(".problem-note")).toHaveCount(0);
  // In Custom boards and in its iCE40 group.
  await expect(page.getByRole("option", { name: "My UPduino", exact: true })).toHaveCount(2);
  await page.keyboard.press("Escape");

  // Deleted: its boards and the group go.
  await writeFile(null);
  await boardButton(page).click();
  await expect(page.locator(".group", { hasText: "Custom boards" })).toHaveCount(0);
  await page.getByRole("combobox").fill("My UPduino");
  await expect(page.getByRole("option", { name: "My UPduino", exact: true })).toHaveCount(0);
});

test("Simulate says Opening GTKWave… until its window appears", async ({ page }) => {
  await start(page);
  const fake = (found: boolean) =>
    page.evaluate(
      (f) => (window as unknown as { __fake: { gtkwaveOpened: (f: boolean) => void } }).__fake.gtkwaveOpened(f),
      found,
    );
  await menu(page, "simulate");
  // The result goes to the output; the status waits for GTKWave, busy.
  await expect(status(page)).toHaveText("Opening GTKWave…");
  await expect(page.locator(".status-bar .spinner")).toBeVisible();
  await expect(output(page)).toContainText(/Simulation done in [\d.]+ s\. GTKWave shows the signals\./);
  await fake(true);
  await expect(status(page)).toHaveText(/^Simulation done in [\d.]+ s\. GTKWave shows the signals\.$/);
  await expect(page.locator(".status-bar .spinner")).toHaveCount(0);
  // Not found within 30 s: it may open behind Logix.
  await menu(page, "simulate");
  await expect(status(page)).toHaveText("Opening GTKWave…");
  await fake(false);
  await expect(status(page)).toHaveText(/GTKWave is taking a while to start; it may open behind this window\.$/);
  // Another command's status isn't replaced by a late answer.
  await menu(page, "simulate");
  await expect(status(page)).toHaveText("Opening GTKWave…");
  await menu(page, "verify");
  await expect(status(page)).toContainText("Build succeeded");
  await fake(true);
  await expect(status(page)).toContainText("Build succeeded");
});

test("Board Info shows the board's FPGA and the boards with the same FPGA", async ({ page }) => {
  await start(page);
  // The board picker: the FPGA as a hover hint, and search by it.
  await boardButton(page).click();
  await page.getByRole("combobox").fill("hx4k");
  const alhambra = page.getByRole("option", { name: "Alhambra II", exact: true }).first();
  await expect(alhambra).toHaveAttribute("title", "FPGA: Lattice iCE40 ICE40HX4K-TQ144 (8k)");
  await page.keyboard.press("Escape");

  // The ⓘ next to the board, and Tools > Board Info.
  await page.getByRole("button", { name: "Board info" }).click();
  const info = page.getByRole("dialog", { name: "Board Info" });
  await expect(info).toContainText("ICE40HX4K-TQ144");
  await expect(info).toContainText("Lattice iCE40");
  await expect(info).toContainText(".pcf");
  await expect(info).toContainText("USB · Flash (default)");
  await page.keyboard.press("Escape");
  await expect(info).toBeHidden();
  await menu(page, "board-info");
  await expect(info).toBeVisible();
  await expect(info).toContainText("The selected board");
  // Every board with this FPGA, the shown and selected one marked.
  const list = info.locator("li", { hasText: /Alhambra II|Kéfir I/ });
  await expect(list).toHaveText(["Alhambra II (shown, selected)", "Kéfir I iCE40-HX4K"]);
  const back = info.getByRole("button", { name: "Back" });
  await expect(back).toHaveCount(0);
  // A board with the same FPGA shows its own info; Back returns.
  await info.getByRole("button", { name: /^Kéfir I/ }).click();
  await expect(info.getByRole("heading", { level: 2 })).toHaveText(/^Board: Kéfir I/);
  // The same list, the marks moved.
  await expect(list).toHaveText(["Alhambra II (selected)", "Kéfir I iCE40-HX4K (shown)"]);
  await expect(info.getByRole("button", { name: "Alhambra II" })).toBeVisible();
  await back.click();
  await expect(info.getByRole("heading", { level: 2 })).toHaveText("Board: Alhambra II");
  await expect(back).toHaveCount(0);
  // Select This Board picks the board shown, as the board picker does.
  await info.getByRole("button", { name: /^Kéfir I/ }).click();
  await info.getByRole("button", { name: "Select This Board" }).click();
  await expect(info).toBeHidden();
  await changeTheBoard(page);
  await expect(boardButton(page)).toHaveText(/Kéfir I/);
});

test("examples show their summary and notes, selected or not", async ({ page }) => {
  await page.goto("/");
  await expect(output(page)).toContainText("Ready.");
  await page.getByRole("button", { name: "From Example…" }).click();
  const dialog = page.getByRole("dialog", { name: "Examples" });
  await dialog.getByRole("searchbox").fill("icenode");
  const blinky = dialog.getByRole("button", { name: /^blinky/ });
  await expect(blinky.locator(".point")).toHaveText(["Blinks an LED", "Not yet tried on the board."]);
  // Not selected: the note still shows.
  await dialog.getByRole("button", { name: /^bare-minimum/ }).click();
  await expect(blinky).not.toHaveClass(/selected/);
  await expect(blinky.locator(".point")).toHaveText(["Blinks an LED", "Not yet tried on the board."]);
});

test("a failed command's banner says why, with one red line", async ({ page }) => {
  await start(page);
  await page.evaluate(() => {
    (window as unknown as { __fake: { saveAsError: unknown } }).__fake.saveAsError = {
      message: "Couldn't save the project.",
      details:
        "Couldn't save the project.: /Users/you/fpga/blinky is inside a Logix project. Choose another location.",
    };
  });
  await menu(page, "save-as");
  // The reason is in the banner, so the closed output doesn't hide it.
  await expect(page.getByRole("alert")).toContainText(
    "Couldn't save the project. /Users/you/fpga/blinky is inside a Logix project. Choose another location.",
  );
  // The status line repeats the message without a second red line.
  await expect(status(page)).toHaveText("Couldn't save the project.");
  await expect(page.locator(".status-bar")).not.toHaveClass(/error/);
});

test("selects a board from the connected devices", async ({ page }) => {
  await start(page);
  await pickBoard(page, "Arty A7-35T (Xilinx, Artix7)");
  await changeTheBoard(page);
  await page.evaluate(() =>
    (window as unknown as { __fake: { menu: (id: string) => void } }).__fake.menu(
      "devices",
    ),
  );
  const dialog = page.getByRole("dialog", { name: "Connected Devices" });
  await expect(dialog.locator("ul.boards li")).toHaveText(["Alhambra II"]);
  // The board's name is a link that selects it, as the board picker does.
  await dialog.getByRole("button", { name: "Alhambra II", exact: true }).click();
  await expect(dialog).toBeHidden();
  await changeTheBoard(page);
  await expect(boardButton(page)).toHaveText(/Alhambra II/);
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
