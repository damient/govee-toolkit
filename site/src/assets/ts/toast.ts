let queued: ReturnType<typeof setTimeout> | undefined;

/** Shows `text` at the foot of the viewport for a moment. A new call replaces the text. */
export function toast(text: string): void {
  let box = document.querySelector<HTMLElement>(".toast");
  if (!box) {
    box = document.createElement("p");
    box.className = "toast";
    box.setAttribute("role", "status");
    document.body.append(box);
  }
  const shown = box;
  shown.textContent = text;
  clearTimeout(queued);
  // The frame between the insert and the class lets the entry transition run.
  requestAnimationFrame(() => {
    shown.classList.add("is-shown");
  });
  queued = setTimeout(() => {
    shown.classList.remove("is-shown");
  }, 1800);
}
