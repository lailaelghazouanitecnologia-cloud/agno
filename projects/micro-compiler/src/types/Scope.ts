/**
 * Represents a lexical scope for variable storage
 */
export interface Scope {
  parent: Scope | null;
  variables: Map<string, any>;
}

/**
 * Create a new scope
 */
export function createScope(parent: Scope | null = null): Scope {
  return {
    parent,
    variables: new Map()
  };
}

/**
 * Get a variable from a scope (searching up the chain)
 */
export function getVariable(scope: Scope, name: string): any {
  let current: Scope | null = scope;
  while (current !== null) {
    if (current.variables.has(name)) {
      return current.variables.get(name);
    }
    current = current.parent;
  }
  return undefined;
}

/**
 * Set a variable in the current scope
 */
export function setVariable(scope: Scope, name: string, value: any): void {
  scope.variables.set(name, value);
}

/**
 * Define a new variable in the current scope
 */
export function defineVariable(scope: Scope, name: string, value: any): void {
  if (scope.variables.has(name)) {
    throw new Error(`Variable '${name}' already defined in current scope`);
  }
  scope.variables.set(name, value);
}

/**
 * Assign to an existing variable (searching up the chain)
 */
export function assignVariable(scope: Scope, name: string, value: any): void {
  let current: Scope | null = scope;
  while (current !== null) {
    if (current.variables.has(name)) {
      current.variables.set(name, value);
      return;
    }
    current = current.parent;
  }
  throw new Error(`Cannot assign to undefined variable '${name}'`);
}