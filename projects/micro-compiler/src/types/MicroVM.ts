import { ASTNode } from './AST';
import { Scope } from './Scope';
import { Value, createValue } from './Value';

/**
 * Execution context for micro-VMs
 */
export interface ExecutionContext {
  scope: Scope;
  output: string[];
  returnValue?: Value;
  shouldReturn?: boolean;
  shouldBreak?: boolean;
  shouldContinue?: boolean;
}

/**
 * Result of micro-VM execution
 */
export interface ExecutionResult {
  value?: Value;
  output?: string[];
  scope?: Scope;
  error?: Error;
}

/**
 * Base interface for all micro-VMs
 */
export interface MicroVM {
  /**
   * Get the name of this micro-VM
   */
  getName(): string;

  /**
   * Check if this micro-VM can handle the given AST node
   */
  canHandle(node: ASTNode): boolean;

  /**
   * Execute the given AST node
   */
  execute(node: ASTNode, context: ExecutionContext): ExecutionResult;

  /**
   * Compile the given AST node to bytecode (optional)
   */
  compile?(node: ASTNode): any[];
}

// Re-export commonly used types and functions
export { ASTNode } from './AST';
export { createValue };

/**
 * Create a new execution context
 */
export function createExecutionContext(scope: Scope): ExecutionContext {
  return {
    scope,
    output: []
  };
}

/**
 * Create a child execution context
 */
export function createChildContext(parent: ExecutionContext): ExecutionContext {
  return {
    ...parent,
    scope: { parent: parent.scope, variables: new Map() },
    output: [...parent.output]
  };
}