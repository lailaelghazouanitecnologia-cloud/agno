import { TokenType, Token } from "../interfaces/index.js";

/**
 * Lexer (Tokenizer) for C source code
 * Converts source code into a stream of tokens
 */
export class Lexer {
  private source: string;
  private position: number = 0;
  private line: number = 1;
  private column: number = 1;
  private tokens: Token[] = [];
  
  // Keyword mapping
  private static readonly KEYWORDS: Map<string, TokenType> = new Map([
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
  
  // Multi-character operators
  private static readonly OPERATORS: Map<string, TokenType> = new Map([
    ["<=", TokenType.LESS_EQUAL],
    [">=", TokenType.GREATER_EQUAL],
    ["==", TokenType.EQUAL],
    ["!=", TokenType.NOT_EQUAL]
  ]);
  
  constructor(source: string) {
    this.source = source;
  }
  
  /**
   * Tokenize the entire source code
   */
  public tokenize(): Token[] {
    this.tokens = [];
    this.position = 0;
    this.line = 1;
    this.column = 1;
    
    while (!this.isAtEnd()) {
      const token = this.scanToken();
      if (token) {
        this.tokens.push(token);
      }
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
  
  /**
   * Scan a single token
   */
  private scanToken(): Token | null {
    const startLine = this.line;
    const startColumn = this.column;
    
    const char = this.advance();
    
    // Skip whitespace
    if (this.isWhitespace(char)) {
      return null;
    }
    
    // Skip comments
    if (char === '/' && this.peek() === '/') {
      this.skipLineComment();
      return null;
    }
    
    if (char === '/' && this.peek() === '*') {
      this.skipBlockComment();
      return null;
    }
    
    // Newlines
    if (char === '\n') {
      this.line++;
      this.column = 1;
      return null;
    }
    
    // String literals
    if (char === '"') {
      return this.readStringLiteral(startLine, startColumn);
    }
    
    // Character literals
    if (char === "'") {
      return this.readCharLiteral(startLine, startColumn);
    }
    
    // Numbers
    if (this.isDigit(char)) {
      return this.readNumber(char, startLine, startColumn);
    }
    
    // Identifiers and keywords
    if (this.isAlpha(char)) {
      return this.readIdentifier(char, startLine, startColumn);
    }
    
    // Operators and punctuation
    switch (char) {
      case '+':
        return this.createToken(TokenType.PLUS, "+", startLine, startColumn);
      case '-':
        return this.createToken(TokenType.MINUS, "-", startLine, startColumn);
      case '*':
        return this.createToken(TokenType.STAR, "*", startLine, startColumn);
      case '/':
        return this.createToken(TokenType.SLASH, "/", startLine, startColumn);
      case '%':
        return this.createToken(TokenType.PERCENT, "%", startLine, startColumn);
      case '<':
        if (this.match('=')) {
          return this.createToken(TokenType.LESS_EQUAL, "<=", startLine, startColumn);
        }
        return this.createToken(TokenType.LESS, "<", startLine, startColumn);
      case '>':
        if (this.match('=')) {
          return this.createToken(TokenType.GREATER_EQUAL, ">=", startLine, startColumn);
        }
        return this.createToken(TokenType.GREATER, ">", startLine, startColumn);
      case '=':
        if (this.match('=')) {
          return this.createToken(TokenType.EQUAL, "==", startLine, startColumn);
        }
        return this.createToken(TokenType.ASSIGN, "=", startLine, startColumn);
      case '!':
        if (this.match('=')) {
          return this.createToken(TokenType.NOT_EQUAL, "!=", startLine, startColumn);
        }
        break;
      case '&':
        return this.createToken(TokenType.AMPERSAND, "&", startLine, startColumn);
      case ';':
        return this.createToken(TokenType.SEMICOLON, ";", startLine, startColumn);
      case ',':
        return this.createToken(TokenType.COMMA, ",", startLine, startColumn);
      case '(':
        return this.createToken(TokenType.LPAREN, "(", startLine, startColumn);
      case ')':
        return this.createToken(TokenType.RPAREN, ")", startLine, startColumn);
      case '{':
        return this.createToken(TokenType.LBRACE, "{", startLine, startColumn);
      case '}':
        return this.createToken(TokenType.RBRACE, "}", startLine, startColumn);
      case '[':
        return this.createToken(TokenType.LBRACKET, "[", startLine, startColumn);
      case ']':
        return this.createToken(TokenType.RBRACKET, "]", startLine, startColumn);
    }
    
    // Unknown character
    return this.createToken(TokenType.UNKNOWN, char, startLine, startColumn);
  }
  
  /**
   * Read a string literal
   */
  private readStringLiteral(startLine: number, startColumn: number): Token {
    let value = "";
    
    while (!this.isAtEnd() && this.peek() !== '"') {
      const char = this.advance();
      
      // Handle escape sequences
      if (char === '\\' && !this.isAtEnd()) {
        const next = this.advance();
        switch (next) {
          case 'n': value += '\n'; break;
          case 't': value += '\t'; break;
          case 'r': value += '\r'; break;
          case '\\': value += '\\'; break;
          case '"': value += '"'; break;
          default: value += next;
        }
      } else {
        value += char;
      }
    }
    
    if (this.isAtEnd()) {
      throw new Error(`Unterminated string literal at line ${startLine}`);
    }
    
    this.advance(); // Consume closing quote
    
    return this.createToken(TokenType.STRING_LITERAL, value, startLine, startColumn);
  }
  
  /**
   * Read a character literal
   */
  private readCharLiteral(startLine: number, startColumn: number): Token {
    let value = "";
    
    if (this.isAtEnd() || this.peek() === "'") {
      throw new Error(`Empty character literal at line ${startLine}`);
    }
    
    const char = this.advance();
    
    // Handle escape sequences
    if (char === '\\' && !this.isAtEnd()) {
      const next = this.advance();
      switch (next) {
        case 'n': value = '\n'; break;
        case 't': value = '\t'; break;
        case 'r': value = '\r'; break;
        case '\\': value = '\\'; break;
        case "'": value = "'"; break;
        case '0': value = '\0'; break;
        default: value = next;
      }
    } else {
      value = char;
    }
    
    if (this.isAtEnd() || this.peek() !== "'") {
      throw new Error(`Unterminated character literal at line ${startLine}`);
    }
    
    this.advance(); // Consume closing quote
    
    return this.createToken(TokenType.CHAR_LITERAL, value, startLine, startColumn);
  }
  
  /**
   * Read a number (integer or float)
   */
  private readNumber(first: string, startLine: number, startColumn: number): Token {
    let value = first;
    let isFloat = false;
    
    while (!this.isAtEnd() && (this.isDigit(this.peek()) || this.peek() === '.')) {
      if (this.peek() === '.') {
        if (isFloat) {
          throw new Error(`Invalid number format at line ${startLine}`);
        }
        isFloat = true;
      }
      value += this.advance();
    }
    
    const type = isFloat ? TokenType.FLOAT_LITERAL : TokenType.INTEGER_LITERAL;
    return this.createToken(type, value, startLine, startColumn);
  }
  
  /**
   * Read an identifier or keyword
   */
  private readIdentifier(first: string, startLine: number, startColumn: number): Token {
    let value = first;
    
    while (!this.isAtEnd() && this.isAlphaNumeric(this.peek())) {
      value += this.advance();
    }
    
    // Check if it's a keyword
    const keywordType = Lexer.KEYWORDS.get(value);
    if (keywordType) {
      return this.createToken(keywordType, value, startLine, startColumn);
    }
    
    return this.createToken(TokenType.IDENTIFIER, value, startLine, startColumn);
  }
  
  /**
   * Skip a line comment (// ...)
   */
  private skipLineComment(): void {
    while (!this.isAtEnd() && this.peek() !== '\n') {
      this.advance();
    }
  }
  
  /**
   * Skip a block comment (/* ... *\/)
   */
  private skipBlockComment(): void {
    this.advance(); // Consume *
    
    while (!this.isAtEnd()) {
      if (this.peek() === '*' && this.peekNext() === '/') {
        this.advance(); // Consume *
        this.advance(); // Consume /
        return;
      }
      
      const char = this.advance();
      if (char === '\n') {
        this.line++;
        this.column = 1;
      }
    }
    
    throw new Error(`Unterminated block comment starting at line ${this.line}`);
  }
  
  /**
   * Create a token
   */
  private createToken(type: TokenType, value: string, line: number, column: number): Token {
    return { type, value, line, column };
  }
  
  /**
   * Advance to the next character
   */
  private advance(): string {
    const char = this.source[this.position++];
    this.column++;
    return char;
  }
  
  /**
   * Peek at the current character without consuming it
   */
  private peek(): string {
    return this.source[this.position] || '\0';
  }
  
  /**
   * Peek at the next character
   */
  private peekNext(): string {
    return this.source[this.position + 1] || '\0';
  }
  
  /**
   * Check if the current character matches the expected one
   */
  private match(expected: string): boolean {
    if (this.isAtEnd() || this.source[this.position] !== expected) {
      return false;
    }
    this.advance();
    return true;
  }
  
  /**
   * Check if we're at the end of the source
   */
  private isAtEnd(): boolean {
    return this.position >= this.source.length;
  }
  
  /**
   * Check if a character is whitespace
   */
  private isWhitespace(char: string): boolean {
    return char === ' ' || char === '\t' || char === '\r';
  }
  
  /**
   * Check if a character is a digit
   */
  private isDigit(char: string): boolean {
    return char >= '0' && char <= '9';
  }
  
  /**
   * Check if a character is alphabetic
   */
  private isAlpha(char: string): boolean {
    return (char >= 'a' && char <= 'z') || (char >= 'A' && char <= 'Z') || char === '_';
  }
  
  /**
   * Check if a character is alphanumeric
   */
  private isAlphaNumeric(char: string): boolean {
    return this.isAlpha(char) || this.isDigit(char);
  }
}