/** Smallest console and editor heights (px) the divider allows. */
export const MIN_CONSOLE_HEIGHT = 60;
export const MIN_EDITOR_HEIGHT = 80;
export const DEFAULT_CONSOLE_HEIGHT = 180;

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
