/**
 * Token types for the C lexer
 */
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
  INTEGER_LITERAL = "INTEGER_LITERAL",
  FLOAT_LITERAL = "FLOAT_LITERAL",
  CHAR_LITERAL = "CHAR_LITERAL",
  STRING_LITERAL = "STRING_LITERAL",
  
  // Identifiers
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
  POINTER = "POINTER",
  
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
  EOF = "EOF",
  UNKNOWN = "UNKNOWN"
}

/**
 * Token representing a lexical unit
 */
export interface Token {
  type: TokenType;
  value: string;
  line: number;
  column: number;
}

/**
 * AST Node types
 */
export enum NodeType {
  PROGRAM = "PROGRAM",
  FUNCTION_DECL = "FUNCTION_DECL",
  FUNCTION_CALL = "FUNCTION_CALL",
  PARAMETER = "PARAMETER",
  PARAMETER_LIST = "PARAMETER_LIST",
  ARGUMENT_LIST = "ARGUMENT_LIST",
  
  // Statements
  COMPOUND_STMT = "COMPOUND_STMT",
  IF_STMT = "IF_STMT",
  WHILE_STMT = "WHILE_STMT",
  FOR_STMT = "FOR_STMT",
  RETURN_STMT = "RETURN_STMT",
  EXPR_STMT = "EXPR_STMT",
  DECL_STMT = "DECL_STMT",
  
  // Expressions
  BINARY_EXPR = "BINARY_EXPR",
  UNARY_EXPR = "UNARY_EXPR",
  ASSIGN_EXPR = "ASSIGN_EXPR",
  CALL_EXPR = "CALL_EXPR",
  IDENTIFIER_EXPR = "IDENTIFIER_EXPR",
  LITERAL_EXPR = "LITERAL_EXPR",
  ARRAY_ACCESS_EXPR = "ARRAY_ACCESS_EXPR",
  POINTER_DEREF_EXPR = "POINTER_DEREF_EXPR",
  ADDRESS_OF_EXPR = "ADDRESS_OF_EXPR",
  
  // Types
  INT_TYPE = "INT_TYPE",
  CHAR_TYPE = "CHAR_TYPE",
  FLOAT_TYPE = "FLOAT_TYPE",
  VOID_TYPE = "VOID_TYPE",
  ARRAY_TYPE = "ARRAY_TYPE",
  POINTER_TYPE = "POINTER_TYPE"
}

/**
 * Base AST Node interface
 */
export interface ASTNode {
  type: NodeType;
  line: number;
  column: number;
}

/**
 * Type information
 */
export interface TypeInfo {
  baseType: "int" | "char" | "float" | "void";
  isPointer: boolean;
  isArray: boolean;
  arraySize?: number;
}

/**
 * Value types in the VM
 */
export type VMValue = number | string | null | VMValue[];

/**
 * Variable symbol
 */
export interface Symbol {
  name: string;
  type: TypeInfo;
  scope: string;
  address?: number;
  isParameter?: boolean;
}

/**
 * Function symbol
 */
export interface FunctionSymbol {
  name: string;
  returnType: TypeInfo;
  parameters: Symbol[];
  hasBody: boolean;
}

/**
 * MicroVM interface - all specialized VMs must implement this
 */
export interface MicroVM {
  name: string;
  initialize(): void;
  execute(node: ASTNode, context: VMContext): VMValue | void;
  reset(): void;
}

/**
 * VM execution context
 */
export interface VMContext {
  currentFunction?: string;
  callStack: CallFrame[];
  breakTarget?: string;
  continueTarget?: string;
  returnValue?: VMValue;
}

/**
 * Call frame for function execution
 */
export interface CallFrame {
  functionName: string;
  returnAddress: number;
  locals: Map<string, VMValue>;
  parameters: VMValue[];
}

/**
 * Compiler result
 */
export interface CompilerResult {
  success: boolean;
  output?: string;
  errors: string[];
  warnings: string[];
}