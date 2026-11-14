/** Appends lines, keeping at most `max` of the newest ones. */
export function appendCapped<T>(list: T[], items: T[], max: number): T[] {
  const next = list.concat(items);
  return next.length > max ? next.slice(next.length - max) : next;
}
