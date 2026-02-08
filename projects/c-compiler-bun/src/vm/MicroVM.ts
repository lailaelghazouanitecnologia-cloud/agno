// Common MicroVM interface that all specialized VMs must implement

import type { ASTNode, RuntimeValue, CompilerError } from "../types/index.js";

// VM execution context
export interface VMContext {
  // Call stack for function calls
  callStack: CallFrame[];

  // Current scope
  currentScope: Scope;

  // Return value from function
  returnValue: RuntimeValue | null;

  // Control flow flags
  shouldReturn: boolean;
  shouldBreak: boolean;
  shouldContinue: boolean;

  // Global symbol table
  globals: Map<string, any>;

  // Memory for variables and arrays
  memory: Memory;

  // Output buffer for printf
  output: string[];

  // Input buffer for scanf
  input: string[];

  // Error tracking
  errors: CompilerError[];
}

// Call frame for function calls
export interface CallFrame {
  functionName: string;
  returnAddress?: number;
  locals: Map<string, any>;
  parameters: Map<string, any>;
}

// Scope for variable tracking
export interface Scope {
  parent?: Scope;
  symbols: Map<string, any>;
  level: number;
}

// Memory management
export interface Memory {
  // Stack memory
  stack: Map<number, RuntimeValue>;

  // Heap memory (for dynamic allocation - not used in basic version)
  heap: Map<number, RuntimeValue>;

  // Next available address
  nextAddress: number;

  // Allocate memory
  allocate(size: number): number;

  // Free memory
  free(address: number): void;

  // Read from memory
  read(address: number): RuntimeValue;

  // Write to memory
  write(address: number, value: RuntimeValue): void;
}

// MicroVM interface
export interface MicroVM {
  // VM name for identification
  readonly name: string;

  // Initialize the VM
  initialize(context: VMContext): void;

  // Check if this VM can handle the given node
  canHandle(node: ASTNode): boolean;

  // Execute the given node
  execute(node: ASTNode, context: VMContext): RuntimeValue;

  // Optional: cleanup after execution
  cleanup?(context: VMContext): void;
}

// Base class for MicroVMs with common functionality
export abstract class BaseMicroVM implements MicroVM {
  abstract readonly name: string;

  initialize(_context: VMContext): void {
    // Default: no initialization needed
  }

  abstract canHandle(node: ASTNode): boolean;
  abstract execute(node: ASTNode, context: VMContext): RuntimeValue;

  protected createError(
    message: string,
    line: number,
    type: "lexer" | "parser" | "semantic" | "runtime" = "runtime"
  ): CompilerError {
    return { message, line, column: 0, type };
  }

  protected addError(context: VMContext, error: CompilerError): void {
    context.errors.push(error);
  }
}