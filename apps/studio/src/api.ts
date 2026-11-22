import { Failure, request, send } from "./http.ts";

export interface Hello {
  name: string;
  /** The version of govee-server. */
  version: string;
  /** The version of the govee-toolkit crate in the server. Absent from an older server. */
  sdk?: string;
}

let answered: Hello | undefined;

/** The answer of `hello()`, once it has answered. */
export function answeredHello(): Hello | undefined {
  return answered;
}

function isHello(value: unknown): value is Hello {
  if (typeof value !== "object" || value === null) return false;
  return "name" in value && typeof value.name === "string" && "version" in value && typeof value.version === "string";
}

/** `GET /api/hello`. Throws when the server does not answer or answers something else. */
export async function hello(): Promise<Hello> {
  const body = await request("/api/hello");
  if (!isHello(body)) throw new Failure("unreadable");
  answered = { name: body.name, version: body.version };
  if ("sdk" in body && typeof body.sdk === "string") answered.sdk = body.sdk;
  return body;
}

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

/** A device that answered a `lan` scan. */
export interface Found {
  /** The device ID, as `AA:BB:CC:DD:EE:FF:00:11`. */
  id: string;
  sku: string;
  /** The name that the configuration gives it. */
  name: string | null;
  /** The family of its device file, for the icon. */
  family: string | null;
}

function isFound(value: unknown): value is Found {
  if (typeof value !== "object" || value === null) return false;
  return (
    "id" in value &&
    typeof value.id === "string" &&
    "sku" in value &&
    typeof value.sku === "string" &&
    "name" in value &&
    (value.name === null || typeof value.name === "string") &&
    "family" in value &&
    (value.family === null || typeof value.family === "string")
  );
}

/** `GET /api/devices`: a `lan` scan. The server answers when the scan ends. */
export async function devices(): Promise<Found[]> {
  const body = await request("/api/devices");
  if (typeof body !== "object" || body === null || !("devices" in body) || !Array.isArray(body.devices)) {
    throw new Failure("unreadable");
  }
  const list: unknown[] = body.devices;
  if (!list.every((item) => isFound(item))) throw new Failure("unreadable");
  return list;
}

function devicePath(id: string, action: string): string {
  return `/api/devices/${encodeURIComponent(id)}/${action}`;
}

/** `POST /api/devices/{id}/identify`: the light turns on, green, at full brightness. */
export async function identify(id: string): Promise<void> {
  await send(devicePath(id, "identify"), "POST", "");
}

/** `PUT /api/devices/{id}/name`. An empty `name` removes the name. Answers the name kept. */
export async function rename(id: string, name: string): Promise<string | null> {
  const body = await send(devicePath(id, "name"), "PUT", JSON.stringify({ name }));
  if (typeof body !== "object" || body === null || !("name" in body)) throw new Failure("unreadable");
  const kept = body.name;
  if (kept !== null && typeof kept !== "string") throw new Failure("unreadable");
  return kept;
}
