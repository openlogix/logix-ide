/**
 * `use:focusOnMount`: focuses the element when it appears, such as a name
 * field after a click on "+". Not the `autofocus` attribute: Svelte applies
 * it only when nothing else has the focus, and on Linux (WebKitGTK) and
 * Windows (WebView2) the button just clicked has it, so the field stayed
 * unfocused there and typing went nowhere (found by the e2e tests on Linux,
 * 2026-11-16).
 */
export function focusOnMount(node: HTMLElement) {
  node.focus();
}
