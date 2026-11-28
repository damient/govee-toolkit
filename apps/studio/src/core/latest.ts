/**
 * The shortest time between the starts of two calls, in milliseconds.
 * `/api/apply` answers in a few milliseconds over `lan`, so the call in
 * flight paces nothing. Some lights lose the last command of a burst of
 * brightness commands 35 ms apart in silence, and take every command at
 * 50 ms. This value doubles that. It paces the manual controls alone: a live
 * path must not go through `latest()`.
 */
const GAP_MS = 100;

/**
 * Wrap `run` so that one call is in flight at most, and two calls start
 * `GAP_MS` apart at least. A call made in that time waits, and a later call
 * replaces it: a drag sends its last value and does not queue requests on
 * the `lan` path. `failed` gets the error of a call, and drops the value that
 * waits.
 */
export function latest<T>(run: (value: T) => Promise<void>, failed: (error: unknown) => void): (value: T) => void {
  let busy = false;
  let waiting: { value: T } | null = null;
  const next = (): void => {
    busy = false;
    const value = waiting;
    waiting = null;
    if (value !== null) go(value.value);
  };
  const go = (value: T): void => {
    if (busy) {
      waiting = { value };
      return;
    }
    busy = true;
    const start = performance.now();
    run(value).then(
      () => {
        setTimeout(next, Math.max(0, GAP_MS - (performance.now() - start)));
      },
      (error: unknown) => {
        busy = false;
        waiting = null;
        failed(error);
      },
    );
  };
  return go;
}
