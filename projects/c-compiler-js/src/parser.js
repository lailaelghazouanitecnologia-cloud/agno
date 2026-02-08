/**
 * Parser for C source code
 * Builds an Abstract Syntax Tree (AST) using recursive descent parsing
 */

class Parser {
  constructor(tokens) {
    this.tokens = tokens;
    this.pos = 0;
  }

  /**
   * Parse the entire program
   */
  parse() {
    const program = {
      type: 'Program',
      declarations: []
    };

    while (!this.isAtEnd()) {
      const decl = this.parseDeclaration();
      if (decl) {
        program.declarations.push(decl);
      }
    }

    return program;
  }

  parseDeclaration() {
    // Try to parse a function declaration
    if (this.check('INT') || this.check('CHAR') || this.check('VOID')) {
      const type = this.advance();
      
      if (this.check('IDENTIFIER')) {
        const name = this.advance();
        
        // Function declaration
        if (this.check('LPAREN')) {
          return this.parseFunctionDeclaration(type, name);
        }
        
        // Variable declaration
        else if (this.check('SEMICOLON') || this.check('ASSIGN') || this.check('LBRACKET')) {
          return this.parseVariableDeclaration(type, name);
        }
      }
    }

    throw this.error('Expected declaration');
  }

  parseFunctionDeclaration(returnType, name) {
    this.consume('LPAREN', 'Expected \'(\' after function name');
    
    const params = [];
    if (!this.check('RPAREN')) {
      do {
        const paramType = this.parseType();
        const paramName = this.consume('IDENTIFIER', 'Expected parameter name');
        params.push({
          type: paramType.type,
          name: paramName.value
        });
      } while (this.match('COMMA'));
    }
    
    this.consume('RPAREN', 'Expected \')\' after parameters');
    
    const body = this.parseCompoundStatement();
    
    return {
      type: 'FunctionDeclaration',
      returnType: returnType.type,
      name: name.value,
      params: params,
      body: body
    };
  }

  parseVariableDeclaration(varType, name) {
    let isArray = false;
    let arraySize = null;
    
    // Check for array declaration
    if (this.match('LBRACKET')) {
      isArray = true;
      if (!this.check('RBRACKET')) {
        const size = this.parseExpression();
        arraySize = size;
      }
      this.consume('RBRACKET', 'Expected \']\' after array size');
    }
    
    let init = null;
    if (this.match('ASSIGN')) {
      init = this.parseExpression();
    }
    
    this.consume('SEMICOLON', 'Expected \';\' after variable declaration');
    
    return {
      type: 'VariableDeclaration',
      varType: varType.type,
      name: name.value,
      isArray: isArray,
      arraySize: arraySize,
      init: init
    };
  }

  parseType() {
    if (this.check('INT') || this.check('CHAR') || this.check('VOID')) {
      return this.advance();
    }
    throw this.error('Expected type');
  }

  parseCompoundStatement() {
    this.consume('LBRACE', 'Expected \'{\'');
    
    const statements = [];
    while (!this.check('RBRACE') && !this.isAtEnd()) {
      const stmt = this.parseStatement();
      if (stmt) {
        statements.push(stmt);
      }
    }
    
    this.consume('RBRACE', 'Expected \'}\'');
    
    return {
      type: 'CompoundStatement',
      statements: statements
    };
  }

  parseStatement() {
    // Variable declaration
    if (this.check('INT') || this.check('CHAR')) {
      const type = this.advance();
      if (this.check('IDENTIFIER')) {
        const name = this.advance();
        return this.parseVariableDeclaration(type, name);
      }
    }
    
    // If statement
    if (this.check('IF')) {
      return this.parseIfStatement();
    }
    
    // While statement
    if (this.check('WHILE')) {
      return this.parseWhileStatement();
    }
    
    // For statement
    if (this.check('FOR')) {
      return this.parseForStatement();
    }
    
    // Return statement
    if (this.check('RETURN')) {
      return this.parseReturnStatement();
    }
    
    // Compound statement
    if (this.check('LBRACE')) {
      return this.parseCompoundStatement();
    }
    
    // Expression statement
    return this.parseExpressionStatement();
  }

  parseIfStatement() {
    this.consume('IF', 'Expected \'if\'');
    this.consume('LPAREN', 'Expected \'(\' after if');
    const condition = this.parseExpression();
    this.consume('RPAREN', 'Expected \')\' after condition');
    
    const thenBranch = this.parseStatement();
    let elseBranch = null;
    
    if (this.match('ELSE')) {
      elseBranch = this.parseStatement();
    }
    
    return {
      type: 'IfStatement',
      condition: condition,
      thenBranch: thenBranch,
      elseBranch: elseBranch
    };
  }

  parseWhileStatement() {
    this.consume('WHILE', 'Expected \'while\'');
    this.consume('LPAREN', 'Expected \'(\' after while');
    const condition = this.parseExpression();
    this.consume('RPAREN', 'Expected \')\' after condition');
    
    const body = this.parseStatement();
    
    return {
      type: 'WhileStatement',
      condition: condition,
      body: body
    };
  }

  parseForStatement() {
    this.consume('FOR', 'Expected \'for\'');
    this.consume('LPAREN', 'Expected \'(\' after for');
    
    let init = null;
    if (!this.check('SEMICOLON')) {
      if (this.check('INT') || this.check('CHAR')) {
        const type = this.advance();
        if (this.check('IDENTIFIER')) {
          const name = this.advance();
          init = this.parseVariableDeclaration(type, name);
        }
      } else {
        init = this.parseExpression();
        this.consume('SEMICOLON', 'Expected \';\' after for init');
      }
    } else {
      this.advance();
    }
    
    let condition = null;
    if (!this.check('SEMICOLON')) {
      condition = this.parseExpression();
    }
    this.consume('SEMICOLON', 'Expected \';\' after for condition');
    
    let update = null;
    if (!this.check('RPAREN')) {
      update = this.parseExpression();
    }
    this.consume('RPAREN', 'Expected \')\' after for clauses');
    
    const body = this.parseStatement();
    
    return {
      type: 'ForStatement',
      init: init,
      condition: condition,
      update: update,
      body: body
    };
  }

  parseReturnStatement() {
    this.consume('RETURN', 'Expected \'return\'');
    
    let value = null;
    if (!this.check('SEMICOLON')) {
      value = this.parseExpression();
    }
    
    this.consume('SEMICOLON', 'Expected \';\' after return');
    
    return {
      type: 'ReturnStatement',
      value: value
    };
  }

  parseExpressionStatement() {
    const expr = this.parseExpression();
    this.consume('SEMICOLON', 'Expected \';\' after expression');
    
    return {
      type: 'ExpressionStatement',
      expression: expr
    };
  }

  parseExpression() {
    return this.parseAssignment();
  }

  parseAssignment() {
    const expr = this.parseLogicalOr();
    
    if (this.match('ASSIGN')) {
      const value = this.parseAssignment();
      return {
        type: 'Assignment',
        target: expr,
        value: value
      };
    }
    
    return expr;
  }

  parseLogicalOr() {
    let expr = this.parseLogicalAnd();
    
    while (this.match('OR')) {
      const operator = this.tokens[this.pos - 1].value;
      const right = this.parseLogicalAnd();
      expr = {
        type: 'Binary',
        operator: operator,
        left: expr,
        right: right
      };
    }
    
    return expr;
  }

  parseLogicalAnd() {
    let expr = this.parseEquality();
    
    while (this.match('AND')) {
      const operator = this.tokens[this.pos - 1].value;
      const right = this.parseEquality();
      expr = {
        type: 'Binary',
        operator: operator,
        left: expr,
        right: right
      };
    }
    
    return expr;
  }

  parseEquality() {
    let expr = this.parseComparison();
    
    while (this.match('EQ') || this.match('NEQ')) {
      const operator = this.tokens[this.pos - 1].value;
      const right = this.parseComparison();
      expr = {
        type: 'Binary',
        operator: operator,
        left: expr,
        right: right
      };
    }
    
    return expr;
  }

  parseComparison() {
    let expr = this.parseShift();
    
    while (this.match('LT') || this.match('GT') || this.match('LTE') || this.match('GTE')) {
      const operator = this.tokens[this.pos - 1].value;
      const right = this.parseShift();
      expr = {
        type: 'Binary',
        operator: operator,
        left: expr,
        right: right
      };
    }
    
    return expr;
  }

  parseShift() {
    let expr = this.parseAdditive();
    
    while (this.match('LSHIFT') || this.match('RSHIFT')) {
      const operator = this.tokens[this.pos - 1].value;
      const right = this.parseAdditive();
      expr = {
        type: 'Binary',
        operator: operator,
        left: expr,
        right: right
      };
    }
    
    return expr;
  }

  parseAdditive() {
    let expr = this.parseMultiplicative();
    
    while (this.match('PLUS') || this.match('MINUS')) {
      const operator = this.tokens[this.pos - 1].value;
      const right = this.parseMultiplicative();
      expr = {
        type: 'Binary',
        operator: operator,
        left: expr,
        right: right
      };
    }
    
    return expr;
  }

  parseMultiplicative() {
    let expr = this.parseUnary();
    
    while (this.match('STAR') || this.match('SLASH') || this.match('PERCENT')) {
      const operator = this.tokens[this.pos - 1].value;
      const right = this.parseUnary();
      expr = {
        type: 'Binary',
        operator: operator,
        left: expr,
        right: right
      };
    }
    
    return expr;
  }

  parseUnary() {
    if (this.match('BANG') || this.match('MINUS') || this.match('AMP') || this.match('STAR') || this.match('INC') || this.match('DEC')) {
      const operator = this.tokens[this.pos - 1].value;
      const operand = this.parseUnary();
      return {
        type: 'Unary',
        operator: operator,
        operand: operand
      };
    }
    
    return this.parsePostfix();
  }

  parsePostfix() {
    let expr = this.parsePrimary();
    
    while (true) {
      // Array access
      if (this.match('LBRACKET')) {
        const index = this.parseExpression();
        this.consume('RBRACKET', 'Expected \']\' after array index');
        expr = {
          type: 'ArrayAccess',
          array: expr,
          index: index
        };
      }
      // Function call
      else if (this.match('LPAREN')) {
        const args = [];
        if (!this.check('RPAREN')) {
          do {
            args.push(this.parseExpression());
          } while (this.match('COMMA'));
        }
        this.consume('RPAREN', 'Expected \')\' after arguments');
        expr = {
          type: 'Call',
          callee: expr,
          arguments: args
        };
      }
      // Postfix increment/decrement
      else if (this.match('INC') || this.match('DEC')) {
        const operator = this.tokens[this.pos - 1].value;
        expr = {
          type: 'Postfix',
          operator: operator,
          operand: expr
        };
      }
      else {
        break;
      }
    }
    
    return expr;
  }

  parsePrimary() {
    // Number literal
    if (this.match('NUMBER')) {
      return {
        type: 'Number',
        value: this.tokens[this.pos - 1].value
      };
    }
    
    // String literal
    if (this.match('STRING')) {
      return {
        type: 'String',
        value: this.tokens[this.pos - 1].value
      };
    }
    
    // Character literal
    if (this.match('CHAR')) {
      return {
        type: 'Char',
        value: this.tokens[this.pos - 1].value
      };
    }
    
    // Identifier
    if (this.match('IDENTIFIER')) {
      return {
        type: 'Identifier',
        name: this.tokens[this.pos - 1].value
      };
    }
    
    // Parenthesized expression
    if (this.match('LPAREN')) {
      const expr = this.parseExpression();
      this.consume('RPAREN', 'Expected \')\' after expression');
      return expr;
    }
    
    throw this.error('Expected expression');
  }

  // Helper methods

  advance() {
    if (!this.isAtEnd()) {
      this.pos++;
    }
    return this.tokens[this.pos - 1];
  }

  match(type) {
    if (this.check(type)) {
      this.advance();
      return true;
    }
    return false;
  }

  check(type) {
    if (this.isAtEnd()) return false;
    return this.tokens[this.pos].type === type;
  }

  consume(type, message) {
    if (this.check(type)) {
      return this.advance();
    }
    throw this.error(message);
  }

  isAtEnd() {
    return this.tokens[this.pos].type === 'EOF';
  }

  error(message) {
    const token = this.tokens[this.pos];
    const line = token.line;
    const col = token.col;
    return new Error(`Parse error at line ${line}, col ${col}: ${message}`);
  }
}

module.exports = Parser;