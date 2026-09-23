// The player loads on the first click only. Before that the page shows the
// thumbnail, and no request goes to the video host.

const LIGHTBOX = "(min-width: 768px)";
const CLOSE_ICON = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M6 6l12 12M18 6 6 18"/></svg>';

function player(id: string): HTMLIFrameElement {
  const frame = document.createElement("iframe");
  frame.src = `https://www.youtube-nocookie.com/embed/${encodeURIComponent(id)}?autoplay=1&playsinline=1&rel=0`;
  frame.title = "Govee Toolkit demo";
  frame.allow = "autoplay; encrypted-media; picture-in-picture; fullscreen";
  frame.referrerPolicy = "strict-origin-when-cross-origin";
  frame.allowFullscreen = true;
  return frame;
}

// The player leaves the page on close, so the video stops.
function lightbox(id: string): void {
  const dialog = document.createElement("dialog");
  dialog.className = "lightbox";
  dialog.setAttribute("aria-label", "Demo video");
  const close = document.createElement("button");
  close.type = "button";
  close.className = "lightbox-close";
  close.setAttribute("aria-label", "Close the video");
  close.innerHTML = CLOSE_ICON;
  const box = document.createElement("div");
  box.className = "lightbox-frame";
  box.append(player(id));
  dialog.append(close, box);
  const shut = () => {
    dialog.close();
    dialog.remove();
  };
  close.addEventListener("click", shut);
  // A click on the backdrop lands on the dialog itself.
  dialog.addEventListener("click", (event) => {
    if (event.target === dialog) shut();
  });
  // Escape closes the dialog without a click.
  dialog.addEventListener("close", () => {
    dialog.remove();
  });
  document.body.append(dialog);
  dialog.showModal();
}

function inline(link: HTMLAnchorElement, id: string): void {
  const frame = player(id);
  const box = document.createElement("div");
  box.className = "hero-video";
  box.append(frame);
  link.replaceWith(box);
  frame.focus();
}

export function video(link: HTMLAnchorElement): void {
  const id = link.dataset.video;
  if (id === undefined || id === "") return;
  link.addEventListener("click", (event) => {
    event.preventDefault();
    if (matchMedia(LIGHTBOX).matches) lightbox(id);
    else inline(link, id);
  });
}
