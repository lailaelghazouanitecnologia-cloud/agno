/**
 * Core interfaces for the micro-VM architecture
 */

import { ASTNode, RuntimeValue, VMInstruction, CompilerResult } from './types.js';

/**
 * Base interface for all MicroVM implementations
 * Each specialized VM must implement this interface
 */
export interface MicroVM {
  /**
   * Unique name/identifier for this VM
   */
  readonly name: string;
  
  /**
   * Execute a single instruction
   * @param instruction The instruction to execute
   * @param context The execution context
   * @returns Result of the execution
   */
  execute(instruction: VMInstruction, context: ExecutionContext): ExecutionResult;
  
  /**
   * Check if this VM can handle the given instruction
   * @param instruction The instruction to check
   * @returns true if this VM can handle the instruction
   */
  canHandle(instruction: VMInstruction): boolean;
  
  /**
   * Reset the VM to its initial state
   */
  reset(): void;
}

/**
 * Execution context shared across all VMs
 */
export interface ExecutionContext {
  /**
   * Current program counter
   */
  pc: number;
  
  /**
   * Call stack
   */
  callStack: CallStackFrame[];
  
  /**
   * Global memory
   */
  memory: Memory;
  
  /**
   * Output buffer for I/O operations
   */
  output: string[];
  
  /**
   * Input buffer for I/O operations
   */
  input: string[];
  
  /**
   * Current function being executed
   */
  currentFunction?: string;
  
  /**
   * Flag indicating if execution should stop
   */
  shouldStop: boolean;
  
  /**
   * Return value from function
   */
  returnValue?: RuntimeValue;
  
  /**
   * Error message if execution failed
   */
  error?: string;
}

/**
 * Result of executing an instruction
 */
export interface ExecutionResult {
  /**
   * Whether execution was successful
   */
  success: boolean;
  
  /**
   * New program counter (if different)
   */
  newPc?: number;
  
  /**
   * Value produced by the instruction (if any)
   */
  value?: RuntimeValue;
  
  /**
   * Error message if execution failed
   */
  error?: string;
  
  /**
   * Whether execution should stop
   */
  shouldStop?: boolean;
}

/**
 * Memory interface for the MemoryVM
 */
export interface Memory {
  /**
   * Allocate space for a variable
   */
  allocate(name: string, type: string, size: number): number;
  
  /**
   * Get value at address
   */
  get(address: number): RuntimeValue | undefined;
  
  /**
   * Set value at address
   */
  set(address: number, value: RuntimeValue): void;
  
  /**
   * Get variable by name
   */
  getVariable(name: string): RuntimeValue | undefined;
  
  /**
   * Set variable by name
   */
  setVariable(name: string, value: RuntimeValue): void;
  
  /**
   * Push value onto stack
   */
  push(value: RuntimeValue): void;
  
  /**
   * Pop value from stack
   */
  pop(): RuntimeValue | undefined;
  
  /**
   * Peek at top of stack
   */
  peek(): RuntimeValue | undefined;
  
  /**
   * Create a new stack frame
   */
  pushFrame(frameName: string): void;
  
  /**
   * Pop current stack frame
   */
  popFrame(): void;
  
  /**
   * Get current stack frame
   */
  getCurrentFrame(): CallStackFrame | undefined;
  
  /**
   * Reset memory to initial state
   */
  reset(): void;
}

/**
 * Call stack frame
 */
export interface CallStackFrame {
  /**
   * Function name
   */
  functionName: string;
  
  /**
   * Return address
   */
  returnAddress: number;
  
  /**
   * Local variables
   */
  locals: Map<string, RuntimeValue>;
  
  /**
   * Parameters
   */
  params: Map<string, RuntimeValue>;
  
  /**
   * Base address for this frame
   */
  baseAddress: number;
}

/**
 * Lexer interface
 */
export interface Lexer {
  /**
   * Tokenize source code
   * @param source C source code
   * @returns Array of tokens
   */
  tokenize(source: string): Token[];
  
  /**
   * Reset lexer state
   */
  reset(): void;
}

/**
 * Token interface
 */
export interface Token {
  type: string;
  value: string;
  line: number;
  column: number;
}

/**
 * Parser interface
 */
export interface Parser {
  /**
   * Parse tokens into AST
   * @param tokens Array of tokens
   * @returns AST root node
   */
  parse(tokens: Token[]): ASTNode;
  
  /**
   * Reset parser state
   */
  reset(): void;
}

/**
 * Compiler interface
 */
export interface Compiler {
  /**
   * Compile source code
   * @param source C source code
   * @returns Compiled instructions
   */
  compile(source: string): VMInstruction[];
  
  /**
   * Compile and run source code
   * @param source C source code
   * @returns Execution result
   */
  compileAndRun(source: string): CompilerResult;
  
  /**
   * Run compiled instructions
   * @param instructions Compiled instructions
   * @returns Execution result
   */
  run(instructions: VMInstruction[]): CompilerResult;
}

/**
 * MicroVM Registry interface
 */
export interface MicroVMRegistry {
  /**
   * Register a MicroVM
   * @param vm The VM to register
   */
  register(vm: MicroVM): void;
  
  /**
   * Unregister a MicroVM
   * @param name Name of the VM to unregister
   */
  unregister(name: string): void;
  
  /**
   * Get a registered MicroVM by name
   * @param name Name of the VM
   * @returns The VM or undefined
   */
  get(name: string): MicroVM | undefined;
  
  /**
   * Get the VM that can handle an instruction
   * @param instruction The instruction
   * @returns The VM that can handle it or undefined
   */
  getHandler(instruction: VMInstruction): MicroVM | undefined;
  
  /**
   * Get all registered VMs
   * @returns Array of all registered VMs
   */
  getAll(): MicroVM[];
  
  /**
   * Reset all registered VMs
   */
  resetAll(): void;
}

/**
 * Type checker interface
 */
export interface TypeChecker {
  /**
   * Check types in an AST
   * @param node AST node to check
   * @returns true if types are valid
   */
  check(node: ASTNode): boolean;
  
  /**
   * Get type of an expression
   * @param node AST node
   * @returns Type of the expression
   */
  getType(node: ASTNode): string | undefined;
  
  /**
   * Check if types are compatible
   * @param type1 First type
   * @param type2 Second type
   * @returns true if types are compatible
   */
  isCompatible(type1: string, type2: string): boolean;
}

/**
 * I/O handler interface
 */
export interface IOHandler {
  /**
   * Print output
   * @param value Value to print
   */
  print(value: RuntimeValue): void;
  
  /**
   * Read input
   * @returns Read value
   */
  read(): RuntimeValue | undefined;
  
  /**
   * Get output buffer
   */
  getOutput(): string[];
  
  /**
   * Clear output buffer
   */
  clearOutput(): void;
  
  /**
   * Set input buffer
   * @param input Input values
   */
  setInput(input: string[]): void;
}