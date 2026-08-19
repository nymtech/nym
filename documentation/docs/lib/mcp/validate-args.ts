// Validate incoming MCP tool arguments against each tool's declared JSON Schema
// before dispatch. The tools declare `inputSchema` as plain JSON Schema (kept
// SDK-free, see tools.ts); ajv compiles and caches one validator per schema.
// This turns a malformed tools/call into a clear "invalid arguments" result at
// the choke point, rather than a vague failure inside the handler when a missing
// or wrong-typed field reaches the embedder or network client.

import Ajv from 'ajv';
import type { ValidateFunction } from 'ajv';

// One Ajv instance for the process.
//   allErrors: report every problem, not just the first.
//   strict: false: the tool schemas are hand-written and carry annotation keys
//     (description); strict mode would reject otherwise-valid schemas.
//   coerceTypes is left OFF: MCP arguments arrive as typed JSON, so a string
//     where a number is required is a real client error, not silent coercion.
const ajv = new Ajv({ allErrors: true, strict: false });

// Compiled validators are cached by schema identity. A WeakMap is deliberate: if
// the tool registry is a singleton (schema objects live for the process) this is a
// compile-once cache; if a host rebuilds the registry per request (new schema
// objects each time), the stale entries are garbage-collected rather than leaking
// on a public endpoint. WeakMap works either way with no lifecycle assumption.
const cache = new WeakMap<object, ValidateFunction>();

function validatorFor(schema: Record<string, unknown>): ValidateFunction {
  let validate = cache.get(schema);
  if (!validate) {
    validate = ajv.compile(schema);
    cache.set(schema, validate);
  }
  return validate;
}

/**
 * Validate `args` against `schema`. Returns a human-readable error string listing
 * every violation, or null if the arguments are valid. Extra properties are
 * allowed unless the schema itself forbids them (the tool schemas do not set
 * additionalProperties, so clients may send extras).
 */
export function validateArgs(schema: Record<string, unknown>, args: unknown): string | null {
  const validate = validatorFor(schema);
  if (validate(args)) return null;
  const errors = (validate.errors ?? [])
    .map((e) => `${e.instancePath || '(root)'} ${e.message}`)
    .join('; ');
  return errors || 'invalid arguments';
}
