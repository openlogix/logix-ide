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
 * with the Alhambra II's blinky (the start screen's Examples…).
 */
async function start(page: Page) {
  await page.goto("/");
  await expect(output(page)).toContainText("Ready.");
  await page.getByRole("button", { name: "Examples…" }).click();
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
async function chooseBoard(page: Page, name = "iCESugar v1.5") {
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

test("shows the engine's package download progress", async ({ page }) => {
  await page.goto("/?engineSetup");
  const bar = page.locator(".status-bar");
  await expect(status(page)).toContainText("Downloading FPGA tools.");
  await expect(bar).toContainText("Downloading oss-cad-suite 42%");
  await expect(bar.locator("progress")).toHaveJSProperty("value", 0.42);
  await expect(output(page)).toContainText("Downloading oss-cad-suite...");
  // The examples work while the FPGA tools download.
  const examples = page.getByRole("button", { name: "Examples…" });
  await expect(examples).toBeEnabled();
  await examples.click();
  await expect(page.getByRole("dialog", { name: "Examples" })).toBeVisible();
  await page.keyboard.press("Escape");
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
  await expect(boardButton(page)).toHaveText(/iCESugar v1\.5/);
  await expect(page.getByTitle(/^Verify/)).toBeDisabled();
  // New Project: the board's pins and an empty design, not blinky.
  await start.getByRole("button", { name: "New Project" }).click();
  await expect(tab(page, "pinout.pcf")).toBeVisible();
  await expect(tab(page, "main_tb.v")).toHaveCount(0);
  await expect(page.locator(".cm-content")).toContainText("assign uart_tx = 1'b1;");
  await expect(output(page)).toContainText(/New project\.\s*Ready\./);
  // Close Project: back to the start screen, with the recent projects.
  await page.evaluate(() => {
    (window as unknown as { __fake: { recents: string[] } }).__fake.recents = [
      "/Users/you/Documents/OpenLogix/IDE/counter",
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
      "/Users/you/Documents/OpenLogix/IDE/counter",
    ];
  });
  await menu(page, "recents-changed");
  await expect(start.getByRole("button", { name: "counter" })).toBeVisible();
  // The board can still be changed; it's used for the next project, and
  // its project options offer new projects on it, as in the editor (no
  // current project to move).
  await pickBoard(page, "iCEBreaker");
  await expect(boardButton(page)).toHaveText(/iCEBreaker/);
  const options = page.getByRole("dialog");
  await expect(options.getByRole("heading")).toHaveText("Project options for board “iCEBreaker”");
  await expect(options.getByRole("button", { name: /^Change the board of/ })).toHaveCount(0);
  await expect(options.getByRole("button", { name: /^blinky/ })).toBeVisible();
  // Cancel: still the start screen, with the board kept for the next project.
  await page.keyboard.press("Escape");
  await expect(options).toHaveCount(0);
  await expect(boardButton(page)).toHaveText(/iCEBreaker/);
  await expect(start).toBeVisible();
  // A recent project brings its own board to the toolbar.
  await start.getByRole("button", { name: "counter" }).click();
  await expect(tab(page, "main.v")).toBeVisible();
  await expect(boardButton(page)).toHaveText(/UPduino/i);
});

test("starts up with the tools ready and the boards listed", async ({ page }) => {
  // The start screen's output: Logix IDE's own first lines (opening a project
  // clears them).
  await page.goto("/");
  await expect(output(page)).toContainText("117 examples available.");
  await start(page);
  await expect(output(page)).not.toContainText("117 examples available.");
  // Logix Engine's 81 boards, grouped by FPGA family. Only the current
  // board's group starts expanded; searching shows matches in all groups.
  await boardButton(page).click();
  await expect(page.locator(".group")).toHaveText([
    /Lattice iCE40\s*27/,
    /Lattice ECP5\s*22/,
    /Gowin GW1N\s*6/,
    /Gowin GW2A\s*3/,
    /Xilinx Artix-7\s*16/,
    /Xilinx Spartan-7\s*2/,
    /Xilinx Zynq-7000\s*5/,
  ]);
  await expect(page.getByRole("option")).toHaveCount(27);
  await page.getByRole("button", { name: /Gowin GW1N/ }).click();
  await expect(page.getByRole("option")).toHaveCount(33);
  await page.getByRole("button", { name: /Lattice iCE40/ }).click();
  await expect(page.getByRole("option")).toHaveCount(6);
  await page.getByRole("combobox").fill("a");
  await expect(page.getByRole("option")).not.toHaveCount(6);
  await page.keyboard.press("Escape");
  await expect(page.getByRole("listbox")).toBeHidden();
  // The iCESugar v1.5 by default, and its blinky (from the examples) is
  // ready to build.
  await expect(boardButton(page)).toHaveText(/iCESugar v1\.5/);
  await expect(tab(page, "pinout.pcf")).toBeVisible();
  await expect(page.locator(".welcome")).toHaveCount(0);
  await expect(page.getByTitle(/^Verify/)).toBeEnabled();
  // Idle: no busy indicator, and the status bar shows the last result.
  await expect(status(page)).toHaveText("Opened example icesugar-1-5/blinky.");
  await expect(page.locator(".spinner")).toHaveCount(0);
});

test("a new project for a board builds right away", async ({ page }) => {
  await start(page);
  await chooseBoard(page);
  await expect(tab(page, "main_tb.v")).toBeVisible();
  await page.getByTitle(/^Verify/).click();
  await expect(status(page)).toContainText("Build succeeded");
  await expect(status(page)).toContainText("1% of logic cells");
  // The output ends with the result, then Ready.
  await expect(output(page)).toContainText(/Build succeeded in .*\s*Ready\.\s*$/);
});

// A GTKWave layout saved in GTKWave is a project file like the sources
// (the user's decision, 2026-11-16): back in Logix IDE, it's an unsaved
// change, even in a project never saved, but no tab (2026-11-17).
test("a layout saved in GTKWave becomes an unsaved project file", async ({ page }) => {
  await start(page);
  await chooseBoard(page);
  await page.evaluate(() => {
    const f = (window as unknown as { __fake: { gtkwaveLayouts: unknown[] } }).__fake;
    f.gtkwaveLayouts = [{ name: "main_tb.gtkw", text: "[*] my layout\n" }];
  });
  await page.evaluate(() => window.dispatchEvent(new Event("focus")));
  await expect(output(page)).toContainText("main_tb.gtkw");
  await expect(tab(page, "main_tb.gtkw")).toHaveCount(0);
  // The project is unsaved now (the window's document state).
  const documentState = () =>
    page.evaluate(
      () =>
        (window as unknown as { __fake: { documentState?: { dirty: boolean } } }).__fake
          .documentState?.dirty,
    );
  await expect.poll(documentState).toBe(true);
  await expect(output(page)).toContainText("Took the layout saved in GTKWave into main_tb.gtkw (not saved yet).");
  // The same layout again changes nothing.
  await page.evaluate(() => window.dispatchEvent(new Event("focus")));
  await expect(output(page).getByText("Took the layout saved in GTKWave")).toHaveCount(1);
});

test("Board Utilization shows the used resources, all on request, and copies them", async ({ page }) => {
  type Fake = { copiedText: string | null };
  const fake = () => page.evaluate(() => (window as unknown as { __fake: Fake }).__fake);
  await start(page);
  await chooseBoard(page);
  await page.getByTitle(/^Verify/).click();
  await expect(status(page)).toContainText("Uses 1% of logic cells, max clock 79.7 MHz.");
  // The output has the engine's report, as it words it.
  await expect(output(page)).toContainText("The design uses 1 of 39 I/O pins (3%).");
  // Project > Board Utilization… builds (instant: nothing changed), then
  // shows the report.
  await menu(page, "build-report");
  const dialog = page.getByRole("dialog", { name: "Board Utilization" });
  const rows = dialog.getByRole("table", { name: "Resources" }).locator("tbody tr");
  // The toolchain's names, with the engine's descriptions; used ones only.
  await expect(rows).toHaveCount(4);
  await expect(rows.first()).toContainText("ICESTORM_LC");
  await expect(rows.first()).toContainText("Logic cell: a 4-input LUT, a flip-flop and carry logic");
  await expect(dialog).toContainText("sys_clk$SB_IO_IN_$glb_clk");
  await dialog.getByLabel(/Show also unused resources \(12\)/).check();
  await expect(rows).toHaveCount(16);
  // Unused rows have no count or share, rather than "0" and "0%".
  const ram = rows.filter({ hasText: "ICESTORM_RAM" }).locator("td");
  await expect(ram.nth(1)).toHaveText("");
  await expect(ram.nth(3)).toHaveText("");
  await dialog.getByLabel("Filter resources").fill("block ram");
  await expect(rows).toHaveCount(1);
  await dialog.getByRole("button", { name: "Copy" }).click();
  await expect(dialog).toContainText("Copied.");
  expect((await fake()).copiedText).toMatch(/^Board Utilization: .* · iCESugar v1.5 · .*\n\nResource +Used +Available +% +Description\nICESTORM_RAM +30 +Block RAM, 4 Kbit\n/s);
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);

  // From the menu it builds first, so the report is of the files as they
  // are now; each time, only the used resources at first.
  await menu(page, "build-report");
  await expect(status(page)).toContainText("Build succeeded");
  await expect(dialog.getByLabel(/Show also unused resources/)).not.toBeChecked();
  await expect(rows).toHaveCount(4);
  await page.keyboard.press("Escape");
  // A failed build: no report, and the last good one is gone.
  await typeAtStart(page, "oops\n");
  await menu(page, "build-report");
  await expect(status(page)).toHaveText("Build failed.");
  await expect(dialog).toHaveCount(0);
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

test("Help > System Info shows the version tree and copies the text", async ({ page }) => {
  type Fake = { copiedText: string | null };
  const fake = () => page.evaluate(() => (window as unknown as { __fake: Fake }).__fake);
  await start(page);
  await menu(page, "system-info");
  const dialog = page.getByRole("dialog", { name: "System Info" });
  // Logix IDE's facts, then the tree, always fully open.
  await expect(dialog).toContainText("Operating system: macOS 26.5 (darwin-arm64)");
  const row = (name: string) => dialog.locator(".row", { hasText: name });
  await expect(row("Logix Engine")).toContainText("2026.11.14");
  await expect(row("logix-core")).toBeVisible();
  await expect(row("OSS CAD Suite")).toBeVisible();
  // Details on click, under the entry; a second click hides them, and
  // hovering shows nothing.
  const details = (name: string) => dialog.getByRole("region", { name: `${name} details` });
  await row("Logix Engine").hover();
  await expect(details("Logix Engine")).toHaveCount(0);
  await row("Logix Engine").click();
  await expect(details("Logix Engine")).toContainText("Logix IDE's remote config 001, prod.jsonc");
  await expect(row("Logix Engine")).toHaveAttribute("aria-expanded", "true");
  await row("Logix Engine").click();
  await expect(details("Logix Engine")).toHaveCount(0);
  // Up and Down move between the entries; Enter shows the details.
  await expect(row("Logix Engine")).toBeFocused();
  await page.keyboard.press("ArrowDown");
  await expect(row("logix-core")).toBeFocused();
  await page.keyboard.press("ArrowUp");
  await expect(row("Logix Engine")).toBeFocused();
  // One entry's details at a time.
  await row("Logix Engine").click();
  await row("logix-core").click();
  await expect(details("Logix Engine")).toHaveCount(0);
  await expect(details("logix-core")).toBeVisible();
  // No developer settings for users.
  await expect(dialog).not.toContainText("Developer settings");
  // Copy gives the text, with the whole tree.
  await dialog.getByRole("button", { name: "Copy" }).click();
  await expect(dialog).toContainText("Copied.");
  const text = (await fake()).copiedText ?? "";
  expect(text).toMatch(/^Logix IDE .*\n\nVersions:\nLogix IDE /s);
  expect(text).toContain("│  │  └─ OSS CAD Suite 2026-11-04");
  // An update while it's open: it reads the report again by itself, and
  // the details shown stay shown.
  await expect(row("logix-catalog")).toContainText("2026.11.14");
  await menu(page, "check-for-updates");
  await expect(row("logix-catalog")).toContainText("2026.11.30");
  await expect(details("logix-core")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
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
  await expect(status(page)).toHaveText("Build failed.");
  await expect(page.locator(".cm-lintRange-error")).toBeVisible();
  await expect(tab(page, "main.v")).toHaveClass(/has-error/);

  // The "main.v:1" in the output jumps to the file (the tool's line; the
  // closing line doesn't repeat it).
  await tab(page, "pinout.pcf").click();
  await expect(page.locator(".file-link", { hasText: "main.v:1" })).toHaveCount(1);
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
  // Clicking the error to select and copy it keeps it, and the name field.
  await page.locator(".tab-input").fill("pins.xxxx");
  await page.keyboard.press("Enter");
  await expect(page.locator(".name-error")).toContainText("Use one of these extensions");
  await page.locator(".name-error").click();
  await expect(page.locator(".name-error")).toBeVisible();
  await expect(page.locator(".tab-input")).toBeVisible();
  // Leaving both cancels.
  await page.locator(".cm-content").click();
  await expect(page.locator(".name-error")).toHaveCount(0);
  await page.getByTitle(/^New file/).click();

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
  await expect(page.getByRole("option", { selected: true }).last()).toHaveText(/iCESugar v1\.5/);
  await page.keyboard.press("Enter");
  const dialog = page.getByRole("dialog", { name: "Examples" });
  await expect(dialog.getByRole("heading")).toHaveText("Project options for board “iCESugar v1.5”");
  // The same board: keep the project, not "change" it.
  await expect(dialog.getByRole("button", { name: /^Change the board of/ })).toHaveCount(0);
  await dialog.getByRole("button", { name: /^Keep the current project, with board “iCESugar v1.5”/ }).click();
  await dialog.getByRole("button", { name: "Keep the Project", exact: true }).click();
  await expect(dialog).toBeHidden();
  await expect(boardButton(page)).toHaveText(/iCESugar v1\.5/);
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
      documentState: { name: string; dirty: boolean } | null;
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
    /yosys[\s\S]*Programming the flash of the iCESugar v1\.5 through its USB port\.[\s\S]*Programmed/,
  );
  await expect(output(page)).toContainText(
    "Programmed the flash of the iCESugar v1.5: the design stays when the board is powered off.",
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

  // Tools > Memory > Flash: back to the default, stored as nothing. The
  // project isn't marked unsaved for it, but Save stores it.
  await menu(page, "variant:memory:flash");
  await expect(summary).toHaveText(/^\s*USB · Flash/);
  await expect.poll(async () => (await fake()).documentState?.dirty).toBe(false);
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
  // From another FPGA than the iCEBreaker's, to see what that implies.
  await chooseBoard(page, "Alhambra II");
  await typeAtStart(page, "// mine\n");
  // Recent projects aren't offered, whatever their board.
  await page.evaluate(() => {
    (window as unknown as { __fake: { recents: string[] } }).__fake.recents = [
      "/Users/you/Documents/OpenLogix/IDE/counter",
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
  // The change, and how many things to check: the red implications show
  // only once the option is chosen.
  await expect(current.locator(".note .point")).toHaveText([
    "FPGA: ICE40HX4K-TQ144 → ICE40UP5K-SG48.",
    "3 things to check: choose this option to see them.",
  ]);
  await expect(current.locator(".note .point.risky")).toHaveCount(0);
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
  // Chosen: the change, then its implications, all red.
  await expect(current.locator(".note .point")).toHaveText([
    "FPGA: ICE40HX4K-TQ144 → ICE40UP5K-SG48.",
    /^FPGA-specific parts/,
    /^The design may not fit/,
    /^The pins, clock frequency and LED and button wiring may differ/,
  ]);
  await expect(current.locator(".note .point.risky")).toHaveCount(3);
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
    "FPGA family: Lattice iCE40 → Lattice ECP5.",
  );
  await other.click();
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
        id: "usb:1-2:1d50:602b:0700000001",
        name: "USB device",
        serial: "0700000001",
        boards: [{ board: "icesugar-1-5", level: "possible", variants: [{}] }],
      },
    ];
    window.dispatchEvent(new Event("focus"));
  });
  await expect(link).toHaveText("Connected");
  await expect(link).toHaveAttribute("title", /may be the iCESugar v1\.5/);
  await boardButton(page).click();
  // No groups from connected devices: only the families.
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
  // It collapses like a family's group.
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
  // Boards are grouped by FPGA family; the current board's group is open.
  const boards = dialog.getByRole("list", { name: "Boards" });
  await expect(boards.getByRole("button", { name: /Lattice iCE40/ })).toHaveAttribute(
    "aria-expanded",
    "true",
  );
  await expect(boards.getByRole("button", { name: /Xilinx Artix-7/ })).toHaveAttribute(
    "aria-expanded",
    "false",
  );
  await expect(boards.getByRole("button", { name: /^Alchitry Au \(/ })).toHaveCount(0);
  await boards.getByRole("button", { name: /Xilinx Artix-7/ }).click();
  await expect(boards.getByRole("button", { name: /^Alchitry Au \(/ })).toBeVisible();
  await dialog.getByRole("button", { name: /^bare-minimum/ }).click();
  await dialog.getByRole("button", { name: "New Project" }).click();
  await expect(dialog).toBeHidden();
  await expect(tab(page, "pinout.pcf")).toBeVisible();
  await expect(tab(page, "main_tb.v")).toHaveCount(0);
  await expect(page.locator(".cm-content")).toContainText("assign uart_tx = 1'b1;");
  await expect(status(page)).toHaveText("Opened bare-minimum for iCESugar v1.5.");
  // Like every command, its output ends with the result, then "Ready.".
  await expect(output(page)).toContainText(
    /Opened bare-minimum for iCESugar v1\.5\.\s*Ready\./,
  );
  await expect(page.locator(".spinner")).toHaveCount(0);
});

test("the editor's right-click menu comments lines", async ({ page }) => {
  await start(page);
  await chooseBoard(page);
  // A line of code (the file starts with comments).
  await page.locator(".cm-line", { hasText: "module main" }).click({ button: "right" });
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
  const first = page.locator(".cm-line", { hasText: "module main" });
  const before = (await first.textContent()) ?? "";
  await menu("toggle-comment");
  await expect(first).toHaveText(`// ${before}`);
  await menu("toggle-comment");
  await expect(first).toHaveText(before);
});

// Tab indents and Shift+Tab un-indents, as in most code editors (the
// user's choice, 2026-11-18); Escape, then Tab, leaves the editor.
test("Tab indents in the editor", async ({ page }) => {
  await start(page);
  const first = page.locator(".cm-line", { hasText: "module main" });
  const before = (await first.textContent()) ?? "";
  await first.click();
  await page.keyboard.press("Tab");
  await expect(first).toHaveText(`  ${before}`);
  await expect(page.locator(".cm-content")).toBeFocused();
  await page.keyboard.press("Shift+Tab");
  await expect(first).toHaveText(before);
  await page.keyboard.press("Escape");
  await page.keyboard.press("Tab");
  await expect(page.locator(".cm-content")).not.toBeFocused();
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
  // A testbench: its own Run Test in the editor's right-click menu.
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
  const first = page.locator(".cm-line", { hasText: "set_io sys_clk" });
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
  await page.getByRole("button", { name: /^Show ▴/ }).click();
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

test("shows the design diagram and goes to a block's source", async ({ page }) => {
  await start(page);
  await chooseBoard(page);
  type Fake = {
    menu: (id: string) => void;
    diagramShown: { key: string; module: string | null } | null;
    diagramClick: (key: string, file: string, line: number) => void;
  };
  await page.evaluate(() => (window as unknown as { __fake: Fake }).__fake.menu("show-diagram"));
  await expect(status(page)).toContainText("Diagram ready");
  // The backend opens the Diagram window, a window of its own, on main.
  const shown = await page.evaluate(() => (window as unknown as { __fake: Fake }).__fake.diagramShown);
  expect(shown?.module).toBeNull();
  // No dialog in the main window: it stays usable.
  await expect(page.getByRole("dialog")).toHaveCount(0);

  // A click on a block in it shows its source line in the editor.
  await page.evaluate(
    (key) => (window as unknown as { __fake: Fake }).__fake.diagramClick(key, "main.v", 3),
    shown!.key,
  );
  await expect(page.locator(".cm-activeLine")).toBeVisible();
  // Another project's click is ignored.
  await page.evaluate(() => (window as unknown as { __fake: Fake }).__fake.diagramClick("other", "main.v", 1));
});

test("the editor's right-click menu shows the diagram of the module at the cursor", async ({ page }) => {
  await start(page);
  await chooseBoard(page);
  type Fake = {
    menu: (id: string) => void;
    editorMenuModule: string | null;
    diagramShown: { key: string; module: string | null } | null;
  };
  await page.locator(".cm-content").click({ button: "right" });
  await expect
    .poll(() => page.evaluate(() => (window as unknown as { __fake: Fake }).__fake.editorMenuModule))
    .toBe("main");
  await page.evaluate(() => (window as unknown as { __fake: Fake }).__fake.menu("show-module-diagram"));
  await expect(status(page)).toContainText("Diagram ready");
  // main is the top module: the diagram starts there.
  const shown = await page.evaluate(() => (window as unknown as { __fake: Fake }).__fake.diagramShown);
  expect(shown?.module).toBeNull();
});

test("developer settings show one banner for the whole session", async ({ page }) => {
  await page.goto("/?developerSettings");
  const banner = page.getByRole("status").filter({ hasText: "developer configuration" });
  await expect(banner).toHaveText(
    "Using a developer configuration. See Help > System Info for details.",
  );
  // Hovering it lists the settings, Logix IDE's and the engine's.
  await expect(banner).toHaveAttribute("title", "LOGIX_DEV_ENGINE_BINARY\nLOGIX_ENGINE_CHANNEL");
  // Still there once setup is done.
  await expect(output(page)).toContainText("Ready.");
  await expect(banner).toBeVisible();
});

test("no banner without developer settings", async ({ page }) => {
  await start(page);
  await expect(page.getByText("developer configuration")).toHaveCount(0);
});

test("Check for Updates shows the progress and what changed", async ({ page }) => {
  await start(page);
  await menu(page, "check-for-updates");
  await expect(output(page)).toContainText("Checking for updates…");
  await expect(output(page)).toContainText("Downloading logix-catalog 2026.11.30...");
  await expect(output(page)).toContainText("Updated logix-catalog to 2026.11.30.");
  await expect(status(page)).toContainText("Updated logix-catalog to 2026.11.30.");
});

test("Upload asks which device when several fit the board", async ({ page }) => {
  await start(page);
  await chooseBoard(page, "Alhambra II");
  await page.evaluate(() => {
    const f = (window as unknown as { __fake: { uploadDevices: unknown[] } }).__fake;
    const device = (serial: string, level: string) => ({
      id: `usb:1-${serial}:0403:6010:${serial}`,
      label: "Alhambra II v1.0A (AlhambraBits, 0403:6010)", likelyBoards: ["alhambra-ii"],
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
  // Up and Down move between the devices' Upload buttons.
  const uploads = picker.getByRole("button", { name: "Upload" });
  await uploads.first().focus();
  await page.keyboard.press("ArrowDown");
  await expect(uploads.nth(1)).toBeFocused();
  await page.keyboard.press("ArrowUp");
  await expect(uploads.first()).toBeFocused();
  await picker.getByRole("listitem").filter({ hasText: "serial B2" }).getByRole("button", { name: "Upload" }).click();
  await expect(picker).toBeHidden();
  await expect(output(page)).toContainText("Programmed the flash of the Alhambra II");
  expect(
    await page.evaluate(
      () => (window as unknown as { __fake: { uploadedDevice: string } }).__fake.uploadedDevice,
    ),
  ).toBe("usb:1-B2:0403:6010:B2");
});

// Windows: a board whose programmer needs WinUSB, which it doesn't have
// yet (FTDI's driver). Upload asks first, then installs it and programs the board.
test("Upload offers the WinUSB driver a board needs", async ({ page }) => {
  type Fake = {
    uploadDevices: unknown[];
    uploadDriver: unknown;
    sameIdsMessage: string | null;
    confirmAnswer: boolean;
    confirms: { title: string; okLabel: string }[];
    driverInstalls: { device: string; allowSameIds: boolean }[];
    driverStatuses: string[];
    uploadedDevice: string;
  };
  const fake = (p: Page) => p.evaluate(() => (window as unknown as { __fake: Fake }).__fake);
  await start(page);
  await chooseBoard(page, "Alhambra II");
  await page.evaluate(() => {
    const f = (window as unknown as { __fake: Fake }).__fake;
    f.uploadDevices = [{
      id: "usb:1-2:0403:6010:", label: "USB device (0403:6010)", likelyBoards: [],
      vid: "0403", pid: "6010", manufacturer: "", product: "", serialNumber: "",
      boards: [{ board: "alhambra-ii", level: "weak" }], buildMatch: "weak", driver: null,
    }];
    f.uploadDriver = { needed: "winusb", interface: 0, current: "ftdibus", ready: false };
  });
  // Asked in Logix IDE's own dialog. Declined: nothing is installed, and
  // Upload says why it stopped.
  await menu(page, "upload");
  const ask = page.getByRole("alertdialog", { name: "Install the board's USB driver?" });
  await expect(ask).toContainText("To program the Alhambra II, Windows needs to use the WinUSB driver for it.");
  await expect(ask).toContainText("What happens");
  await ask.getByRole("button", { name: "Cancel" }).click();
  await expect(ask).toHaveCount(0);
  await expect(status(page)).toContainText("Cancelled: the driver wasn't installed. Nothing was uploaded");
  expect((await fake(page)).driverInstalls).toEqual([]);

  // Agreed, while another device with the same USB IDs is connected: asked
  // again, then installed for all, and the board is programmed.
  await page.evaluate(() => {
    const f = (window as unknown as { __fake: Fake }).__fake;
    f.sameIdsMessage = "Another connected device has the same USB IDs (0403:6010): Dual RS232-HS.";
  });
  await menu(page, "upload");
  await page.getByRole("alertdialog", { name: "Install the board's USB driver?" }).getByRole("button", { name: "Install Driver" }).click();
  const same = page.getByRole("alertdialog", { name: "Other devices of the same kind are connected" });
  await expect(same).toContainText("Dual RS232-HS");
  await same.getByRole("button", { name: "Continue for All" }).click();
  await expect(output(page)).toContainText("Installed the WinUSB driver for the Alhambra II.");
  await expect(output(page)).toContainText("Programmed the flash of the Alhambra II");
  const f = await fake(page);
  expect(f.driverInstalls).toEqual([{ device: "usb:1-2:0403:6010:", allowSameIds: true }]);
  // The status said so while the driver was installed, not "Uploading…".
  expect(f.driverStatuses).toEqual(["Installing the board's driver, which may take a minute or two…"]);
  // Programmed with the device's ID after the driver (its serial number now read).
  expect(f.uploadedDevice).toBe("usb:1-2:0403:6010:FT1");

  // Next time the driver is ready: no question.
  await menu(page, "upload");
  await expect(status(page)).toContainText("Upload succeeded");
  await expect(page.getByRole("alertdialog")).toHaveCount(0);
});

// Linux: a board without a udev rule can't be opened. Upload asks first,
// then adds the rule (Linux asks for the password) and programs the board.
test("Upload offers the udev rule a board needs", async ({ page }) => {
  type Fake = {
    uploadDevices: unknown[];
    uploadDriver: unknown;
    driverInstalls: { device: string; allowSameIds: boolean }[];
    driverStatuses: string[];
  };
  const fake = (p: Page) => p.evaluate(() => (window as unknown as { __fake: Fake }).__fake);
  await start(page);
  await chooseBoard(page, "Alhambra II");
  await page.evaluate(() => {
    const f = (window as unknown as { __fake: Fake }).__fake;
    f.uploadDevices = [{
      id: "usb:1-2:0403:6010:", label: "USB device (0403:6010)", likelyBoards: [],
      vid: "0403", pid: "6010", manufacturer: "", product: "", serialNumber: "",
      boards: [{ board: "alhambra-ii", level: "weak" }], buildMatch: "weak", driver: null,
    }];
    f.uploadDriver = { needed: "udev-rule", interface: null, current: null, ready: false };
  });
  await menu(page, "upload");
  const ask = page.getByRole("alertdialog", { name: "Let your user account program the board?" });
  await expect(ask).toContainText("To program the Alhambra II, Linux needs a udev rule that lets your user account use it.");
  await expect(ask).toContainText("Linux asks for your password.");
  // The output says why Upload stopped, and waits for the answer.
  // No "Ready." right after it (a background notice may still follow).
  await expect(output(page)).toContainText(/The board needs a udev rule first\.(?!\s*Ready\.)/);
  await ask.getByRole("button", { name: "Cancel" }).click();
  await expect(status(page)).toContainText("Cancelled: the udev rule wasn't added. Nothing was uploaded");
  // The answer ends the output, then Ready. A background notice (such as
  // the language server timing out on a slow runner) may come before or
  // after it, but no "Ready." while Upload waits for the answer.
  await expect(output(page)).toContainText(
    /The board needs a udev rule first\.(?:(?!Ready\.)[\s\S])*Cancelled: the udev rule wasn't added\. Nothing was uploaded: choose Upload again when you're ready\.\s*Ready\./,
  );
  expect((await fake(page)).driverInstalls).toEqual([]);

  await menu(page, "upload");
  await page.getByRole("alertdialog", { name: "Let your user account program the board?" }).getByRole("button", { name: "Add udev Rule" }).click();
  await expect(output(page)).toContainText("Added a udev rule for the Alhambra II.");
  await expect(output(page)).toContainText("Programmed the flash of the Alhambra II");
  const f = await fake(page);
  expect(f.driverInstalls).toEqual([{ device: "usb:1-2:0403:6010:", allowSameIds: false }]);
  expect(f.driverStatuses).toEqual(["Adding the board's udev rule…"]);
});

// A failed driver change shows the engine's reason, and its details in the
// log: such as the commands to run when Linux can't ask for the password.
test("Connected Devices shows why a driver change failed", async ({ page }) => {
  type Fake = { usb: unknown[]; driverError: { message: string; details: string } | null; menu: (id: string) => void };
  await start(page);
  await page.evaluate(() => {
    const f = (window as unknown as { __fake: Fake }).__fake;
    f.usb = [{
      id: "usb:1-4:0403:6010:", label: "Alhambra II v1.0A (AlhambraBits, 0403:6010)", likelyBoards: ["alhambra-ii"],
      vid: "0403", pid: "6010", manufacturer: "AlhambraBits", product: "Alhambra II v1.0A", serialNumber: "",
      boards: [{ board: "alhambra-ii", level: "strong" }], buildMatch: null,
      driver: { needed: "udev-rule", interface: null, current: null, ready: false },
    }];
    f.driverError = {
      message: "Linux can't ask for your password here (no password dialog). Run the commands in the details in a terminal instead.",
      details: "sudo udevadm control --reload-rules\nsudo udevadm trigger",
    };
    f.menu("devices");
  });
  const dialog = page.getByRole("dialog", { name: "Connected Devices" });
  await dialog.getByRole("button", { name: "Add udev Rule…" }).click();
  await page.getByRole("alertdialog").getByRole("button", { name: "Add udev Rule" }).click();
  const work = dialog.locator("section.work");
  await expect(work.locator(".work-status.error")).toContainText("Linux can't ask for your password here");
  await work.getByRole("button", { name: /Show log/ }).click();
  await expect(work.locator(".work-output")).toContainText("Failed: Linux can't ask for your password here");
  await expect(work.locator(".work-output")).toContainText("sudo udevadm control --reload-rules");
  await expect(work.locator(".work-output")).toContainText("sudo udevadm trigger");
});

test("Connected Devices installs and removes a board's driver", async ({ page }) => {
  type Fake = {
    usb: unknown[];
    driverInstalls: { device: string }[];
    driverRemovals: string[];
    driverHold: Promise<void> | null;
    release: () => void;
    menu: (id: string) => void;
  };
  await start(page);
  await page.evaluate(() => {
    const f = (window as unknown as { __fake: Fake }).__fake;
    f.usb = [{
      id: "usb:1-4:0403:6010:", label: "Alhambra II v1.0A (AlhambraBits, 0403:6010)", likelyBoards: ["alhambra-ii"],
      vid: "0403", pid: "6010", manufacturer: "AlhambraBits", product: "Alhambra II v1.0A", serialNumber: "",
      boards: [{ board: "alhambra-ii", level: "strong" }], buildMatch: null,
      driver: { needed: "winusb", interface: 0, current: "ftdibus", ready: false },
    }];
    f.menu("devices");
  });
  const dialog = page.getByRole("dialog", { name: "Connected Devices" });
  await expect(dialog).toContainText("Driver: needs WinUSB on USB interface 0 to program the board (it has FTDI's driver now).");
  // While it installs, the dialog's bottom shows it with the engine's
  // phase and output (held here as Windows' prompt would).
  await page.evaluate(() => {
    const f = (window as unknown as { __fake: Fake }).__fake;
    f.driverHold = new Promise((resolve) => (f.release = resolve));
  });
  await dialog.getByRole("button", { name: "Install Driver…" }).click();
  // Asked in Logix IDE's own dialog, above Connected Devices.
  const ask = page.getByRole("alertdialog", { name: "Install the board's USB driver?" });
  await expect(ask).toContainText("Right now Windows uses FTDI's driver for it.");
  await ask.getByRole("button", { name: "Install Driver" }).click();
  const work = dialog.locator("section.work");
  await expect(work).toContainText("Installing the board's driver, which may take a minute or two…");
  await expect(work).toContainText("Waiting for permission");
  await expect(work.locator(".spinner")).toBeVisible();
  // The log is collapsed by default.
  await expect(work.locator(".work-output")).toHaveCount(0);
  await page.evaluate(() => (window as unknown as { __fake: Fake }).__fake.release());
  await expect(dialog).toContainText("Driver: WinUSB, ready to program the board.");
  // Ended: the status line says so in green, and the log ends the same way.
  await expect(work.locator(".work-status.ok")).toContainText("Installed the WinUSB driver for the Alhambra II.");
  await expect(work.locator(".spinner")).toHaveCount(0);
  await work.getByRole("button", { name: /Show log/ }).click();
  await expect(work.locator(".work-output")).toContainText("$ wdi-simple.exe --vid 0x0403 --pid 0x6010");
  const installEnd = (await work.locator(".work-output").textContent())?.trim().split("\n").slice(-2);
  expect(installEnd?.[0]).toMatch(/^Done in \d+ s\.$/);
  expect(installEnd?.[1]).toBe("Installed the WinUSB driver for the Alhambra II.");
  await expect(status(page)).toContainText("Installed the WinUSB driver for the Alhambra II");
  // Remove, then Cancel: the panel shows that, not the install before it.
  await dialog.getByRole("button", { name: "Remove Driver…" }).click();
  await page.getByRole("alertdialog", { name: "Remove the board's USB driver?" }).getByRole("button", { name: "Cancel" }).click();
  await expect(work.locator(".work-status.error")).toHaveText("Cancelled: the driver wasn't removed.");
  await expect(status(page)).toContainText("Cancelled: the driver wasn't removed.");
  await dialog.getByRole("button", { name: "Remove Driver…" }).click();
  await page.getByRole("alertdialog", { name: "Remove the board's USB driver?" }).getByRole("button", { name: "Remove Driver" }).click();
  await expect(dialog).toContainText("it has FTDI's driver now");
  await expect(work.locator(".work-output")).toContainText("$ pnputil /delete-driver oem12.inf");
  // One line each: how long it took, what was done, what Windows does now.
  const removeEnd = (await work.locator(".work-output").textContent())?.trim().split("\n").slice(-3);
  expect(removeEnd?.[0]).toMatch(/^Done in \d+ s\.$/);
  expect(removeEnd?.slice(1)).toEqual([
    "Removed the WinUSB driver for the Alhambra II.",
    "Windows uses its original driver again.",
  ]);
  // Refresh starts over: no status or log of the change before.
  await dialog.getByRole("button", { name: "Refresh" }).click();
  await expect(work.locator(".work-status")).toHaveCount(0);
  // Close, at the bottom.
  await dialog.getByRole("button", { name: "Close", exact: true }).click();
  await expect(dialog).toHaveCount(0);
  const f = await page.evaluate(() => (window as unknown as { __fake: Fake }).__fake);
  expect(f.driverInstalls.map((i) => i.device)).toEqual(["usb:1-4:0403:6010:"]);
  expect(f.driverRemovals).toEqual(["usb:1-4:0403:6010:"]);
});

// Windows: a board with FTDI's driver can't be opened, so its names are
// unknown. It's listed first, before devices that aren't boards, and says
// which board it may be (the user's requests, 2026-11-14).
test("Connected Devices lists a board with no driver first, and says what it may be", async ({ page }) => {
  type Fake = { usb: unknown[]; unreachable: unknown[]; menu: (id: string) => void };
  await start(page);
  await chooseBoard(page, "Alhambra II");
  await page.evaluate(() => {
    const f = (window as unknown as { __fake: Fake }).__fake;
    f.usb = [{
      id: "usb:1-1:046d:c52b:", label: "USB Receiver (Logitech, 046d:c52b)", likelyBoards: [],
      vid: "046d", pid: "c52b", manufacturer: "Logitech", product: "USB Receiver", serialNumber: "",
      boards: [], buildMatch: null, driver: null,
    }];
    f.unreachable = [{
      id: "usb:1-4:0403:6010:", label: "USB device (0403:6010)", vid: "0403", pid: "6010",
      action: "install_drivers", actionDescription: "Install the driver this device needs.",
      driver: { needed: "winusb", interface: 0, current: "ftdibus", ready: false },
      boards: [{ board: "alhambra-ii", level: "weak" }, { board: "icestick", level: "weak" }],
    }];
    f.menu("devices");
  });
  const dialog = page.getByRole("dialog", { name: "Connected Devices" });
  await expect(dialog.locator("li.device button.device-toggle .name")).toHaveText([
    "USB device (0403:6010)",
    "USB Receiver (Logitech, 046d:c52b)",
  ]);
  await expect(dialog).toContainText(
    "This may be your Alhambra II (the board selected in the toolbar): it has the same USB IDs. It needs its driver first, so it can be programmed.",
  );
  await expect(dialog.getByRole("button", { name: /^Boards with these USB IDs\s*2$/ })).toBeVisible();
  await dialog.getByRole("button", { name: "Install Driver…" }).click();
  await page.getByRole("alertdialog", { name: "Install the board's USB driver?" }).getByRole("button", { name: "Install Driver" }).click();
  await expect(dialog.locator("section.work")).toContainText("Installed the WinUSB driver for the Alhambra II.");
});

test("lists the connected devices", async ({ page }) => {
  await start(page);
  await chooseBoard(page, "Alhambra II");
  await page.evaluate(() => (window as unknown as { __fake: { useAlhambra: () => void } }).__fake.useAlhambra());
  await page.evaluate(() =>
    (window as unknown as { __fake: { menu: (id: string) => void } }).__fake.menu(
      "devices",
    ),
  );
  const dialog = page.getByRole("dialog", { name: "Connected Devices" });
  await expect(dialog).toContainText("Alhambra II v1.0A (AlhambraBits, 0403:6010)");
  // Its serial number on its own line, under the name.
  await expect(dialog.locator(".serial")).toHaveText("Serial: FTATANKC");
  // The device is a group, open, with its matches as a bullet list.
  const device = dialog.getByRole("button", { name: /Alhambra II v1.0A/ });
  await expect(device).toHaveAttribute("aria-expanded", "true");
  const matches = dialog.getByRole("button", { name: /^Matching boards/ });
  await expect(matches).toHaveAttribute("aria-expanded", "true");
  await expect(dialog.locator("ul.boards li")).toHaveText(["Alhambra II (selected)"]);
  // A strong match: the weaker ones aren't listed.
  await expect(dialog.getByRole("button", { name: /^Other possible boards/ })).toHaveCount(0);
  // Copy: all of it as text, for a problem report.
  await dialog.getByRole("button", { name: "Copy", exact: true }).click();
  await expect(dialog.locator(".copy-note")).toHaveText("Copied.");
  const copied = await page.evaluate(() => (window as unknown as { __fake: { copiedText: string | null } }).__fake.copiedText);
  expect(copied).toMatch(/^Connected Devices\nSelected board: Alhambra II \(alhambra-ii\)\n\nAlhambra II v1\.0A \(AlhambraBits, 0403:6010\)\n  ID: usb:/);
  expect(copied).toContain("  Matching boards: Alhambra II");
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
        id: "usb:1-4:0403:6010:", label: "Dual RS232-HS (FTDI, 0403:6010)", likelyBoards: [],
        vid: "0403", pid: "6010", manufacturer: "FTDI",
        product: "Dual RS232-HS", serialNumber: "", buildMatch: null, driver: null,
        boards: [{ board: "alchitry-au", level: "weak" }, { board: "icestick", level: "weak" }],
      },
    ];
    (window as unknown as { __fake: { menu: (id: string) => void } }).__fake.menu("devices");
  });
  const others = dialog.getByRole("button", { name: /^Other possible boards\s*2$/ });
  await expect(others).toHaveAttribute("aria-expanded", "false");
  await others.click();
  await expect(dialog.getByRole("button", { name: /^Alchitry Au/ })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
  // The toolbar's Connected / Not connected status opens it too.
  await page.getByRole("button", { name: /^(Connected|Not connected)$/ }).click();
  await expect(dialog).toBeVisible();
});

test("a project's boards folder, edited outside Logix IDE, adds its own boards", async ({ page }) => {
  type Fake = {
    customBoardsOnDisk: Record<string, Record<string, string>>;
    taskBoardsFolder: unknown;
    scannedBoardsFolder: unknown;
  };
  const folder = "/Users/you/Documents/OpenLogix/IDE/mine";
  // boards/my-upduino.json, as users write it, so editors check the file.
  const myBoard = {
    $schema: "https://www.openlogix.org/schemas/logix-board-v1.json",
    name: "My UPduino",
    fpga_id: "ice40up5k-sg48",
    variants: [
      {
        selector: { connection: "usb", memory: "flash" }, programmer: "iceprog",
        args: "-d d:${BUS}/${DEV} ${BITSTREAM}",
        usb: { vid: "0403", pid: "6014" }, tested: false,
      },
    ],
  };
  /** Writes (or, with null, deletes) boards/my-upduino.json on the fake disk, as another editor would. */
  const writeFile = async (text: string | null) => {
    await page.evaluate(
      ([dir, t]) => {
        const disk = (window as unknown as { __fake: Fake }).__fake.customBoardsOnDisk;
        if (t === null) delete disk[dir as string];
        else disk[dir as string] = { "my-upduino": t as string };
        // The IDE checks the file when the window gets the focus back.
        window.dispatchEvent(new Event("focus"));
      },
      [folder, text],
    );
  };
  await page.goto(`/?recents=${folder}`);
  await writeFile(JSON.stringify(myBoard, null, 2));
  await expect(output(page)).toContainText("Ready.");
  await page.getByRole("button", { name: /mine/ }).click();
  // Not a tab: it's edited outside Logix IDE.
  await expect(tab(page, "main.v")).toBeVisible();
  await expect(tab(page, "my-upduino.json")).toHaveCount(0);

  // Listed first in "Custom boards" (expanded), and first in its
  // family's group too, with the board icon and a tooltip saying
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
  await expect(first).toHaveAttribute("title", /boards folder/);
  await page.getByRole("combobox").fill("My UPduino");
  await page.getByRole("option", { name: "My UPduino", exact: true }).first().click();
  // Its project options: the current project, with the change note, and
  // no examples or new projects, saying why.
  const options = page.getByRole("dialog", { name: "Examples" });
  await expect(options.getByRole("heading")).toHaveText("Project options for custom board “My UPduino”");
  await expect(options.locator(".current .note .point").first()).toHaveText(/^Board: UPduino v3\.1 → My UPduino\. Same FPGA: /);
  await expect(options.locator(".custom-note")).toContainText("Custom boards have no examples");
  await expect(options.getByRole("button", { name: /^bare-minimum/ })).toHaveCount(0);
  await expect(options.getByRole("button", { name: /^blinky/ })).toHaveCount(0);
  // Not applied until chosen.
  await expect(boardButton(page)).toHaveText(/UPduino v3\.1/);
  await changeTheBoard(page);
  await expect(options).toBeHidden();
  await expect(boardButton(page)).toHaveText(/My UPduino/);
  await expect(boardButton(page).locator(".custom")).toHaveCount(1);
  await expect(boardButton(page)).toHaveAttribute(
    "title",
    "Custom board: My UPduino\nFrom this project's boards folder",
  );
  // The Board Explorer lists it with the catalog's boards, marked as in the picker.
  await page.getByRole("button", { name: "Show the board in the Board Explorer" }).click();
  const explorer = page.getByRole("dialog", { name: "Board Explorer" });
  await expect(explorer.getByLabel("Details")).toContainText("Custom board");
  await expect(explorer.getByLabel("Details")).toContainText("This project's boards folder");
  const mine = explorer.locator(".item.selected");
  await expect(mine).toHaveText(/My UPduino/);
  await expect(mine.locator(".custom")).toHaveAttribute("title", "From this project's boards folder");
  await page.keyboard.press("Escape");

  // The project folder goes with every task and device scan (the engine
  // reads the file there), and the output says where the board is from.
  await menu(page, "verify");
  await expect(status(page)).toContainText("Build succeeded");
  await expect(output(page)).toContainText("The board is from this project's boards folder.");
  const fake = () => page.evaluate(() => (window as unknown as { __fake: Fake }).__fake);
  expect((await fake()).taskBoardsFolder).toBe(folder);
  await expect.poll(async () => (await fake()).scannedBoardsFolder).toBe(folder);

  // Broken in another editor: its problems go to the output at line and
  // column, the board list says so in one line and leaves its boards out,
  // and there's no banner.
  await writeFile("oops");
  // At the file's full path: it's edited outside Logix IDE. Noticed in the
  // background, so "Ready." comes again after it: it's always last when idle.
  await expect(output(page)).toContainText(`${folder}/boards/my-upduino.json:1:1: error: Expected value.`);
  await expect(output(page)).toContainText(/Expected value\.\s*Ready\.\s*$/);
  await expect(page.locator(".banner")).toHaveCount(0);
  await boardButton(page).click();
  // The note is in the Custom boards group, which stays, empty.
  await expect(page.locator(".problem-note")).toContainText("boards folder has problems");
  await expect(page.locator(".group").first()).toContainText("Custom boards");
  await expect(page.locator(".group").first().locator(".count")).toHaveText("0");
  await page.getByRole("combobox").fill("My UPduino");
  await expect(page.getByRole("option", { name: "My UPduino", exact: true })).toHaveCount(0);
  await page.keyboard.press("Escape");
  // A task stops with the same problems.
  await menu(page, "verify");
  await expect(output(page)).toContainText(`${folder}/boards/my-upduino.json:1:1: error: Expected value.`);
  // The status line counts them rather than repeating them.
  await expect(status(page)).toHaveText(
    "Build failed: the project's boards folder has problems.",
  );

  // Fixed: said once, and the boards are back.
  await writeFile(JSON.stringify(myBoard));
  await expect(output(page)).toContainText("boards folder has no problems now.");
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
  // Not found within 30 s: it may open behind Logix IDE.
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

test("the Board Explorer shows the board's FPGA and the boards with the same FPGA", async ({ page }) => {
  await start(page);
  await chooseBoard(page, "Alhambra II");
  // The board picker: the FPGA as a hover hint, and search by it.
  await boardButton(page).click();
  await page.getByRole("combobox").fill("hx4k");
  const alhambra = page.getByRole("option", { name: "Alhambra II", exact: true }).first();
  await expect(alhambra).toHaveAttribute("title", "FPGA: Lattice iCE40 ICE40HX4K-TQ144 (8k)");
  await page.keyboard.press("Escape");

  // The ⓘ next to the board opens the Explorer at the board.
  await page.getByRole("button", { name: "Show the board in the Board Explorer" }).click();
  const explorer = page.getByRole("dialog", { name: "Board Explorer" });
  const details = explorer.getByLabel("Details");
  await expect(details.getByRole("heading", { level: 3 })).toHaveText("Alhambra II");
  await expect(details).toContainText("ICE40HX4K-TQ144");
  await expect(details).toContainText("Lattice iCE40");
  await expect(details).toContainText(".pcf");
  await expect(details).toContainText("USB · Flash (default)");
  // Its resources, from the engine's catalog.
  const resources = details.locator("table.resources");
  await expect(resources.getByRole("row", { name: /^Clock|clk/ }).first()).toContainText("12 MHz");
  // It's selected in the Boards list, its family's group open.
  await expect(explorer.getByRole("tab", { name: "Boards" })).toHaveAttribute("aria-selected", "true");
  await expect(explorer.locator(".item.selected")).toHaveText(/Alhambra II/);
  await expect(explorer.getByRole("button", { name: "Select This Board" })).toBeDisabled();
  await page.keyboard.press("Escape");
  await expect(explorer).toBeHidden();

  // Tools > Board Explorer opens it too. A board with the same FPGA shows its own details; Back returns.
  await menu(page, "board-explorer");
  await expect(details.getByRole("heading", { level: 3 })).toHaveText("Alhambra II");
  const back = explorer.getByRole("button", { name: "Back" });
  await expect(back).toHaveCount(0);
  await details.getByRole("button", { name: "AZUKAR v2" }).click();
  await expect(details.getByRole("heading", { level: 3 })).toHaveText("AZUKAR v2");
  await back.click();
  await expect(details.getByRole("heading", { level: 3 })).toHaveText("Alhambra II");
  // Select This Board picks the board shown, as the board picker does.
  await details.getByRole("button", { name: "AZUKAR v2" }).click();
  await explorer.getByRole("button", { name: "Select This Board" }).click();
  await expect(explorer).toBeHidden();
  await changeTheBoard(page);
  await expect(boardButton(page)).toHaveText(/AZUKAR v2/);
});

test("the Board Explorer links families, FPGAs, boards and examples", async ({ page }) => {
  await start(page);
  await menu(page, "board-explorer");
  const explorer = page.getByRole("dialog", { name: "Board Explorer" });
  const details = explorer.getByLabel("Details");
  const title = details.getByRole("heading", { level: 3 });
  // Families: every family the engine supports, with or without boards.
  await explorer.getByRole("tab", { name: "Families" }).click();
  const list = explorer.getByRole("list", { name: "Families" });
  await expect(list.getByRole("button")).toHaveCount(8);
  await list.getByRole("button", { name: /Gowin GW5A/ }).click();
  await expect(title).toHaveText("Gowin GW5A");
  await expect(details).toContainText("No boards yet.");
  await expect(details).toContainText("FPGAs48");
  // A family's FPGA, then its board, then the board's example, and back up by links.
  await explorer.getByRole("tab", { name: "FPGAs" }).click();
  await explorer.getByRole("searchbox").fill("up5k-sg48");
  await explorer.getByRole("button", { name: /^ICE40UP5K-SG48/ }).click();
  await expect(title).toHaveText("ICE40UP5K-SG48");
  await expect(details).toContainText("ice40up5k-sg48");
  await details.getByRole("button", { name: "iCEBreaker", exact: true }).click();
  await expect(title).toHaveText("iCEBreaker");
  await expect(explorer.getByRole("tab", { name: "Boards" })).toHaveAttribute("aria-selected", "true");
  await details.getByRole("button", { name: "blinky" }).click();
  await expect(title).toHaveText("blinky");
  await details.getByRole("button", { name: "Lattice iCE40" }).click();
  await expect(title).toHaveText("Lattice iCE40");
  // Open Example opens the example shown.
  await explorer.getByRole("button", { name: "Back" }).click();
  await expect(title).toHaveText("blinky");
  await explorer.getByRole("button", { name: "Open Example" }).click();
  await expect(explorer).toBeHidden();
  await expect(status(page)).toHaveText("Opened example icebreaker/blinky.");
});

// Up and Down move through lists (the user's request, 2026-11-18: Up and
// Down only, no Left and Right); Tab goes from list to list.
test("Up and Down move through the lists", async ({ page }) => {
  await page.goto("/");
  await expect(output(page)).toContainText("Ready.");
  await page.getByRole("button", { name: "Examples…" }).click();
  const dialog = page.getByRole("dialog", { name: "Examples" });
  const boards = dialog.getByRole("list", { name: "Boards" });
  const examples = dialog.getByRole("list", { name: "Examples" });
  const focused = page.locator(":focus");
  // Down from the search field: the selected board.
  const first = await boards.locator("button.selected").textContent();
  await page.keyboard.press("ArrowDown");
  await expect(boards.locator("button.selected")).toBeFocused();
  // Down to the next entry: a board is selected as a click would; a
  // family's toggle only takes the focus. Up comes back.
  await page.keyboard.press("ArrowDown");
  await expect(focused).not.toHaveText(first ?? "");
  if (!(await focused.evaluate((el) => el.classList.contains("family-toggle")))) {
    await expect(focused).toHaveClass(/selected/);
  }
  await page.keyboard.press("ArrowUp");
  await expect(boards.locator("button.selected")).toHaveText(first ?? "");
  await expect(boards.locator("button.selected")).toBeFocused();
  // Tab: on to the examples list, one stop; Down selects the next example.
  await page.keyboard.press("Tab");
  const ex = examples.getByRole("button");
  await expect(ex.first()).toBeFocused();
  await page.keyboard.press("ArrowDown");
  await expect(ex.nth(1)).toBeFocused();
  await expect(ex.nth(1)).toHaveClass(/selected/);
  await page.keyboard.press("ArrowUp");
  await expect(ex.first()).toHaveClass(/selected/);
  // Enter opens it.
  await page.keyboard.press("Enter");
  await expect(dialog).toBeHidden();
  await expect(tab(page, "pinout.pcf")).toBeVisible();

  // The file tab menu, as a native one: nothing highlighted when it opens,
  // Down goes to its first item, then the next; Escape closes.
  await tab(page, "pinout.pcf").click({ button: "right" });
  const menuItems = page.getByRole("menu").getByRole("menuitem");
  const enabled = menuItems.and(page.locator(":enabled"));
  await expect(enabled.first()).not.toBeFocused();
  await page.keyboard.press("ArrowDown");
  await expect(enabled.first()).toBeFocused();
  await page.keyboard.press("ArrowDown");
  await expect(enabled.nth(1)).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("menu")).toHaveCount(0);
  // Its keys work while it's open even when something else, such as the
  // editor, has the focus: Down goes to its first item.
  await tab(page, "pinout.pcf").click({ button: "right" });
  await page.locator(".cm-content").focus();
  await page.keyboard.press("ArrowDown");
  await expect(enabled.first()).toBeFocused();
  await page.locator(".cm-content").focus();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("menu")).toHaveCount(0);

  // The Board Explorer: after a click on an item (which macOS doesn't
  // focus by itself), Down shows the next one.
  await menu(page, "board-explorer");
  const explorer = page.getByRole("dialog", { name: "Board Explorer" });
  const items = explorer.locator("button.item");
  await items.first().click();
  await expect(items.first()).toBeFocused();
  await page.keyboard.press("ArrowDown");
  await expect(items.nth(1)).toBeFocused();
  await expect(items.nth(1)).toHaveClass(/selected/);
  await page.keyboard.press("Escape");
});

test("examples show their summary and notes, selected or not", async ({ page }) => {
  await page.goto("/");
  await expect(output(page)).toContainText("Ready.");
  await page.getByRole("button", { name: "Examples…" }).click();
  const dialog = page.getByRole("dialog", { name: "Examples" });
  await dialog.getByRole("searchbox").fill("Primer 20K Lite");
  const blinky = dialog.getByRole("button", { name: /^blinky/ });
  const points = ["Blinks a segment of a 7-segment PMOD", "Not yet tried on the board."];
  await expect(blinky.locator(".point")).toHaveText(points);
  // Not selected: the note still shows.
  await dialog.getByRole("button", { name: /^bare-minimum/ }).click();
  await expect(blinky).not.toHaveClass(/selected/);
  await expect(blinky.locator(".point")).toHaveText(points);
});

test("a failed command's banner says why, with one red line", async ({ page }) => {
  await start(page);
  await page.evaluate(() => {
    (window as unknown as { __fake: { saveAsError: unknown } }).__fake.saveAsError = {
      message: "Couldn't save the project.",
      details:
        "Couldn't save the project.: /Users/you/fpga/blinky is inside a Logix IDE project. Choose another location.",
    };
  });
  await menu(page, "save-as");
  // The reason is in the banner, so the closed output doesn't hide it.
  await expect(page.getByRole("alert")).toContainText(
    "Couldn't save the project. /Users/you/fpga/blinky is inside a Logix IDE project. Choose another location.",
  );
  // The status line repeats the message without a second red line.
  await expect(status(page)).toHaveText("Couldn't save the project.");
  await expect(page.locator(".status-bar")).not.toHaveClass(/error/);
});

test("selects a board from the connected devices", async ({ page }) => {
  await start(page);
  await page.evaluate(() => (window as unknown as { __fake: { useAlhambra: () => void } }).__fake.useAlhambra());
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
    "/Users/you/Documents/OpenLogix/IDE",
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

test("a tab's right-click menu has the commands for its file", async ({ page }) => {
  await start(page);
  await chooseBoard(page);
  type Fake = { diagramShown: { key: string; module: string | null } | null };
  await tab(page, "main.v").first().click({ button: "right" });
  const menu = page.getByRole("menu");
  await expect(menu.getByRole("menuitem", { name: "Auto Format" })).toBeVisible();
  await expect(menu.getByRole("menuitem", { name: "Run Test" })).toHaveCount(0);
  await menu.getByRole("menuitem", { name: "Module Diagram" }).click();
  await expect(status(page)).toContainText("Diagram ready");
  // main.v's module is main, the top module: the whole design.
  const shown = await page.evaluate(() => (window as unknown as { __fake: Fake }).__fake.diagramShown);
  expect(shown?.module).toBeNull();
});

test("the Serial Monitor opens the board's serial port, shows what arrives and sends what's typed", async ({ page }) => {
  type Fake = {
    serialOpened: { port: string; baud: number; device: string | null }[];
    serialSent: string[];
    serialReceive: (text: string, events?: string[]) => void;
    savedBaud: number | null | undefined;
  };
  const fake = () => page.evaluate(() => (window as unknown as { __fake: Fake }).__fake);
  await start(page);
  await menu(page, "serial-monitor");
  const monitor = page.getByRole("log", { name: "Serial Monitor output" });
  // Opened by itself: the board's UART, as Logix Engine marks it, with the device.
  await expect(monitor).toContainText("Opened /dev/cu.usbmodem0700000001 (the iCESugar v1.5, interface 1) at 115200 baud.");
  expect((await fake()).serialOpened).toEqual([
    { port: "/dev/cu.usbmodem0700000001", baud: 115200, device: "usb:1-1:1d50:602b:0700000001" },
  ]);
  // Text from the board, in pieces, becomes lines.
  await page.evaluate(() => {
    const f = (window as unknown as { __fake: Fake }).__fake;
    f.serialReceive("Hello from ");
    f.serialReceive("your FPGA board\r\n");
  });
  await expect(monitor.locator(".line").nth(-2)).toHaveText("Hello from your FPGA board");
  // A typed line goes out with the chosen ending, and the board echoes it.
  const input = page.getByRole("textbox", { name: "Message to send" });
  await input.fill("AB");
  await input.press("Enter");
  await expect.poll(async () => (await fake()).serialSent).toEqual(["AB\n"]);
  await expect(monitor.locator(".line").nth(-2)).toHaveText("AB");
  // The line feed started the next line at once, empty, as in a terminal.
  await expect(monitor.locator(".line").last()).toHaveText("");
  // The engine's notes, such as a pause for Upload, show in the output.
  await page.evaluate(() =>
    (window as unknown as { __fake: Fake }).__fake.serialReceive("", [
      "The serial port was closed while the board is programmed.",
    ]),
  );
  await expect(monitor).toContainText("The serial port was closed while the board is programmed.");
  // Clear empties the tab shown: the Serial Monitor here, not the output.
  await page.getByRole("button", { name: "Clear", exact: true }).click();
  await expect(monitor.locator(".line")).toHaveCount(0);
  await expect(output(page)).toBeHidden();
  await page.evaluate(() => (window as unknown as { __fake: Fake }).__fake.serialReceive("Hello from your FPGA board\r\n"));
  await expect(monitor.locator(".line").last()).toHaveText("Hello from your FPGA board");
  // The Output tab and back: the monitor kept its lines.
  await page.getByRole("tab", { name: "Output" }).click();
  await expect(output(page)).toBeVisible();
  await expect(output(page)).toContainText("Ready.");
  await page.getByRole("tab", { name: "Serial Monitor" }).click();
  await expect(monitor).toContainText("Hello from your FPGA board");
  // Another speed reopens the port at it, and is saved with the project.
  await page.getByRole("combobox", { name: "Baud rate" }).selectOption("9600");
  await expect.poll(async () => (await fake()).serialOpened.at(-1)?.baud).toBe(9600);
  await menu(page, "save-as");
  await expect.poll(async () => (await fake()).savedBaud).toBe(9600);
});

test("the Serial Monitor asks for the port when it can't tell, once", async ({ page }) => {
  type Fake = { useAlhambra: () => void; serialOpened: { port: string }[] };
  const fake = () => page.evaluate(() => (window as unknown as { __fake: Fake }).__fake);
  await start(page);
  // The Alhambra II's two ports, while the toolbar's board is the iCESugar
  // v1.5: neither is the board's UART, so none opens by itself.
  await page.evaluate(() => (window as unknown as { __fake: Fake }).__fake.useAlhambra());
  await menu(page, "serial-monitor");
  const pick = page.getByRole("combobox", { name: "Serial port" });
  await expect(pick).toBeVisible();
  await expect(page.getByRole("status").filter({ hasText: "Choose the board's serial port." })).toBeVisible();
  // The port shows once: the menu, no name beside it.
  await expect(page.locator(".monitor .port")).not.toContainText("/dev/cu.usbserial-FTATANKC1No port");
  expect((await fake()).serialOpened).toEqual([]);
  // Choosing one opens it.
  await pick.selectOption("/dev/cu.usbserial-FTATANKC1");
  await expect(page.getByRole("log", { name: "Serial Monitor output" })).toContainText(
    "Opened /dev/cu.usbserial-FTATANKC1",
  );
  expect((await fake()).serialOpened.map((o) => o.port)).toEqual(["/dev/cu.usbserial-FTATANKC1"]);
});
