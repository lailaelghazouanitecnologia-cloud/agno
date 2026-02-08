import { Token, TokenType } from '../types/Token';
import {
  ASTNode,
  Program,
  Statement,
  Expression,
  ExpressionStatement,
  VariableDeclaration,
  BlockStatement,
  IfStatement,
  WhileStatement,
  FunctionDeclaration,
  PrintStatement,
  BinaryExpression,
  UnaryExpression,
  LiteralExpression,
  IdentifierExpression,
  CallExpression,
  AssignmentExpression
} from '../types/AST';
import { createValue } from '../types/Value';

/**
 * Parser for the Volt language
 * Builds an Abstract Syntax Tree from tokens
 */
export class Parser {
  private tokens: Token[];
  private current: number = 0;

  constructor(tokens: Token[]) {
    this.tokens = tokens;
  }

  /**
   * Parse the entire token list into an AST
   */
  public parse(): Program {
    const statements: Statement[] = [];

    while (!this.isAtEnd()) {
      const stmt = this.declaration();
      if (stmt) {
        statements.push(stmt);
      }
    }

    return { type: 'Program', statements };
  }

  /**
   * Parse a declaration (variable or function)
   */
  private declaration(): Statement | null {
    try {
      if (this.match(TokenType.FUNCTION)) {
        return this.functionDeclaration();
      }
      if (this.match(TokenType.LET)) {
        return this.variableDeclaration();
      }
      return this.statement();
    } catch (error) {
      this.synchronize();
      return null;
    }
  }

  /**
   * Parse a function declaration
   */
  private functionDeclaration(): FunctionDeclaration {
    const name = this.consume(TokenType.IDENTIFIER, 'Expect function name').lexeme;

    this.consume(TokenType.LEFT_PAREN, "Expect '(' after function name");
    const parameters: string[] = [];

    if (!this.check(TokenType.RIGHT_PAREN)) {
      do {
        const param = this.consume(TokenType.IDENTIFIER, 'Expect parameter name').lexeme;
        parameters.push(param);
      } while (this.match(TokenType.COMMA));
    }

    this.consume(TokenType.RIGHT_PAREN, "Expect ')' after parameters");
    this.consume(TokenType.LEFT_BRACE, "Expect '{' before function body");
    const body = this.block();

    return {
      type: 'FunctionDeclaration',
      name,
      parameters,
      body
    };
  }

  /**
   * Parse a variable declaration
   */
  private variableDeclaration(): VariableDeclaration {
    const name = this.consume(TokenType.IDENTIFIER, 'Expect variable name').lexeme;

    let initializer: Expression | undefined;
    if (this.match(TokenType.EQUAL)) {
      initializer = this.expression();
    }

    this.consume(TokenType.SEMICOLON, "Expect ';' after variable declaration");

    return {
      type: 'VariableDeclaration',
      name,
      initializer
    };
  }

  /**
   * Parse a statement
   */
  private statement(): Statement {
    if (this.match(TokenType.IF)) {
      return this.ifStatement();
    }
    if (this.match(TokenType.WHILE)) {
      return this.whileStatement();
    }
    if (this.match(TokenType.LEFT_BRACE)) {
      return this.block();
    }
    if (this.match(TokenType.PRINT)) {
      return this.printStatement();
    }

    return this.expressionStatement();
  }

  /**
   * Parse an if statement
   */
  private ifStatement(): IfStatement {
    this.consume(TokenType.LEFT_PAREN, "Expect '(' after 'if'");
    const condition = this.expression();
    this.consume(TokenType.RIGHT_PAREN, "Expect ')' after if condition");

    const thenBranch = this.statement();
    let elseBranch: BlockStatement | null = null;

    if (this.match(TokenType.ELSE)) {
      elseBranch = this.statement() as BlockStatement;
    }

    return {
      type: 'IfStatement',
      condition,
      thenBranch,
      elseBranch
    };
  }

  /**
   * Parse a while statement
   */
  private whileStatement(): WhileStatement {
    this.consume(TokenType.LEFT_PAREN, "Expect '(' after 'while'");
    const condition = this.expression();
    this.consume(TokenType.RIGHT_PAREN, "Expect ')' after while condition");

    const body = this.statement();

    return {
      type: 'WhileStatement',
      condition,
      body
    };
  }

  /**
   * Parse a block statement
   */
  private block(): BlockStatement {
    const statements: Statement[] = [];

    while (!this.check(TokenType.RIGHT_BRACE) && !this.isAtEnd()) {
      const stmt = this.declaration();
      if (stmt) {
        statements.push(stmt);
      }
    }

    this.consume(TokenType.RIGHT_BRACE, "Expect '}' after block");

    return {
      type: 'BlockStatement',
      statements
    };
  }

  /**
   * Parse a print statement
   */
  private printStatement(): PrintStatement {
    this.consume(TokenType.LEFT_PAREN, "Expect '(' after 'print'");
    const expression = this.expression();
    this.consume(TokenType.RIGHT_PAREN, "Expect ')' after print expression");
    this.consume(TokenType.SEMICOLON, "Expect ';' after print statement");

    return {
      type: 'PrintStatement',
      expression
    };
  }

  /**
   * Parse an expression statement
   */
  private expressionStatement(): ExpressionStatement {
    const expr = this.expression();
    this.consume(TokenType.SEMICOLON, "Expect ';' after expression");
    return {
      type: 'ExpressionStatement',
      expression: expr
    };
  }

  /**
   * Parse an expression
   */
  private expression(): Expression {
    return this.assignment();
  }

  /**
   * Parse an assignment expression
   */
  private assignment(): Expression {
    const expr = this.equality();

    if (this.match(TokenType.EQUAL)) {
      const equals = this.previous();
      const value = this.assignment();

      if (expr.type === 'IdentifierExpression') {
        return {
          type: 'AssignmentExpression',
          name: (expr as IdentifierExpression).name,
          value
        } as AssignmentExpression;
      }

      throw new Error('Invalid assignment target');
    }

    return expr;
  }

  /**
   * Parse equality expressions (==, !=)
   */
  private equality(): Expression {
    let expr = this.comparison();

    while (this.match(TokenType.EQUAL_EQUAL, TokenType.BANG_EQUAL)) {
      const operator = this.previous().lexeme;
      const right = this.comparison();
      expr = {
        type: 'BinaryExpression',
        operator,
        left: expr,
        right
      } as BinaryExpression;
    }

    return expr;
  }

  /**
   * Parse comparison expressions (<, >, <=, >=)
   */
  private comparison(): Expression {
    let expr = this.term();

    while (
      this.match(
        TokenType.LESS,
        TokenType.LESS_EQUAL,
        TokenType.GREATER,
        TokenType.GREATER_EQUAL
      )
    ) {
      const operator = this.previous().lexeme;
      const right = this.term();
      expr = {
        type: 'BinaryExpression',
        operator,
        left: expr,
        right
      } as BinaryExpression;
    }

    return expr;
  }

  /**
   * Parse term expressions (+, -)
   */
  private term(): Expression {
    let expr = this.factor();

    while (this.match(TokenType.PLUS, TokenType.MINUS)) {
      const operator = this.previous().lexeme;
      const right = this.factor();
      expr = {
        type: 'BinaryExpression',
        operator,
        left: expr,
        right
      } as BinaryExpression;
    }

    return expr;
  }

  /**
   * Parse factor expressions (*, /, %)
   */
  private factor(): Expression {
    let expr = this.unary();

    while (this.match(TokenType.STAR, TokenType.SLASH, TokenType.PERCENT)) {
      const operator = this.previous().lexeme;
      const right = this.unary();
      expr = {
        type: 'BinaryExpression',
        operator,
        left: expr,
        right
      } as BinaryExpression;
    }

    return expr;
  }

  /**
   * Parse unary expressions (-)
   */
  private unary(): Expression {
    if (this.match(TokenType.MINUS)) {
      const operator = this.previous().lexeme;
      const right = this.unary();
      return {
        type: 'UnaryExpression',
        operator,
        operand: right
      } as UnaryExpression;
    }

    return this.primary();
  }

  /**
   * Parse primary expressions
   */
  private primary(): Expression {
    if (this.match(TokenType.NUMBER)) {
      return {
        type: 'LiteralExpression',
        value: createValue(this.previous().literal)
      } as LiteralExpression;
    }

    if (this.match(TokenType.STRING)) {
      return {
        type: 'LiteralExpression',
        value: createValue(this.previous().literal)
      } as LiteralExpression;
    }

    if (this.match(TokenType.TRUE)) {
      return {
        type: 'LiteralExpression',
        value: createValue(true)
      } as LiteralExpression;
    }

    if (this.match(TokenType.FALSE)) {
      return {
        type: 'LiteralExpression',
        value: createValue(false)
      } as LiteralExpression;
    }

    if (this.match(TokenType.IDENTIFIER)) {
      // Save the identifier token before checking for function call
      const identifier = this.previous();
      
      // Check if this is a function call
      if (this.match(TokenType.LEFT_PAREN)) {
        return this.finishCall(identifier);
      }

      return {
        type: 'IdentifierExpression',
        name: identifier.lexeme
      } as IdentifierExpression;
    }

    if (this.match(TokenType.LEFT_PAREN)) {
      const expr = this.expression();
      this.consume(TokenType.RIGHT_PAREN, "Expect ')' after expression");
      return expr;
    }

    throw new Error(`Expect expression at line ${this.peek().line}`);
  }

  /**
   * Finish parsing a function call
   */
  private finishCall(callee: Token): CallExpression {
    const args: Expression[] = [];

    if (!this.check(TokenType.RIGHT_PAREN)) {
      do {
        args.push(this.expression());
      } while (this.match(TokenType.COMMA));
    }

    this.consume(TokenType.RIGHT_PAREN, "Expect ')' after arguments");

    return {
      type: 'CallExpression',
      callee: {
        type: 'IdentifierExpression',
        name: callee.lexeme
      } as IdentifierExpression,
      arguments: args
    } as CallExpression;
  }

  /**
   * Check if current token matches any of the given types
   */
  private match(...types: TokenType[]): boolean {
    for (const type of types) {
      if (this.check(type)) {
        this.advance();
        return true;
      }
    }
    return false;
  }

  /**
   * Check if current token is of given type
   */
  private check(type: TokenType): boolean {
    if (this.isAtEnd()) {
      return false;
    }
    return this.peek().type === type;
  }

  /**
   * Consume current token if it matches type, otherwise throw error
   */
  private consume(type: TokenType, message: string): Token {
    if (this.check(type)) {
      return this.advance();
    }
    throw new Error(`${message} at line ${this.peek().line}`);
  }

  /**
   * Advance to next token and return previous
   */
  private advance(): Token {
    if (!this.isAtEnd()) {
      this.current++;
    }
    return this.tokens[this.current - 1];
  }

  /**
   * Check if at end of tokens
   */
  private isAtEnd(): boolean {
    return this.peek().type === TokenType.EOF;
  }

  /**
   * Get current token
   */
  private peek(): Token {
    return this.tokens[this.current];
  }

  /**
   * Get previous token
   */
  private previous(): Token {
    return this.tokens[this.current - 1];
  }

  /**
   * Synchronize parser after error
   */
  private synchronize(): void {
    this.advance();

    while (!this.isAtEnd()) {
      if (this.previous().type === TokenType.SEMICOLON) {
        return;
      }

      switch (this.peek().type) {
        case TokenType.FUNCTION:
        case TokenType.LET:
        case TokenType.IF:
        case TokenType.WHILE:
        case TokenType.PRINT:
          return;
      }

      this.advance();
    }
  }
}