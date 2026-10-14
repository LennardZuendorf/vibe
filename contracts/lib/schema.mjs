// Minimal JSON Schema subset validator (zero dependencies).
//
// Supported keywords: $schema, $comment, title, description, x-vibe-compat,
// $defs, $ref (local "#/$defs/<name>" only), type, const, enum, required,
// properties, additionalProperties (absent or true only), items, minItems,
// minLength, minimum, pattern. Any other keyword is a schema error: a schema
// must not rely on a keyword this validator would silently ignore.

const KEYWORDS = new Set([
  '$schema', '$comment', 'title', 'description', 'x-vibe-compat', '$defs', '$ref',
  'type', 'const', 'enum', 'required', 'properties', 'additionalProperties',
  'items', 'minItems', 'minLength', 'minimum', 'pattern',
]);

const isObject = (v) => v !== null && typeof v === 'object' && !Array.isArray(v);

/** Returns a list of problems with `schema` itself; empty when usable. */
export function checkSchema(schema, at = '#') {
  if (!isObject(schema)) return [`${at}: a schema must be an object`];
  const errors = [];
  for (const key of Object.keys(schema)) {
    if (!KEYWORDS.has(key)) errors.push(`${at}: unsupported keyword "${key}"`);
  }
  if ('additionalProperties' in schema && schema.additionalProperties !== true) {
    errors.push(`${at}: additionalProperties must be absent or true (readers ignore unknown fields)`);
  }
  if ('$ref' in schema && !/^#\/\$defs\/[A-Za-z0-9_-]+$/.test(schema.$ref)) {
    errors.push(`${at}: $ref must be "#/$defs/<name>"`);
  }
  for (const [name, sub] of Object.entries(schema.$defs ?? {})) errors.push(...checkSchema(sub, `${at}/$defs/${name}`));
  for (const [name, sub] of Object.entries(schema.properties ?? {})) errors.push(...checkSchema(sub, `${at}/properties/${name}`));
  if ('items' in schema) errors.push(...checkSchema(schema.items, `${at}/items`));
  return errors;
}

function typeOf(v) {
  if (v === null) return 'null';
  if (Array.isArray(v)) return 'array';
  if (Number.isInteger(v)) return 'integer';
  return typeof v;
}

function typeMatches(want, v) {
  const got = typeOf(v);
  return want === got || (want === 'number' && got === 'integer');
}

/** Validates `value` against `schema`; `root` resolves $ref. Returns error strings. */
export function validate(root, schema, value, at = '$') {
  if (schema.$ref) {
    const name = schema.$ref.slice('#/$defs/'.length);
    const target = root.$defs?.[name];
    if (!target) return [`${at}: unresolved $ref ${schema.$ref}`];
    return validate(root, target, value, at);
  }
  const errors = [];
  if (schema.type !== undefined) {
    const types = Array.isArray(schema.type) ? schema.type : [schema.type];
    if (!types.some((t) => typeMatches(t, value))) {
      return [`${at}: expected ${types.join('|')}, got ${typeOf(value)}`];
    }
  }
  if ('const' in schema && JSON.stringify(value) !== JSON.stringify(schema.const)) {
    errors.push(`${at}: expected const ${JSON.stringify(schema.const)}`);
  }
  if (schema.enum && !schema.enum.some((e) => JSON.stringify(e) === JSON.stringify(value))) {
    errors.push(`${at}: ${JSON.stringify(value)} is not one of ${JSON.stringify(schema.enum)}`);
  }
  if (typeof value === 'string') {
    if (schema.minLength !== undefined && [...value].length < schema.minLength) errors.push(`${at}: shorter than ${schema.minLength}`);
    if (schema.pattern !== undefined && !new RegExp(schema.pattern, 'u').test(value)) errors.push(`${at}: does not match ${schema.pattern}`);
  }
  if (typeof value === 'number' && schema.minimum !== undefined && value < schema.minimum) {
    errors.push(`${at}: below minimum ${schema.minimum}`);
  }
  if (Array.isArray(value)) {
    if (schema.minItems !== undefined && value.length < schema.minItems) errors.push(`${at}: fewer than ${schema.minItems} items`);
    if (schema.items) value.forEach((item, i) => errors.push(...validate(root, schema.items, item, `${at}[${i}]`)));
  }
  if (isObject(value)) {
    for (const key of schema.required ?? []) {
      if (!(key in value)) errors.push(`${at}: missing required "${key}"`);
    }
    for (const [key, sub] of Object.entries(schema.properties ?? {})) {
      if (key in value) errors.push(...validate(root, sub, value[key], `${at}.${key}`));
    }
  }
  return errors;
}
