/**
 * Up and Down arrow keys in a list (the user's request, 2026-11-18: Up and
 * Down only, never Left and Right): a Svelte action on the list's element.
 * Down moves the focus to the next entry, Up to the previous one, stopping
 * at the ends; Enter and Space then do what a click does, as on any button.
 *
 * - `items`: a CSS selector for the list's entries (focusable elements, such
 *   as its buttons). Disabled and hidden ones are skipped.
 * - `select`: an optional selector for entries that a move also clicks, so
 *   moving shows them as clicking does (a board in Examples shows its
 *   examples). Others, such as a group's open/close toggle, are only
 *   focused.
 */
export type ArrowNavOptions = { items: string; select?: string };

/** The list's entries that can take the focus now, in order. */
export function navItems(node: HTMLElement, items: string): HTMLElement[] {
  return [...node.querySelectorAll<HTMLElement>(items)].filter(
    (el) => !(el as HTMLButtonElement).disabled && el.getClientRects().length > 0,
  );
}

/** Focuses `el`, scrolls it into view and, when it matches `select`, clicks it. */
export function moveTo(el: HTMLElement, select?: string) {
  el.focus();
  el.scrollIntoView({ block: "nearest" });
  if (select && el.matches(select)) el.click();
}

export function arrowNav(node: HTMLElement, options: ArrowNavOptions) {
  let opts = options;
  function onKey(e: KeyboardEvent) {
    if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
    if (e.altKey || e.ctrlKey || e.metaKey || e.shiftKey) return;
    const list = navItems(node, opts.items);
    const at = list.indexOf(document.activeElement as HTMLElement);
    if (at < 0) return;
    e.preventDefault();
    const next = list[at + (e.key === "ArrowDown" ? 1 : -1)];
    if (next) moveTo(next, opts.select);
  }
  node.addEventListener("keydown", onKey);
  return {
    update(o: ArrowNavOptions) {
      opts = o;
    },
    destroy() {
      node.removeEventListener("keydown", onKey);
    },
  };
}
