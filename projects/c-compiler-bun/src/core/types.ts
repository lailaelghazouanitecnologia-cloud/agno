/**
 * Core type definitions for the C compiler
 */

// Supported C types
export enum CType {
  INT = 'int',
  CHAR = 'char',
  FLOAT = 'float',
  VOID = 'void',
  POINTER = 'pointer',
  ARRAY = 'array',
  FUNCTION = 'function',
}

// Token types
export enum TokenType {
  // Keywords
  INT = 'INT',
  CHAR = 'CHAR',
  FLOAT = 'FLOAT',
  VOID = 'VOID',
  IF = 'IF',
  ELSE = 'ELSE',
  WHILE = 'WHILE',
  FOR = 'FOR',
  RETURN = 'RETURN',
  
  // Literals
  INTEGER_LITERAL = 'INTEGER_LITERAL',
  FLOAT_LITERAL = 'FLOAT_LITERAL',
  CHAR_LITERAL = 'CHAR_LITERAL',
  STRING_LITERAL = 'STRING_LITERAL',
  
  // Identifiers
  IDENTIFIER = 'IDENTIFIER',
  
  // Operators
  PLUS = 'PLUS',
  MINUS = 'MINUS',
  STAR = 'STAR',
  SLASH = 'SLASH',
  PERCENT = 'PERCENT',
  
  // Comparison operators
  LESS = 'LESS',
  GREATER = 'GREATER',
  LESS_EQUAL = 'LESS_EQUAL',
  GREATER_EQUAL = 'GREATER_EQUAL',
  EQUAL = 'EQUAL',
  NOT_EQUAL = 'NOT_EQUAL',
  
  // Assignment
  ASSIGN = 'ASSIGN',
  
  // Pointer operators
  AMPERSAND = 'AMPERSAND',
  
  // Punctuation
  SEMICOLON = 'SEMICOLON',
  COMMA = 'COMMA',
  LPAREN = 'LPAREN',
  RPAREN = 'RPAREN',
  LBRACE = 'LBRACE',
  RBRACE = 'RBRACE',
  LBRACKET = 'LBRACKET',
  RBRACKET = 'RBRACKET',
  
  // Special
  EOF = 'EOF',
  UNKNOWN = 'UNKNOWN',
}

// Token interface
export interface Token {
  type: TokenType;
  value: string;
  line: number;
  column: number;
}

// AST Node types
export enum NodeType {
  // Program structure
  PROGRAM = 'PROGRAM',
  FUNCTION_DECL = 'FUNCTION_DECL',
  FUNCTION_DEF = 'FUNCTION_DEF',
  
  // Statements
  COMPOUND_STMT = 'COMPOUND_STMT',
  EXPR_STMT = 'EXPR_STMT',
  RETURN_STMT = 'RETURN_STMT',
  IF_STMT = 'IF_STMT',
  WHILE_STMT = 'WHILE_STMT',
  FOR_STMT = 'FOR_STMT',
  
  // Declarations
  VAR_DECL = 'VAR_DECL',
  ARRAY_DECL = 'ARRAY_DECL',
  PARAM_DECL = 'PARAM_DECL',
  
  // Expressions
  BINARY_EXPR = 'BINARY_EXPR',
  UNARY_EXPR = 'UNARY_EXPR',
  CALL_EXPR = 'CALL_EXPR',
  IDENTIFIER_EXPR = 'IDENTIFIER_EXPR',
  LITERAL_EXPR = 'LITERAL_EXPR',
  ARRAY_ACCESS_EXPR = 'ARRAY_ACCESS_EXPR',
  POINTER_DEREF_EXPR = 'POINTER_DEREF_EXPR',
  ADDRESS_OF_EXPR = 'ADDRESS_OF_EXPR',
  ASSIGN_EXPR = 'ASSIGN_EXPR',
}

// Base AST node
export interface ASTNode {
  type: NodeType;
  line?: number;
}

// Expression nodes
export interface BinaryExpr extends ASTNode {
  type: NodeType.BINARY_EXPR;
  operator: string;
  left: ASTNode;
  right: ASTNode;
}

export interface UnaryExpr extends ASTNode {
  type: NodeType.UNARY_EXPR;
  operator: string;
  operand: ASTNode;
}

export interface CallExpr extends ASTNode {
  type: NodeType.CALL_EXPR;
  callee: string;
  args: ASTNode[];
}

export interface IdentifierExpr extends ASTNode {
  type: NodeType.IDENTIFIER_EXPR;
  name: string;
}

export interface LiteralExpr extends ASTNode {
  type: NodeType.LITERAL_EXPR;
  valueType: CType;
  value: number | string;
}

export interface ArrayAccessExpr extends ASTNode {
  type: NodeType.ARRAY_ACCESS_EXPR;
  array: ASTNode;
  index: ASTNode;
}

export interface PointerDerefExpr extends ASTNode {
  type: NodeType.POINTER_DEREF_EXPR;
  operand: ASTNode;
}

export interface AddressOfExpr extends ASTNode {
  type: NodeType.ADDRESS_OF_EXPR;
  operand: ASTNode;
}

export interface AssignExpr extends ASTNode {
  type: NodeType.ASSIGN_EXPR;
  target: ASTNode;
  value: ASTNode;
}

// Statement nodes
export interface CompoundStmt extends ASTNode {
  type: NodeType.COMPOUND_STMT;
  statements: ASTNode[];
}

export interface ExprStmt extends ASTNode {
  type: NodeType.EXPR_STMT;
  expression: ASTNode;
}

export interface ReturnStmt extends ASTNode {
  type: NodeType.RETURN_STMT;
  value?: ASTNode;
}

export interface IfStmt extends ASTNode {
  type: NodeType.IF_STMT;
  condition: ASTNode;
  thenBranch: ASTNode;
  elseBranch?: ASTNode;
  elseIfConditions?: ASTNode[];
  elseIfBranches?: ASTNode[];
}

export interface WhileStmt extends ASTNode {
  type: NodeType.WHILE_STMT;
  condition: ASTNode;
  body: ASTNode;
}

export interface ForStmt extends ASTNode {
  type: NodeType.FOR_STMT;
  init?: ASTNode;
  condition?: ASTNode;
  update?: ASTNode;
  body: ASTNode;
}

// Declaration nodes
export interface VarDecl extends ASTNode {
  type: NodeType.VAR_DECL;
  varType: CType;
  name: string;
  init?: ASTNode;
}

export interface ArrayDecl extends ASTNode {
  type: NodeType.ARRAY_DECL;
  elementType: CType;
  name: string;
  size: ASTNode;
}

export interface ParamDecl extends ASTNode {
  type: NodeType.PARAM_DECL;
  paramType: CType;
  name: string;
}

export interface FunctionDef extends ASTNode {
  type: NodeType.FUNCTION_DEF;
  returnType: CType;
  name: string;
  params: ParamDecl[];
  body: CompoundStmt;
}

export interface Program extends ASTNode {
  type: NodeType.PROGRAM;
  functions: FunctionDef[];
}

// Runtime value types
export enum RuntimeType {
  INT = 'int',
  FLOAT = 'float',
  CHAR = 'char',
  POINTER = 'pointer',
  ARRAY = 'array',
  VOID = 'void',
}

export interface RuntimeValue {
  type: RuntimeType;
  value: number | string | RuntimeValue[];
}

// Memory location
export interface MemoryLocation {
  address: number;
  type: RuntimeType;
  value: RuntimeValue;
}

// Stack frame for function calls
export interface StackFrame {
  functionName: string;
  returnAddress: number;
  locals: Map<string, MemoryLocation>;
  params: Map<string, MemoryLocation>;
}

// Compiler result
export interface CompilerResult {
  success: boolean;
  exitCode: number;
  output: string;
  errors: string[];
}

// MicroVM operation types
export enum VMOperation {
  // Arithmetic
  ADD = 'ADD',
  SUB = 'SUB',
  MUL = 'MUL',
  DIV = 'DIV',
  MOD = 'MOD',
  
  // Comparison
  LESS = 'LESS',
  GREATER = 'GREATER',
  LESS_EQUAL = 'LESS_EQUAL',
  GREATER_EQUAL = 'GREATER_EQUAL',
  EQUAL = 'EQUAL',
  NOT_EQUAL = 'NOT_EQUAL',
  
  // Control flow
  JUMP = 'JUMP',
  JUMP_IF_TRUE = 'JUMP_IF_TRUE',
  JUMP_IF_FALSE = 'JUMP_IF_FALSE',
  
  // Memory
  LOAD = 'LOAD',
  STORE = 'STORE',
  ALLOCATE = 'ALLOCATE',
  PUSH = 'PUSH',
  POP = 'POP',
  
  // Function
  CALL = 'CALL',
  RET = 'RET',
  
  // I/O
  PRINT = 'PRINT',
  READ = 'READ',
  
  // Type
  CAST = 'CAST',
}

// VM instruction
export interface VMInstruction {
  operation: VMOperation;
  operands: (RuntimeValue | string | number)[];
}