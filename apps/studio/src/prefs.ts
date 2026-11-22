// The choices of this browser. Each origin keeps its own: the desktop app and
// a browser tab do not share them.

const PREFIX = "studio.";

/** The stored value of `name`, or `fallback` when it is missing, unknown or unreadable. */
export function read<T extends string>(name: string, allowed: readonly T[], fallback: T): T {
  try {
    const value = localStorage.getItem(PREFIX + name);
    return allowed.find((option) => option === value) ?? fallback;
  } catch {
    return fallback;
  }
}

/** Store `value`. A browser that refuses storage keeps the choice until the next reload. */
export function write(name: string, value: string): void {
  try {
    localStorage.setItem(PREFIX + name, value);
  } catch {
    // Private mode or blocked storage.
  }
}
