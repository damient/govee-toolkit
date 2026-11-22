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
  const response = await fetch("/api/hello");
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  const body: unknown = await response.json();
  if (!isHello(body)) throw new Error("unexpected answer");
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
    typeof value.control === "boolean"
  );
}

// An error answer carries `{ "error" }`.
async function read(response: Response): Promise<Share> {
  const body: unknown = await response.json();
  if (!response.ok) {
    const error = typeof body === "object" && body !== null && "error" in body ? String(body.error) : "";
    throw new Error(error || `HTTP ${response.status}`);
  }
  if (!isShare(body)) throw new Error("unexpected answer");
  return body;
}

/** `GET /api/share`. */
export async function share(): Promise<Share> {
  return read(await fetch("/api/share"));
}

/** `POST /api/share`: open the server to the network, or close it. */
export async function setShare(shared: boolean): Promise<Share> {
  const body = JSON.stringify({ shared });
  return read(await fetch("/api/share", { method: "POST", headers: { "Content-Type": "application/json" }, body }));
}
