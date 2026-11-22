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
