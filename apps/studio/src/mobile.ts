import { type Share, setShare, share } from "./api.ts";
import { explain, t } from "./i18n.ts";
import { icon } from "./nav.ts";
import { qr } from "./qr.ts";

const PHONE = "M6 5a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2zM11 4h2M12 17v.01";
const COPY = "M8 10a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2h-8a2 2 0 0 1-2-2zM16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2";
const CHECK = "M5 12l5 5L20 7";

function element<K extends keyof HTMLElementTagNameMap>(tag: K, className: string, text = ""): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  node.className = className;
  node.textContent = text;
  return node;
}

/** The QR code, or an empty tile while sharing is off. */
function tile(state: Share): HTMLElement {
  const box = element("div", "qr-tile");
  if (state.url === null) {
    box.classList.add("off");
    box.append(icon(PHONE));
    return box;
  }
  const code = qr(state.url);
  code.setAttribute("aria-label", state.url);
  box.append(code);
  return box;
}

function address(url: string): HTMLElement {
  const row = element("div", "share-url");
  const link = element("a", "", url.replace(/\/$/u, ""));
  link.href = url;
  link.target = "_blank";
  link.rel = "noopener";
  const copy = element("button", "icon-button");
  copy.type = "button";
  copy.title = t("mobile.copy");
  copy.setAttribute("aria-label", t("mobile.copy"));
  copy.append(icon(COPY));
  copy.addEventListener("click", () => {
    void navigator.clipboard.writeText(url).then(() => {
      copy.replaceChildren(icon(CHECK));
      copy.title = t("mobile.copied");
      setTimeout(() => {
        copy.replaceChildren(icon(COPY));
        copy.title = t("mobile.copy");
      }, 1500);
    });
  });
  row.append(link, copy);
  return row;
}

/** The note under the switch: who can switch, what sharing opens, or the last error. */
function note(state: Share, error: string): HTMLElement {
  if (error) return element("p", "share-note error", error);
  if (state.fixed) return element("p", "share-note", t("mobile.fixed"));
  if (!state.control) return element("p", "share-note", t("mobile.elsewhere"));
  return element("p", "share-note", t("mobile.warning"));
}

function draw(box: HTMLElement, state: Share, error = ""): void {
  const body = element("div", "share-body");
  body.append(
    element("h2", "share-title", t("mobile.title")),
    element("p", "share-lead", t(state.url === null ? "mobile.off" : "mobile.scan")),
  );
  if (state.url !== null) body.append(address(state.url));

  if (!state.fixed) {
    const row = element("div", "share-switch");
    const toggle = element("button", "switch");
    toggle.type = "button";
    toggle.setAttribute("role", "switch");
    toggle.setAttribute("aria-checked", String(state.shared));
    toggle.setAttribute("aria-label", t("mobile.share"));
    toggle.disabled = !state.control;
    toggle.addEventListener("click", () => {
      toggle.disabled = true;
      setShare(!state.shared).then(
        (next) => {
          draw(box, next);
        },
        (failure: unknown) => {
          draw(box, state, explain(failure));
        },
      );
    });
    const name = element("span", "", t("mobile.share"));
    name.setAttribute("aria-hidden", "true");
    row.append(name, toggle);
    body.append(row);
  }
  body.append(note(state, error));

  const card = element("div", "share-card");
  card.append(tile(state), body);
  box.replaceChildren(card);
}

/** Draw the Mobile view: the address of Studio on the network, as a QR code. */
export function mobile(view: HTMLElement): void {
  view.classList.add("centered");
  // The view also holds the note for a small screen: draw beside it.
  const box = element("div", "");
  view.append(box);
  share().then(
    (state) => {
      draw(box, state);
    },
    (failure: unknown) => {
      box.replaceChildren(element("p", "share-note error", explain(failure)));
    },
  );
}
