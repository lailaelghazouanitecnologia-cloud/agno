/**
 * Lexer - Tokenizes C source code
 */

import { TokenType, Token } from '../core/types.js';

/**
 * Lexer class for tokenizing C source code
 */
export class Lexer {
  private source: string;
  private position: number;
  private line: number;
  private column: number;
  private tokens: Token[];

  constructor(source: string = '') {
    this.source = source;
    this.position = 0;
    this.line = 1;
    this.column = 1;
    this.tokens = [];
  }

  /**
   * Tokenize the source code
   */
  tokenize(source?: string): Token[] {
    if (source !== undefined) {
      this.source = source;
    }
    this.position = 0;
    this.line = 1;
    this.column = 1;
    this.tokens = [];

    while (this.position < this.source.length) {
      this.skipWhitespace();
      
      if (this.position >= this.source.length) {
        break;
      }

      const char = this.source[this.position];

      // Skip comments
      if (char === '/' && this.peek() === '*') {
        this.skipBlockComment();
        continue;
      }

      if (char === '/' && this.peek() === '/') {
        this.skipLineComment();
        continue;
      }

      // String literal
      if (char === '"') {
        this.tokens.push(this.readString());
        continue;
      }

      // Character literal
      if (char === "'") {
        this.tokens.push(this.readChar());
        continue;
      }

      // Number
      if (this.isDigit(char)) {
        this.tokens.push(this.readNumber());
        continue;
      }

      // Identifier or keyword
      if (this.isAlpha(char) || char === '_') {
        this.tokens.push(this.readIdentifier());
        continue;
      }

      // Operators and punctuation
      this.tokens.push(this.readOperator());
    }

    // Add EOF token
    this.tokens.push({
      type: TokenType.EOF,
      value: '',
      line: this.line,
      column: this.column,
    });

    return this.tokens;
  }

  /**
   * Get current position
   */
  getPosition(): { line: number; column: number } {
    return { line: this.line, column: this.column };
  }

  /**
   * Reset lexer state
   */
  reset(): void {
    this.position = 0;
    this.line = 1;
    this.column = 1;
    this.tokens = [];
  }

  private peek(offset: number = 1): string {
    return this.source[this.position + offset] || '';
  }

  private advance(): string {
    const char = this.source[this.position];
    this.position++;
    if (char === '\n') {
      this.line++;
      this.column = 1;
    } else {
      this.column++;
    }
    return char;
  }

  private skipWhitespace(): void {
    while (this.position < this.source.length) {
      const char = this.source[this.position];
      if (char === ' ' || char === '\t' || char === '\n' || char === '\r') {
        this.advance();
      } else {
        break;
      }
    }
  }

  private skipBlockComment(): void {
    this.advance(); // '/'
    this.advance(); // '*'
    
    while (this.position < this.source.length) {
      if (this.source[this.position] === '*' && this.peek() === '/') {
        this.advance();
        this.advance();
        return;
      }
      this.advance();
    }
  }

  private skipLineComment(): void {
    this.advance(); // '/'
    this.advance(); // '/'
    
    while (this.position < this.source.length) {
      if (this.source[this.position] === '\n') {
        return;
      }
      this.advance();
    }
  }

  private readString(): Token {
    const startLine = this.line;
    const startColumn = this.column;
    this.advance(); // '"'

    let value = '';
    while (this.position < this.source.length) {
      const char = this.source[this.position];
      
      if (char === '\\') {
        this.advance();
        const escaped = this.source[this.position];
        switch (escaped) {
          case 'n': value += '\n'; break;
          case 't': value += '\t'; break;
          case 'r': value += '\r'; break;
          case '\\': value += '\\'; break;
          case '"': value += '"'; break;
          case "'": value += "'"; break;
          default: value += escaped;
        }
        this.advance();
        continue;
      }

      if (char === '"') {
        this.advance();
        return {
          type: TokenType.STRING,
          value,
          line: startLine,
          column: startColumn,
        };
      }

      value += char;
      this.advance();
    }

    throw new Error('Unterminated string literal');
  }

  private readChar(): Token {
    const startLine = this.line;
    const startColumn = this.column;
    this.advance(); // "'"

    let value = '';
    while (this.position < this.source.length) {
      const char = this.source[this.position];
      
      if (char === '\\') {
        this.advance();
        const escaped = this.source[this.position];
        switch (escaped) {
          case 'n': value += '\n'; break;
          case 't': value += '\t'; break;
          case 'r': value += '\r'; break;
          case '\\': value += '\\'; break;
          case '"': value += '"'; break;
          case "'": value += "'"; break;
          case '0': value += '\0'; break;
          default: value += escaped;
        }
        this.advance();
        continue;
      }

      if (char === "'") {
        this.advance();
        return {
          type: TokenType.CHARACTER,
          value,
          line: startLine,
          column: startColumn,
        };
      }

      value += char;
      this.advance();
    }

    throw new Error('Unterminated character literal');
  }

  private readNumber(): Token {
    const startLine = this.line;
    const startColumn = this.column;

    let value = '';
    let isFloat = false;

    while (this.position < this.source.length) {
      const char = this.source[this.position];
      
      if (char === '.' && !isFloat) {
        isFloat = true;
        value += char;
        this.advance();
        continue;
      }

      if (this.isDigit(char)) {
        value += char;
        this.advance();
        continue;
      }

      break;
    }

    return {
      type: isFloat ? TokenType.FLOAT_LITERAL : TokenType.INTEGER,
      value,
      line: startLine,
      column: startColumn,
    };
  }

  private readIdentifier(): Token {
    const startLine = this.line;
    const startColumn = this.column;

    let value = '';
    while (this.position < this.source.length) {
      const char = this.source[this.position];
      
      if (this.isAlphaNumeric(char) || char === '_') {
        value += char;
        this.advance();
        continue;
      }

      break;
    }

    // Check for keywords
    const keyword = this.getKeyword(value);
    if (keyword) {
      return {
        type: keyword,
        value,
        line: startLine,
        column: startColumn,
      };
    }

    return {
      type: TokenType.IDENTIFIER,
      value,
      line: startLine,
      column: startColumn,
    };
  }

  private readOperator(): Token {
    const startLine = this.line;
    const startColumn = this.column;
    const char = this.advance();

    // Multi-character operators
    const twoChar = char + this.peek();
    
    switch (twoChar) {
      case '==': this.advance(); return { type: TokenType.EQUAL, value: '==', line: startLine, column: startColumn };
      case '!=': this.advance(); return { type: TokenType.NOT_EQUAL, value: '!=', line: startLine, column: startColumn };
      case '<=': this.advance(); return { type: TokenType.LESS_EQUAL, value: '<=', line: startLine, column: startColumn };
      case '>=': this.advance(); return { type: TokenType.GREATER_EQUAL, value: '>=', line: startLine, column: startColumn };
      case '&&': this.advance(); return { type: TokenType.AND, value: '&&', line: startLine, column: startColumn };
      case '||': this.advance(); return { type: TokenType.OR, value: '||', line: startLine, column: startColumn };
      case '++': this.advance(); return { type: TokenType.PLUS, value: '++', line: startLine, column: startColumn };
      case '--': this.advance(); return { type: TokenType.MINUS, value: '--', line: startLine, column: startColumn };
      case '<<': this.advance(); return { type: TokenType.LESS, value: '<<', line: startLine, column: startColumn };
      case '>>': this.advance(); return { type: TokenType.GREATER, value: '>>', line: startLine, column: startColumn };
      case '+=': this.advance(); return { type: TokenType.PLUS_ASSIGN, value: '+=', line: startLine, column: startColumn };
      case '-=': this.advance(); return { type: TokenType.MINUS_ASSIGN, value: '-=', line: startLine, column: startColumn };
      case '*=': this.advance(); return { type: TokenType.MULTIPLY_ASSIGN, value: '*=', line: startLine, column: startColumn };
      case '/=': this.advance(); return { type: TokenType.DIVIDE_ASSIGN, value: '/=', line: startLine, column: startColumn };
      case '->': this.advance(); return { type: TokenType.ARROW, value: '->', line: startLine, column: startColumn };
    }

    // Single-character operators
    switch (char) {
      case '+': return { type: TokenType.PLUS, value: '+', line: startLine, column: startColumn };
      case '-': return { type: TokenType.MINUS, value: '-', line: startLine, column: startColumn };
      case '*': return { type: TokenType.MULTIPLY, value: '*', line: startLine, column: startColumn };
      case '/': return { type: TokenType.DIVIDE, value: '/', line: startLine, column: startColumn };
      case '%': return { type: TokenType.MODULO, value: '%', line: startLine, column: startColumn };
      case '<': return { type: TokenType.LESS, value: '<', line: startLine, column: startColumn };
      case '>': return { type: TokenType.GREATER, value: '>', line: startLine, column: startColumn };
      case '=': return { type: TokenType.ASSIGN, value: '=', line: startLine, column: startColumn };
      case '!': return { type: TokenType.NOT, value: '!', line: startLine, column: startColumn };
      case '&': return { type: TokenType.ADDRESS, value: '&', line: startLine, column: startColumn };
      case '|': return { type: TokenType.OR, value: '|', line: startLine, column: startColumn };
      case '^': return { type: TokenType.UNKNOWN, value: '^', line: startLine, column: startColumn };
      case '~': return { type: TokenType.UNKNOWN, value: '~', line: startLine, column: startColumn };
      case '?': return { type: TokenType.UNKNOWN, value: '?', line: startLine, column: startColumn };
      case ':': return { type: TokenType.UNKNOWN, value: ':', line: startLine, column: startColumn };
      case ';': return { type: TokenType.SEMICOLON, value: ';', line: startLine, column: startColumn };
      case ',': return { type: TokenType.COMMA, value: ',', line: startLine, column: startColumn };
      case '.': return { type: TokenType.DOT, value: '.', line: startLine, column: startColumn };
      case '(': return { type: TokenType.LEFT_PAREN, value: '(', line: startLine, column: startColumn };
      case ')': return { type: TokenType.RIGHT_PAREN, value: ')', line: startLine, column: startColumn };
      case '{': return { type: TokenType.LEFT_BRACE, value: '{', line: startLine, column: startColumn };
      case '}': return { type: TokenType.RIGHT_BRACE, value: '}', line: startLine, column: startColumn };
      case '[': return { type: TokenType.LEFT_BRACKET, value: '[', line: startLine, column: startColumn };
      case ']': return { type: TokenType.RIGHT_BRACKET, value: ']', line: startLine, column: startColumn };
      default:
        return { type: TokenType.UNKNOWN, value: char, line: startLine, column: startColumn };
    }
  }

  private getKeyword(value: string): TokenType | undefined {
    const keywords: Record<string, TokenType> = {
      'int': TokenType.INT,
      'char': TokenType.CHAR,
      'float': TokenType.FLOAT,
      'void': TokenType.VOID,
      'if': TokenType.IF,
      'else': TokenType.ELSE,
      'while': TokenType.WHILE,
      'for': TokenType.FOR,
      'return': TokenType.RETURN,
      'break': TokenType.BREAK,
      'continue': TokenType.CONTINUE,
    };

    return keywords[value];
  }

  private isDigit(char: string): boolean {
    return char >= '0' && char <= '9';
  }

  private isAlpha(char: string): boolean {
    return (char >= 'a' && char <= 'z') || (char >= 'A' && char <= 'Z');
  }

  private isAlphaNumeric(char: string): boolean {
    return this.isAlpha(char) || this.isDigit(char);
  }
}