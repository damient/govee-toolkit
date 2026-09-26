let queued: ReturnType<typeof setTimeout> | undefined;

function created(): HTMLElement {
  const box = document.createElement("p");
  box.className = "toast";
  box.setAttribute("role", "status");
  document.body.append(box);
  return box;
}

/** Shows `text` at the foot of the viewport for a moment. A new call replaces the text. */
export function toast(text: string): void {
  const box = document.querySelector<HTMLElement>(".toast") ?? created();
  box.textContent = text;
  clearTimeout(queued);
  // The frame between the insert and the class lets the entry transition run.
  requestAnimationFrame(() => {
    box.classList.add("is-shown");
  });
  queued = setTimeout(() => {
    box.classList.remove("is-shown");
  }, 1800);
}
