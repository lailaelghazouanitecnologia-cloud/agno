/**
 * Lexer (Tokenizer) for C source code
 * Converts source code into a stream of tokens
 */

import { TokenType, Token, CompilerError } from "./interfaces.js";

export class Lexer {
  private source: string;
  private tokens: Token[] = [];
  private start: number = 0;
  private current: number = 0;
  private line: number = 1;
  private column: number = 1;

  // Keywords mapping
  private readonly keywords: Map<string, TokenType> = new Map([
    ["int", TokenType.INT],
    ["char", TokenType.CHAR],
    ["float", TokenType.FLOAT],
    ["void", TokenType.VOID],
    ["if", TokenType.IF],
    ["else", TokenType.ELSE],
    ["while", TokenType.WHILE],
    ["for", TokenType.FOR],
    ["return", TokenType.RETURN]
  ]);

  constructor(source: string) {
    this.source = source;
  }

  /**
   * Tokenize the entire source code
   */
  public tokenize(): Token[] {
    while (!this.isAtEnd()) {
      this.start = this.current;
      this.scanToken();
    }

    // Add EOF token
    this.tokens.push({
      type: TokenType.EOF,
      value: "",
      line: this.line,
      column: this.column
    });

    return this.tokens;
  }

  private isAtEnd(): boolean {
    return this.current >= this.source.length;
  }

  private scanToken(): void {
    const char = this.advance();

    switch (char) {
      // Whitespace
      case " ":
      case "\t":
      case "\r":
        break;

      // Newlines
      case "\n":
        this.line++;
        this.column = 1;
        break;

      // Single-character tokens
      case "(":
        this.addToken(TokenType.LPAREN);
        break;
      case ")":
        this.addToken(TokenType.RPAREN);
        break;
      case "{":
        this.addToken(TokenType.LBRACE);
        break;
      case "}":
        this.addToken(TokenType.RBRACE);
        break;
      case "[":
        this.addToken(TokenType.LBRACKET);
        break;
      case "]":
        this.addToken(TokenType.RBRACKET);
        break;
      case ";":
        this.addToken(TokenType.SEMICOLON);
        break;
      case ",":
        this.addToken(TokenType.COMMA);
        break;

      // Operators
      case "+":
        this.addToken(TokenType.PLUS);
        break;
      case "-":
        this.addToken(TokenType.MINUS);
        break;
      case "*":
        this.addToken(TokenType.STAR);
        break;
      case "%":
        this.addToken(TokenType.PERCENT);
        break;

      // Possibly multi-character operators
      case "/":
        if (this.match("/")) {
          // Single-line comment - skip until newline
          while (this.peek() !== "\n" && !this.isAtEnd()) {
            this.advance();
          }
        } else if (this.match("*")) {
          // Multi-line comment
          this.blockComment();
        } else {
          this.addToken(TokenType.SLASH);
        }
        break;

      case "!":
        if (this.match("=")) {
          this.addToken(TokenType.NOT_EQUAL);
        } else {
          throw new CompilerError(`Unexpected character '!'`, this.line, this.column);
        }
        break;

      case "=":
        if (this.match("=")) {
          this.addToken(TokenType.EQUAL);
        } else {
          this.addToken(TokenType.ASSIGN);
        }
        break;

      case "<":
        if (this.match("=")) {
          this.addToken(TokenType.LESS_EQUAL);
        } else {
          this.addToken(TokenType.LESS);
        }
        break;

      case ">":
        if (this.match("=")) {
          this.addToken(TokenType.GREATER_EQUAL);
        } else {
          this.addToken(TokenType.GREATER);
        }
        break;

      case "&":
        this.addToken(TokenType.AMPERSAND);
        break;

      // String literals
      case '"':
        this.string();
        break;

      // Digit or identifier
      default:
        if (this.isDigit(char)) {
          this.number();
        } else if (this.isAlpha(char)) {
          this.identifier();
        } else {
          throw new CompilerError(
            `Unexpected character '${char}'`,
            this.line,
            this.column
          );
        }
        break;
    }
  }

  private advance(): string {
    const char = this.source[this.current++];
    this.column++;
    return char;
  }

  private peek(): string {
    if (this.isAtEnd()) return "\0";
    return this.source[this.current];
  }

  private peekNext(): string {
    if (this.current + 1 >= this.source.length) return "\0";
    return this.source[this.current + 1];
  }

  private match(expected: string): boolean {
    if (this.isAtEnd()) return false;
    if (this.source[this.current] !== expected) return false;

    this.current++;
    this.column++;
    return true;
  }

  private addToken(type: TokenType, value?: string): void {
    const text = this.source.substring(this.start, this.current);
    this.tokens.push({
      type,
      value: value || text,
      line: this.line,
      column: this.column - (this.current - this.start)
    });
  }

  private string(): void {
    while (this.peek() !== '"' && !this.isAtEnd()) {
      if (this.peek() === "\n") {
        this.line++;
        this.column = 1;
      }
      this.advance();
    }

    if (this.isAtEnd()) {
      throw new CompilerError("Unterminated string", this.line, this.column);
    }

    // Closing quote
    this.advance();

    // Get string value (without quotes)
    const value = this.source.substring(this.start + 1, this.current - 1);
    this.addToken(TokenType.STRING, value);
  }

  private number(): void {
    while (this.isDigit(this.peek())) {
      this.advance();
    }

    // Check for decimal point
    if (this.peek() === "." && this.isDigit(this.peekNext())) {
      this.advance(); // Consume decimal point

      while (this.isDigit(this.peek())) {
        this.advance();
      }
    }

    const value = this.source.substring(this.start, this.current);
    this.addToken(TokenType.NUMBER, value);
  }

  private identifier(): void {
    while (this.isAlphaNumeric(this.peek())) {
      this.advance();
    }

    const text = this.source.substring(this.start, this.current);

    // Check if it's a keyword
    const type = this.keywords.get(text);
    if (type !== undefined) {
      this.addToken(type);
    } else {
      this.addToken(TokenType.IDENTIFIER, text);
    }
  }

  private blockComment(): void {
    while (!this.isAtEnd() && !(this.peek() === "*" && this.peekNext() === "/")) {
      if (this.peek() === "\n") {
        this.line++;
        this.column = 1;
      }
      this.advance();
    }

    if (this.isAtEnd()) {
      throw new CompilerError("Unterminated block comment", this.line, this.column);
    }

    // Consume */
    this.advance();
    this.advance();
  }

  private isDigit(char: string): boolean {
    return char >= "0" && char <= "9";
  }

  private isAlpha(char: string): boolean {
    return (char >= "a" && char <= "z") ||
           (char >= "A" && char <= "Z") ||
           char === "_";
  }

  private isAlphaNumeric(char: string): boolean {
    return this.isAlpha(char) || this.isDigit(char);
  }
}