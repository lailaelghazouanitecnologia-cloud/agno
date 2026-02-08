/**
 * Token types for the Volt language
 */
export enum TokenType {
  // Literals
  NUMBER = 'NUMBER',
  STRING = 'STRING',
  IDENTIFIER = 'IDENTIFIER',

  // Keywords
  LET = 'LET',
  FUNCTION = 'FUNCTION',
  IF = 'IF',
  ELSE = 'ELSE',
  WHILE = 'WHILE',
  PRINT = 'PRINT',
  TRUE = 'TRUE',
  FALSE = 'FALSE',

  // Operators
  PLUS = 'PLUS',
  MINUS = 'MINUS',
  STAR = 'STAR',
  SLASH = 'SLASH',
  PERCENT = 'PERCENT',
  EQUAL_EQUAL = 'EQUAL_EQUAL',
  BANG_EQUAL = 'BANG_EQUAL',
  LESS = 'LESS',
  LESS_EQUAL = 'LESS_EQUAL',
  GREATER = 'GREATER',
  GREATER_EQUAL = 'GREATER_EQUAL',
  EQUAL = 'EQUAL',

  // Punctuation
  LEFT_PAREN = 'LEFT_PAREN',
  RIGHT_PAREN = 'RIGHT_PAREN',
  LEFT_BRACE = 'LEFT_BRACE',
  RIGHT_BRACE = 'RIGHT_BRACE',
  COMMA = 'COMMA',
  SEMICOLON = 'SEMICOLON',

  // Special
  EOF = 'EOF'
}

/**
 * Represents a single token in the source code
 */
export interface Token {
  type: TokenType;
  lexeme: string;
  literal: any;
  line: number;
  column: number;
}

/**
 * Create a new token
 */
export function createToken(
  type: TokenType,
  lexeme: string,
  literal: any,
  line: number,
  column: number
): Token {
  return { type, lexeme, literal, line, column };
}