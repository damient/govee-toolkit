import "./sheet.css";
import { t } from "../../core/i18n.ts";
import { BACK, icon } from "../icon/icon.ts";


export interface SheetOptions {
  /** The content of the head. */
  head: Node[];
  /** The controls on the right of the head. */
  end?: Node[];
  /** The name of the dialog for a screen reader. */
  label: string;
  /** Runs once, after the panel closes. */
  closed?: () => void;
}

export interface Sheet {
  /** The content under the head. It scrolls. */
  body: HTMLElement;
  /** The row at the bottom. It does not scroll. */
  foot: HTMLElement;
  close(): void;
}

/** Open a panel. A click on the backdrop, Esc and the back button of a phone close it. */
export function sheet(options: SheetOptions): Sheet {
  const dialog = document.createElement("dialog");
  dialog.className = "sheet";
  dialog.setAttribute("aria-label", options.label);
  const head = document.createElement("header");
  head.className = "sheet-head";
  const shut = document.createElement("button");
  shut.type = "button";
  shut.className = "icon-button sheet-back";
  shut.setAttribute("aria-label", t("sheet.close"));
  shut.append(icon(BACK));
  const end = document.createElement("div");
  end.className = "sheet-end";
  end.append(...(options.end ?? []));
  head.append(shut, ...options.head, end);
  const body = document.createElement("div");
  body.className = "sheet-body";
  const foot = document.createElement("footer");
  foot.className = "sheet-foot";
  dialog.append(head, body, foot);

  let open = true;
  const close = (): void => {
    if (!open) return;
    open = false;
    dialog.close();
    dialog.remove();
    options.closed?.();
  };
  shut.addEventListener("click", close);
  dialog.addEventListener("cancel", (event) => {
    event.preventDefault();
    close();
  });
  // The backdrop is the dialog itself, outside its box.
  dialog.addEventListener("pointerdown", (event) => {
    if (event.target !== dialog) return;
    const box = dialog.getBoundingClientRect();
    const inside =
      event.clientX >= box.left && event.clientX <= box.right && event.clientY >= box.top && event.clientY <= box.bottom;
    if (!inside) close();
  });
  document.body.append(dialog);
  dialog.showModal();
  // The dialog takes the focus itself: the first control would show its ring and its tooltip.
  dialog.tabIndex = -1;
  dialog.focus();
  return { body, foot, close };
}
