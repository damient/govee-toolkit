import { Failure, request, send } from "./http.ts";

export interface Hello {
  name: string;
  version: string;
}

function isHello(value: unknown): value is Hello {
  if (typeof value !== "object" || value === null) return false;
  return "name" in value && typeof value.name === "string" && "version" in value && typeof value.version === "string";
}

/** `GET /api/hello`. Throws when the server does not answer or answers something else. */
export async function hello(): Promise<Hello> {
  const body = await request("/api/hello");
  if (!isHello(body)) throw new Failure("unreadable");
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
