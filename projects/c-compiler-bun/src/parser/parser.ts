import { Token, TokenType, NodeType, ASTNode, TypeInfo } from "../interfaces/index.js";

/**
 * Parser - Recursive descent parser for C
 * Builds an Abstract Syntax Tree (AST) from tokens
 */
export class Parser {
  private tokens: Token[];
  private current: number = 0;
  
  constructor(tokens: Token[]) {
    this.tokens = tokens;
  }
  
  /**
   * Parse the entire program
   */
  public parse(): ASTNode {
    const statements: ASTNode[] = [];
    
    while (!this.isAtEnd()) {
      const decl = this.parseDeclaration();
      if (decl) {
        statements.push(decl);
      }
    }
    
    return {
      type: NodeType.PROGRAM,
      line: 1,
      column: 1,
      statements
    } as any;
  }
  
  /**
   * Parse a declaration (function or variable)
   */
  private parseDeclaration(): ASTNode | null {
    // Try to parse a function
    if (this.checkFunctionDeclaration()) {
      return this.parseFunctionDeclaration();
    }
    
    // Try to parse a variable declaration
    if (this.checkVariableDeclaration()) {
      return this.parseVariableDeclaration();
    }
    
    // Skip unknown tokens
    this.advance();
    return null;
  }
  
  /**
   * Check if the next tokens form a function declaration
   */
  private checkFunctionDeclaration(): boolean {
    const save = this.current;
    
    try {
      // type identifier ( ... ) {
      if (!this.isType()) return false;
      this.advance();
      
      if (!this.match(TokenType.IDENTIFIER)) return false;
      
      if (!this.match(TokenType.LPAREN)) return false;
      
      // Skip parameters
      while (!this.check(TokenType.RPAREN) && !this.isAtEnd()) {
        this.advance();
      }
      
      if (!this.match(TokenType.RPAREN)) return false;
      
      return this.check(TokenType.LBRACE);
    } finally {
      this.current = save;
    }
  }
  
  /**
   * Check if the next tokens form a variable declaration
   */
  private checkVariableDeclaration(): boolean {
    const save = this.current;
    
    try {
      if (!this.isType()) return false;
      this.advance();
      
      if (!this.match(TokenType.IDENTIFIER)) return false;
      
      // Could be followed by =, ;, [, or ,
      return this.check(TokenType.ASSIGN) || 
             this.check(TokenType.SEMICOLON) ||
             this.check(TokenType.LBRACKET) ||
             this.check(TokenType.COMMA);
    } finally {
      this.current = save;
    }
  }
  
  /**
   * Parse a function declaration
   */
  private parseFunctionDeclaration(): ASTNode {
    const startToken = this.peek();
    
    // Parse return type
    const returnType = this.parseType();
    
    // Parse function name
    const name = this.consume(TokenType.IDENTIFIER, "Expected function name").value;
    
    // Parse parameters
    this.consume(TokenType.LPAREN, "Expected '(' after function name");
    const parameters = this.parseParameters();
    this.consume(TokenType.RPAREN, "Expected ')' after parameters");
    
    // Parse function body
    this.consume(TokenType.LBRACE, "Expected '{' before function body");
    const body = this.parseCompoundStatement();
    
    return {
      type: NodeType.FUNCTION_DECL,
      line: startToken.line,
      column: startToken.column,
      name,
      returnType,
      parameters,
      body
    } as any;
  }
  
  /**
   * Parse function parameters
   */
  private parseParameters(): any[] {
    const parameters: any[] = [];
    
    if (!this.check(TokenType.RPAREN)) {
      do {
        const paramType = this.parseType();
        const paramName = this.consume(TokenType.IDENTIFIER, "Expected parameter name").value;
        
        parameters.push({
          type: NodeType.PARAMETER,
          name: paramName,
          paramType,
          line: paramType.line,
          column: paramType.column
        });
      } while (this.match(TokenType.COMMA));
    }
    
    return parameters;
  }
  
  /**
   * Parse a variable declaration
   */
  private parseVariableDeclaration(): ASTNode {
    const startToken = this.peek();
    
    // Parse type
    const varType = this.parseType();
    
    // Parse variable name(s)
    const declarators: any[] = [];
    
    do {
      const name = this.consume(TokenType.IDENTIFIER, "Expected variable name").value;
      let initializer: ASTNode | null = null;
      let arraySize: ASTNode | null = null;
      
      // Check for array declaration
      if (this.match(TokenType.LBRACKET)) {
        if (!this.check(TokenType.RBRACKET)) {
          arraySize = this.parseExpression();
        }
        this.consume(TokenType.RBRACKET, "Expected ']' after array size");
      }
      
      // Check for initializer
      if (this.match(TokenType.ASSIGN)) {
        initializer = this.parseExpression();
      }
      
      declarators.push({
        name,
        initializer,
        arraySize
      });
    } while (this.match(TokenType.COMMA));
    
    this.consume(TokenType.SEMICOLON, "Expected ';' after variable declaration");
    
    return {
      type: NodeType.DECL_STMT,
      line: startToken.line,
      column: startToken.column,
      varType,
      declarators
    } as any;
  }
  
  /**
   * Parse a type
   */
  private parseType(): TypeInfo {
    const token = this.advance();
    let baseType: "int" | "char" | "float" | "void" = "int";
    
    switch (token.type) {
      case TokenType.INT:
        baseType = "int";
        break;
      case TokenType.CHAR:
        baseType = "char";
        break;
      case TokenType.FLOAT:
        baseType = "float";
        break;
      case TokenType.VOID:
        baseType = "void";
        break;
      default:
        throw this.error(token, "Expected type");
    }
    
    // Check for pointer type
    let isPointer = false;
    while (this.match(TokenType.STAR)) {
      isPointer = true;
    }
    
    return {
      baseType,
      isPointer,
      isArray: false
    };
  }
  
  /**
   * Parse a compound statement (block)
   */
  private parseCompoundStatement(): ASTNode {
    const startToken = this.previous();
    const statements: ASTNode[] = [];
    
    while (!this.check(TokenType.RBRACE) && !this.isAtEnd()) {
      const stmt = this.parseStatement();
      if (stmt) {
        statements.push(stmt);
      }
    }
    
    this.consume(TokenType.RBRACE, "Expected '}' after block");
    
    return {
      type: NodeType.COMPOUND_STMT,
      line: startToken.line,
      column: startToken.column,
      statements
    } as any;
  }
  
  /**
   * Parse a statement
   */
  private parseStatement(): ASTNode | null {
    if (this.match(TokenType.IF)) {
      return this.parseIfStatement();
    }
    
    if (this.match(TokenType.WHILE)) {
      return this.parseWhileStatement();
    }
    
    if (this.match(TokenType.FOR)) {
      return this.parseForStatement();
    }
    
    if (this.match(TokenType.RETURN)) {
      return this.parseReturnStatement();
    }
    
    if (this.check(TokenType.LBRACE)) {
      return this.parseCompoundStatement();
    }
    
    if (this.checkVariableDeclaration()) {
      return this.parseVariableDeclaration();
    }
    
    return this.parseExpressionStatement();
  }
  
  /**
   * Parse an if statement
   */
  private parseIfStatement(): ASTNode {
    const startToken = this.previous();
    
    this.consume(TokenType.LPAREN, "Expected '(' after 'if'");
    const condition = this.parseExpression();
    this.consume(TokenType.RPAREN, "Expected ')' after if condition");
    
    const thenBranch = this.parseStatement();
    let elseBranch: ASTNode | null = null;
    
    if (this.match(TokenType.ELSE)) {
      elseBranch = this.parseStatement();
    }
    
    return {
      type: NodeType.IF_STMT,
      line: startToken.line,
      column: startToken.column,
      condition,
      thenBranch,
      elseBranch
    } as any;
  }
  
  /**
   * Parse a while statement
   */
  private parseWhileStatement(): ASTNode {
    const startToken = this.previous();
    
    this.consume(TokenType.LPAREN, "Expected '(' after 'while'");
    const condition = this.parseExpression();
    this.consume(TokenType.RPAREN, "Expected ')' after while condition");
    
    const body = this.parseStatement();
    
    return {
      type: NodeType.WHILE_STMT,
      line: startToken.line,
      column: startToken.column,
      condition,
      body
    } as any;
  }
  
  /**
   * Parse a for statement
   */
  private parseForStatement(): ASTNode {
    const startToken = this.previous();
    
    this.consume(TokenType.LPAREN, "Expected '(' after 'for'");
    
    // Parse initialization
    let init: ASTNode | null = null;
    if (!this.match(TokenType.SEMICOLON)) {
      if (this.checkVariableDeclaration()) {
        init = this.parseVariableDeclaration();
      } else {
        init = this.parseExpression();
        this.consume(TokenType.SEMICOLON, "Expected ';' after for initialization");
      }
    }
    
    // Parse condition
    let condition: ASTNode | null = null;
    if (!this.check(TokenType.SEMICOLON)) {
      condition = this.parseExpression();
    }
    this.consume(TokenType.SEMICOLON, "Expected ';' after for condition");
    
    // Parse increment
    let increment: ASTNode | null = null;
    if (!this.check(TokenType.RPAREN)) {
      increment = this.parseExpression();
    }
    this.consume(TokenType.RPAREN, "Expected ')' after for clauses");
    
    const body = this.parseStatement();
    
    return {
      type: NodeType.FOR_STMT,
      line: startToken.line,
      column: startToken.column,
      init,
      condition,
      increment,
      body
    } as any;
  }
  
  /**
   * Parse a return statement
   */
  private parseReturnStatement(): ASTNode {
    const startToken = this.previous();
    
    let value: ASTNode | null = null;
    if (!this.check(TokenType.SEMICOLON)) {
      value = this.parseExpression();
    }
    
    this.consume(TokenType.SEMICOLON, "Expected ';' after return value");
    
    return {
      type: NodeType.RETURN_STMT,
      line: startToken.line,
      column: startToken.column,
      value
    } as any;
  }
  
  /**
   * Parse an expression statement
   */
  private parseExpressionStatement(): ASTNode {
    const startToken = this.peek();
    const expr = this.parseExpression();
    this.consume(TokenType.SEMICOLON, "Expected ';' after expression");
    
    return {
      type: NodeType.EXPR_STMT,
      line: startToken.line,
      column: startToken.column,
      expression: expr
    } as any;
  }
  
  /**
   * Parse an expression
   */
  private parseExpression(): ASTNode {
    return this.parseAssignment();
  }
  
  /**
   * Parse assignment expression
   */
  private parseAssignment(): ASTNode {
    const expr = this.parseOr();
    
    if (this.match(TokenType.ASSIGN)) {
      const operator = this.previous();
      const value = this.parseAssignment();
      
      return {
        type: NodeType.ASSIGN_EXPR,
        line: operator.line,
        column: operator.column,
        left: expr,
        right: value
      } as any;
    }
    
    return expr;
  }
  
  /**
   * Parse logical OR
   */
  private parseOr(): ASTNode {
    return this.parseAnd();
  }
  
  /**
   * Parse logical AND
   */
  private parseAnd(): ASTNode {
    return this.parseEquality();
  }
  
  /**
   * Parse equality expression (==, !=)
   */
  private parseEquality(): ASTNode {
    let expr = this.parseComparison();
    
    while (this.match(TokenType.EQUAL) || this.match(TokenType.NOT_EQUAL)) {
      const operator = this.previous();
      const right = this.parseComparison();
      
      expr = {
        type: NodeType.BINARY_EXPR,
        line: operator.line,
        column: operator.column,
        left: expr,
        operator: operator.value,
        right
      } as any;
    }
    
    return expr;
  }
  
  /**
   * Parse comparison expression (<, >, <=, >=)
   */
  private parseComparison(): ASTNode {
    let expr = this.parseTerm();
    
    while (this.match(TokenType.LESS) || 
           this.match(TokenType.GREATER) || 
           this.match(TokenType.LESS_EQUAL) || 
           this.match(TokenType.GREATER_EQUAL)) {
      const operator = this.previous();
      const right = this.parseTerm();
      
      expr = {
        type: NodeType.BINARY_EXPR,
        line: operator.line,
        column: operator.column,
        left: expr,
        operator: operator.value,
        right
      } as any;
    }
    
    return expr;
  }
  
  /**
   * Parse term expression (+, -)
   */
  private parseTerm(): ASTNode {
    let expr = this.parseFactor();
    
    while (this.match(TokenType.PLUS) || this.match(TokenType.MINUS)) {
      const operator = this.previous();
      const right = this.parseFactor();
      
      expr = {
        type: NodeType.BINARY_EXPR,
        line: operator.line,
        column: operator.column,
        left: expr,
        operator: operator.value,
        right
      } as any;
    }
    
    return expr;
  }
  
  /**
   * Parse factor expression (*, /, %)
   */
  private parseFactor(): ASTNode {
    let expr = this.parseUnary();
    
    while (this.match(TokenType.STAR) || 
           this.match(TokenType.SLASH) || 
           this.match(TokenType.PERCENT)) {
      const operator = this.previous();
      const right = this.parseUnary();
      
      expr = {
        type: NodeType.BINARY_EXPR,
        line: operator.line,
        column: operator.column,
        left: expr,
        operator: operator.value,
        right
      } as any;
    }
    
    return expr;
  }
  
  /**
   * Parse unary expression
   */
  private parseUnary(): ASTNode {
    if (this.match(TokenType.MINUS) || this.match(TokenType.AMPERSAND) || this.match(TokenType.STAR)) {
      const operator = this.previous();
      const operand = this.parseUnary();
      
      let nodeType: NodeType;
      if (operator.value === "&") {
        nodeType = NodeType.ADDRESS_OF_EXPR;
      } else if (operator.value === "*") {
        nodeType = NodeType.POINTER_DEREF_EXPR;
      } else {
        nodeType = NodeType.UNARY_EXPR;
      }
      
      return {
        type: nodeType,
        line: operator.line,
        column: operator.column,
        operator: operator.value,
        operand
      } as any;
    }
    
    return this.parsePostfix();
  }
  
  /**
   * Parse postfix expression (function calls, array access)
   */
  private parsePostfix(): ASTNode {
    let expr = this.parsePrimary();
    
    while (true) {
      // Function call
      if (this.match(TokenType.LPAREN)) {
        const args = this.parseArguments();
        this.consume(TokenType.RPAREN, "Expected ')' after arguments");
        
        expr = {
          type: NodeType.CALL_EXPR,
          line: expr.line,
          column: expr.column,
          callee: expr,
          arguments: args
        } as any;
      }
      // Array access
      else if (this.match(TokenType.LBRACKET)) {
        const index = this.parseExpression();
        this.consume(TokenType.RBRACKET, "Expected ']' after index");
        
        expr = {
          type: NodeType.ARRAY_ACCESS_EXPR,
          line: expr.line,
          column: expr.column,
          array: expr,
          index
        } as any;
      }
      else {
        break;
      }
    }
    
    return expr;
  }
  
  /**
   * Parse function call arguments
   */
  private parseArguments(): ASTNode[] {
    const args: ASTNode[] = [];
    
    if (!this.check(TokenType.RPAREN)) {
      do {
        args.push(this.parseExpression());
      } while (this.match(TokenType.COMMA));
    }
    
    return args;
  }
  
  /**
   * Parse a primary expression
   */
  private parsePrimary(): ASTNode {
    // Integer literal
    if (this.match(TokenType.INTEGER_LITERAL)) {
      const token = this.previous();
      return {
        type: NodeType.LITERAL_EXPR,
        line: token.line,
        column: token.column,
        valueType: "int",
        value: parseInt(token.value, 10)
      } as any;
    }
    
    // Float literal
    if (this.match(TokenType.FLOAT_LITERAL)) {
      const token = this.previous();
      return {
        type: NodeType.LITERAL_EXPR,
        line: token.line,
        column: token.column,
        valueType: "float",
        value: parseFloat(token.value)
      } as any;
    }
    
    // Character literal
    if (this.match(TokenType.CHAR_LITERAL)) {
      const token = this.previous();
      return {
        type: NodeType.LITERAL_EXPR,
        line: token.line,
        column: token.column,
        valueType: "char",
        value: token.value.charCodeAt(0)
      } as any;
    }
    
    // String literal
    if (this.match(TokenType.STRING_LITERAL)) {
      const token = this.previous();
      return {
        type: NodeType.LITERAL_EXPR,
        line: token.line,
        column: token.column,
        valueType: "string",
        value: token.value
      } as any;
    }
    
    // Identifier
    if (this.match(TokenType.IDENTIFIER)) {
      const token = this.previous();
      return {
        type: NodeType.IDENTIFIER_EXPR,
        line: token.line,
        column: token.column,
        name: token.value
      } as any;
    }
    
    // Parenthesized expression
    if (this.match(TokenType.LPAREN)) {
      const expr = this.parseExpression();
      this.consume(TokenType.RPAREN, "Expected ')' after expression");
      return expr;
    }
    
    throw this.error(this.peek(), "Expected expression");
  }
  
  /**
   * Check if current token is a type
   */
  private isType(): boolean {
    return this.check(TokenType.INT) || 
           this.check(TokenType.CHAR) || 
           this.check(TokenType.FLOAT) || 
           this.check(TokenType.VOID);
  }
  
  /**
   * Check if current token matches type
   */
  private check(type: TokenType): boolean {
    if (this.isAtEnd()) return false;
    return this.peek().type === type;
  }
  
  /**
   * Check if current token matches any of the types
   */
  private checkAny(...types: TokenType[]): boolean {
    for (const type of types) {
      if (this.check(type)) return true;
    }
    return false;
  }
  
  /**
   * Consume token if it matches type, otherwise throw error
   */
  private consume(type: TokenType, message: string): Token {
    if (this.check(type)) return this.advance();
    throw this.error(this.peek(), message);
  }
  
  /**
   * Advance to next token
   */
  private advance(): Token {
    if (!this.isAtEnd()) this.current++;
    return this.previous();
  }
  
  /**
   * Check if we're at the end
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
   * Match and consume token if it matches type
   */
  private match(type: TokenType): boolean {
    if (this.check(type)) {
      this.advance();
      return true;
    }
    return false;
  }
  
  /**
   * Create an error
   */
  private error(token: Token, message: string): Error {
    return new Error(`[Line ${token.line}] ${message}`);
  }
}