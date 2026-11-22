import { restore, share, undoImport } from "./api.ts";
import { explain, t } from "./i18n.ts";
import { ask } from "./modal.ts";

/** Set before the reload that follows an import, and cleared on the first read. */
const IMPORTED = "studio.imported";

/** Whether the page loads right after an import. The answer is `true` once. */
function justImported(): boolean {
  try {
    const set = sessionStorage.getItem(IMPORTED) !== null;
    sessionStorage.removeItem(IMPORTED);
    return set;
  } catch {
    return false;
  }
}

/** Show `failure` in `note`. */
export function fail(note: HTMLElement, failure: unknown): void {
  note.classList.add("error");
  note.textContent = explain(failure);
}

function confirmImport(name: string): Promise<boolean> {
  return ask({
    title: t("data.confirm.title"),
    body: t("data.confirm.body", { file: name }),
    actions: [
      { label: t("modal.cancel"), value: false },
      { label: t("data.confirm.import"), value: true, primary: true },
    ],
  });
}

async function importFile(file: File): Promise<boolean> {
  if (!(await confirmImport(file.name))) return false;
  await restore(await file.text());
  try {
    sessionStorage.setItem(IMPORTED, "1");
  } catch {
    // Blocked storage: the page does not offer Restore after the reload.
  }
  location.reload();
  return true;
}

function button(text: string): HTMLButtonElement {
  const node = document.createElement("button");
  node.type = "button";
  node.className = "button";
  node.textContent = text;
  return node;
}

/**
 * The Restore button: shown on the first view of Settings after an import.
 * The server keeps its copies, but the button goes when the user leaves.
 */
function undo(note: HTMLElement): HTMLButtonElement {
  const node = button(t("data.restore"));
  node.title = t("data.restore.hint");
  node.hidden = !justImported();
  node.addEventListener("click", () => {
    void (async () => {
      const yes = await ask({
        title: t("data.restore.title"),
        body: t("data.restore.body"),
        actions: [
          { label: t("modal.cancel"), value: false },
          { label: t("data.restore"), value: true, primary: true },
        ],
      });
      if (!yes) return;
      node.disabled = true;
      await undoImport();
      location.reload();
    })().catch((failure: unknown) => {
      node.disabled = false;
      fail(note, failure);
    });
  });
  return node;
}

function two(n: number): string {
  return String(n).padStart(2, "0");
}

/** `govee-toolkit-studio-2026-11-22-1430.json`, in the local time of `at`. */
export function fileName(at: Date): string {
  const day = `${at.getFullYear()}-${two(at.getMonth() + 1)}-${two(at.getDate())}`;
  return `govee-toolkit-studio-${day}-${two(at.getHours())}${two(at.getMinutes())}.json`;
}

/** The Import button, and the file input that it opens. */
function picker(note: HTMLElement): [HTMLButtonElement, HTMLInputElement] {
  const load = button(t("settings.data.import"));
  const file = document.createElement("input");
  file.type = "file";
  file.accept = "application/json,.json";
  file.hidden = true;
  load.addEventListener("click", () => {
    file.click();
  });
  file.addEventListener("change", () => {
    const chosen = file.files?.[0];
    if (!chosen) return;
    load.disabled = true;
    importFile(chosen).then(
      (reloads) => {
        if (reloads) return;
        load.disabled = false;
        file.value = "";
      },
      (failure: unknown) => {
        load.disabled = false;
        file.value = "";
        fail(note, failure);
      },
    );
  });
  return [load, file];
}

/**
 * The Export, Import and Restore buttons. `note` shows an error. Only the computer of
 * the server can export or import: the backup can hold the API key.
 */
export function backup(note: HTMLElement): HTMLElement {
  const group = document.createElement("div");
  group.className = "buttons";
  const save = document.createElement("a");
  save.className = "button";
  save.href = "/api/backup";
  save.download = fileName(new Date());
  save.textContent = t("settings.data.export");
  save.addEventListener("click", () => {
    save.download = fileName(new Date());
  });
  const [load, file] = picker(note);
  const back = undo(note);
  share().then(
    (state) => {
      if (state.local) return;
      save.removeAttribute("href");
      save.setAttribute("aria-disabled", "true");
      load.disabled = true;
      back.hidden = true;
      note.textContent = t("settings.data.elsewhere");
    },
    () => {
      // The buttons stay: the server answers each action on its own.
    },
  );
  group.append(save, load, back, file);
  return group;
}
