import { ask } from "../../../components/modal/modal.ts";
import { element } from "../../../core/dom.ts";
import { t } from "../../../core/i18n.ts";
import { presets } from "../../../core/presets.ts";

/** `Preset #n`, with `n` above the count of presets and a name that no preset has. */
function defaultName(): string {
  const taken = new Set(presets().map((preset) => preset.name));
  let n = presets().length + 1;
  while (taken.has(t("panel.preset.default", { n: String(n) }))) n += 1;
  return t("panel.preset.default", { n: String(n) });
}

function nameField(value: string): HTMLInputElement {
  const input = element("input", "panel-preset-name");
  input.value = value;
  input.maxLength = 64;
  // `showModal` focuses the first button without it, and Enter then answers that button.
  input.autofocus = true;
  input.setAttribute("aria-label", t("panel.preset.name"));
  input.addEventListener("input", () => {
    input.dispatchEvent(new Event("change", { bubbles: true }));
  });
  input.addEventListener("keydown", (event) => {
    if (event.key !== "Enter") return;
    // The focus goes back to the button that opened the modal: the same key must not press it again.
    event.preventDefault();
    input.closest("dialog")?.querySelector<HTMLButtonElement>(".button.primary:not(:disabled)")?.click();
  });
  requestAnimationFrame(() => {
    input.select();
  });
  return input;
}

/** The name of a preset, or `null` on a cancel. `start` fills the field, else `Preset #n`. */
export async function askName(start: string | null): Promise<string | null> {
  const input = nameField(start ?? defaultName());
  const saved = await ask({
    title: t("panel.preset.title"),
    content: input,
    ready: () => input.value.trim() !== "",
    actions: [
      { label: t("modal.cancel"), value: false },
      { label: t("panel.preset.save"), value: true, primary: true },
    ],
  });
  return saved ? input.value.trim() : null;
}

/** `true` when the user confirms the deletion of the preset `name`. */
export function confirmDelete(name: string): Promise<boolean> {
  return ask({
    title: t("panel.preset.delete.title", { name }),
    body: t("panel.preset.delete.body"),
    actions: [
      { label: t("modal.cancel"), value: false },
      { label: t("panel.preset.delete"), value: true, primary: true },
    ],
  });
}
