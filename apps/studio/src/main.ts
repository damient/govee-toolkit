interface Hello {
  name: string;
  version: string;
  skus: number;
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

async function hello(): Promise<void> {
  try {
    const response = await fetch("/api/hello");
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const answer = (await response.json()) as Hello;
    setStatus("ok", "connected");
    text("version", `${answer.name} ${answer.version}`);
    text("skus", answer.skus.toString());
  } catch (error) {
    setStatus("error", error instanceof Error ? error.message : "unreachable");
  }
}

void hello();
