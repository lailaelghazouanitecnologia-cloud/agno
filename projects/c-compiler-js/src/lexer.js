/**
 * Lexer/Tokenizer for C source code
 * Converts C source code into a stream of tokens
 */

class Lexer {
  constructor(source) {
    this.source = source;
    this.pos = 0;
    this.line = 1;
    this.col = 1;
    this.tokens = [];
  }

  /**
   * Tokenize the entire source code
   */
  tokenize() {
    while (this.pos < this.source.length) {
      this.skipWhitespace();
      
      if (this.pos >= this.source.length) break;

      const char = this.source[this.pos];

      // Skip comments
      if (char === '/' && this.pos + 1 < this.source.length) {
        const next = this.source[this.pos + 1];
        if (next === '/') {
          this.skipLineComment();
          continue;
        } else if (next === '*') {
          this.skipBlockComment();
          continue;
        }
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
      if (/[0-9]/.test(char)) {
        this.tokens.push(this.readNumber());
        continue;
      }

      // Identifier or keyword
      if (/[a-zA-Z_]/.test(char)) {
        this.tokens.push(this.readIdentifier());
        continue;
      }

      // Operators and punctuation
      const token = this.readOperatorOrPunctuation();
      if (token) {
        this.tokens.push(token);
        continue;
      }

      // Unknown character
      throw new Error(`Unexpected character '${char}' at line ${this.line}, col ${this.col}`);
    }

    // Add EOF token
    this.tokens.push({
      type: 'EOF',
      value: null,
      line: this.line,
      col: this.col
    });

    return this.tokens;
  }

  skipWhitespace() {
    while (this.pos < this.source.length) {
      const char = this.source[this.pos];
      if (char === ' ' || char === '\t' || char === '\r') {
        this.advance();
      } else if (char === '\n') {
        this.line++;
        this.col = 1;
        this.advance();
      } else {
        break;
      }
    }
  }

  skipLineComment() {
    while (this.pos < this.source.length && this.source[this.pos] !== '\n') {
      this.advance();
    }
  }

  skipBlockComment() {
    this.advance(); // '/'
    this.advance(); // '*'
    
    while (this.pos + 1 < this.source.length) {
      if (this.source[this.pos] === '*' && this.source[this.pos + 1] === '/') {
        this.advance();
        this.advance();
        return;
      }
      if (this.source[this.pos] === '\n') {
        this.line++;
        this.col = 1;
      }
      this.advance();
    }
    
    throw new Error('Unterminated block comment');
  }

  readString() {
    const startLine = this.line;
    const startCol = this.col;
    let value = '';
    
    this.advance(); // opening quote
    
    while (this.pos < this.source.length) {
      const char = this.source[this.pos];
      
      if (char === '\\') {
        this.advance();
        if (this.pos < this.source.length) {
          const escaped = this.source[this.pos];
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
        }
      } else if (char === '"') {
        this.advance(); // closing quote
        return {
          type: 'STRING',
          value: value,
          line: startLine,
          col: startCol
        };
      } else {
        value += char;
        this.advance();
      }
    }
    
    throw new Error('Unterminated string literal');
  }

  readChar() {
    const startLine = this.line;
    const startCol = this.col;
    
    this.advance(); // opening quote
    
    if (this.pos >= this.source.length) {
      throw new Error('Unterminated character literal');
    }
    
    let value = this.source[this.pos];
    this.advance();
    
    if (this.pos >= this.source.length || this.source[this.pos] !== "'") {
      throw new Error('Unterminated character literal');
    }
    this.advance(); // closing quote
    
    return {
      type: 'CHAR',
      value: value,
      line: startLine,
      col: startCol
    };
  }

  readNumber() {
    const startLine = this.line;
    const startCol = this.col;
    let value = '';
    
    while (this.pos < this.source.length && /[0-9]/.test(this.source[this.pos])) {
      value += this.source[this.pos];
      this.advance();
    }
    
    return {
      type: 'NUMBER',
      value: parseInt(value, 10),
      line: startLine,
      col: startCol
    };
  }

  readIdentifier() {
    const startLine = this.line;
    const startCol = this.col;
    let value = '';
    
    while (this.pos < this.source.length && /[a-zA-Z0-9_]/.test(this.source[this.pos])) {
      value += this.source[this.pos];
      this.advance();
    }
    
    // Check if it's a keyword
    const keywords = {
      'int': 'INT', 'char': 'CHAR', 'void': 'VOID',
      'if': 'IF', 'else': 'ELSE', 'while': 'WHILE', 'for': 'FOR',
      'return': 'RETURN', 'printf': 'PRINTF',
      'sizeof': 'SIZEOF'
    };
    
    const type = keywords[value] || 'IDENTIFIER';
    
    return {
      type: type,
      value: value,
      line: startLine,
      col: startCol
    };
  }

  readOperatorOrPunctuation() {
    const startLine = this.line;
    const startCol = this.col;
    
    // Multi-character operators
    const twoChar = this.source.substr(this.pos, 2);
    const twoCharMap = {
      '==': 'EQ', '!=': 'NEQ', '<=': 'LTE', '>=': 'GTE',
      '&&': 'AND', '||': 'OR', '++': 'INC', '--': 'DEC',
      '<<': 'LSHIFT', '>>': 'RSHIFT', '->': 'ARROW'
    };
    
    if (twoCharMap[twoChar]) {
      this.advance();
      this.advance();
      return {
        type: twoCharMap[twoChar],
        value: twoChar,
        line: startLine,
        col: startCol
      };
    }
    
    // Single-character operators and punctuation
    const char = this.source[this.pos];
    const singleCharMap = {
      '+': 'PLUS', '-': 'MINUS', '*': 'STAR', '/': 'SLASH', '%': 'PERCENT',
      '=': 'ASSIGN', '<': 'LT', '>': 'GT', '!': 'BANG',
      '&': 'AMP', '|': 'PIPE', '^': 'CARET', '~': 'TILDE',
      '?': 'QUESTION', ':': 'COLON',
      '(': 'LPAREN', ')': 'RPAREN',
      '{': 'LBRACE', '}': 'RBRACE',
      '[': 'LBRACKET', ']': 'RBRACKET',
      ',': 'COMMA', ';': 'SEMICOLON', '.': 'DOT'
    };
    
    if (singleCharMap[char]) {
      this.advance();
      return {
        type: singleCharMap[char],
        value: char,
        line: startLine,
        col: startCol
      };
    }
    
    return null;
  }

  advance() {
    this.pos++;
    this.col++;
  }
}

module.exports = Lexer;