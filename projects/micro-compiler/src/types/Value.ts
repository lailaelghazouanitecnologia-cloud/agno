/**
 * Runtime value types
 */
export enum ValueType {
  NUMBER = 'NUMBER',
  STRING = 'STRING',
  BOOLEAN = 'BOOLEAN',
  NULL = 'NULL',
  FUNCTION = 'FUNCTION',
  UNDEFINED = 'UNDEFINED'
}

/**
 * Runtime value wrapper
 */
export interface Value {
  type: ValueType;
  data: any;
}

/**
 * Create a runtime value
 */
export function createValue(data: any): Value {
  if (data === null || data === undefined) {
    return { type: ValueType.NULL, data: null };
  }
  if (typeof data === 'number') {
    return { type: ValueType.NUMBER, data };
  }
  if (typeof data === 'string') {
    return { type: ValueType.STRING, data };
  }
  if (typeof data === 'boolean') {
    return { type: ValueType.BOOLEAN, data };
  }
  if (typeof data === 'function') {
    return { type: ValueType.FUNCTION, data };
  }
  return { type: ValueType.UNDEFINED, data };
}

/**
 * Convert value to JavaScript primitive
 */
export function valueToPrimitive(value: Value): any {
  return value.data;
}

/**
 * Check if two values are equal
 */
export function valuesEqual(a: Value, b: Value): boolean {
  if (a.type !== b.type) {
    // Allow number comparison with different types
    if (a.type === ValueType.NUMBER && b.type === ValueType.NUMBER) {
      return a.data === b.data;
    }
    return false;
  }
  return a.data === b.data;
}

/**
 * Convert value to boolean for condition checks
 */
export function valueToBoolean(value: Value): boolean {
  switch (value.type) {
    case ValueType.NUMBER:
      return value.data !== 0;
    case ValueType.STRING:
      return value.data.length > 0;
    case ValueType.BOOLEAN:
      return value.data;
    case ValueType.NULL:
    case ValueType.UNDEFINED:
      return false;
    default:
      return true;
  }
}