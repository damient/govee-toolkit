import { Failure, request, send } from "./http.ts";

export interface Share {
  /** `--listen` opens the server to the network already: the switch does nothing. */
  fixed: boolean;
  shared: boolean;
  /** The address for the other devices, while sharing is on. */
  url: string | null;
  /** This client can switch sharing: it runs on the computer of the server. */
  control: boolean;
  /** This client runs on the computer of the server: it can export and import. */
  local: boolean;
}

function isShare(value: unknown): value is Share {
  if (typeof value !== "object" || value === null) return false;
  return (
    "fixed" in value &&
    typeof value.fixed === "boolean" &&
    "shared" in value &&
    typeof value.shared === "boolean" &&
    "url" in value &&
    (value.url === null || typeof value.url === "string") &&
    "control" in value &&
    typeof value.control === "boolean" &&
    "local" in value &&
    typeof value.local === "boolean"
  );
}

function asShare(body: unknown): Share {
  if (!isShare(body)) throw new Failure("unreadable");
  return body;
}

/** `GET /api/share`. */
export async function share(): Promise<Share> {
  return asShare(await request("/api/share"));
}

/** `POST /api/share`: open the server to the network, or close it. */
export async function setShare(shared: boolean): Promise<Share> {
  return asShare(await send("/api/share", "POST", JSON.stringify({ shared })));
}

/** `POST /api/backup`: replace every file of the server with the backup `text`. */
export async function restore(text: string): Promise<void> {
  await send("/api/backup", "POST", text);
}

/** `GET /api/backup/restore`: the server keeps the files from before an import, so that Restore can put them back. */
export async function restorable(): Promise<boolean> {
  const answer = await request("/api/backup/restore");
  return typeof answer === "object" && answer !== null && "restorable" in answer && answer.restorable === true;
}

/** `POST /api/backup/restore`: put back the files from before the last import. */
export async function undoImport(): Promise<void> {
  await send("/api/backup/restore", "POST", "");
}

export interface ResetChoice {
  /** Theme, language and the advanced switch. */
  prefs: boolean;
  /** The `devices:` entries of the configuration file (names, modes, groups), and the rooms. */
  devices: boolean;
}

/** `POST /api/reset`. Only the computer of the server can reset. */
export async function reset(choice: ResetChoice): Promise<void> {
  await send("/api/reset", "POST", JSON.stringify(choice));
}

/** A device of the configuration, or a device that answered a `lan` scan. */
export interface Found {
  /** The device ID, as `AA:BB:CC:DD:EE:FF:00:11`. */
  id: string;
  /** `null` for a device of the configuration that gives no SKU. */
  sku: string | null;
  /** The name that the configuration gives it. */
  name: string | null;
  /** The family of its device file, for the icon. */
  family: string | null;
  /** The configuration has an entry for it. */
  configured: boolean;
  /** It answered the scan: `null` until a scan ends. */
  online: boolean | null;
}

/** The label of a light: its name, its SKU, or its ID. */
export const labelOf = (light: Found): string => light.name ?? light.sku ?? light.id;

const nullableString = (value: unknown): boolean => value === null || typeof value === "string";

type Record = Pick<Found, "id" | "sku" | "name" | "family">;

function isRecord(value: unknown): value is Record {
  if (typeof value !== "object" || value === null) return false;
  return (
    "id" in value &&
    typeof value.id === "string" &&
    "sku" in value &&
    nullableString(value.sku) &&
    "name" in value &&
    nullableString(value.name) &&
    "family" in value &&
    nullableString(value.family)
  );
}

function isScanned(value: unknown): value is Record & { configured: boolean } {
  return isRecord(value) && "configured" in value && typeof value.configured === "boolean";
}

export function listOf(body: unknown): unknown[] {
  if (typeof body !== "object" || body === null || !("devices" in body) || !Array.isArray(body.devices)) {
    throw new Failure("unreadable");
  }
  return body.devices;
}

const found = (record: Record, configured: boolean, online: boolean | null): Found => ({
  id: record.id,
  sku: record.sku,
  name: record.name,
  family: record.family,
  configured,
  online,
});

/** `GET /api/devices`: the devices of the configuration, at once, with no scan. */
export async function devices(): Promise<Found[]> {
  const list = listOf(await request("/api/devices"));
  if (!list.every((item) => isRecord(item))) throw new Failure("unreadable");
  return list.map((record) => found(record, true, null));
}

/** `GET /api/scan`: the devices that answer a `lan` scan. The server answers when the scan ends. */
export async function scan(): Promise<Found[]> {
  const list = listOf(await request("/api/scan"));
  if (!list.every((item) => isScanned(item))) throw new Failure("unreadable");
  return list.map((record) => found(record, record.configured, true));
}

/** A device that the configuration keeps. A blank `name` removes the name. */
export interface Keep {
  id: string;
  sku: string | null;
  name: string;
}

/**
 * `PUT /api/devices`: write the device entries in one go. A new entry gets
 * the SKU and the `lan` mode. `remove` deletes the entries of these IDs.
 */
export async function saveDevices(keep: Keep[], remove: string[]): Promise<void> {
  await send("/api/devices", "PUT", JSON.stringify({ keep, remove }));
}

export function devicePath(id: string, action: string): string {
  return `/api/devices/${encodeURIComponent(id)}/${action}`;
}

/** `POST /api/devices/{id}/identify`: the light turns on, green, at full brightness. */
export async function identify(id: string): Promise<void> {
  await send(devicePath(id, "identify"), "POST", "");
}

/** A room of `rooms.json`. A new room has an empty `id` until the server gives it one. */
export interface Room {
  id: string;
  name: string;
  /** Device IDs. A device can be in more than one room. */
  devices: string[];
}

function isRoom(value: unknown): value is Room {
  if (typeof value !== "object" || value === null) return false;
  return (
    "id" in value &&
    typeof value.id === "string" &&
    "name" in value &&
    typeof value.name === "string" &&
    "devices" in value &&
    Array.isArray(value.devices) &&
    value.devices.every((id: unknown) => typeof id === "string")
  );
}

function asRooms(body: unknown): Room[] {
  if (typeof body !== "object" || body === null || !("rooms" in body) || !Array.isArray(body.rooms)) {
    throw new Failure("unreadable");
  }
  const list: unknown[] = body.rooms;
  if (!list.every((item) => isRoom(item))) throw new Failure("unreadable");
  return list;
}

/** `GET /api/rooms`. */
export async function rooms(): Promise<Room[]> {
  return asRooms(await request("/api/rooms"));
}

/** `PUT /api/rooms`: replace every room. The answer gives each new room its `id`. */
export async function saveRooms(list: Room[]): Promise<Room[]> {
  return asRooms(await send("/api/rooms", "PUT", JSON.stringify({ rooms: list })));
}
