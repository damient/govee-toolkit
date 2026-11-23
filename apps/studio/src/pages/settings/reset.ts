import { type ResetChoice, reset } from "../../core/api.ts";
import { fail } from "./data.ts";
import { type Key, t } from "../../core/i18n.ts";
import { ask } from "../../components/modal/modal.ts";

const PARTS: readonly { key: Key; part: keyof ResetChoice }[] = [
  { key: "reset.prefs", part: "prefs" },
  { key: "reset.devices", part: "devices" },
];

function checkbox(group: HTMLElement, key: Key): HTMLInputElement {
  const label = document.createElement("label");
  const input = document.createElement("input");
  input.type = "checkbox";
  label.append(input, t(key));
  group.append(label);
  return input;
}

/** The checkbox list of the modal, and a getter for the parts that it holds. All checks or clears every part. */
function choices(): [HTMLElement, () => ResetChoice] {
  const group = document.createElement("fieldset");
  group.className = "choices";
  const inputs = PARTS.map(({ key }) => checkbox(group, key));
  const all = checkbox(group, "reset.all");
  all.addEventListener("change", () => {
    for (const input of inputs) input.checked = all.checked;
  });
  for (const input of inputs) {
    input.addEventListener("change", () => {
      const checked = inputs.filter((item) => item.checked).length;
      all.checked = checked === inputs.length;
      all.indeterminate = checked > 0 && checked < inputs.length;
    });
  }
  const chosen = (): ResetChoice => ({
    prefs: inputs[0]?.checked === true,
    devices: inputs[1]?.checked === true,
  });
  return [group, chosen];
}

function some(choice: ResetChoice): boolean {
  return choice.prefs || choice.devices;
}

async function choose(): Promise<ResetChoice | undefined> {
  const [content, chosen] = choices();
  const yes = await ask({
    title: t("reset.title"),
    content,
    ready: () => some(chosen()),
    actions: [
      { label: t("modal.cancel"), value: false },
      { label: t("reset.confirm"), value: true, primary: true },
    ],
  });
  return yes && some(chosen()) ? chosen() : undefined;
}

function confirm(choice: ResetChoice): Promise<boolean> {
  const list = document.createElement("ul");
  list.className = "summary";
  for (const { key, part } of PARTS) {
    if (!choice[part]) continue;
    const item = document.createElement("li");
    item.textContent = t(key);
    list.append(item);
  }
  return ask({
    title: t("reset.sure.title"),
    body: t("reset.sure.body"),
    content: list,
    actions: [
      { label: t("modal.cancel"), value: false },
      { label: t("reset.confirm"), value: true, primary: true },
    ],
  });
}

/** The button of the Reset row. Only the computer of the server can reset. `note` shows an error. */
export function resetButton(note: HTMLElement, local: Promise<boolean>): HTMLButtonElement {
  const node = document.createElement("button");
  node.type = "button";
  node.className = "button xs";
  node.textContent = t("settings.reset.choose");
  node.addEventListener("click", () => {
    void (async () => {
      const choice = await choose();
      if (!choice || !(await confirm(choice))) return;
      node.disabled = true;
      await reset(choice);
      location.reload();
    })().catch((failure: unknown) => {
      node.disabled = false;
      fail(note, failure);
    });
  });
  local.then(
    (here) => {
      node.disabled = !here;
    },
    () => {
      // The button stays: the server answers the reset on its own.
    },
  );
  return node;
}
