const COPY_ICON = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15V5a2 2 0 0 1 2-2h8"/></svg>';
const DONE_ICON = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 12.5 9.5 18 20 6.5"/></svg>';

// The button carries the icon and the label from here: a page without the
// script then shows nothing to press.
export function copy(button) {
  const label = button.textContent.trim() || "Copy";
  const rest = () => {
    button.innerHTML = COPY_ICON;
    button.removeAttribute("data-state");
    say(label);
  };
  const say = (text) => {
    button.setAttribute("aria-label", text);
    button.title = text;
  };

  let queued = null;
  rest();
  button.addEventListener("click", async () => {
    clearTimeout(queued);
    try {
      await navigator.clipboard.writeText(button.dataset.copy);
      button.innerHTML = DONE_ICON;
      button.dataset.state = "done";
      say("Copied");
    } catch {
      button.dataset.state = "failed";
      say("Copy failed");
    }
    queued = setTimeout(rest, 1600);
  });
}
