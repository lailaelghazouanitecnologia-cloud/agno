/**
 * Parser - Recursive descent parser that builds AST from tokens
 */

import { Token, TokenType, ASTNode, NodeType, BinaryOp, UnaryOp } from '../core/types.js';

/**
 * Parser class for building AST from tokens
 */
export class Parser {
  private tokens: Token[];
  private position: number;
  private currentToken: Token;

  constructor() {
    this.tokens = [];
    this.position = 0;
    this.currentToken = { type: TokenType.EOF, value: '', line: 0, column: 0 };
  }

  /**
   * Parse tokens into AST
   */
  parse(tokens: Token[]): ASTNode {
    this.tokens = tokens || [];
    this.position = 0;
    this.currentToken = this.tokens[0] || { type: TokenType.EOF, value: '', line: 0, column: 0 };

    const program: ASTNode = {
      type: 'Program',
      line: 0,
      column: 0,
      declarations: [],
    };

    while (this.currentToken.type !== TokenType.EOF) {
      // Skip semicolons between declarations
      if (this.currentToken.type === TokenType.SEMICOLON) {
        this.advance();
        continue;
      }

      // Parse function or declaration
      if (this.isType(this.currentToken)) {
        const node = this.parseDeclaration();
        program.declarations!.push(node);
      } else {
        throw this.error('Expected type or function declaration');
      }
    }

    return program;
  }

  /**
   * Parse a declaration (variable or function)
   */
  private parseDeclaration(): ASTNode {
    const type = this.currentToken;
    this.advance();

    const name = this.currentToken;
    if (name.type !== TokenType.IDENTIFIER) {
      throw this.error('Expected identifier after type');
    }
    this.advance();

    // Check if this is a function definition
    if (this.currentToken.type === TokenType.LEFT_PAREN) {
      return this.parseFunctionDefinition(type, name.value);
    }

    // Variable declaration
    const node: ASTNode = {
      type: 'Declaration',
      line: type.line,
      column: type.column,
      varType: type.value,
      name: name.value,
      init: null,
    };

    // Check for initialization
    if (this.currentToken.type === TokenType.ASSIGN) {
      this.advance();
      node.init = this.parseExpression();
    }

    // Expect semicolon
    if (this.currentToken.type !== TokenType.SEMICOLON) {
      throw this.error('Expected semicolon after declaration');
    }
    this.advance();

    return node;
  }

  /**
   * Parse a function definition
   */
  private parseFunctionDefinition(returnType: Token, name: string): ASTNode {
    this.advance(); // '('

    const params: ASTNode[] = [];

    // Parse parameters
    if (this.currentToken.type !== TokenType.RIGHT_PAREN) {
      params.push(this.parseParameter());
      
      while (this.currentToken.type === TokenType.COMMA) {
        this.advance();
        params.push(this.parseParameter());
      }
    }

    if (this.currentToken.type !== TokenType.RIGHT_PAREN) {
      throw this.error('Expected ) after parameters');
    }
    this.advance();

    // Parse function body
    const body = this.parseBlock();

    return {
      type: 'Function',
      line: returnType.line,
      column: returnType.column,
      name,
      returnType: returnType.value,
      parameters: params,
      body,
    };
  }

  /**
   * Parse a function parameter
   */
  private parseParameter(): ASTNode {
    const type = this.currentToken;
    if (!this.isType(type)) {
      throw this.error('Expected type in parameter');
    }
    this.advance();

    const name = this.currentToken;
    if (name.type !== TokenType.IDENTIFIER) {
      throw this.error('Expected identifier in parameter');
    }
    this.advance();

    return {
      type: 'Parameter',
      line: type.line,
      column: type.column,
      varType: type.value,
      name: name.value,
    };
  }

  /**
   * Parse a block statement
   */
  private parseBlock(): ASTNode {
    if (this.currentToken.type !== TokenType.LEFT_BRACE) {
      throw this.error('Expected {');
    }
    this.advance();

    const statements: ASTNode[] = [];

    while (this.currentToken.type !== TokenType.RIGHT_BRACE) {
      if (this.currentToken.type === TokenType.EOF) {
        throw this.error('Unterminated block');
      }

      // Skip semicolons between statements
      if (this.currentToken.type === TokenType.SEMICOLON) {
        this.advance();
        continue;
      }

      statements.push(this.parseStatement());
    }

    this.advance(); // '}'

    return {
      type: 'Block',
      line: 0,
      column: 0,
      statements,
    };
  }

  /**
   * Parse a statement
   */
  private parseStatement(): ASTNode {
    // Return statement
    if (this.currentToken.type === TokenType.RETURN) {
      return this.parseReturn();
    }

    // If statement
    if (this.currentToken.type === TokenType.IF) {
      return this.parseIf();
    }

    // While statement
    if (this.currentToken.type === TokenType.WHILE) {
      return this.parseWhile();
    }

    // For statement
    if (this.currentToken.type === TokenType.FOR) {
      return this.parseFor();
    }

    // Variable declaration
    if (this.isType(this.currentToken)) {
      return this.parseDeclaration();
    }

    // Expression statement
    const expr = this.parseExpression();

    // Expect semicolon
    if (this.currentToken.type !== TokenType.SEMICOLON) {
      throw this.error('Expected semicolon after expression');
    }
    this.advance();

    return {
      type: 'ExpressionStatement',
      line: expr.line,
      column: expr.column,
      expression: expr,
    };
  }

  /**
   * Parse a return statement
   */
  private parseReturn(): ASTNode {
    const line = this.currentToken.line;
    const column = this.currentToken.column;
    this.advance();

    let value: ASTNode | null = null;
    if (this.currentToken.type !== TokenType.SEMICOLON) {
      value = this.parseExpression();
    }

    if (this.currentToken.type !== TokenType.SEMICOLON) {
      throw this.error('Expected semicolon after return');
    }
    this.advance();

    return {
      type: 'ReturnStatement',
      line,
      column,
      value,
    };
  }

  /**
   * Parse an if statement
   */
  private parseIf(): ASTNode {
    const line = this.currentToken.line;
    const column = this.currentToken.column;
    this.advance();

    if (this.currentToken.type !== TokenType.LEFT_PAREN) {
      throw this.error('Expected ( after if');
    }
    this.advance();

    const condition = this.parseExpression();

    if (this.currentToken.type !== TokenType.RIGHT_PAREN) {
      throw this.error('Expected ) after if condition');
    }
    this.advance();

    const thenBranch = this.parseStatement();
    let elseBranch: ASTNode | null = null;

    if (this.currentToken.type === TokenType.ELSE) {
      this.advance();
      elseBranch = this.parseStatement();
    }

    return {
      type: 'IfStatement',
      line,
      column,
      condition,
      thenBranch,
      elseBranch,
    };
  }

  /**
   * Parse a while statement
   */
  private parseWhile(): ASTNode {
    const line = this.currentToken.line;
    const column = this.currentToken.column;
    this.advance();

    if (this.currentToken.type !== TokenType.LEFT_PAREN) {
      throw this.error('Expected ( after while');
    }
    this.advance();

    const condition = this.parseExpression();

    if (this.currentToken.type !== TokenType.RIGHT_PAREN) {
      throw this.error('Expected ) after while condition');
    }
    this.advance();

    const body = this.parseStatement();

    return {
      type: 'WhileStatement',
      line,
      column,
      condition,
      body,
    };
  }

  /**
   * Parse a for statement
   */
  private parseFor(): ASTNode {
    const line = this.currentToken.line;
    const column = this.currentToken.column;
    this.advance();

    if (this.currentToken.type !== TokenType.LEFT_PAREN) {
      throw this.error('Expected ( after for');
    }
    this.advance();

    let init: ASTNode | null = null;
    if (this.currentToken.type !== TokenType.SEMICOLON) {
      if (this.isType(this.currentToken)) {
        init = this.parseDeclaration();
      } else {
        init = this.parseExpression();
        if (this.currentToken.type === TokenType.SEMICOLON) {
          this.advance();
        }
      }
    } else {
      this.advance();
    }

    let condition: ASTNode | null = null;
    if (this.currentToken.type !== TokenType.SEMICOLON) {
      condition = this.parseExpression();
    }
    if (this.currentToken.type !== TokenType.SEMICOLON) {
      throw this.error('Expected ; after for condition');
    }
    this.advance();

    let update: ASTNode | null = null;
    if (this.currentToken.type !== TokenType.RIGHT_PAREN) {
      update = this.parseExpression();
    }

    if (this.currentToken.type !== TokenType.RIGHT_PAREN) {
      throw this.error('Expected ) after for clauses');
    }
    this.advance();

    const body = this.parseStatement();

    return {
      type: 'ForStatement',
      line,
      column,
      init,
      condition,
      update,
      body,
    };
  }

  /**
   * Parse an expression
   */
  private parseExpression(): ASTNode {
    return this.parseAssignment();
  }

  /**
   * Parse an assignment expression
   */
  private parseAssignment(): ASTNode {
    const left = this.parseOr();

    if (this.currentToken.type === TokenType.ASSIGN) {
      const line = this.currentToken.line;
      const column = this.currentToken.column;
      this.advance();
      const right = this.parseAssignment();

      return {
        type: 'Assignment',
        line,
        column,
        left,
        right,
      };
    }

    return left;
  }

  /**
   * Parse logical OR
   */
  private parseOr(): ASTNode {
    let left = this.parseAnd();

    while (this.currentToken.type === TokenType.OR) {
      const line = this.currentToken.line;
      const column = this.currentToken.column;
      this.advance();
      const right = this.parseAnd();

      left = {
        type: 'BinaryOp',
        line,
        column,
        operator: BinaryOp.OR,
        left,
        right,
      };
    }

    return left;
  }

  /**
   * Parse logical AND
   */
  private parseAnd(): ASTNode {
    let left = this.parseEquality();

    while (this.currentToken.type === TokenType.AND) {
      const line = this.currentToken.line;
      const column = this.currentToken.column;
      this.advance();
      const right = this.parseEquality();

      left = {
        type: 'BinaryOp',
        line,
        column,
        operator: BinaryOp.AND,
        left,
        right,
      };
    }

    return left;
  }

  /**
   * Parse equality expressions
   */
  private parseEquality(): ASTNode {
    let left = this.parseComparison();

    while (this.currentToken.type === TokenType.EQUAL || this.currentToken.type === TokenType.NOT_EQUAL) {
      const line = this.currentToken.line;
      const column = this.currentToken.column;
      const operator = this.currentToken.type === TokenType.EQUAL ? BinaryOp.EQUAL : BinaryOp.NOT_EQUAL;
      this.advance();
      const right = this.parseComparison();

      left = {
        type: 'BinaryOp',
        line,
        column,
        operator,
        left,
        right,
      };
    }

    return left;
  }

  /**
   * Parse comparison expressions
   */
  private parseComparison(): ASTNode {
    let left = this.parseTerm();

    while (
      this.currentToken.type === TokenType.LESS ||
      this.currentToken.type === TokenType.LESS_EQUAL ||
      this.currentToken.type === TokenType.GREATER ||
      this.currentToken.type === TokenType.GREATER_EQUAL
    ) {
      const line = this.currentToken.line;
      const column = this.currentToken.column;
      let operator: BinaryOp;
      
      switch (this.currentToken.type) {
        case TokenType.LESS:
          operator = BinaryOp.LESS;
          break;
        case TokenType.LESS_EQUAL:
          operator = BinaryOp.LESS_EQUAL;
          break;
        case TokenType.GREATER:
          operator = BinaryOp.GREATER;
          break;
        case TokenType.GREATER_EQUAL:
          operator = BinaryOp.GREATER_EQUAL;
          break;
        default:
          throw this.error('Unknown comparison operator');
      }
      
      this.advance();
      const right = this.parseTerm();

      left = {
        type: 'BinaryOp',
        line,
        column,
        operator,
        left,
        right,
      };
    }

    return left;
  }

  /**
   * Parse term expressions (+, -)
   */
  private parseTerm(): ASTNode {
    let left = this.parseFactor();

    while (this.currentToken.type === TokenType.PLUS || this.currentToken.type === TokenType.MINUS) {
      const line = this.currentToken.line;
      const column = this.currentToken.column;
      const operator = this.currentToken.type === TokenType.PLUS ? BinaryOp.ADD : BinaryOp.SUBTRACT;
      this.advance();
      const right = this.parseFactor();

      left = {
        type: 'BinaryOp',
        line,
        column,
        operator,
        left,
        right,
      };
    }

    return left;
  }

  /**
   * Parse factor expressions (*, /, %)
   */
  private parseFactor(): ASTNode {
    let left = this.parseUnary();

    while (
      this.currentToken.type === TokenType.MULTIPLY ||
      this.currentToken.type === TokenType.DIVIDE ||
      this.currentToken.type === TokenType.MODULO
    ) {
      const line = this.currentToken.line;
      const column = this.currentToken.column;
      let operator: BinaryOp;
      
      switch (this.currentToken.type) {
        case TokenType.MULTIPLY:
          operator = BinaryOp.MULTIPLY;
          break;
        case TokenType.DIVIDE:
          operator = BinaryOp.DIVIDE;
          break;
        case TokenType.MODULO:
          operator = BinaryOp.MODULO;
          break;
        default:
          throw this.error('Unknown factor operator');
      }
      
      this.advance();
      const right = this.parseUnary();

      left = {
        type: 'BinaryOp',
        line,
        column,
        operator,
        left,
        right,
      };
    }

    return left;
  }

  /**
   * Parse unary expressions
   */
  private parseUnary(): ASTNode {
    if (
      this.currentToken.type === TokenType.PLUS ||
      this.currentToken.type === TokenType.MINUS ||
      this.currentToken.type === TokenType.NOT ||
      this.currentToken.type === TokenType.ADDRESS
    ) {
      const line = this.currentToken.line;
      const column = this.currentToken.column;
      let operator: UnaryOp;
      
      switch (this.currentToken.type) {
        case TokenType.PLUS:
          operator = UnaryOp.PLUS;
          break;
        case TokenType.MINUS:
          operator = UnaryOp.MINUS;
          break;
        case TokenType.NOT:
          operator = UnaryOp.NOT;
          break;
        case TokenType.ADDRESS:
          operator = UnaryOp.ADDRESS;
          break;
        default:
          throw this.error('Unknown unary operator');
      }
      
      this.advance();
      const operand = this.parseUnary();

      return {
        type: 'UnaryOp',
        line,
        column,
        operator,
        operand,
      };
    }

    return this.parsePostfix();
  }

  /**
   * Parse postfix expressions (function calls, array access, dereference)
   */
  private parsePostfix(): ASTNode {
    let left = this.parsePrimary();

    while (true) {
      // Function call
      if (this.currentToken.type === TokenType.LEFT_PAREN) {
        const line = this.currentToken.line;
        const column = this.currentToken.column;
        this.advance();

        const args: ASTNode[] = [];
        if (this.currentToken.type !== TokenType.RIGHT_PAREN) {
          args.push(this.parseExpression());
          
          while (this.currentToken.type === TokenType.COMMA) {
            this.advance();
            args.push(this.parseExpression());
          }
        }

        if (this.currentToken.type !== TokenType.RIGHT_PAREN) {
          throw this.error('Expected ) after function arguments');
        }
        this.advance();

        left = {
          type: 'Call',
          line,
          column,
          callee: left,
          args,
        };
      }
      // Array access
      else if (this.currentToken.type === TokenType.LEFT_BRACKET) {
        const line = this.currentToken.line;
        const column = this.currentToken.column;
        this.advance();

        const index = this.parseExpression();

        if (this.currentToken.type !== TokenType.RIGHT_BRACKET) {
          throw this.error('Expected ] after array index');
        }
        this.advance();

        left = {
          type: 'ArrayAccess',
          line,
          column,
          array: left,
          index,
        };
      }
      // Dereference (pointer)
      else if (this.currentToken.type === TokenType.MULTIPLY) {
        const line = this.currentToken.line;
        const column = this.currentToken.column;
        this.advance();

        left = {
          type: 'Dereference',
          line,
          column,
          pointer: left,
        };
      }
      else {
        break;
      }
    }

    return left;
  }

  /**
   * Parse primary expressions
   */
  private parsePrimary(): ASTNode {
    // Number literal
    if (this.currentToken.type === TokenType.INTEGER || this.currentToken.type === TokenType.FLOAT) {
      const token = this.currentToken;
      this.advance();
      return {
        type: 'Number',
        line: token.line,
        column: token.column,
        value: parseFloat(token.value),
      };
    }

    // String literal
    if (this.currentToken.type === TokenType.STRING) {
      const token = this.currentToken;
      this.advance();
      return {
        type: 'String',
        line: token.line,
        column: token.column,
        value: token.value,
      };
    }

    // Character literal
    if (this.currentToken.type === TokenType.CHARACTER) {
      const token = this.currentToken;
      this.advance();
      return {
        type: 'Char',
        line: token.line,
        column: token.column,
        value: token.value,
      };
    }

    // Identifier
    if (this.currentToken.type === TokenType.IDENTIFIER) {
      const token = this.currentToken;
      this.advance();
      return {
        type: 'Identifier',
        line: token.line,
        column: token.column,
        name: token.value,
      };
    }

    // Parenthesized expression
    if (this.currentToken.type === TokenType.LEFT_PAREN) {
      this.advance();
      const expr = this.parseExpression();
      
      if (this.currentToken.type !== TokenType.RIGHT_PAREN) {
        throw this.error('Expected ) after expression');
      }
      this.advance();
      
      return expr;
    }

    throw this.error('Expected expression');
  }

  /**
   * Check if token is a type
   */
  private isType(token: Token): boolean {
    return (
      token.type === TokenType.INT ||
      token.type === TokenType.CHAR ||
      token.type === TokenType.FLOAT ||
      token.type === TokenType.VOID
    );
  }

  /**
   * Advance to next token
   */
  private advance(): void {
    this.position++;
    if (this.position < this.tokens.length) {
      this.currentToken = this.tokens[this.position];
    } else {
      this.currentToken = { type: TokenType.EOF, value: '', line: 0, column: 0 };
    }
  }

  /**
   * Create an error
   */
  private error(message: string): Error {
    return new Error(
      `${message} at line ${this.currentToken.line}, column ${this.currentToken.column}`
    );
  }
}