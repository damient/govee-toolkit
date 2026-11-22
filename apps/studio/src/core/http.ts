// One request to the API, and its failure as a kind that the UI can explain.

/** Why a request failed. `explain` in i18n.ts gives the text. */
export type Kind = "offline" | "outdated" | "unreadable" | "refused";

export class Failure extends Error {
  readonly kind: Kind;

  /** `detail` is the `error` of the answer, for `refused`. */
  constructor(kind: Kind, detail = "") {
    super(detail || kind);
    this.kind = kind;
  }
}

function errorOf(body: unknown): string {
  return typeof body === "object" && body !== null && "error" in body ? String(body.error) : "";
}

/** Send a request and answer the JSON body. Throws a `Failure` on any error. */
export async function request(path: string, init: RequestInit = {}): Promise<unknown> {
  let response: Response;
  try {
    response = await fetch(path, init);
  } catch {
    throw new Failure("offline");
  }
  // A server older than Studio has no such route, and answers 404 or 405 with no body.
  if (response.status === 404 || response.status === 405) throw new Failure("outdated");
  let body: unknown;
  try {
    body = await response.json();
  } catch {
    throw new Failure(response.ok ? "unreadable" : "refused", response.ok ? "" : `HTTP ${response.status}`);
  }
  if (!response.ok) throw new Failure("refused", errorOf(body) || `HTTP ${response.status}`);
  return body;
}

/** `request` with a JSON body. */
export function send(path: string, method: string, body: string): Promise<unknown> {
  return request(path, { method, headers: { "Content-Type": "application/json" }, body });
}
