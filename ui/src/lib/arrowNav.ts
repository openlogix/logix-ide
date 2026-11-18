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
 * - `whileOpen`: Up and Down work wherever the focus is, as long as the
 *   list is shown (a menu, which something else may take the focus from
 *   right after it opens); from outside it, Down goes to the first entry
 *   and Up to the last.
 *
 * A click on an entry also gives it the focus: macOS's web view doesn't
 * focus a clicked button, and the arrows then had nothing to start from
 * (found by the user, 2026-11-18).
 */
export type ArrowNavOptions = { items: string; select?: string; whileOpen?: boolean };

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
    const down = e.key === "ArrowDown";
    const list = navItems(node, opts.items);
    const at = list.indexOf(document.activeElement as HTMLElement);
    if (at < 0) {
      if (!opts.whileOpen || list.length === 0) return;
      e.preventDefault();
      e.stopPropagation();
      moveTo(down ? list[0] : list[list.length - 1], opts.select);
      return;
    }
    e.preventDefault();
    if (opts.whileOpen) e.stopPropagation();
    const next = list[at + (down ? 1 : -1)];
    if (next) moveTo(next, opts.select);
  }
  function onClick(e: MouseEvent) {
    const el = (e.target as Element | null)?.closest<HTMLElement>(opts.items);
    if (el && node.contains(el) && document.activeElement !== el) el.focus();
  }
  // A list that works while open listens on the window, first (capture),
  // so a focused editor doesn't take the keys.
  const target: HTMLElement | Window = opts.whileOpen ? window : node;
  target.addEventListener("keydown", onKey as EventListener, opts.whileOpen);
  node.addEventListener("click", onClick);
  return {
    update(o: ArrowNavOptions) {
      opts = o;
    },
    destroy() {
      target.removeEventListener("keydown", onKey as EventListener, opts.whileOpen);
      node.removeEventListener("click", onClick);
    },
  };
}
