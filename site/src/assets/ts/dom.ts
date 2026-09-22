/** The element with this id, or null. An empty id matches nothing. */
export function byId(id: string): HTMLElement | null {
  return id === "" ? null : document.querySelector<HTMLElement>(`#${CSS.escape(id)}`);
}
