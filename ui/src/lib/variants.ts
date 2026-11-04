// A board's variants: its dimensions (every board has the engine's two
// standard ones, Connection and Memory) and the configurations that exist.
// A dimension with more than one value is a choice, shown next to the board
// and in Tools; one with a single value is a fact (e.g. "uploads to RAM").
// Pure functions, unit-tested in variants.test.ts.
//
// A project stores the values the user chose (`[variant]` in
// logix-ide.toml), only when they aren't the board's default. As in the
// engine, the first configuration with those values is used, and values the
// board doesn't have are ignored (so an old project still opens).

import type {
  Board,
  Configuration,
  ConnectedBoard,
  Variant,
  VariantMenu,
} from "./backend";

const matches = (c: Configuration, variant: Variant) =>
  Object.entries(variant).every(([k, v]) => c.when[k] === v);

/** The variant without values the board doesn't have. */
function known(board: Board, variant: Variant): Variant {
  const out: Variant = {};
  for (const d of board.dimensions) {
    const v = variant[d.id];
    if (v !== undefined && d.values.some((x) => x.id === v)) out[d.id] = v;
  }
  return out;
}

/**
 * The configuration a variant picks: the first with its values, or else
 * the board's default. Null only for a board without configurations.
 */
export function configurationOf(board: Board, variant: Variant): Configuration | null {
  const wanted = known(board, variant);
  return (
    board.configurations.find((c) => matches(c, wanted)) ??
    board.configurations[0] ??
    null
  );
}

/** Every dimension's value for the variant (what the engine will use). */
export function resolve(board: Board, variant: Variant): Variant {
  return { ...(configurationOf(board, variant)?.when ?? {}) };
}

/** What to store for a variant: {} for the board's default configuration. */
export function toStore(board: Board, variant: Variant): Variant {
  const c = configurationOf(board, variant);
  return c && c !== board.configurations[0] ? { ...c.when } : {};
}

/** The label of a dimension's value, e.g. "RAM" (the id if unknown). */
export function valueLabel(board: Board, dim: string, value: string): string {
  const d = board.dimensions.find((x) => x.id === dim);
  return d?.values.find((v) => v.id === value)?.label ?? value;
}

/** The dimensions the user can choose in: those with more than one value. */
export function choices(board: Board | null) {
  return board ? board.dimensions.filter((d) => d.values.length > 1) : [];
}

/** Whether the board has anything to choose (the toolbar's option button). */
export function hasChoice(board: Board | null): boolean {
  return choices(board).length > 0;
}

/**
 * The toolbar's summary: the value of every dimension with a choice, e.g.
 * "FT232H adapter · RAM". Empty for a board without choices.
 */
export function summary(board: Board, variant: Variant): string {
  const when = resolve(board, variant);
  return choices(board)
    .map((d) => valueLabel(board, d.id, when[d.id]))
    .join(" · ");
}

/**
 * How an upload reaches the board, as a fact, e.g. "Uploads to RAM through
 * the FT232H adapter.". The connection is left out when it's the board's
 * own USB port or not known.
 */
export function uploadFact(board: Board, variant: Variant): string {
  const when = resolve(board, variant);
  const memory = when.memory ? valueLabel(board, "memory", when.memory) : null;
  if (!memory) return "";
  const connection = when.connection;
  const through =
    connection && !["usb", "unspecified"].includes(connection)
      ? ` through the ${valueLabel(board, "connection", connection)}`
      : "";
  return `Uploads to ${memory}${through}.`;
}

/** After a successful upload: what to know, e.g. that RAM is lost at power-off. */
export function afterUpload(board: Board, variant: Variant): string | null {
  return resolve(board, variant).memory === "ram"
    ? "Uploaded to RAM: the design is lost when the board is powered off."
    : null;
}

/** Whether `value` exists with the variant's other current values. */
export function isAvailable(
  board: Board,
  variant: Variant,
  dim: string,
  value: string,
): boolean {
  const when = { ...resolve(board, variant), [dim]: value };
  return board.configurations.some((c) => matches(c, when));
}

/**
 * Chooses `value` for `dim`. If it doesn't exist with the other current
 * values, the first configuration that has it is used, and `changed` names
 * the other dimensions that changed, e.g. ["Connection: FT232H adapter"].
 * Returns the variant to store.
 */
export function choose(
  board: Board,
  variant: Variant,
  dim: string,
  value: string,
): { variant: Variant; changed: string[] } {
  const current = resolve(board, variant);
  const wanted = { ...current, [dim]: value };
  const c =
    board.configurations.find((x) => matches(x, wanted)) ??
    board.configurations.find((x) => x.when[dim] === value);
  if (!c) return { variant: toStore(board, variant), changed: [] };
  const changed = choices(board)
    .filter((d) => d.id !== dim && c.when[d.id] !== current[d.id])
    .map((d) => `${d.label}: ${valueLabel(board, d.id, c.when[d.id])}`);
  return { variant: toStore(board, c.when), changed };
}

/** The Tools submenus for the board's choices (none for most boards). */
export function menus(board: Board | null, variant: Variant): VariantMenu[] {
  if (!board) return [];
  const when = resolve(board, variant);
  return choices(board).map((d) => ({
    id: d.id,
    label: d.label,
    values: d.values.map((v) => ({
      id: v.id,
      label: v.label,
      checked: when[d.id] === v.id,
      enabled: isAvailable(board, variant, d.id, v.id),
    })),
  }));
}

/** Whether the board is connected the way its variant programs it. */
export function isConnected(
  board: Board | null,
  boardId: string | null,
  variant: Variant,
  connected: ConnectedBoard[],
): boolean {
  if (!boardId) return false;
  const found = connected.find((c) => c.board === boardId);
  if (!found) return false;
  if (!board || !hasChoice(board)) return true;
  const when = resolve(board, variant);
  return found.variants.some((v) =>
    Object.entries(when).every(([k, x]) => v[k] === x),
  );
}

/**
 * Parses a Tools submenu id, "variant:<dimension>:<value>". Null for other
 * menu ids.
 */
export function parseMenuId(id: string): { dim: string; value: string } | null {
  const m = /^variant:([^:]+):(.+)$/.exec(id);
  return m ? { dim: m[1], value: m[2] } : null;
}
