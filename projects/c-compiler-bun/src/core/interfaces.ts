/**
 * Core interfaces for the C compiler micro-VM architecture
 */

import { Token, ASTNode, ValueType, BinaryOp, UnaryOp } from './types.js';

/**
 * Base MicroVM interface - all specialized VMs must implement this
 */
export interface MicroVM {
  /** Unique name for this VM */
  readonly name: string;
  
  /** Initialize the VM with optional context */
  initialize?(context?: unknown): void;
  
  /** Execute an operation and return result */
  execute(node: ASTNode, context: ExecutionContext): ExecutionResult;
  
  /** Check if this VM can handle the given node */
  canHandle(node: ASTNode): boolean;
  
  /** Reset VM state */
  reset?(): void;
  
  /** Get current VM state */
  getState?(): unknown;
  
  /** Set VM state */
  setState?(state: unknown): void;
}

/**
 * Execution context passed to VMs
 */
export interface ExecutionContext {
  /** Current memory state */
  memory: MemoryState;
  
  /** Current function call stack */
  callStack: CallStack;
  
  /** Current output buffer */
  output: string[];
  
  /** Current input buffer */
  input: string[];
  
  /** Current scope depth */
  scopeDepth: number;
  
  /** Whether we're in a loop */
  inLoop: boolean;
  
  /** Whether we should break/continue */
  breakFlag: boolean;
  continueFlag: boolean;
  
  /** Return value from function */
  returnValue?: ValueType;
  
  /** Whether we've returned */
  hasReturned: boolean;
}

/**
 * Memory state for variable storage
 */
export interface MemoryState {
  /** Stack frames for each scope */
  frames: StackFrame[];
  
  /** Current frame index */
  currentFrame: number;
  
  /** Heap for dynamic allocations */
  heap: Map<string, ValueType>;
  
  /** Get a variable value */
  get(name: string): ValueType | undefined;
  
  /** Set a variable value */
  set(name: string, value: ValueType): void;
  
  /** Allocate a new variable */
  allocate(name: string, type: string, value?: ValueType): void;
  
  /** Push a new stack frame */
  pushFrame(): void;
  
  /** Pop current stack frame */
  popFrame(): void;
  
  /** Get current frame */
  getCurrentFrame(): StackFrame;
}

/**
 * Stack frame representing a scope
 */
export interface StackFrame {
  /** Variables in this frame */
  variables: Map<string, Variable>;
  
  /** Parent frame index */
  parent?: number;
  
  /** Frame type (global, function, block) */
  type: 'global' | 'function' | 'block';
}

/**
 * Variable information
 */
export interface Variable {
  /** Variable name */
  name: string;
  
  /** Variable type */
  type: string;
  
  /** Current value */
  value: ValueType;
  
  /** Whether this is an array */
  isArray: boolean;
  
  /** Array dimensions (if applicable) */
  dimensions?: number[];
  
  /** Whether this is a pointer */
  isPointer: boolean;
  
  /** Pointer address (if applicable) */
  address?: string;
}

/**
 * Call stack for function calls
 */
export interface CallStack {
  /** Stack of function calls */
  frames: CallFrame[];
  
  /** Current frame index */
  currentFrame: number;
  
  /** Push a new call frame */
  push(frame: CallFrame): void;
  
  /** Pop current call frame */
  pop(): CallFrame | undefined;
  
  /** Get current call frame */
  getCurrent(): CallFrame | undefined;
  
  /** Peek at top frame without popping */
  peek(): CallFrame | undefined;
}

/**
 * Function call frame
 */
export interface CallFrame {
  /** Function name */
  functionName: string;
  
  /** Return address (instruction pointer) */
  returnAddress?: number;
  
  /** Local variables */
  locals: Map<string, ValueType>;
  
  /** Parameters */
  parameters: Map<string, ValueType>;
  
  /** Return value */
  returnValue?: ValueType;
}

/**
 * Result of VM execution
 */
export interface ExecutionResult {
  /** Result value */
  value?: ValueType;
  
  /** Whether execution was successful */
  success: boolean;
  
  /** Error message if failed */
  error?: string;
  
  /** Side effects (output, state changes) */
  sideEffects?: {
    output?: string[];
    memoryChanges?: MemoryChange[];
  };
}

/**
 * Memory change record
 */
export interface MemoryChange {
  /** Variable name */
  variable: string;
  
  /** Old value */
  oldValue?: ValueType;
  
  /** New value */
  newValue: ValueType;
}

/**
 * Lexer interface
 */
export interface Lexer {
  /** Tokenize source code */
  tokenize(source: string): Token[];
  
  /** Get current position */
  getPosition(): { line: number; column: number };
  
  /** Reset lexer state */
  reset(): void;
}

/**
 * Parser interface
 */
export interface Parser {
  /** Parse tokens into AST */
  parse(tokens: Token[]): ASTNode;
  
  /** Get current token */
  getCurrentToken(): Token | undefined;
  
  /** Reset parser state */
  reset(): void;
}

/**
 * Compiler interface
 */
export interface Compiler {
  /** Compile source code */
  compile(source: string): CompilationResult;
  
  /** Run compiled code */
  run(ast: ASTNode, input?: string[]): ExecutionResult;
  
  /** Compile and run in one step */
  compileAndRun(source: string, input?: string[]): ExecutionResult;
}

/**
 * Compilation result
 */
export interface CompilationResult {
  /** Whether compilation was successful */
  success: boolean;
  
  /** Generated AST */
  ast?: ASTNode;
  
  /** Compilation errors */
  errors: CompilationError[];
  
  /** Warnings */
  warnings: CompilationWarning[];
}

/**
 * Compilation error
 */
export interface CompilationError {
  /** Error message */
  message: string;
  
  /** Line number */
  line: number;
  
  /** Column number */
  column: number;
  
  /** Error type */
  type: 'syntax' | 'semantic' | 'type';
}

/**
 * Compilation warning
 */
export interface CompilationWarning {
  /** Warning message */
  message: string;
  
  /** Line number */
  line: number;
  
  /** Column number */
  column: number;
}

/**
 * VM Registry interface
 */
export interface VMRegistry {
  /** Register a VM */
  register(vm: MicroVM): void;
  
  /** Unregister a VM by name */
  unregister(name: string): void;
  
  /** Get a VM by name */
  get(name: string): MicroVM | undefined;
  
  /** Find a VM that can handle the given node */
  find(node: ASTNode): MicroVM | undefined;
  
  /** Get all registered VMs */
  getAll(): MicroVM[];
  
  /** Clear all VMs */
  clear(): void;
}

/**
 * Type checker interface
 */
export interface TypeChecker {
  /** Check type of an expression */
  checkType(node: ASTNode, context: ExecutionContext): string;
  
  /** Check if types are compatible */
  areCompatible(type1: string, type2: string): boolean;
  
  /** Perform type cast */
  cast(value: ValueType, fromType: string, toType: string): ValueType;
}

/**
 * IO handler interface
 */
export interface IOHandler {
  /** Write output */
  write(format: string, args: ValueType[]): void;
  
  /** Read input */
  read(format: string): ValueType[];
  
  /** Get output buffer */
  getOutput(): string[];
  
  /** Clear output buffer */
  clearOutput(): void;
  
  /** Set input buffer */
  setInput(input: string[]): void;
}