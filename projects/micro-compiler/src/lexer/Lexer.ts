import { TokenType, createToken, Token } from '../types/Token';

/**
 * Keywords in the Volt language
 */
const KEYWORDS: Record<string, TokenType> = {
  'let': TokenType.LET,
  'function': TokenType.FUNCTION,
  'if': TokenType.IF,
  'else': TokenType.ELSE,
  'while': TokenType.WHILE,
  'print': TokenType.PRINT,
  'true': TokenType.TRUE,
  'false': TokenType.FALSE
};

/**
 * Lexer (Tokenizer) for the Volt language
 */
export class Lexer {
  private source: string;
  private tokens: Token[] = [];
  private start: number = 0;
  private current: number = 0;
  private line: number = 1;
  private column: number = 1;

  constructor(source: string) {
    this.source = source;
  }

  /**
   * Tokenize the entire source code
   */
  public tokenize(): Token[] {
    this.tokens = [];
    this.start = 0;
    this.current = 0;
    this.line = 1;
    this.column = 1;

    while (!this.isAtEnd()) {
      this.start = this.current;
      this.scanToken();
    }

    this.tokens.push(
      createToken(TokenType.EOF, '', null, this.line, this.column)
    );

    return this.tokens;
  }

  /**
   * Scan a single token
   */
  private scanToken(): void {
    const char = this.advance();

    switch (char) {
      // Whitespace
      case ' ':
      case '\t':
      case '\r':
        break;

      case '\n':
        this.line++;
        this.column = 0;
        break;

      // Punctuation
      case '(':
        this.addToken(TokenType.LEFT_PAREN);
        break;
      case ')':
        this.addToken(TokenType.RIGHT_PAREN);
        break;
      case '{':
        this.addToken(TokenType.LEFT_BRACE);
        break;
      case '}':
        this.addToken(TokenType.RIGHT_BRACE);
        break;
      case ',':
        this.addToken(TokenType.COMMA);
        break;
      case ';':
        this.addToken(TokenType.SEMICOLON);
        break;

      // Operators (potentially multi-character)
      case '+':
        this.addToken(TokenType.PLUS);
        break;
      case '-':
        this.addToken(TokenType.MINUS);
        break;
      case '*':
        this.addToken(TokenType.STAR);
        break;
      case '/':
        this.addToken(TokenType.SLASH);
        break;
      case '%':
        this.addToken(TokenType.PERCENT);
        break;
      case '=':
        this.addToken(this.match('=') ? TokenType.EQUAL_EQUAL : TokenType.EQUAL);
        break;
      case '!':
        this.addToken(this.match('=') ? TokenType.BANG_EQUAL : TokenType.BANG_EQUAL);
        break;
      case '<':
        this.addToken(this.match('=') ? TokenType.LESS_EQUAL : TokenType.LESS);
        break;
      case '>':
        this.addToken(this.match('=') ? TokenType.GREATER_EQUAL : TokenType.GREATER);
        break;

      // String literals
      case '"':
        this.string();
        break;

      default:
        if (this.isDigit(char)) {
          this.number();
        } else if (this.isAlpha(char)) {
          this.identifier();
        } else {
          throw new Error(
            `Unexpected character '${char}' at line ${this.line}, column ${this.column}`
          );
        }
        break;
    }
  }

  /**
   * Handle string literals
   */
  private string(): void {
    while (this.peek() !== '"' && !this.isAtEnd()) {
      if (this.peek() === '\n') {
        this.line++;
        this.column = 0;
      }
      this.advance();
    }

    if (this.isAtEnd()) {
      throw new Error(`Unterminated string at line ${this.line}`);
    }

    // Closing quote
    this.advance();

    // Get the string value (without quotes)
    const value = this.source.substring(this.start + 1, this.current - 1);
    this.addToken(TokenType.STRING, value);
  }

  /**
   * Handle number literals
   */
  private number(): void {
    while (this.isDigit(this.peek())) {
      this.advance();
    }

    // Handle decimal part
    if (this.peek() === '.' && this.isDigit(this.peekNext())) {
      this.advance(); // Consume decimal point

      while (this.isDigit(this.peek())) {
        this.advance();
      }
    }

    const value = parseFloat(this.source.substring(this.start, this.current));
    this.addToken(TokenType.NUMBER, value);
  }

  /**
   * Handle identifiers and keywords
   */
  private identifier(): void {
    while (this.isAlphaNumeric(this.peek())) {
      this.advance();
    }

    const text = this.source.substring(this.start, this.current);
    const type = KEYWORDS[text];

    if (type) {
      this.addToken(type);
    } else {
      this.addToken(TokenType.IDENTIFIER);
    }
  }

  /**
   * Add a token to the list
   */
  private addToken(type: TokenType, literal: any = null): void {
    const lexeme = this.source.substring(this.start, this.current);
    this.tokens.push(
      createToken(type, lexeme, literal, this.line, this.column - (this.current - this.start))
    );
  }

  /**
   * Check if we're at the end of the source
   */
  private isAtEnd(): boolean {
    return this.current >= this.source.length;
  }

  /**
   * Advance and return the current character
   */
  private advance(): string {
    const char = this.source.charAt(this.current);
    this.current++;
    this.column++;
    return char;
  }

  /**
   * Look at the current character without consuming it
   */
  private peek(): string {
    if (this.isAtEnd()) return '\0';
    return this.source.charAt(this.current);
  }

  /**
   * Look at the next character
   */
  private peekNext(): string {
    if (this.current + 1 >= this.source.length) return '\0';
    return this.source.charAt(this.current + 1);
  }

  /**
   * Check if current character matches expected
   */
  private match(expected: string): boolean {
    if (this.isAtEnd()) return false;
    if (this.source.charAt(this.current) !== expected) return false;

    this.current++;
    this.column++;
    return true;
  }

  /**
   * Check if character is a digit
   */
  private isDigit(char: string): boolean {
    return char >= '0' && char <= '9';
  }

  /**
   * Check if character is alphabetic
   */
  private isAlpha(char: string): boolean {
    return (char >= 'a' && char <= 'z') || (char >= 'A' && char <= 'Z') || char === '_';
  }

  /**
   * Check if character is alphanumeric
   */
  private isAlphaNumeric(char: string): boolean {
    return this.isAlpha(char) || this.isDigit(char);
  }
}