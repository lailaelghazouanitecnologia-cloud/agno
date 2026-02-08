/**
 * Parser - Recursive descent parser that builds AST from tokens
 */

import {
  Token,
  TokenType,
  ASTNode,
  ProgramNode,
  FunctionDeclNode,
  ParamNode,
  VarDeclNode,
  ArrayDeclNode,
  CompoundStmtNode,
  StmtNode,
  IfStmtNode,
  WhileStmtNode,
  ForStmtNode,
  ReturnStmtNode,
  ExprStmtNode,
  ExprNode,
  BinaryExprNode,
  UnaryExprNode,
  CallExprNode,
  ArrayAccessNode,
  IdentifierNode,
  NumberNode,
  StringNode,
  NodeType,
  CompilerError
} from "./interfaces.js";

export class Parser {
  private tokens: Token[];
  private current: number = 0;

  constructor(tokens: Token[]) {
    this.tokens = tokens;
  }

  /**
   * Parse the entire token stream into an AST
   */
  public parse(): ProgramNode {
    const declarations: (FunctionDeclNode | VarDeclNode)[] = [];

    while (!this.isAtEnd()) {
      const decl = this.declaration();
      if (decl) {
        declarations.push(decl);
      }
    }

    return {
      type: NodeType.PROGRAM,
      declarations
    };
  }

  private isAtEnd(): boolean {
    return this.peek().type === TokenType.EOF;
  }

  private peek(): Token {
    return this.tokens[this.current];
  }

  private previous(): Token {
    return this.tokens[this.current - 1];
  }

  private advance(): Token {
    if (!this.isAtEnd()) {
      this.current++;
    }
    return this.previous();
  }

  private check(type: TokenType): boolean {
    if (this.isAtEnd()) return false;
    return this.peek().type === type;
  }

  private match(...types: TokenType[]): boolean {
    for (const type of types) {
      if (this.check(type)) {
        this.advance();
        return true;
      }
    }
    return false;
  }

  private consume(type: TokenType, message: string): Token {
    if (this.check(type)) return this.advance();

    const token = this.peek();
    throw new CompilerError(
      `${message} at line ${token.line}`,
      token.line,
      token.column
    );
  }

  private synchronize(): void {
    this.advance();

    while (!this.isAtEnd()) {
      if (this.previous().type === TokenType.SEMICOLON) return;

      switch (this.peek().type) {
        case TokenType.INT:
        case TokenType.CHAR:
        case TokenType.FLOAT:
        case TokenType.VOID:
        case TokenType.IF:
        case TokenType.WHILE:
        case TokenType.FOR:
        case TokenType.RETURN:
          return;
      }

      this.advance();
    }
  }

  // ============================================================================
  // Declaration parsing
  // ============================================================================

  private declaration(): FunctionDeclNode | VarDeclNode | null {
    try {
      // Check for function or variable declaration
      if (this.match(TokenType.INT, TokenType.CHAR, TokenType.FLOAT, TokenType.VOID)) {
        const type = this.previous().value;

        if (this.check(TokenType.IDENTIFIER)) {
          const nameToken = this.peek();
          this.advance();

          // Check for function (followed by '(')
          if (this.match(TokenType.LPAREN)) {
            return this.functionDecl(type, nameToken.value);
          }
          // Check for array declaration (followed by '[')
          else if (this.match(TokenType.LBRACKET)) {
            return this.arrayDecl(type, nameToken.value);
          }
          // Variable declaration
          else {
            return this.varDecl(type, nameToken.value);
          }
        }
      }

      // Statement (if not a declaration)
      const stmt = this.statement();
      if (stmt) {
        // For now, we'll skip standalone statements in global scope
        return null;
      }

      return null;
    } catch (error) {
      this.synchronize();
      return null;
    }
  }

  private functionDecl(returnType: string, name: string): FunctionDeclNode {
    const params: ParamNode[] = [];

    // Parse parameters
    if (!this.check(TokenType.RPAREN)) {
      do {
        if (this.match(TokenType.INT, TokenType.CHAR, TokenType.FLOAT)) {
          const paramType = this.previous().value;
          const paramName = this.consume(TokenType.IDENTIFIER, "Expect parameter name").value;
          params.push({
            type: NodeType.PARAM,
            name: paramName,
            paramType
          });
        }
      } while (this.match(TokenType.COMMA));
    }

    this.consume(TokenType.RPAREN, "Expect ')' after parameters");

    // Parse function body
    const body = this.compoundStmt();

    return {
      type: NodeType.FUNCTION_DECL,
      name,
      returnType,
      params,
      body
    };
  }

  private varDecl(varType: string, name: string): VarDeclNode {
    let init: ExprNode | undefined;

    // Check for initialization
    if (this.match(TokenType.ASSIGN)) {
      init = this.expression();
    }

    this.consume(TokenType.SEMICOLON, "Expect ';' after variable declaration");

    return {
      type: NodeType.VAR_DECL,
      name,
      varType,
      init
    };
  }

  private arrayDecl(elementType: string, name: string): ArrayDeclNode {
    const size = this.expression();
    this.consume(TokenType.RBRACKET, "Expect ']' after array size");
    this.consume(TokenType.SEMICOLON, "Expect ';' after array declaration");

    return {
      type: NodeType.ARRAY_DECL,
      name,
      elementType,
      size
    };
  }

  // ============================================================================
  // Statement parsing
  // ============================================================================

  private statement(): StmtNode | null {
    if (this.match(TokenType.IF)) {
      return this.ifStmt();
    }
    if (this.match(TokenType.WHILE)) {
      return this.whileStmt();
    }
    if (this.match(TokenType.FOR)) {
      return this.forStmt();
    }
    if (this.match(TokenType.RETURN)) {
      return this.returnStmt();
    }
    if (this.match(TokenType.LBRACE)) {
      return this.compoundStmt();
    }

    // Expression statement or declaration
    if (this.match(TokenType.INT, TokenType.CHAR, TokenType.FLOAT)) {
      const type = this.previous().value;
      const name = this.consume(TokenType.IDENTIFIER, "Expect variable name").value;

      // Array declaration
      if (this.match(TokenType.LBRACKET)) {
        return this.arrayDecl(type, name);
      }

      // Variable declaration
      return this.varDecl(type, name);
    }

    return this.exprStmt();
  }

  private ifStmt(): IfStmtNode {
    this.consume(TokenType.LPAREN, "Expect '(' after 'if'");
    const condition = this.expression();
    this.consume(TokenType.RPAREN, "Expect ')' after if condition");

    const thenBranch = this.statement();
    let elseBranch: StmtNode | undefined;
    const elseifBranches: { condition: ExprNode; body: StmtNode }[] = [];

    // Handle else-if chains
    if (this.match(TokenType.ELSE)) {
      if (this.check(TokenType.IF)) {
        this.advance(); // consume 'if'
        this.consume(TokenType.LPAREN, "Expect '(' after 'else if'");
        const elseifCondition = this.expression();
        this.consume(TokenType.RPAREN, "Expect ')' after else if condition");
        const elseifBody = this.statement();
        elseifBranches.push({ condition: elseifCondition, body: elseifBody });
      } else {
        elseBranch = this.statement();
      }
    }

    return {
      type: NodeType.IF_STMT,
      condition,
      thenBranch,
      elseBranch,
      elseifBranches
    };
  }

  private whileStmt(): WhileStmtNode {
    this.consume(TokenType.LPAREN, "Expect '(' after 'while'");
    const condition = this.expression();
    this.consume(TokenType.RPAREN, "Expect ')' after while condition");
    const body = this.statement();

    return {
      type: NodeType.WHILE_STMT,
      condition,
      body
    };
  }

  private forStmt(): ForStmtNode {
    this.consume(TokenType.LPAREN, "Expect '(' after 'for'");

    // Initialization
    let init: StmtNode | undefined;
    if (this.match(TokenType.SEMICOLON)) {
      init = undefined;
    } else if (this.match(TokenType.INT, TokenType.CHAR, TokenType.FLOAT)) {
      const type = this.previous().value;
      const name = this.consume(TokenType.IDENTIFIER, "Expect variable name").value;
      init = this.varDecl(type, name);
    } else {
      init = this.exprStmt();
    }

    // Condition
    let condition: ExprNode | undefined;
    if (!this.check(TokenType.SEMICOLON)) {
      condition = this.expression();
    }
    this.consume(TokenType.SEMICOLON, "Expect ';' after for condition");

    // Increment
    let increment: ExprNode | undefined;
    if (!this.check(TokenType.RPAREN)) {
      increment = this.expression();
    }
    this.consume(TokenType.RPAREN, "Expect ')' after for increment");

    const body = this.statement();

    return {
      type: NodeType.FOR_STMT,
      init,
      condition,
      increment,
      body
    };
  }

  private returnStmt(): ReturnStmtNode {
    let value: ExprNode | undefined;

    if (!this.check(TokenType.SEMICOLON)) {
      value = this.expression();
    }

    this.consume(TokenType.SEMICOLON, "Expect ';' after return value");

    return {
      type: NodeType.RETURN_STMT,
      value
    };
  }

  private compoundStmt(): CompoundStmtNode {
    const statements: StmtNode[] = [];

    while (!this.check(TokenType.RBRACE) && !this.isAtEnd()) {
      const stmt = this.statement();
      if (stmt) {
        statements.push(stmt);
      }
    }

    this.consume(TokenType.RBRACE, "Expect '}' after block");

    return {
      type: NodeType.COMPOUND_STMT,
      statements
    };
  }

  private exprStmt(): ExprStmtNode {
    const expr = this.expression();
    this.consume(TokenType.SEMICOLON, "Expect ';' after expression");
    return {
      type: NodeType.EXPR_STMT,
      expression: expr
    };
  }

  // ============================================================================
  // Expression parsing (precedence climbing)
  // ============================================================================

  private expression(): ExprNode {
    return this.assignment();
  }

  private assignment(): ExprNode {
    const expr = this.equality();

    if (this.match(TokenType.ASSIGN)) {
      // For now, simple assignment
      return expr;
    }

    return expr;
  }

  private equality(): ExprNode {
    let expr = this.relational();

    while (this.match(TokenType.EQUAL, TokenType.NOT_EQUAL)) {
      const operator = this.previous().value;
      const right = this.relational();
      expr = {
        type: NodeType.BINARY_EXPR,
        operator,
        left: expr,
        right
      };
    }

    return expr;
  }

  private relational(): ExprNode {
    let expr = this.additive();

    while (this.match(TokenType.LESS, TokenType.LESS_EQUAL, TokenType.GREATER, TokenType.GREATER_EQUAL)) {
      const operator = this.previous().value;
      const right = this.additive();
      expr = {
        type: NodeType.BINARY_EXPR,
        operator,
        left: expr,
        right
      };
    }

    return expr;
  }

  private additive(): ExprNode {
    let expr = this.multiplicative();

    while (this.match(TokenType.PLUS, TokenType.MINUS)) {
      const operator = this.previous().value;
      const right = this.multiplicative();
      expr = {
        type: NodeType.BINARY_EXPR,
        operator,
        left: expr,
        right
      };
    }

    return expr;
  }

  private multiplicative(): ExprNode {
    let expr = this.unary();

    while (this.match(TokenType.STAR, TokenType.SLASH, TokenType.PERCENT)) {
      const operator = this.previous().value;
      const right = this.unary();
      expr = {
        type: NodeType.BINARY_EXPR,
        operator,
        left: expr,
        right
      };
    }

    return expr;
  }

  private unary(): ExprNode {
    if (this.match(TokenType.MINUS, TokenType.AMPERSAND)) {
      const operator = this.previous().value;
      const operand = this.unary();
      return {
        type: NodeType.UNARY_EXPR,
        operator,
        operand
      };
    }

    return this.primary();
  }

  private primary(): ExprNode {
    // Number literal
    if (this.match(TokenType.NUMBER)) {
      const value = parseFloat(this.previous().value);
      return {
        type: NodeType.NUMBER,
        value
      };
    }

    // String literal
    if (this.match(TokenType.STRING)) {
      return {
        type: NodeType.STRING,
        value: this.previous().value
      };
    }

    // Identifier or function call
    if (this.match(TokenType.IDENTIFIER)) {
      const name = this.previous().value;

      // Function call
      if (this.match(TokenType.LPAREN)) {
        const args: ExprNode[] = [];

        if (!this.check(TokenType.RPAREN)) {
          do {
            args.push(this.expression());
          } while (this.match(TokenType.COMMA));
        }

        this.consume(TokenType.RPAREN, "Expect ')' after arguments");

        return {
          type: NodeType.CALL_EXPR,
          callee: name,
          args
        };
      }

      // Array access
      if (this.match(TokenType.LBRACKET)) {
        const index = this.expression();
        this.consume(TokenType.RBRACKET, "Expect ']' after array index");
        return {
          type: NodeType.ARRAY_ACCESS,
          array: name,
          index
        };
      }

      // Simple identifier
      return {
        type: NodeType.IDENTIFIER,
        name
      };
    }

    // Parenthesized expression
    if (this.match(TokenType.LPAREN)) {
      const expr = this.expression();
      this.consume(TokenType.RPAREN, "Expect ')' after expression");
      return expr;
    }

    const token = this.peek();
    throw new CompilerError(
      `Unexpected token '${token.type}'`,
      token.line,
      token.column
    );
  }
}