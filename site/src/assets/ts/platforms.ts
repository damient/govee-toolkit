import { paint, wire } from "./tablist.ts";

// One system at a time in `#get`. The address `#get-<os>` opens that system,
// else the first card is open on load. With JavaScript blocked, every pane shows.
export function platforms(element: HTMLElement): void {
  const buttons = [...element.querySelectorAll<HTMLButtonElement>("button[data-os]")];
  const panes = [...element.querySelectorAll<HTMLElement>(".get-pane")];
  const show = (os: string | undefined): void => {
    paint(buttons, panes, "os", os);
  };
  const asked = (): string | undefined => {
    const os = /^#get-(\w+)$/u.exec(location.hash)?.[1];
    return buttons.some((button) => button.dataset.os === os) ? os : undefined;
  };
  // The pane of the address is hidden until `show` runs, so the browser cannot scroll to it.
  // On load, a smooth scroll that starts before the `load` event stops halfway.
  const follow = (behavior: ScrollBehavior): void => {
    const os = asked();
    if (os === undefined) return;
    show(os);
    (element.closest("section") ?? element).scrollIntoView({ block: "start", behavior });
  };
  wire(element, "os", buttons, show);
  show(asked() ?? buttons[0]?.dataset.os);
  if (document.readyState === "complete") follow("instant");
  else addEventListener("load", () => { follow("instant"); }, { once: true });
  addEventListener("hashchange", () => { follow("smooth"); });
}
