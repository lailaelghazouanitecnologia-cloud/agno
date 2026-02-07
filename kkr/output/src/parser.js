/**
 * Parser para C
 * Construye un AST usando recursive descent parsing
 */

class Parser {
  constructor(tokens) {
    this.tokens = tokens;
    this.pos = 0;
  }

  /**
   * Obtiene el token actual
   */
  peek() {
    return this.tokens[this.pos] || { type: 'EOF', value: '' };
  }

  /**
   * Avanza al siguiente token
   */
  advance() {
    return this.tokens[this.pos++];
  }

  /**
   * Verifica el tipo del token actual
   */
  check(type, value = null) {
    const token = this.peek();
    if (value !== null) {
      return token.type === type && token.value === value;
    }
    return token.type === type;
  }

  /**
   * Consume un token esperado o lanza error
   */
  consume(type, value = null, errorMsg = null) {
    const token = this.peek();
    const matches = value !== null 
      ? token.type === type && token.value === value
      : token.type === type;
    
    if (matches) {
      return this.advance();
    }
    
    const expected = value !== null ? `${type}('${value}')` : type;
    const found = `${token.type}('${token.value}')`;
    throw new Error(errorMsg || `Expected ${expected}, found ${found} at line ${token.line}`);
  }

  /**
   * Verifica si estamos al final
   */
  isEOF() {
    return this.check('EOF');
  }

  /**
   * Punto de entrada principal
   */
  parse() {
    const program = {
      type: 'Program',
      declarations: []
    };

    while (!this.isEOF()) {
      const decl = this.parseDeclaration();
      if (decl) {
        program.declarations.push(decl);
      }
    }

    return program;
  }

  /**
   * Parsea una declaración (variable, función, struct)
   */
  parseDeclaration() {
    // Skip posibles declaraciones forward
    if (this.check('KEYWORD', 'struct')) {
      return this.parseStructDeclaration();
    }

    const type = this.parseType();
    if (!type) {
      throw new Error('Expected type at line ' + this.peek().line);
    }

    const name = this.consume('IDENTIFIER').value;

    // Declaración de función
    if (this.check('PUNCTUATION', '(')) {
      return this.parseFunctionDeclaration(type, name);
    }

    // Declaración de variable
    return this.parseVariableDeclaration(type, name);
  }

  /**
   * Parsea un tipo (incluyendo punteros)
   */
  parseType() {
    if (!this.check('KEYWORD')) {
      return null;
    }

    const baseType = this.advance().value;
    let pointerLevel = 0;

    while (this.check('OPERATOR', '*')) {
      pointerLevel++;
      this.advance();
    }

    return {
      baseType: baseType,
      pointerLevel: pointerLevel
    };
  }

  /**
   * Parsea una declaración de struct
   */
  parseStructDeclaration() {
    this.consume('KEYWORD', 'struct');
    const name = this.consume('IDENTIFIER').value;
    this.consume('PUNCTUATION', '{');

    const fields = [];
    while (!this.check('PUNCTUATION', '}')) {
      const type = this.parseType();
      const fieldName = this.consume('IDENTIFIER').value;
      this.consume('PUNCTUATION', ';');
      fields.push({ type, name: fieldName });
    }

    this.consume('PUNCTUATION', '}');
    this.consume('PUNCTUATION', ';');

    return {
      type: 'StructDeclaration',
      name: name,
      fields: fields
    };
  }

  /**
   * Parsea una declaración de función
   */
  parseFunctionDeclaration(returnType, name) {
    this.consume('PUNCTUATION', '(');

    const params = [];
    if (!this.check('PUNCTUATION', ')')) {
      do {
        const paramType = this.parseType();
        const paramName = this.consume('IDENTIFIER').value;
        params.push({ type: paramType, name: paramName });
      } while (this.check('PUNCTUATION', ',') && this.advance());
    }

    this.consume('PUNCTUATION', ')');

    // Cuerpo de la función
    const body = this.parseCompoundStatement();

    return {
      type: 'FunctionDeclaration',
      name: name,
      returnType: returnType,
      params: params,
      body: body
    };
  }

  /**
   * Parsea una declaración de variable
   */
  parseVariableDeclaration(varType, name) {
    let init = null;

    if (this.check('OPERATOR', '=')) {
      this.advance();
      init = this.parseExpression();
    }

    this.consume('PUNCTUATION', ';');

    return {
      type: 'VariableDeclaration',
      varType: varType,
      name: name,
      init: init
    };
  }

  /**
   * Parsea un bloque de instrucciones
   */
  parseCompoundStatement() {
    this.consume('PUNCTUATION', '{');

    const statements = [];
    while (!this.check('PUNCTUATION', '}')) {
      statements.push(this.parseStatement());
    }

    this.consume('PUNCTUATION', '}');

    return {
      type: 'CompoundStatement',
      statements: statements
    };
  }

  /**
   * Parsea una instrucción
   */
  parseStatement() {
    // if/else
    if (this.check('KEYWORD', 'if')) {
      return this.parseIfStatement();
    }

    // while
    if (this.check('KEYWORD', 'while')) {
      return this.parseWhileStatement();
    }

    // for
    if (this.check('KEYWORD', 'for')) {
      return this.parseForStatement();
    }

    // return
    if (this.check('KEYWORD', 'return')) {
      return this.parseReturnStatement();
    }

    // Bloque
    if (this.check('PUNCTUATION', '{')) {
      return this.parseCompoundStatement();
    }

    // Declaración de variable
    const type = this.parseType();
    if (type) {
      const name = this.consume('IDENTIFIER').value;
      return this.parseVariableDeclaration(type, name);
    }

    // Expresión
    const expr = this.parseExpression();
    this.consume('PUNCTUATION', ';');

    return {
      type: 'ExpressionStatement',
      expression: expr
    };
  }

  /**
   * Parsea una instrucción if/else
   */
  parseIfStatement() {
    this.consume('KEYWORD', 'if');
    this.consume('PUNCTUATION', '(');
    const condition = this.parseExpression();
    this.consume('PUNCTUATION', ')');
    const thenBranch = this.parseStatement();

    let elseBranch = null;
    if (this.check('KEYWORD', 'else')) {
      this.advance();
      elseBranch = this.parseStatement();
    }

    return {
      type: 'IfStatement',
      condition: condition,
      then: thenBranch,
      else: elseBranch
    };
  }

  /**
   * Parsea una instrucción while
   */
  parseWhileStatement() {
    this.consume('KEYWORD', 'while');
    this.consume('PUNCTUATION', '(');
    const condition = this.parseExpression();
    this.consume('PUNCTUATION', ')');
    const body = this.parseStatement();

    return {
      type: 'WhileStatement',
      condition: condition,
      body: body
    };
  }

  /**
   * Parsea una instrucción for
   */
  parseForStatement() {
    this.consume('KEYWORD', 'for');
    this.consume('PUNCTUATION', '(');

    let init = null;
    if (!this.check('PUNCTUATION', ';')) {
      const type = this.parseType();
      if (type) {
        const name = this.consume('IDENTIFIER').value;
        init = this.parseVariableDeclaration(type, name);
      } else {
        init = this.parseExpression();
        this.consume('PUNCTUATION', ';');
      }
    } else {
      this.consume('PUNCTUATION', ';');
    }

    let condition = null;
    if (!this.check('PUNCTUATION', ';')) {
      condition = this.parseExpression();
    }
    this.consume('PUNCTUATION', ';');

    let update = null;
    if (!this.check('PUNCTUATION', ')')) {
      update = this.parseExpression();
    }
    this.consume('PUNCTUATION', ')');

    const body = this.parseStatement();

    return {
      type: 'ForStatement',
      init: init,
      condition: condition,
      update: update,
      body: body
    };
  }

  /**
   * Parsea una instrucción return
   */
  parseReturnStatement() {
    this.consume('KEYWORD', 'return');

    let value = null;
    if (!this.check('PUNCTUATION', ';')) {
      value = this.parseExpression();
    }

    this.consume('PUNCTUATION', ';');

    return {
      type: 'ReturnStatement',
      value: value
    };
  }

  /**
   * Parsea una expresión (con precedencia de operadores)
   */
  parseExpression() {
    return this.parseAssignment();
  }

  /**
   * Asignación (=, +=, -=, etc.)
   */
  parseAssignment() {
    const left = this.parseConditional();

    if (this.check('OPERATOR', '=') || 
        this.check('OPERATOR', '+=') ||
        this.check('OPERATOR', '-=') ||
        this.check('OPERATOR', '*=') ||
        this.check('OPERATOR', '/=')) {
      const op = this.advance().value;
      const right = this.parseAssignment();
      return {
        type: 'AssignmentExpression',
        operator: op,
        left: left,
        right: right
      };
    }

    return left;
  }

  /**
   * Operador condicional (?:)
   */
  parseConditional() {
    const condition = this.parseLogicalOr();

    if (this.check('OPERATOR', '?')) {
      this.advance();
      const thenExpr = this.parseExpression();
      this.consume('OPERATOR', ':');
      const elseExpr = this.parseExpression();
      return {
        type: 'ConditionalExpression',
        condition: condition,
        then: thenExpr,
        else: elseExpr
      };
    }

    return condition;
  }

  /**
   * OR lógico (||)
   */
  parseLogicalOr() {
    let left = this.parseLogicalAnd();

    while (this.check('OPERATOR', '||')) {
      const op = this.advance().value;
      const right = this.parseLogicalAnd();
      left = {
        type: 'BinaryExpression',
        operator: op,
        left: left,
        right: right
      };
    }

    return left;
  }

  /**
   * AND lógico (&&)
   */
  parseLogicalAnd() {
    let left = this.parseEquality();

    while (this.check('OPERATOR', '&&')) {
      const op = this.advance().value;
      const right = this.parseEquality();
      left = {
        type: 'BinaryExpression',
        operator: op,
        left: left,
        right: right
      };
    }

    return left;
  }

  /**
   * Igualdad (==, !=)
   */
  parseEquality() {
    let left = this.parseRelational();

    while (this.check('OPERATOR', '==') || this.check('OPERATOR', '!=')) {
      const op = this.advance().value;
      const right = this.parseRelational();
      left = {
        type: 'BinaryExpression',
        operator: op,
        left: left,
        right: right
      };
    }

    return left;
  }

  /**
   * Relacional (<, >, <=, >=)
   */
  parseRelational() {
    let left = this.parseShift();

    while (this.check('OPERATOR', '<') || this.check('OPERATOR', '>') ||
           this.check('OPERATOR', '<=') || this.check('OPERATOR', '>=')) {
      const op = this.advance().value;
      const right = this.parseShift();
      left = {
        type: 'BinaryExpression',
        operator: op,
        left: left,
        right: right
      };
    }

    return left;
  }

  /**
   * Shift (<<, >>)
   */
  parseShift() {
    let left = this.parseAdditive();

    while (this.check('OPERATOR', '<<') || this.check('OPERATOR', '>>')) {
      const op = this.advance().value;
      const right = this.parseAdditive();
      left = {
        type: 'BinaryExpression',
        operator: op,
        left: left,
        right: right
      };
    }

    return left;
  }

  /**
   * Aditivo (+, -)
   */
  parseAdditive() {
    let left = this.parseMultiplicative();

    while (this.check('OPERATOR', '+') || this.check('OPERATOR', '-')) {
      const op = this.advance().value;
      const right = this.parseMultiplicative();
      left = {
        type: 'BinaryExpression',
        operator: op,
        left: left,
        right: right
      };
    }

    return left;
  }

  /**
   * Multiplicativo (*, /, %)
   */
  parseMultiplicative() {
    let left = this.parseUnary();

    while (this.check('OPERATOR', '*') || this.check('OPERATOR', '/') || 
           this.check('OPERATOR', '%')) {
      const op = this.advance().value;
      const right = this.parseUnary();
      left = {
        type: 'BinaryExpression',
        operator: op,
        left: left,
        right: right
      };
    }

    return left;
  }

  /**
   * Unario (!, -, ++, --, *, &)
   */
  parseUnary() {
    if (this.check('OPERATOR', '!') || this.check('OPERATOR', '-') ||
        this.check('OPERATOR', '++') || this.check('OPERATOR', '--') ||
        this.check('OPERATOR', '*') || this.check('OPERATOR', '&')) {
      const op = this.advance().value;
      const operand = this.parseUnary();
      return {
        type: 'UnaryExpression',
        operator: op,
        operand: operand
      };
    }

    return this.parsePostfix();
  }

  /**
   * Postfix (func call, array access, ++, --)
   */
  parsePostfix() {
    let expr = this.parsePrimary();

    while (true) {
      // Llamada a función
      if (this.check('PUNCTUATION', '(')) {
        this.advance();
        const args = [];
        if (!this.check('PUNCTUATION', ')')) {
          do {
            args.push(this.parseExpression());
          } while (this.check('PUNCTUATION', ',') && this.advance());
        }
        this.consume('PUNCTUATION', ')');
        expr = {
          type: 'CallExpression',
          callee: expr,
          arguments: args
        };
      }
      // Acceso a miembro (.)
      else if (this.check('OPERATOR', '.')) {
        this.advance();
        const property = this.consume('IDENTIFIER').value;
        expr = {
          type: 'MemberExpression',
          object: expr,
          property: property,
          computed: false
        };
      }
      // Acceso a puntero (->)
      else if (this.check('OPERATOR', '->')) {
        this.advance();
        const property = this.consume('IDENTIFIER').value;
        expr = {
          type: 'MemberExpression',
          object: expr,
          property: property,
          computed: false,
          pointer: true
        };
      }
      // Postfix ++/--
      else if (this.check('OPERATOR', '++') || this.check('OPERATOR', '--')) {
        const op = this.advance().value;
        expr = {
          type: 'UnaryExpression',
          operator: op,
          operand: expr,
          postfix: true
        };
      }
      else {
        break;
      }
    }

    return expr;
  }

  /**
   * Expresiones primarias (literales, identificadores, paréntesis)
   */
  parsePrimary() {
    // Paréntesis
    if (this.check('PUNCTUATION', '(')) {
      this.advance();
      const expr = this.parseExpression();
      this.consume('PUNCTUATION', ')');
      return expr;
    }

    // Literal entero
    if (this.check('INT_LITERAL')) {
      const token = this.advance();
      return {
        type: 'IntLiteral',
        value: parseInt(token.value)
      };
    }

    // Literal flotante
    if (this.check('FLOAT_LITERAL')) {
      const token = this.advance();
      return {
        type: 'FloatLiteral',
        value: parseFloat(token.value)
      };
    }

    // String literal
    if (this.check('STRING_LITERAL')) {
      const token = this.advance();
      return {
        type: 'StringLiteral',
        value: token.value
      };
    }

    // Char literal
    if (this.check('CHAR_LITERAL')) {
      const token = this.advance();
      return {
        type: 'CharLiteral',
        value: token.value
      };
    }

    // Identificador
    if (this.check('IDENTIFIER')) {
      const token = this.advance();
      return {
        type: 'Identifier',
        name: token.value
      };
    }

    throw new Error('Unexpected token: ' + this.peek().type + ' at line ' + this.peek().line);
  }
}

module.exports = Parser;