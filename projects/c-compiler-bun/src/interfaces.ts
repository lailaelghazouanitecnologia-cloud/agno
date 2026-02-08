/**
 * Core Interfaces for the C Compiler Micro-VM Architecture
 */

// ============================================================================
// Token Types (for Lexer)
// ============================================================================

export enum TokenType {
  // Keywords
  INT = "INT",
  CHAR = "CHAR",
  FLOAT = "FLOAT",
  VOID = "VOID",
  IF = "IF",
  ELSE = "ELSE",
  WHILE = "WHILE",
  FOR = "FOR",
  RETURN = "RETURN",
  
  // Literals
  NUMBER = "NUMBER",
  STRING = "STRING",
  IDENTIFIER = "IDENTIFIER",
  
  // Operators
  PLUS = "PLUS",
  MINUS = "MINUS",
  STAR = "STAR",
  SLASH = "SLASH",
  PERCENT = "PERCENT",
  
  // Comparison operators
  LESS = "LESS",
  GREATER = "GREATER",
  LESS_EQUAL = "LESS_EQUAL",
  GREATER_EQUAL = "GREATER_EQUAL",
  EQUAL = "EQUAL",
  NOT_EQUAL = "NOT_EQUAL",
  
  // Assignment
  ASSIGN = "ASSIGN",
  
  // Pointer operators
  AMPERSAND = "AMPERSAND",
  
  // Punctuation
  SEMICOLON = "SEMICOLON",
  COMMA = "COMMA",
  LPAREN = "LPAREN",
  RPAREN = "RPAREN",
  LBRACE = "LBRACE",
  RBRACE = "RBRACE",
  LBRACKET = "LBRACKET",
  RBRACKET = "RBRACKET",
  
  // Special
  EOF = "EOF"
}

export interface Token {
  type: TokenType;
  value: string;
  line: number;
  column: number;
}

// ============================================================================
// AST Node Types (for Parser)
// ============================================================================

export enum NodeType {
  // Statements
  PROGRAM = "PROGRAM",
  FUNCTION_DECL = "FUNCTION_DECL",
  VAR_DECL = "VAR_DECL",
  ARRAY_DECL = "ARRAY_DECL",
  IF_STMT = "IF_STMT",
  WHILE_STMT = "WHILE_STMT",
  FOR_STMT = "FOR_STMT",
  RETURN_STMT = "RETURN_STMT",
  EXPR_STMT = "EXPR_STMT",
  COMPOUND_STMT = "COMPOUND_STMT",
  
  // Expressions
  BINARY_EXPR = "BINARY_EXPR",
  UNARY_EXPR = "UNARY_EXPR",
  CALL_EXPR = "CALL_EXPR",
  ARRAY_ACCESS = "ARRAY_ACCESS",
  IDENTIFIER = "IDENTIFIER",
  NUMBER = "NUMBER",
  STRING = "STRING",
  
  // Special
  PARAM = "PARAM"
}

export interface ASTNode {
  type: NodeType;
  line?: number;
}

export interface ProgramNode extends ASTNode {
  type: NodeType.PROGRAM;
  declarations: (FunctionDeclNode | VarDeclNode)[];
}

export interface FunctionDeclNode extends ASTNode {
  type: NodeType.FUNCTION_DECL;
  name: string;
  returnType: string;
  params: ParamNode[];
  body: CompoundStmtNode;
}

export interface ParamNode extends ASTNode {
  type: NodeType.PARAM;
  name: string;
  paramType: string;
}

export interface VarDeclNode extends ASTNode {
  type: NodeType.VAR_DECL;
  name: string;
  varType: string;
  init?: ExprNode;
}

export interface ArrayDeclNode extends ASTNode {
  type: NodeType.ARRAY_DECL;
  name: string;
  elementType: string;
  size: ExprNode;
}

export interface CompoundStmtNode extends ASTNode {
  type: NodeType.COMPOUND_STMT;
  statements: StmtNode[];
}

export type StmtNode = 
  | VarDeclNode
  | ArrayDeclNode
  | IfStmtNode
  | WhileStmtNode
  | ForStmtNode
  | ReturnStmtNode
  | ExprStmtNode
  | CompoundStmtNode;

export interface IfStmtNode extends ASTNode {
  type: NodeType.IF_STMT;
  condition: ExprNode;
  thenBranch: StmtNode;
  elseBranch?: StmtNode;
  elseifBranches?: { condition: ExprNode; body: StmtNode }[];
}

export interface WhileStmtNode extends ASTNode {
  type: NodeType.WHILE_STMT;
  condition: ExprNode;
  body: StmtNode;
}

export interface ForStmtNode extends ASTNode {
  type: NodeType.FOR_STMT;
  init?: StmtNode;
  condition?: ExprNode;
  increment?: ExprNode;
  body: StmtNode;
}

export interface ReturnStmtNode extends ASTNode {
  type: NodeType.RETURN_STMT;
  value?: ExprNode;
}

export interface ExprStmtNode extends ASTNode {
  type: NodeType.EXPR_STMT;
  expression: ExprNode;
}

export type ExprNode =
  | BinaryExprNode
  | UnaryExprNode
  | CallExprNode
  | ArrayAccessNode
  | IdentifierNode
  | NumberNode
  | StringNode;

export interface BinaryExprNode extends ASTNode {
  type: NodeType.BINARY_EXPR;
  operator: string;
  left: ExprNode;
  right: ExprNode;
}

export interface UnaryExprNode extends ASTNode {
  type: NodeType.UNARY_EXPR;
  operator: string;
  operand: ExprNode;
}

export interface CallExprNode extends ASTNode {
  type: NodeType.CALL_EXPR;
  callee: string;
  args: ExprNode[];
}

export interface ArrayAccessNode extends ASTNode {
  type: NodeType.ARRAY_ACCESS;
  array: string;
  index: ExprNode;
}

export interface IdentifierNode extends ASTNode {
  type: NodeType.IDENTIFIER;
  name: string;
}

export interface NumberNode extends ASTNode {
  type: NodeType.NUMBER;
  value: number;
}

export interface StringNode extends ASTNode {
  type: NodeType.STRING;
  value: string;
}

// ============================================================================
// Type System
// ============================================================================

export enum CType {
  INT = "int",
  CHAR = "char",
  FLOAT = "float",
  VOID = "void",
  ARRAY = "array",
  POINTER = "pointer"
}

export interface TypeInfo {
  type: CType;
  baseType?: CType;  // For arrays and pointers
  isArray?: boolean;
  isPointer?: boolean;
  arraySize?: number;
}

// ============================================================================
// Runtime Values
// ============================================================================

export enum ValueType {
  INT = "int",
  FLOAT = "float",
  CHAR = "char",
  STRING = "string",
  POINTER = "pointer",
  ARRAY = "array",
  VOID = "void"
}

export interface RuntimeValue {
  type: ValueType;
  value: any;
}

export interface IntValue extends RuntimeValue {
  type: ValueType.INT;
  value: number;
}

export interface FloatValue extends RuntimeValue {
  type: ValueType.FLOAT;
  value: number;
}

export interface CharValue extends RuntimeValue {
  type: ValueType.CHAR;
  value: string;
}

export interface StringValue extends RuntimeValue {
  type: ValueType.STRING;
  value: string;
}

export interface PointerValue extends RuntimeValue {
  type: ValueType.POINTER;
  value: string;  // Variable name
  address?: number;
}

export interface ArrayValue extends RuntimeValue {
  type: ValueType.ARRAY;
  value: RuntimeValue[];
  elementType: ValueType;
}

// ============================================================================
// MicroVM Interface
// ============================================================================

export interface ExecutionContext {
  // Current function being executed
  currentFunction?: string;
  
  // Return value for current function
  returnValue?: RuntimeValue;
  
  // Should return from current function
  shouldReturn?: boolean;
  
  // Control flow flags
  breakLoop?: boolean;
  continueLoop?: boolean;
  
  // Call stack depth
  callDepth?: number;
}

export interface MicroVM {
  // VM name/identifier
  name: string;
  
  // Execute an AST node and return result
  execute(node: ASTNode, context: ExecutionContext): RuntimeValue | void;
  
  // Optional: Validate node before execution
  validate?(node: ASTNode): boolean;
  
  // Optional: Get VM capabilities
  capabilities?: string[];
}

// ============================================================================
// Memory Management
// ============================================================================

export interface StackFrame {
  functionName: string;
  variables: Map<string, RuntimeValue>;
  returnAddress?: number;
}

export interface MemoryManager {
  // Stack frames for function calls
  stackFrames: StackFrame[];
  
  // Global variables
  globals: Map<string, RuntimeValue>;
  
  // Heap for dynamic allocation (future)
  heap: Map<number, RuntimeValue>;
  
  // Current stack frame
  currentFrame(): StackFrame;
  
  // Push new stack frame
  pushFrame(functionName: string): StackFrame;
  
  // Pop stack frame
  popFrame(): StackFrame | null;
  
  // Get variable value
  getVariable(name: string): RuntimeValue | undefined;
  
  // Set variable value
  setVariable(name: string, value: RuntimeValue): void;
  
  // Declare new variable
  declareVariable(name: string, value: RuntimeValue): void;
}

// ============================================================================
// Compiler Configuration
// ============================================================================

export interface CompilerOptions {
  debug?: boolean;
  verbose?: boolean;
  outputAST?: boolean;
  outputTokens?: boolean;
  strictTypes?: boolean;
}

export interface CompileResult {
  success: boolean;
  output?: RuntimeValue;
  errors: string[];
  warnings: string[];
  executionTime?: number;
}

// ============================================================================
// Error Types
// ============================================================================

export class CompilerError extends Error {
  constructor(
    message: string,
    public line?: number,
    public column?: number
  ) {
    super(message);
    this.name = "CompilerError";
  }
}

export class RuntimeError extends Error {
  constructor(
    message: string,
    public line?: number
  ) {
    super(message);
    this.name = "RuntimeError";
  }
}

export class TypeError extends CompilerError {
  constructor(
    message: string,
    line?: number,
    column?: number
  ) {
    super(message, line, column);
    this.name = "TypeError";
  }
}