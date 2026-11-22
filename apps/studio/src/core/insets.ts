/**
 * Set `data-camera` on the root to the side of the camera in landscape: `left`
 * or `right`. The type of `landscape-primary` puts the top of the phone, and with
 * it the camera, on the left.
 */
export function watchCamera(): void {
  // Safari before 16.4 has no `screen.orientation`: both sides keep the inset.
  if (!("orientation" in screen)) return;
  const root = document.documentElement;
  const update = (): void => {
    const { type } = screen.orientation;
    if (type === "landscape-primary") root.dataset["camera"] = "left";
    else if (type === "landscape-secondary") root.dataset["camera"] = "right";
    else delete root.dataset["camera"];
  };
  update();
  screen.orientation.addEventListener("change", update);
}
