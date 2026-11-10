// The Board Explorer's model (Tools > Board Explorer…): the FPGA families,
// FPGAs and boards Logix Engine knows (the project's own boards included)
// and Logix IDE's examples, as one set of items of four kinds that link to each
// other. Every kind works the same way: a list grouped by family (families
// themselves are one flat list), and details made of facts, link sections
// and text sections. The words are the IDE's; the facts come from the
// engine's catalog and the examples.

import type { Board, Example, FpgaFamily } from "./backend";
import { summary as variantLabel } from "./variants";

export type Kind = "family" | "fpga" | "board" | "example";

/** One item: its kind and its ID (an example's is "<board>/<name>"). */
export interface Ref {
  kind: Kind;
  id: string;
}

/** The kinds in the order of the Explorer's tabs, with their names. */
export const KINDS: { kind: Kind; label: string }[] = [
  { kind: "family", label: "Families" },
  { kind: "fpga", label: "FPGAs" },
  { kind: "board", label: "Boards" },
  { kind: "example", label: "Examples" },
];

export function sameRef(a: Ref | null, b: Ref | null): boolean {
  return !!a && !!b && a.kind === b.kind && a.id === b.id;
}

export function exampleId(e: Pick<Example, "board" | "name">): string {
  return `${e.board}/${e.name}`;
}

interface FamilyNode {
  id: string;
  label: string;
  pinExtension: string | null;
  order: number;
}

interface FpgaNode {
  id: string;
  partNumber: string;
  size: string;
  family: string;
}

/** Everything the Explorer shows, with the links between items. */
export interface Model {
  families: FamilyNode[];
  fpgas: Map<string, FpgaNode>;
  boards: Map<string, Board>;
  examples: Map<string, Example>;
  boardsOfFpga: Map<string, Board[]>;
  examplesOfBoard: Map<string, Example[]>;
}

const OTHER = "";

/**
 * The model from the engine's families (with their FPGAs), the boards
 * offered and the examples. Boards and examples whose family or FPGA the
 * engine doesn't describe still show up, under "Other".
 */
export function buildModel(families: FpgaFamily[], boards: Board[], examples: Example[]): Model {
  const fams: FamilyNode[] = families.map((f, i) => ({
    id: f.id,
    label: f.label,
    pinExtension: f.pinExtension,
    order: i,
  }));
  const fpgas = new Map<string, FpgaNode>();
  for (const f of families) for (const x of f.fpgas) fpgas.set(x.id, { ...x, family: f.id });
  const boardMap = new Map(boards.map((b) => [b.id, b]));
  const boardsOfFpga = new Map<string, Board[]>();
  for (const b of boards) {
    if (!b.fpga) continue;
    boardsOfFpga.set(b.fpga.id, [...(boardsOfFpga.get(b.fpga.id) ?? []), b]);
  }
  const exampleMap = new Map(examples.map((e) => [exampleId(e), e]));
  const examplesOfBoard = new Map<string, Example[]>();
  for (const e of examples) examplesOfBoard.set(e.board, [...(examplesOfBoard.get(e.board) ?? []), e]);
  const known = new Set(fams.map((f) => f.id));
  if (boards.some((b) => !known.has(b.family)) || examples.some((e) => !boardMap.has(e.board))) {
    fams.push({ id: OTHER, label: "Other", pinExtension: null, order: fams.length });
  }
  return { families: fams, fpgas, boards: boardMap, examples: exampleMap, boardsOfFpga, examplesOfBoard };
}

/** An item in a list: its name, a short hint on the right, and its family. */
export interface ListItem {
  ref: Ref;
  label: string;
  hint: string;
  family: string;
  /** One of the project's own boards (boards folder). */
  custom?: boolean;
}

/** A list's group: a family's items (families are one group, `label` null). */
export interface ListGroup {
  key: string;
  label: string | null;
  items: ListItem[];
}

function familyOf(m: Model, kind: Kind, id: string): string {
  if (kind === "family") return id;
  if (kind === "fpga") return m.fpgas.get(id)?.family ?? OTHER;
  if (kind === "board") {
    const f = m.boards.get(id)?.family ?? OTHER;
    return m.families.some((x) => x.id === f) ? f : OTHER;
  }
  const e = m.examples.get(id);
  return e ? familyOf(m, "board", e.board) : OTHER;
}

/** The family an item is listed under. */
export function familyOfRef(m: Model, ref: Ref): FamilyNode | undefined {
  const id = familyOf(m, ref.kind, ref.id);
  return m.families.find((f) => f.id === id);
}

/** An item's name for people. */
export function labelOf(m: Model, ref: Ref): string {
  switch (ref.kind) {
    case "family":
      return m.families.find((f) => f.id === ref.id)?.label ?? ref.id;
    case "fpga":
      return m.fpgas.get(ref.id)?.partNumber ?? ref.id;
    case "board":
      return m.boards.get(ref.id)?.description ?? ref.id;
    case "example":
      return m.examples.get(ref.id)?.name ?? ref.id;
  }
}

/** Compares names for sorting, ignoring case. */
function caseless(a: string, b: string): number {
  return a.localeCompare(b, undefined, { sensitivity: "base" });
}

function count(n: number, what: string): string {
  return `${n} ${what}${n === 1 ? "" : "s"}`;
}

function familyBoards(m: Model, family: string): Board[] {
  return [...m.boards.values()].filter((b) => familyOf(m, "board", b.id) === family);
}

/** Every item of a kind, by family in the engine's order. */
function allItems(m: Model, kind: Kind): ListItem[] {
  const item = (id: string, label: string, hint: string): ListItem => ({
    ref: { kind, id },
    label,
    hint,
    family: familyOf(m, kind, id),
  });
  const byName = (a: ListItem, b: ListItem) => caseless(a.label, b.label);
  switch (kind) {
    case "family":
      return m.families.map((f) => item(f.id, f.label, count(familyBoards(m, f.id).length, "board")));
    case "fpga":
      return [...m.fpgas.values()]
        .map((f) => {
          const n = m.boardsOfFpga.get(f.id)?.length ?? 0;
          return item(f.id, f.partNumber, n ? count(n, "board") : "");
        })
        .sort(byName);
    case "board":
      return [...m.boards.values()]
        .map((b) => ({ ...item(b.id, b.description, b.fpga?.partNumber ?? ""), custom: b.custom }))
        .sort(byName);
    case "example":
      return [...m.examples.values()]
        .map((e) => item(exampleId(e), e.name, labelOf(m, { kind: "board", id: e.board })))
        .sort((a, b) => byName(a, b) || caseless(a.hint, b.hint));
  }
}

/** The words an item is found by. */
function searchText(m: Model, ref: Ref): string[] {
  switch (ref.kind) {
    case "family":
      return [labelOf(m, ref), ref.id];
    case "fpga":
      return [labelOf(m, ref), ref.id];
    case "board": {
      const b = m.boards.get(ref.id);
      return [labelOf(m, ref), ref.id, b?.fpga?.partNumber ?? "", b?.fpga?.id ?? ""];
    }
    case "example": {
      const e = m.examples.get(ref.id);
      return [e?.name ?? "", e?.summary ?? "", labelOf(m, { kind: "board", id: e?.board ?? "" }), e?.board ?? ""];
    }
  }
}

/**
 * A tab's list: the items of `kind`, grouped by family (families: one
 * group). With a search query, only matching items; a family whose name
 * matches keeps all its items; empty groups are left out.
 */
export function listGroups(m: Model, kind: Kind, query: string): ListGroup[] {
  const q = query.trim().toLowerCase();
  const has = (t: string) => t.toLowerCase().includes(q);
  const items = allItems(m, kind);
  if (kind === "family") {
    const shown = q ? items.filter((i) => searchText(m, i.ref).some(has)) : items;
    return shown.length ? [{ key: "families", label: null, items: shown }] : [];
  }
  return m.families
    .map((f) => {
      const all = items.filter((i) => i.family === f.id);
      const shown = !q || has(f.label) ? all : all.filter((i) => searchText(m, i.ref).some(has));
      return { key: f.id || "other", label: f.label, items: shown };
    })
    .filter((g) => g.items.length > 0);
}

// ---- Details ----

/** A fact: a label and its value, which may link to another item. */
export interface Fact {
  label: string;
  value: string;
  link?: Ref;
  /** Shown as code, such as an ID to type in a file. */
  code?: boolean;
}

/** One row of a board's resources. */
export interface ResourceRow {
  name: string;
  type: string;
  pins: string;
  notes: string;
}

/** A section of the details: links, lines of text, or the resources. */
/** A link in the details: the item, its name, a hint, and whether it's a custom board. */
export interface Link {
  ref: Ref;
  label: string;
  hint: string;
  custom?: boolean;
}

export type Section =
  | { title: string; kind: "links"; links: Link[]; empty: string }
  | { title: string; kind: "lines"; lines: string[] }
  | { title: string; kind: "resources"; rows: ResourceRow[]; empty: string };

export interface Details {
  /** "Board", "FPGA" and so on, before the title ("Custom board" for the project's own). */
  kindLabel: string;
  title: string;
  facts: Fact[];
  sections: Section[];
}

const KIND_LABEL: Record<Kind, string> = { family: "FPGA family", fpga: "FPGA", board: "Board", example: "Example" };

function boardLinks(m: Model, boards: Board[]): Link[] {
  return [...boards]
    .sort((a, b) => caseless(a.description, b.description))
    .map((b) => ({ ref: { kind: "board", id: b.id }, label: b.description, hint: "", custom: b.custom }));
}

function exampleLinks(m: Model, examples: Example[], withBoard: boolean): Link[] {
  return examples.map((e) => ({
    ref: { kind: "example", id: exampleId(e) },
    label: e.name,
    hint: withBoard ? labelOf(m, { kind: "board", id: e.board }) : e.summary,
  }));
}

function familyFact(m: Model, family: string): Fact {
  const f = m.families.find((x) => x.id === family);
  return f && f.id !== OTHER
    ? { label: "Family", value: f.label, link: { kind: "family", id: f.id } }
    : { label: "Family", value: "Unknown" };
}

/** The details of an item, or null if the model doesn't have it. */
export function detailsOf(m: Model, ref: Ref): Details | null {
  const base = { kindLabel: KIND_LABEL[ref.kind], title: labelOf(m, ref) };
  switch (ref.kind) {
    case "family": {
      const f = m.families.find((x) => x.id === ref.id);
      if (!f) return null;
      const fpgas = [...m.fpgas.values()].filter((x) => x.family === f.id);
      const boards = familyBoards(m, f.id);
      const examples = boards.flatMap((b) => m.examplesOfBoard.get(b.id) ?? []);
      return {
        ...base,
        facts: [
          ...(f.id ? [{ label: "Family ID", value: f.id, code: true }] : []),
          ...(f.pinExtension ? [{ label: "Pin file", value: `.${f.pinExtension}` }] : []),
          { label: "FPGAs", value: String(fpgas.length) },
          { label: "Boards", value: String(boards.length) },
          { label: "Examples", value: String(examples.length) },
        ],
        sections: [
          { title: "Boards", kind: "links", links: boardLinks(m, boards), empty: "No boards yet." },
          {
            title: "FPGAs",
            kind: "links",
            links: fpgas
              .sort((a, b) => a.partNumber.localeCompare(b.partNumber))
              .map((x) => {
                const n = m.boardsOfFpga.get(x.id)?.length ?? 0;
                return { ref: { kind: "fpga", id: x.id }, label: x.partNumber, hint: n ? count(n, "board") : "" };
              }),
            empty: "None.",
          },
        ],
      };
    }
    case "fpga": {
      const f = m.fpgas.get(ref.id);
      if (!f) return null;
      const boards = m.boardsOfFpga.get(f.id) ?? [];
      return {
        ...base,
        facts: [
          familyFact(m, f.family),
          { label: "Size", value: f.size },
          { label: "FPGA ID", value: f.id, code: true },
        ],
        sections: [
          { title: "Boards", kind: "links", links: boardLinks(m, boards), empty: "No boards with this FPGA yet." },
          {
            title: "Examples",
            kind: "links",
            links: exampleLinks(m, boards.flatMap((b) => m.examplesOfBoard.get(b.id) ?? []), true),
            empty: "None.",
          },
        ],
      };
    }
    case "board": {
      const b = m.boards.get(ref.id);
      if (!b) return null;
      if (b.custom) base.kindLabel = "Custom board";
      const same = b.fpga ? (m.boardsOfFpga.get(b.fpga.id) ?? []).filter((x) => x.id !== b.id) : [];
      return {
        ...base,
        facts: [
          familyFact(m, familyOf(m, "board", b.id)),
          b.fpga
            ? { label: "FPGA", value: b.fpga.partNumber, link: { kind: "fpga", id: b.fpga.id } }
            : { label: "FPGA", value: "Unknown" },
          ...(b.fpga?.size ? [{ label: "Size", value: b.fpga.size }] : []),
          ...(b.pinExtension ? [{ label: "Pin file", value: `.${b.pinExtension}` }] : []),
          { label: "Board ID", value: b.id, code: true },
          ...(b.custom ? [{ label: "From", value: "This project's boards folder" }] : []),
        ],
        sections: [
          {
            title: "Ways to program it",
            kind: "lines",
            lines: b.configurations.map(
              (c, i) =>
                `${variantLabel(b, c.when)}${i === 0 ? " (default)" : ""}${c.tested ? "" : " (not yet tried on a board)"}` +
                (c.userNotes ? `. ${c.userNotes}` : ""),
            ),
          },
          ...(b.requires?.length
            ? [{ title: "Every design must", kind: "lines" as const, lines: b.requires.map(requirementText) }]
            : []),
          { title: "Resources", kind: "resources", rows: resourceRows(b), empty: "Not described yet." },
          ...(aiHintLines(b).length ? [{ title: "AI hints", kind: "lines" as const, lines: aiHintLines(b) }] : []),
          { title: "Examples", kind: "links", links: exampleLinks(m, m.examplesOfBoard.get(b.id) ?? [], false), empty: "None." },
          { title: "Boards with the same FPGA", kind: "links", links: boardLinks(m, same), empty: "None." },
        ],
      };
    }
    case "example": {
      const e = m.examples.get(ref.id);
      if (!e) return null;
      const b = m.boards.get(e.board);
      return {
        ...base,
        facts: [
          { label: "Board", value: labelOf(m, { kind: "board", id: e.board }), link: { kind: "board", id: e.board } },
          ...(b?.fpga ? [{ label: "FPGA", value: b.fpga.partNumber, link: { kind: "fpga" as const, id: b.fpga.id } }] : []),
          familyFact(m, familyOf(m, "example", ref.id)),
        ],
        sections: [{ title: "About", kind: "lines", lines: [e.summary, ...e.notes].filter(Boolean) }],
      };
    }
  }
}

// ---- Resources ----

type Json = Record<string, unknown>;

const str = (v: unknown) => (v === null || v === undefined ? "" : String(v));
const pinList = (v: unknown) => (Array.isArray(v) ? v.map((p) => (p === null ? "–" : str(p))).join(", ") : str(v));

const TYPE_LABEL: Record<string, [string, string]> = {
  clock: ["Clock", "Clocks"],
  leds: ["LED", "LEDs"],
  rgb_led: ["RGB LED", "RGB LEDs"],
  led_matrix: ["LED matrix", "LED matrix"],
  buttons: ["Button", "Buttons"],
  switches: ["Switch", "Switches"],
  uart: ["UART", "UART"],
  pmod: ["PMOD", "PMOD"],
  header: ["Header", "Header"],
  unclassified: ["Other pins", "Other pins"],
};

/** What a requirement asks, in words. */
export function requirementText(q: Json): string {
  if (q.type === "drive") {
    const pins = Array.isArray(q.pins) ? q.pins.map(str).join(", ") : "";
    return `Drive ${str(q.name)} (${pins}) ${str(q.level)}${q.why ? `: ${str(q.why)}` : ""}`;
  }
  if (q.type === "setup") return str(q.text);
  return str(q.text ?? q.type);
}

/**
 * A board's `ai_hints`, the board's own first, then each resource's by port
 * name ("rgb_led: …"): text for AI agents writing designs, shown as is.
 */
export function aiHintLines(b: Pick<Board, "aiHints" | "resources">): string[] {
  const own = b.aiHints ?? [];
  const per = Object.entries(b.resources ?? {})
    .sort(([a], [z]) => a.localeCompare(z))
    .flatMap(([name, r]) => (Array.isArray(r.ai_hints) ? r.ai_hints.map((h) => `${name}: ${str(h)}`) : []));
  return [...own, ...per];
}

/**
 * A board's resources as rows, by port name: the type in words, the pins
 * (labeled where a type has several kinds, such as a UART's TX and RX),
 * and notes (frequency, active level, I/O standard, requirements, notes).
 */
export function resourceRows(b: Pick<Board, "resources">): ResourceRow[] {
  return Object.entries(b.resources ?? {})
    .map(([name, raw]) => {
      const r = raw as Json;
      const t = str(r.type);
      const many = Array.isArray(r.pins) && r.pins.length > 1;
      const type = TYPE_LABEL[t]?.[many ? 1 : 0] ?? t;
      const labeled = (fields: [string, string][]) =>
        fields
          .filter(([f]) => r[f] !== undefined)
          .map(([f, l]) => `${l} ${str(r[f])}`)
          .join(", ");
      let pins: string;
      if (t === "clock") pins = str(r.pin);
      else if (t === "rgb_led") pins = labeled([["red", "R"], ["green", "G"], ["blue", "B"]]);
      else if (t === "uart") pins = labeled([["tx", "TX"], ["rx", "RX"], ["rts", "RTS"], ["cts", "CTS"]]);
      else if (t === "led_matrix") {
        const colors = (r.colors ?? {}) as Json;
        const c = [["red", "R"], ["green", "G"], ["blue", "B"]]
          .filter(([k]) => colors[k] !== undefined)
          .map(([k, l]) => `${l} ${str(colors[k])}`);
        pins = [pinList(r.leds), c.join(", ")].filter(Boolean).join("; ");
      } else pins = pinList(r.pins);
      const notes = [
        t === "clock" && r.mhz !== undefined ? `${str(r.mhz)} MHz` : "",
        r.active ? `active ${str(r.active)}` : "",
        str(r.io_standard),
        ...(Array.isArray(r.requires) ? r.requires.map((q) => `Needs: ${requirementText(q as Json)}`) : []),
        str(r.user_notes),
      ].filter(Boolean);
      return { name, type, pins, notes: notes.join(". ") };
    })
    .sort((a, b) => a.name.localeCompare(b.name));
}
