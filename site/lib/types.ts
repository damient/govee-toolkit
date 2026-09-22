// The shape of the two inputs the renderers read: `dist/catalog.json`, which
// `xtask catalog` writes, and `content/reference.json`. Each type lists the
// fields a renderer reads and no more, so a field the site ignores can change
// without a change here.

/** A mode a device file can declare. `MODES` in `config.ts` holds the order. */
export type Mode = "lan" | "ble" | "cloud";

/** `unknown` is the default: nobody probed the mode. */
export type Support = "full" | "capped" | "partial" | "none" | "unknown";

/** An inclusive `[low, high]` pair. */
export type Range = [number, number];

export interface Catalog {
  schema_version: number;
  devices: Device[];
}

export interface Device {
  sku: string;
  name: string;
  family?: string;
  aliases?: string[];
  candidate_aliases?: string[];
  capabilities?: Record<string, Capability | null>;
  modes?: Partial<Record<Mode, ModeEntry>>;
  commands?: Partial<Record<Mode, Record<string, Command>>>;
  measurements?: { segment_grid?: unknown } & Record<string, unknown>;
  verified?: Verified;
  dmx?: { personalities?: Personality[] };
}

/** The segment counts are the fields a model page shows. */
export interface Capability {
  count?: number;
  native_pixels?: number;
}

export interface ModeEntry {
  support?: Support;
  capabilities?: string[];
}

export interface Command {
  role?: string;
  args?: Record<string, { role?: string; range?: Range }>;
}

export interface Verified {
  date?: string;
  firmware?: string;
  by?: string;
}

export interface Personality {
  personality: string;
  width?: number;
  /** Absent where the personality serves no table. `error` says why. */
  channels?: Channel[];
  error?: string;
}

export interface Channel {
  offset: number;
  slot: string;
  component?: string;
  zone?: number;
  unreached?: boolean;
  values?: { slots: Range; label: string }[];
}

export interface Reference {
  intro: string;
  groups: RefGroup[];
}

export interface RefGroup {
  id: string;
  title: string;
  modes?: Mode[];
  entries: RefEntry[];
}

/** One language id to one example. */
export type Examples = Record<string, string>;

/** The value of one `{name}` placeholder. */
export type Values = Record<string, string | number | undefined>;

export interface RefEntry {
  id: string;
  title: string;
  summary: string;
  detail?: string;
  modes?: Mode[];
  roles?: string[];
  args?: Record<string, string>;
  values?: Values;
  action?: { order: number; title: string; summary?: string; segments?: boolean };
  examples: Examples;
  adds?: { needs?: string[]; examples: Examples }[];
}

/** One entry of the documentation menu. */
export interface NavEntry {
  url: string;
  title: string;
  order: number;
}

/** One entry of an "On this page" menu. */
export interface TocEntry {
  id: string;
  text: string;
  code?: boolean;
  children?: TocEntry[];
}

/** A `[label, url]` pair of a breadcrumb trail. */
export type Crumb = [string, string];
