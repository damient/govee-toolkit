/**
 * Control Govee devices from your own network.
 *
 * Every behavior is in the addon this module re-exports, which binds the
 * `govee-toolkit` core. What is added here is what an addon cannot declare:
 * the asynchronous iterators, and the disposer a segment stream closes on.
 */
import {
  Catalog,
  Config,
  CORE_VERSION,
  Device,
  DeviceHandle,
  DeviceStatus,
  EventStream,
  Govee,
  Health,
  modes,
  Reply,
  SegmentStream,
  Served,
  StatusStream,
  VERSION,
} from "../binding.cjs";

export {
  Catalog,
  Config,
  CORE_VERSION,
  Device,
  DeviceHandle,
  DeviceStatus,
  EventStream,
  Govee,
  Health,
  Reply,
  SegmentStream,
  Served,
  StatusStream,
  VERSION,
};

/** One RGB triple, each channel 0-255. */
export type Color = [number, number, number];

/** A value a device file's command takes as an argument. */
export type Arg =
  | boolean
  | number
  | string
  | Uint8Array
  | number[]
  | Color[];

/** A segment count, or the name of one the device file declares. */
export type Resolution = number | "app" | "native";

/** Frames per second, or the name of a rate the device file declares. */
export type Rate = number | "measured";

/**
 * One record the SDK reports. `event` says which one it is; the rest is the
 * record's own, as `govee watch --json` prints it.
 */
export type GoveeEvent = { event: string; [field: string]: unknown };

/** Every mode name the core knows, in the order the core lists them. */
export const MODES: readonly string[] = Object.freeze(modes());

/**
 * `Symbol.asyncDispose` reached Node after the floor this package supports,
 * so the well-known symbol is taken where the runtime declares it and made
 * where it does not. `await using` reads the same key either way.
 */
const asyncDispose: symbol = Symbol.asyncDispose ?? Symbol.for("Symbol.asyncDispose");

/** `null` from the source is where it ends. */
function iterate<T>(
  source: { next(): Promise<T | null | undefined> },
): AsyncIterableIterator<T> {
  return {
    [Symbol.asyncIterator]() {
      return this;
    },
    async next(): Promise<IteratorResult<T>> {
      const value = await source.next();
      return value === null || value === undefined
        ? { value: undefined, done: true }
        : { value, done: false };
    },
  };
}

Object.defineProperty(EventStream.prototype, Symbol.asyncIterator, {
  value(this: EventStream) {
    return iterate<GoveeEvent>(this);
  },
});

Object.defineProperty(StatusStream.prototype, Symbol.asyncIterator, {
  value(this: StatusStream) {
    return iterate<DeviceStatus>(this);
  },
});

Object.defineProperty(SegmentStream.prototype, asyncDispose, {
  value(this: SegmentStream) {
    return this.close();
  },
});

declare module "../binding.cjs" {
  interface EventStream {
    [Symbol.asyncIterator](): AsyncIterableIterator<GoveeEvent>;
  }
  interface StatusStream {
    [Symbol.asyncIterator](): AsyncIterableIterator<DeviceStatus>;
  }
  interface SegmentStream {
    [Symbol.asyncDispose](): Promise<void>;
  }
}
