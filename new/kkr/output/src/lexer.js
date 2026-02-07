/**
 * Lexer/Tokenizador para C
 * Convierte código fuente C en una secuencia de tokens
 */

class Lexer {
  constructor(source) {
    this.source = source;
    this.pos = 0;
    this.line = 1;
    this.col = 1;
    this.tokens = [];
    
    // Palabras reservadas de C
    this.keywords = new Set([
      'int', 'float', 'char', 'void', 'if', 'else', 
      'while', 'for', 'return', 'struct', 'const',
      'static', 'extern', 'sizeof', 'typedef'
    ]);
    
    // Operadores de múltiples caracteres
    this.multiCharOps = {
      '==': '==', '!=': '!=', '<=': '<=', '>=': '>=',
      '&&': '&&', '||': '||', '++': '++', '--': '--',
      '<<': '<<', '>>': '>>', '+=': '+=', '-=': '-=',
      '*=': '*=', '/=': '/=', '->': '->'
    };
  }

  /**
   * Obtiene el caracter actual
   */
  peek(offset = 0) {
    return this.source[this.pos + offset] || '';
  }

  /**
   * Avanza al siguiente caracter
   */
  advance() {
    const ch = this.source[this.pos++];
    if (ch === '\n') {
      this.line++;
      this.col = 1;
    } else {
      this.col++;
    }
    return ch;
  }

  /**
   * Verifica si estamos al final del archivo
   */
  isEOF() {
    return this.pos >= this.source.length;
  }

  /**
   * Salta espacios en blanco
   */
  skipWhitespace() {
    while (!this.isEOF() && /\s/.test(this.peek())) {
      this.advance();
    }
  }

  /**
   * Procesa comentarios // y /* */
   */
  skipComment() {
    if (this.peek() === '/' && this.peek(1) === '/') {
      // Comentario de línea //
      while (!this.isEOF() && this.peek() !== '\n') {
        this.advance();
      }
      return true;
    } else if (this.peek() === '/' && this.peek(1) === '*') {
      // Comentario de bloque /* */
      this.advance(); // /
      this.advance(); // *
      while (!this.isEOF()) {
        if (this.peek() === '*' && this.peek(1) === '/') {
          this.advance(); // *
          this.advance(); // /
          return true;
        }
        this.advance();
      }
      throw new Error('Unterminated block comment at line ' + this.line);
    }
    return false;
  }

  /**
   * Lee un identificador o palabra clave
   */
  readIdentifier() {
    let start = this.pos;
    while (!this.isEOF() && /[a-zA-Z0-9_]/.test(this.peek())) {
      this.advance();
    }
    const value = this.source.slice(start, this.pos);
    return {
      type: this.keywords.has(value) ? 'KEYWORD' : 'IDENTIFIER',
      value: value,
      line: this.line,
      col: this.col - value.length
    };
  }

  /**
   * Lee un número (entero o flotante)
   */
  readNumber() {
    let start = this.pos;
    let isFloat = false;

    // Parte entera
    while (!this.isEOF() && /[0-9]/.test(this.peek())) {
      this.advance();
    }

    // Parte decimal
    if (this.peek() === '.' && /[0-9]/.test(this.peek(1))) {
      isFloat = true;
      this.advance(); // .
      while (!this.isEOF() && /[0-9]/.test(this.peek())) {
        this.advance();
      }
    }

    // Exponente
    if ((this.peek() === 'e' || this.peek() === 'E') && 
        /[0-9]/.test(this.peek(1))) {
      isFloat = true;
      this.advance(); // e o E
      if (this.peek() === '+' || this.peek() === '-') {
        this.advance();
      }
      while (!this.isEOF() && /[0-9]/.test(this.peek())) {
        this.advance();
      }
    }

    const value = this.source.slice(start, this.pos);
    return {
      type: isFloat ? 'FLOAT_LITERAL' : 'INT_LITERAL',
      value: value,
      line: this.line,
      col: this.col - value.length
    };
  }

  /**
   * Lee un string literal
   */
  readString() {
    this.advance(); // "
    let start = this.pos;
    let value = '';

    while (!this.isEOF() && this.peek() !== '"') {
      if (this.peek() === '\\') {
        // Secuencia de escape
        this.advance();
        const escaped = this.peek();
        switch (escaped) {
          case 'n': value += '\n'; break;
          case 't': value += '\t'; break;
          case 'r': value += '\r'; break;
          case '\\': value += '\\'; break;
          case '"': value += '"'; break;
          case '\'': value += '\''; break;
          default: value += escaped;
        }
        this.advance();
      } else {
        value += this.peek();
        this.advance();
      }
    }

    if (this.isEOF()) {
      throw new Error('Unterminated string literal at line ' + this.line);
    }

    this.advance(); // "
    return {
      type: 'STRING_LITERAL',
      value: value,
      line: this.line,
      col: this.col - value.length - 2
    };
  }

  /**
   * Lee un char literal
   */
  readChar() {
    this.advance(); // '
    let value = '';

    if (this.peek() === '\\') {
      this.advance();
      const escaped = this.peek();
      switch (escaped) {
        case 'n': value = '\n'; break;
        case 't': value = '\t'; break;
        case 'r': value = '\r'; break;
        case '\\': value = '\\'; break;
        case '\'': value = '\''; break;
        case '"': value = '"'; break;
        case '0': value = '\0'; break;
        default: value = escaped;
      }
      this.advance();
    } else {
      value = this.peek();
      this.advance();
    }

    if (this.peek() !== "'") {
      throw new Error('Unterminated char literal at line ' + this.line);
    }
    this.advance(); // '

    return {
      type: 'CHAR_LITERAL',
      value: value,
      line: this.line,
      col: this.col - 3
    };
  }

  /**
   * Lee un operador o puntuación
   */
  readOperator() {
    const ch = this.peek();
    const next = this.peek(1);
    const twoChar = ch + next;

    // Verificar operadores de dos caracteres
    if (this.multiCharOps[twoChar]) {
      this.advance();
      this.advance();
      return {
        type: 'OPERATOR',
        value: twoChar,
        line: this.line,
        col: this.col - 2
      };
    }

    // Operadores de un carácter
    if (['+', '-', '*', '/', '%', '=', '<', '>', '!', '&', '|', '^', '~'].includes(ch)) {
      this.advance();
      return {
        type: 'OPERATOR',
        value: ch,
        line: this.line,
        col: this.col - 1
      };
    }

    // Puntuación
    if (['{', '}', '(', ')', '[', ']', ';', ',', ':', '?'].includes(ch)) {
      this.advance();
      return {
        type: 'PUNCTUATION',
        value: ch,
        line: this.line,
        col: this.col - 1
      };
    }

    throw new Error(`Unknown character '${ch}' at line ${this.line}, col ${this.col}`);
  }

  /**
   * Tokeniza todo el código fuente
   */
  tokenize() {
    while (!this.isEOF()) {
      this.skipWhitespace();

      // Verificar comentarios
      if (this.skipComment()) {
        continue;
      }

      if (this.isEOF()) break;

      const ch = this.peek();

      // Identificador o palabra clave
      if (/[a-zA-Z_]/.test(ch)) {
        this.tokens.push(this.readIdentifier());
        continue;
      }

      // Número
      if (/[0-9]/.test(ch)) {
        this.tokens.push(this.readNumber());
        continue;
      }

      // String literal
      if (ch === '"') {
        this.tokens.push(this.readString());
        continue;
      }

      // Char literal
      if (ch === "'") {
        this.tokens.push(this.readChar());
        continue;
      }

      // Operador o puntuación
      this.tokens.push(this.readOperator());
    }

    // Token EOF
    this.tokens.push({
      type: 'EOF',
      value: '',
      line: this.line,
      col: this.col
    });

    return this.tokens;
  }
}

module.exports = Lexer;