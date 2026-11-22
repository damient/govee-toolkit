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

/** `POST /api/backup/restore`: put back the files from before the last import. */
export async function undoImport(): Promise<void> {
  await send("/api/backup/restore", "POST", "");
}

export interface ResetChoice {
  /** Theme, language and the advanced switch. */
  prefs: boolean;
  /** The `devices:` entries of the configuration file: names, modes, groups. */
  devices: boolean;
}

/** `POST /api/reset`. Only the computer of the server can reset. */
export async function reset(choice: ResetChoice): Promise<void> {
  await send("/api/reset", "POST", JSON.stringify(choice));
}

/** A device that answered a `lan` scan, or a device of the configuration. */
export interface Found {
  /** The device ID, as `AA:BB:CC:DD:EE:FF:00:11`. */
  id: string;
  /** `null` for a device of the configuration that gives no SKU and did not answer. */
  sku: string | null;
  /** The name that the configuration gives it. */
  name: string | null;
  /** The family of its device file, for the icon. */
  family: string | null;
  /** The configuration has an entry for it. */
  configured: boolean;
  /** It answered the scan. */
  online: boolean;
}

const nullableString = (value: unknown): boolean => value === null || typeof value === "string";

function isFound(value: unknown): value is Found {
  if (typeof value !== "object" || value === null) return false;
  return (
    "id" in value &&
    typeof value.id === "string" &&
    "sku" in value &&
    nullableString(value.sku) &&
    "name" in value &&
    nullableString(value.name) &&
    "family" in value &&
    nullableString(value.family) &&
    "configured" in value &&
    typeof value.configured === "boolean" &&
    "online" in value &&
    typeof value.online === "boolean"
  );
}

/** `GET /api/devices`: a `lan` scan, and the devices of the configuration. The server answers when the scan ends. */
export async function devices(): Promise<Found[]> {
  const body = await request("/api/devices");
  if (typeof body !== "object" || body === null || !("devices" in body) || !Array.isArray(body.devices)) {
    throw new Failure("unreadable");
  }
  const list: unknown[] = body.devices;
  if (!list.every((item) => isFound(item))) throw new Failure("unreadable");
  return list;
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

function devicePath(id: string, action: string): string {
  return `/api/devices/${encodeURIComponent(id)}/${action}`;
}

/** `POST /api/devices/{id}/identify`: the light turns on, green, at full brightness. */
export async function identify(id: string): Promise<void> {
  await send(devicePath(id, "identify"), "POST", "");
}
