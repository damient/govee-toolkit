interface Hello {
  name: string;
  version: string;
  skus: number;
}

interface Device {
  id: string;
  sku: string;
  name: string | null;
  modes: { mode: string; health: { state: string } | null }[];
}

function text(id: string, value: string): void {
  const element = document.getElementById(id);
  if (element) element.textContent = value;
}

function setStatus(state: "ok" | "error", label: string): void {
  const element = document.getElementById("status");
  if (!element) return;
  element.dataset["state"] = state;
  element.textContent = label;
}

function message(error: unknown): string {
  return error instanceof Error ? error.message : "unreachable";
}

async function hello(): Promise<void> {
  try {
    const response = await fetch("/api/hello");
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const answer = (await response.json()) as Hello;
    setStatus("ok", "connected");
    text("version", `${answer.name} ${answer.version}`);
    text("skus", answer.skus.toString());
  } catch (error) {
    setStatus("error", message(error));
  }
}

function span(className: string, value: string, tag = "span"): HTMLElement {
  const element = document.createElement(tag);
  element.className = className;
  element.textContent = value;
  return element;
}

function row(device: Device): HTMLLIElement {
  const item = document.createElement("li");
  const what = document.createElement("div");
  what.className = "what";
  what.append(span("title", device.name ?? device.sku), span("id", device.id, "code"));
  const state = device.modes.find((m) => m.mode === "lan")?.health?.state;
  const badge = span("status", "lan");
  badge.dataset["state"] = state === "ok" ? "ok" : "pending";
  item.append(what, badge);
  return item;
}

async function scan(): Promise<void> {
  const button = document.getElementById("scan") as HTMLButtonElement | null;
  const hint = document.getElementById("devices-hint");
  const list = document.getElementById("devices");
  if (!button || !hint || !list) return;
  button.disabled = true;
  hint.hidden = false;
  hint.dataset["state"] = "pending";
  hint.textContent = "Scanning the LAN…";
  try {
    const response = await fetch("/api/devices");
    const answer = (await response.json()) as { devices?: Device[]; error?: string };
    if (!response.ok) throw new Error(answer.error ?? `HTTP ${response.status}`);
    const devices = answer.devices ?? [];
    list.replaceChildren(...devices.map(row));
    list.hidden = devices.length === 0;
    hint.hidden = devices.length > 0;
    hint.textContent = "No device answered on the LAN. Check that LAN Control is on in the Govee Home app.";
  } catch (error) {
    list.hidden = true;
    hint.dataset["state"] = "error";
    hint.textContent = message(error);
  } finally {
    button.disabled = false;
  }
}

document.getElementById("scan")?.addEventListener("click", () => void scan());
void hello();
void scan();
