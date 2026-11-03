import type { ShowOutput } from "./backend";

/** Smallest console and editor heights (px) the divider allows. */
export const MIN_CONSOLE_HEIGHT = 60;
export const MIN_EDITOR_HEIGHT = 80;
export const DEFAULT_CONSOLE_HEIGHT = 180;
/**
 * Dragging the divider below this console height (px) hides the output,
 * as Hide does: the area shrinks to the status bar while dragging and the
 * output is hidden on release.
 */
export const COLLAPSE_CONSOLE_HEIGHT = 40;

/** Whether a divider drag to `requested` px hides the output. */
export function dragHidesConsole(requested: number): boolean {
  return requested < COLLAPSE_CONSOLE_HEIGHT;
}

/**
 * Clamps a requested console height so both the console and the editor
 * stay usable inside a split area `areaHeight` px tall (`dividerHeight`
 * included).
 */
export function clampConsoleHeight(
  requested: number,
  areaHeight: number,
  dividerHeight: number,
): number {
  const max = Math.max(
    MIN_CONSOLE_HEIGHT,
    areaHeight - dividerHeight - MIN_EDITOR_HEIGHT,
  );
  return Math.round(Math.min(max, Math.max(MIN_CONSOLE_HEIGHT, requested)));
}

/**
 * Whether the hidden output comes back by itself: when a command starts
 * (Verify, Upload, Run Tests, Simulate), or when something fails, per the
 * "Show the output" setting.
 */
export function outputComesBack(when: ShowOutput, reason: "command" | "error"): boolean {
  if (when === "never") return false;
  return reason === "error" || when === "onCommand";
}
