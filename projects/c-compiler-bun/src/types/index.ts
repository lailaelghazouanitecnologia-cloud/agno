// Core type definitions for the C compiler

// Token types
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

  // Operators
  PLUS = "PLUS",
  MINUS = "MINUS",
  STAR = "STAR",
  SLASH = "SLASH",
  PERCENT = "PERCENT",
  LESS = "LESS",
  GREATER = "GREATER",
  LESS_EQUAL = "LESS_EQUAL",
  GREATER_EQUAL = "GREATER_EQUAL",
  EQUAL_EQUAL = "EQUAL_EQUAL",
  BANG_EQUAL = "BANG_EQUAL",
  EQUAL = "EQUAL",
  BANG = "BANG",
  AMPERSAND = "AMPERSAND",
  PIPE = "PIPE",
  CARET = "CARET",
  TILDE = "TILDE",

  // Punctuation
  SEMICOLON = "SEMICOLON",
  COMMA = "COMMA",
  LPAREN = "LPAREN",
  RPAREN = "RPAREN",
  LBRACE = "LBRACE",
  RBRACE = "RBRACE",
  LBRACKET = "LBRACKET",
  RBRACKET = "RBRACKET",

  // Literals
  NUMBER = "NUMBER",
  STRING = "STRING",
  CHARACTER = "CHARACTER",

  // Identifiers
  IDENTIFIER = "IDENTIFIER",

  // Special
  EOF = "EOF",
  UNKNOWN = "UNKNOWN"
}

// Token interface
export interface Token {
  type: TokenType;
  value: string;
  line: number;
  column: number;
}

// Value types for runtime
export type RuntimeValue = number | string | RuntimeValue[] | null | PointerValue;

export interface PointerValue {
  address: number;
  type: CType;
}

// C types
export enum CType {
  INT = "int",
  CHAR = "char",
  FLOAT = "float",
  VOID = "void",
  POINTER = "pointer",
  ARRAY = "array",
  FUNCTION = "function"
}

// Type information
export interface TypeInfo {
  base: CType;
  isPointer: boolean;
  isArray: boolean;
  arraySize?: number;
  pointerTo?: CType;
}

// Symbol table entry
export interface Symbol {
  name: string;
  type: TypeInfo;
  value?: RuntimeValue;
  isFunction: boolean;
  parameters?: Parameter[];
  address?: number;
  isArray?: boolean;
  arraySize?: number;
}

export interface Parameter {
  name: string;
  type: TypeInfo;
}

// AST Node types
export type ASTNode =
  | ProgramNode
  | FunctionDeclNode
  | FunctionDefNode
  | VarDeclNode
  | ParamNode
  | CompoundStmtNode
  | ExprStmtNode
  | IfStmtNode
  | WhileStmtNode
  | ForStmtNode
  | ReturnStmtNode
  | BreakStmtNode
  | ContinueStmtNode
  | BinaryExprNode
  | UnaryExprNode
  | AssignExprNode
  | CallExprNode
  | NumberLiteralNode
  | StringLiteralNode
  | CharLiteralNode
  | IdentifierNode
  | ArrayAccessNode
  | ArrayDeclNode
  | PointerDeclNode
  | AddressOfNode
  | DereferenceNode;

// Base AST node
export interface BaseNode {
  type: string;
  line: number;
}

// Program node
export interface ProgramNode extends BaseNode {
  type: "Program";
  declarations: (FunctionDeclNode | FunctionDefNode | VarDeclNode)[];
}

// Function declaration
export interface FunctionDeclNode extends BaseNode {
  type: "FunctionDecl";
  returnType: TypeInfo;
  name: string;
  parameters: ParamNode[];
}

// Function definition
export interface FunctionDefNode extends BaseNode {
  type: "FunctionDef";
  returnType: TypeInfo;
  name: string;
  parameters: ParamNode[];
  body: CompoundStmtNode;
}

// Parameter node
export interface ParamNode extends BaseNode {
  type: "Param";
  paramType: TypeInfo;
  name: string;
}

// Variable declaration
export interface VarDeclNode extends BaseNode {
  type: "VarDecl";
  varType: TypeInfo;
  name: string;
  init?: ASTNode;
  isArray?: boolean;
  arraySize?: number;
  isPointer?: boolean;
}

// Compound statement (block)
export interface CompoundStmtNode extends BaseNode {
  type: "CompoundStmt";
  statements: ASTNode[];
}

// Expression statement
export interface ExprStmtNode extends BaseNode {
  type: "ExprStmt";
  expression: ASTNode;
}

// If statement
export interface IfStmtNode extends BaseNode {
  type: "IfStmt";
  condition: ASTNode;
  thenBranch: ASTNode;
  elseBranch?: ASTNode;
  elseIfs?: { condition: ASTNode; body: ASTNode }[];
}

// While statement
export interface WhileStmtNode extends BaseNode {
  type: "WhileStmt";
  condition: ASTNode;
  body: ASTNode;
}

// For statement
export interface ForStmtNode extends BaseNode {
  type: "ForStmt";
  init?: ASTNode;
  condition?: ASTNode;
  update?: ASTNode;
  body: ASTNode;
}

// Return statement
export interface ReturnStmtNode extends BaseNode {
  type: "ReturnStmt";
  value?: ASTNode;
}

// Break statement
export interface BreakStmtNode extends BaseNode {
  type: "BreakStmt";
}

// Continue statement
export interface ContinueStmtNode extends BaseNode {
  type: "ContinueStmt";
}

// Binary expression
export interface BinaryExprNode extends BaseNode {
  type: "BinaryExpr";
  operator: TokenType;
  left: ASTNode;
  right: ASTNode;
}

// Unary expression
export interface UnaryExprNode extends BaseNode {
  type: "UnaryExpr";
  operator: TokenType;
  operand: ASTNode;
  prefix: boolean;
}

// Assignment expression
export interface AssignExprNode extends BaseNode {
  type: "AssignExpr";
  target: ASTNode;
  value: ASTNode;
}

// Function call expression
export interface CallExprNode extends BaseNode {
  type: "CallExpr";
  function: ASTNode;
  arguments: ASTNode[];
}

// Number literal
export interface NumberLiteralNode extends BaseNode {
  type: "NumberLiteral";
  value: number;
  isFloat: boolean;
}

// String literal
export interface StringLiteralNode extends BaseNode {
  type: "StringLiteral";
  value: string;
}

// Character literal
export interface CharLiteralNode extends BaseNode {
  type: "CharLiteral";
  value: string;
}

// Identifier
export interface IdentifierNode extends BaseNode {
  type: "Identifier";
  name: string;
}

// Array access
export interface ArrayAccessNode extends BaseNode {
  type: "ArrayAccess";
  array: ASTNode;
  index: ASTNode;
}

// Array declaration
export interface ArrayDeclNode extends BaseNode {
  type: "ArrayDecl";
  elementType: TypeInfo;
  name: string;
  size: ASTNode;
}

// Pointer declaration
export interface PointerDeclNode extends BaseNode {
  type: "PointerDecl";
  pointerType: TypeInfo;
  name: string;
}

// Address-of operator
export interface AddressOfNode extends BaseNode {
  type: "AddressOf";
  operand: ASTNode;
}

// Dereference operator
export interface DereferenceNode extends BaseNode {
  type: "Dereference";
  operand: ASTNode;
}

// Compiler error
export interface CompilerError {
  message: string;
  line: number;
  column: number;
  type: "lexer" | "parser" | "semantic" | "runtime";
}

// Runtime result
export interface RuntimeResult {
  value: RuntimeValue;
  error?: CompilerError;
}