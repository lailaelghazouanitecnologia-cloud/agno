/**
 * Core type definitions for the C compiler
 */

/**
 * Token types
 */
export enum TokenType {
  // Keywords
  INT = 'INT',
  CHAR = 'CHAR',
  FLOAT = 'FLOAT',
  VOID = 'VOID',
  IF = 'IF',
  ELSE = 'ELSE',
  ELSE_IF = 'ELSE_IF',
  WHILE = 'WHILE',
  FOR = 'FOR',
  RETURN = 'RETURN',
  BREAK = 'BREAK',
  CONTINUE = 'CONTINUE',
  
  // Literals
  INTEGER = 'INTEGER',
  FLOAT_LITERAL = 'FLOAT_LITERAL',
  CHARACTER = 'CHARACTER',
  STRING = 'STRING',
  
  // Identifiers
  IDENTIFIER = 'IDENTIFIER',
  
  // Operators
  PLUS = 'PLUS',
  MINUS = 'MINUS',
  MULTIPLY = 'MULTIPLY',
  DIVIDE = 'DIVIDE',
  MODULO = 'MODULO',
  
  // Comparison operators
  LESS = 'LESS',
  GREATER = 'GREATER',
  LESS_EQUAL = 'LESS_EQUAL',
  GREATER_EQUAL = 'GREATER_EQUAL',
  EQUAL = 'EQUAL',
  NOT_EQUAL = 'NOT_EQUAL',
  
  // Logical operators
  AND = 'AND',
  OR = 'OR',
  NOT = 'NOT',
  
  // Assignment
  ASSIGN = 'ASSIGN',
  PLUS_ASSIGN = 'PLUS_ASSIGN',
  MINUS_ASSIGN = 'MINUS_ASSIGN',
  MULTIPLY_ASSIGN = 'MULTIPLY_ASSIGN',
  DIVIDE_ASSIGN = 'DIVIDE_ASSIGN',
  
  // Pointer operators
  ADDRESS = 'ADDRESS',
  DEREFERENCE = 'DEREFERENCE',
  
  // Punctuation
  SEMICOLON = 'SEMICOLON',
  COMMA = 'COMMA',
  DOT = 'DOT',
  ARROW = 'ARROW',
  
  // Brackets
  LEFT_PAREN = 'LEFT_PAREN',
  RIGHT_PAREN = 'RIGHT_PAREN',
  LEFT_BRACE = 'LEFT_BRACE',
  RIGHT_BRACE = 'RIGHT_BRACE',
  LEFT_BRACKET = 'LEFT_BRACKET',
  RIGHT_BRACKET = 'RIGHT_BRACKET',
  
  // Special
  EOF = 'EOF',
  UNKNOWN = 'UNKNOWN',
}

/**
 * Token representation
 */
export interface Token {
  type: TokenType;
  value: string;
  line: number;
  column: number;
}

/**
 * AST node types - using string values for test compatibility
 */
export enum NodeType {
  // Program
  PROGRAM = 'PROGRAM',
  
  // Declarations
  DECLARATION = 'DECLARATION',
  FUNCTION_DEF = 'FUNCTION_DEF',
  PARAMETER = 'PARAMETER',
  TYPE_SPECIFIER = 'TYPE_SPECIFIER',
  
  // Statements
  BLOCK = 'BLOCK',
  IF = 'IF',
  WHILE = 'WHILE',
  FOR = 'FOR',
  RETURN = 'RETURN',
  BREAK = 'BREAK',
  CONTINUE = 'CONTINUE',
  EXPRESSION_STMT = 'EXPRESSION_STMT',
  
  // Expressions
  BINARY_EXPR = 'BINARY_EXPR',
  UNARY_EXPR = 'UNARY_EXPR',
  ASSIGNMENT_EXPR = 'ASSIGNMENT_EXPR',
  CALL_EXPR = 'CALL_EXPR',
  ARRAY_ACCESS = 'ARRAY_ACCESS',
  
  // Literals
  INTEGER_LITERAL = 'INTEGER_LITERAL',
  FLOAT_LITERAL = 'FLOAT_LITERAL',
  CHAR_LITERAL = 'CHAR_LITERAL',
  STRING_LITERAL = 'STRING_LITERAL',
  
  // References
  IDENTIFIER_EXPR = 'IDENTIFIER_EXPR',
  ADDRESS_EXPR = 'ADDRESS_EXPR',
  DEREFERENCE_EXPR = 'DEREFERENCE_EXPR',
}

/**
 * AST node base
 */
export interface ASTNode {
  type: NodeType | string;
  line: number;
  column: number;
  [key: string]: unknown;
}

/**
 * Value types
 */
export type ValueType = number | string | boolean | null | ValueType[];

/**
 * Binary operators
 */
export enum BinaryOp {
  // Arithmetic
  ADD = '+',
  SUB = '-',
  MUL = '*',
  DIV = '/',
  MOD = '%',
  
  // Comparison
  LT = '<',
  GT = '>',
  LTE = '<=',
  GTE = '>=',
  EQ = '==',
  NEQ = '!=',
  
  // Logical
  AND = '&&',
  OR = '||',
  
  // Assignment
  ASSIGN = '=',
  ADD_ASSIGN = '+=',
  SUB_ASSIGN = '-=',
  MUL_ASSIGN = '*=',
  DIV_ASSIGN = '/=',
}

/**
 * Unary operators
 */
export enum UnaryOp {
  // Arithmetic
  POS = '+',
  NEG = '-',
  
  // Logical
  NOT = '!',
  BIT_NOT = '~',
  
  // Pointer
  ADDRESS = '&',
  DEREFERENCE = '*',
  
  // Increment/Decrement
  PRE_INC = '++',
  PRE_DEC = '--',
  POST_INC = '++',
  POST_DEC = '--',
}

/**
 * C types
 */
export enum CType {
  INT = 'int',
  CHAR = 'char',
  FLOAT = 'float',
  VOID = 'void',
  POINTER = 'pointer',
  ARRAY = 'array',
}

/**
 * Type information
 */
export interface TypeInfo {
  base: CType;
  isPointer: boolean;
  isArray: boolean;
  arrayDimensions?: number[];
  pointerDepth?: number;
}

/**
 * Function signature
 */
export interface FunctionSignature {
  name: string;
  returnType: TypeInfo;
  parameters: ParameterInfo[];
  isVariadic: boolean;
}

/**
 * Parameter information
 */
export interface ParameterInfo {
  name: string;
  type: TypeInfo;
}

/**
 * Symbol table entry
 */
export interface SymbolEntry {
  name: string;
  type: TypeInfo;
  kind: 'variable' | 'function' | 'parameter';
  scope: number;
  value?: ValueType;
  functionSignature?: FunctionSignature;
}

/**
 * Scope information
 */
export interface Scope {
  id: number;
  parent?: number;
  symbols: Map<string, SymbolEntry>;
}

/**
 * Symbol table
 */
export interface SymbolTable {
  scopes: Scope[];
  currentScope: number;
  
  /** Enter a new scope */
  enterScope(): void;
  
  /** Exit current scope */
  exitScope(): void;
  
  /** Add a symbol to current scope */
  addSymbol(entry: SymbolEntry): void;
  
  /** Look up a symbol */
  lookup(name: string): SymbolEntry | undefined;
  
  /** Look up in current scope only */
  lookupLocal(name: string): SymbolEntry | undefined;
}