/**
 * Wrap `run` so that one call is in flight at most. A call made while one
 * runs waits, and a later call replaces it: a drag sends its last value and
 * does not queue requests on the `lan` path. `failed` gets the error of a
 * call, and drops the value that waits.
 */
export function latest<T>(run: (value: T) => Promise<void>, failed: (error: unknown) => void): (value: T) => void {
  let busy = false;
  let waiting: { value: T } | null = null;
  const go = (value: T): void => {
    if (busy) {
      waiting = { value };
      return;
    }
    busy = true;
    run(value).then(
      () => {
        busy = false;
        const next = waiting;
        waiting = null;
        if (next !== null) go(next.value);
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
