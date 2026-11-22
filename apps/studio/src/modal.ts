// A modal dialog over the page. `<dialog>` keeps the focus inside, and Esc
// closes it as the first action (the one that changes nothing).

export interface Action<T> {
  label: string;
  value: T;
  /** The action that the user came for: the colored button. */
  primary?: boolean;
}

export interface Modal<T> {
  title: string;
  body: string;
  /** The first action is the answer to Esc. */
  actions: readonly [Action<T>, ...Action<T>[]];
}

/** Show `modal` and answer the value of the action that the user picks. */
export function ask<T>(modal: Modal<T>): Promise<T> {
  const dialog = document.createElement("dialog");
  dialog.className = "modal";
  const title = document.createElement("h2");
  title.id = "modal-title";
  title.textContent = modal.title;
  const body = document.createElement("p");
  body.textContent = modal.body;
  const buttons = document.createElement("div");
  buttons.className = "buttons";
  dialog.setAttribute("aria-labelledby", title.id);
  dialog.append(title, body, buttons);

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
    dialog.addEventListener("cancel", (event) => {
      event.preventDefault();
      close(modal.actions[0].value);
    });
    document.body.append(dialog);
    dialog.showModal();
  });
}
