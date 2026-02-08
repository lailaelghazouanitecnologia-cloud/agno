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
  FUNCTION = 'function'
}

// Type information with pointer/array depth
export interface TypeInfo {
  base: CType;
  pointerDepth: number;
  isArray: boolean;
  arraySize?: number;
}

// Token types
export enum TokenType {
  // Keywords
  INT = 'int',
  CHAR = 'char',
  FLOAT = 'float',
  VOID = 'void',
  IF = 'if',
  ELSE = 'else',
  WHILE = 'while',
  FOR = 'for',
  RETURN = 'return',
  
  // Literals
  INT_LITERAL = 'INT_LITERAL',
  FLOAT_LITERAL = 'FLOAT_LITERAL',
  CHAR_LITERAL = 'CHAR_LITERAL',
  STRING_LITERAL = 'STRING_LITERAL',
  IDENTIFIER = 'IDENTIFIER',
  
  // Operators
  PLUS = '+',
  MINUS = '-',
  STAR = '*',
  SLASH = '/',
  PERCENT = '%',
  
  // Comparison operators
  LESS = '<',
  GREATER = '>',
  LESS_EQUAL = '<=',
  GREATER_EQUAL = '>=',
  EQUAL = '==',
  NOT_EQUAL = '!=',
  
  // Assignment
  ASSIGN = '=',
  
  // Pointer operators
  AMPERSAND = '&',
  
  // Punctuation
  SEMICOLON = ';',
  COMMA = ',',
  LPAREN = '(',
  RPAREN = ')',
  LBRACE = '{',
  RBRACE = '}',
  LBRACKET = '[',
  RBRACKET = ']',
  
  // Special
  EOF = 'EOF',
  UNKNOWN = 'UNKNOWN'
}

// Token representation
export interface Token {
  type: TokenType;
  value: string;
  line: number;
  column: number;
}

// AST Node types
export enum NodeType {
  // Program
  PROGRAM = 'PROGRAM',
  
  // Declarations
  FUNCTION_DECL = 'FUNCTION_DECL',
  FUNCTION_DEF = 'FUNCTION_DEF',
  PARAM_DECL = 'PARAM_DECL',
  VAR_DECL = 'VAR_DECL',
  ARRAY_DECL = 'ARRAY_DECL',
  
  // Statements
  COMPOUND_STMT = 'COMPOUND_STMT',
  IF_STMT = 'IF_STMT',
  WHILE_STMT = 'WHILE_STMT',
  FOR_STMT = 'FOR_STMT',
  RETURN_STMT = 'RETURN_STMT',
  EXPR_STMT = 'EXPR_STMT',
  
  // Expressions
  BINARY_EXPR = 'BINARY_EXPR',
  UNARY_EXPR = 'UNARY_EXPR',
  CALL_EXPR = 'CALL_EXPR',
  ARRAY_ACCESS_EXPR = 'ARRAY_ACCESS_EXPR',
  POINTER_DEREF_EXPR = 'POINTER_DEREF_EXPR',
  ADDRESS_OF_EXPR = 'ADDRESS_OF_EXPR',
  ASSIGN_EXPR = 'ASSIGN_EXPR',
  
  // Literals and identifiers
  INT_LITERAL = 'INT_LITERAL',
  FLOAT_LITERAL = 'FLOAT_LITERAL',
  CHAR_LITERAL = 'CHAR_LITERAL',
  STRING_LITERAL = 'STRING_LITERAL',
  IDENTIFIER_EXPR = 'IDENTIFIER_EXPR'
}

// Base AST node
export interface ASTNode {
  type: NodeType;
  line: number;
  column: number;
}

// Expression nodes
export interface ExpressionNode extends ASTNode {
  exprType: TypeInfo;
}

// Binary expression
export interface BinaryExpr extends ExpressionNode {
  type: NodeType.BINARY_EXPR;
  operator: TokenType;
  left: ExpressionNode;
  right: ExpressionNode;
}

// Unary expression
export interface UnaryExpr extends ExpressionNode {
  type: NodeType.UNARY_EXPR;
  operator: TokenType;
  operand: ExpressionNode;
}

// Assignment expression
export interface AssignExpr extends ExpressionNode {
  type: NodeType.ASSIGN_EXPR;
  target: ExpressionNode;
  value: ExpressionNode;
}

// Function call expression
export interface CallExpr extends ExpressionNode {
  type: NodeType.CALL_EXPR;
  functionName: string;
  arguments: ExpressionNode[];
}

// Array access expression
export interface ArrayAccessExpr extends ExpressionNode {
  type: NodeType.ARRAY_ACCESS_EXPR;
  array: ExpressionNode;
  index: ExpressionNode;
}

// Pointer dereference expression
export interface PointerDerefExpr extends ExpressionNode {
  type: NodeType.POINTER_DEREF_EXPR;
  operand: ExpressionNode;
}

// Address of expression
export interface AddressOfExpr extends ExpressionNode {
  type: NodeType.ADDRESS_OF_EXPR;
  operand: ExpressionNode;
}

// Identifier expression
export interface IdentifierExpr extends ExpressionNode {
  type: NodeType.IDENTIFIER_EXPR;
  name: string;
}

// Literal expressions
export interface IntLiteral extends ExpressionNode {
  type: NodeType.INT_LITERAL;
  value: number;
}

export interface FloatLiteral extends ExpressionNode {
  type: NodeType.FLOAT_LITERAL;
  value: number;
}

export interface CharLiteral extends ExpressionNode {
  type: NodeType.CHAR_LITERAL;
  value: string;
}

export interface StringLiteral extends ExpressionNode {
  type: NodeType.STRING_LITERAL;
  value: string;
}

// Statement nodes
export interface StatementNode extends ASTNode {}

// Compound statement (block)
export interface CompoundStmt extends StatementNode {
  type: NodeType.COMPOUND_STMT;
  statements: StatementNode[];
}

// If statement
export interface IfStmt extends StatementNode {
  type: NodeType.IF_STMT;
  condition: ExpressionNode;
  thenBranch: StatementNode;
  elseBranch?: StatementNode;
}

// While statement
export interface WhileStmt extends StatementNode {
  type: NodeType.WHILE_STMT;
  condition: ExpressionNode;
  body: StatementNode;
}

// For statement
export interface ForStmt extends StatementNode {
  type: NodeType.FOR_STMT;
  init?: StatementNode | ExpressionNode;
  condition?: ExpressionNode;
  update?: ExpressionNode;
  body: StatementNode;
}

// Return statement
export interface ReturnStmt extends StatementNode {
  type: NodeType.RETURN_STMT;
  value?: ExpressionNode;
}

// Expression statement
export interface ExprStmt extends StatementNode {
  type: NodeType.EXPR_STMT;
  expression: ExpressionNode;
}

// Declaration nodes
export interface DeclarationNode extends ASTNode {}

// Variable declaration
export interface VarDecl extends DeclarationNode {
  type: NodeType.VAR_DECL;
  varType: TypeInfo;
  name: string;
  init?: ExpressionNode;
}

// Array declaration
export interface ArrayDecl extends DeclarationNode {
  type: NodeType.ARRAY_DECL;
  elementType: TypeInfo;
  name: string;
  size: ExpressionNode;
}

// Parameter declaration
export interface ParamDecl extends DeclarationNode {
  type: NodeType.PARAM_DECL;
  paramType: TypeInfo;
  name: string;
}

// Function declaration
export interface FunctionDecl extends DeclarationNode {
  type: NodeType.FUNCTION_DECL;
  returnType: TypeInfo;
  name: string;
  parameters: ParamDecl[];
}

// Function definition
export interface FunctionDef extends DeclarationNode {
  type: NodeType.FUNCTION_DEF;
  returnType: TypeInfo;
  name: string;
  parameters: ParamDecl[];
  body: CompoundStmt;
}

// Program node
export interface ProgramNode extends ASTNode {
  type: NodeType.PROGRAM;
  declarations: (FunctionDecl | FunctionDef | VarDecl)[];
}

// Runtime value types
export enum ValueType {
  INT = 'INT',
  FLOAT = 'FLOAT',
  CHAR = 'CHAR',
  POINTER = 'POINTER',
  ARRAY = 'ARRAY',
  VOID = 'VOID'
}

// Runtime value
export interface RuntimeValue {
  type: ValueType;
  value: any;
}

// Stack frame for function calls
export interface StackFrame {
  functionName: string;
  returnAddress?: number;
  locals: Map<string, RuntimeValue>;
  parameters: RuntimeValue[];
}

// Compilation context
export interface CompilationContext {
  currentFunction?: string;
  symbolTable: Map<string, TypeInfo>;
  errorCount: number;
  errors: CompileError[];
}

// Compilation error
export interface CompileError {
  message: string;
  line: number;
  column: number;
}

// Execution context for VM
export interface ExecutionContext {
  stack: StackFrame[];
  globalScope: Map<string, RuntimeValue>;
  heap: Map<number, RuntimeValue>;
  currentFrame: number;
  output: string[];
  input: string[];
  pc: number; // Program counter
  returnValue?: RuntimeValue;
  shouldReturn: boolean;
  shouldBreak: boolean;
  shouldContinue: boolean;
}