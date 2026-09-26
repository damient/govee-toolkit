// The shape of a tool answer, and the enum a schema builds from catalog values.

import type { CallToolResult } from "@modelcontextprotocol/server";
import { z } from "zod";

/** A structured answer, with the same JSON as text for a client that reads no `structuredContent`. */
export function ok(value: Record<string, unknown>): CallToolResult {
  return {
    content: [{ type: "text", text: JSON.stringify(value) }],
    structuredContent: value,
  };
}

/** A thrown error as an answer carries it: the binding code, or `error` where it has none. */
export interface Failure {
  code: string;
  message: string;
}

export function failure(error: unknown): Failure {
  const code = typeof error === "object" && error !== null && "code" in error ? String(error.code) : "error";
  return { code, message: error instanceof Error ? error.message : String(error) };
}

/** A failure, with the binding error code first: `unknown_sku: …`. */
export function fail(error: unknown): CallToolResult {
  const { code, message } = failure(error);
  return { isError: true, content: [{ type: "text", text: `${code}: ${message}` }] };
}

/** An error that carries a `code`, as a binding error does. */
export function codeError(code: string, message: string): Error {
  return Object.assign(new Error(message), { code });
}

/** An enum over values read at startup. A list the catalog leaves empty accepts any string. */
export function oneOf(values: readonly string[]): z.ZodType<string> {
  const [first, ...rest] = values;
  return first === undefined ? z.string() : z.enum([first, ...rest]);
}

/** Said in every read tool: an unverified value is not a supported one. */
export const UNVERIFIED =
  "A value of `?` or `unknown` means that nobody verified it on hardware. It does not mean that the device supports it.";
