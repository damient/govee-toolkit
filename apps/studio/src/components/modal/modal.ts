import "./modal.css";

export interface Action<T> {
  label: string;
  value: T;
  /** The action that the user came for: the colored button. */
  primary?: boolean;
}

export interface Modal<T> {
  title: string;
  body?: string;
  /** A form or a list under the body. The caller reads its state after the answer. */
  content?: HTMLElement;
  /** Checked on each change in `content`: the primary actions stay disabled while it answers `false`. */
  ready?: () => boolean;
  /** The first action is the answer to Esc. */
  actions: readonly [Action<T>, ...Action<T>[]];
}

export function ask<T>(modal: Modal<T>): Promise<T> {
  const dialog = document.createElement("dialog");
  dialog.className = "modal";
  const title = document.createElement("h2");
  title.id = "modal-title";
  title.textContent = modal.title;
  const buttons = document.createElement("div");
  buttons.className = "buttons";
  dialog.setAttribute("aria-labelledby", title.id);
  dialog.append(title);
  if (modal.body !== undefined) {
    const body = document.createElement("p");
    body.textContent = modal.body;
    dialog.append(body);
  }
  if (modal.content) dialog.append(modal.content);
  dialog.append(buttons);

  return new Promise((resolve) => {
    const close = (value: T): void => {
      dialog.close();
      dialog.remove();
      resolve(value);
    };
    for (const action of modal.actions) {
      const button = document.createElement("button");
      button.type = "button";
      button.className = action.primary === true ? "button primary" : "button";
      button.textContent = action.label;
      button.addEventListener("click", () => {
        close(action.value);
      });
      buttons.append(button);
    }
    const primaries = [...buttons.querySelectorAll<HTMLButtonElement>(".primary")];
    const check = (): void => {
      const ready = modal.ready?.() ?? true;
      for (const button of primaries) button.disabled = !ready;
    };
    modal.content?.addEventListener("change", check);
    check();
    dialog.addEventListener("cancel", (event) => {
      event.preventDefault();
      close(modal.actions[0].value);
    });
    document.body.append(dialog);
    dialog.showModal();
  });
}
