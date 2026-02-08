/**
 * Parser for TypeScript - builds AST from tokens
 */

import { Lexer, Token, TokenType } from "./lexer";
import * as AST from "./ast";

export class Parser {
  private tokens: Token[];
  private pos: number = 0;

  constructor(source: string) {
    const lexer = new Lexer(source);
    this.tokens = lexer.tokenize();
  }

  public parse(): AST.Program {
    const body: AST.Statement[] = [];

    while (!this.isAtEnd()) {
      const stmt = this.parseStatement();
      if (stmt) {
        body.push(stmt);
      }
    }

    return {
      type: AST.NodeType.Program,
      body,
    };
  }

  private isAtEnd(): boolean {
    return this.peek().type === TokenType.EOF;
  }

  private peek(): Token {
    return this.tokens[this.pos];
  }

  private advance(): Token {
    if (!this.isAtEnd()) {
      this.pos++;
    }
    return this.tokens[this.pos - 1];
  }

  private previous(): Token {
    return this.tokens[this.pos - 1];
  }

  private check(type: TokenType): boolean {
    if (this.isAtEnd()) return false;
    return this.peek().type === type;
  }

  private checkAny(...types: TokenType[]): boolean {
    if (this.isAtEnd()) return false;
    return types.includes(this.peek().type);
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
    throw new Error(`${message} at line ${this.peek().loc.start.line}`);
  }

  private parseStatement(): AST.Statement | null {
    // Import/Export declarations
    if (this.match(TokenType.Import)) {
      return this.parseImportDeclaration();
    }

    if (this.match(TokenType.Export)) {
      return this.parseExportDeclaration();
    }

    // Type declarations (removed in transpilation)
    if (this.check(TokenType.Interface)) {
      this.skipInterfaceDeclaration();
      return null;
    }

    if (this.check(TokenType.Type)) {
      this.skipTypeAliasDeclaration();
      return null;
    }

    if (this.check(TokenType.Enum)) {
      return this.parseEnumDeclaration();
    }

    // Variable declarations
    if (this.match(TokenType.Let, TokenType.Const, TokenType.Var)) {
      return this.parseVariableDeclaration();
    }

    // Function declarations
    if (this.match(TokenType.Function)) {
      return this.parseFunctionDeclaration();
    }

    // Class declarations
    if (this.match(TokenType.Class)) {
      return this.parseClassDeclaration();
    }

    // Control flow statements
    if (this.match(TokenType.Return)) {
      return this.parseReturnStatement();
    }

    if (this.match(TokenType.If)) {
      return this.parseIfStatement();
    }

    if (this.match(TokenType.For)) {
      return this.parseForStatement();
    }

    if (this.match(TokenType.While)) {
      return this.parseWhileStatement();
    }

    if (this.match(TokenType.LeftBrace)) {
      return this.parseBlockStatement();
    }

    // Expression statement
    return this.parseExpressionStatement();
  }

  private parseImportDeclaration(): AST.ImportDeclaration {
    const specifiers: AST.ImportSpecifier[] = [];

    // Check for default import
    if (this.check(TokenType.Identifier) && !this.check(TokenType.From)) {
      const local = this.parseIdentifier();
      specifiers.push({
        kind: "default",
        local,
      });
    }

    // Check for named imports
    if (this.match(TokenType.LeftBrace)) {
      while (!this.check(TokenType.RightBrace) && !this.isAtEnd()) {
        const imported = this.parseIdentifier();

        let local: AST.Identifier = imported;
        if (this.match(TokenType.As)) {
          local = this.parseIdentifier();
        }

        specifiers.push({
          kind: "named",
          local,
          imported,
        });

        if (!this.match(TokenType.Comma)) break;
      }
      this.consume(TokenType.RightBrace, "Expected '}' after import specifiers");
    }

    // Check for namespace import
    if (this.match(TokenType.Asterisk)) {
      this.consume(TokenType.As, "Expected 'as' after *");
      const local = this.parseIdentifier();
      specifiers.push({
        kind: "namespace",
        local,
      });
    }

    this.consume(TokenType.From, "Expected 'from' after import specifiers");
    const sourceToken = this.consume(TokenType.StringLiteral, "Expected string literal for import source");
    const source = sourceToken.value;

    this.consume(TokenType.Semicolon, "Expected ';' after import declaration");

    return {
      type: AST.NodeType.ImportDeclaration,
      source,
      specifiers,
    };
  }

  private parseExportDeclaration(): AST.ExportDeclaration {
    let declaration: AST.Statement | undefined;
    let specifiers: AST.ExportSpecifier[] | undefined;
    let source: string | undefined;

    if (this.match(TokenType.Default)) {
      // export default ...
      if (this.check(TokenType.Function)) {
        declaration = this.parseFunctionDeclaration();
      } else if (this.check(TokenType.Class)) {
        declaration = this.parseClassDeclaration();
      } else {
        const expr = this.parseExpression();
        declaration = {
          type: AST.NodeType.ExpressionStatement,
          expression: expr,
        };
      }
    } else if (this.match(TokenType.LeftBrace)) {
      // export { ... }
      specifiers = [];
      while (!this.check(TokenType.RightBrace) && !this.isAtEnd()) {
        const local = this.parseIdentifier();

        let exported: AST.Identifier = local;
        if (this.match(TokenType.As)) {
          exported = this.parseIdentifier();
        }

        specifiers.push({ local, exported });

        if (!this.match(TokenType.Comma)) break;
      }
      this.consume(TokenType.RightBrace, "Expected '}' after export specifiers");

      if (this.match(TokenType.From)) {
        const sourceToken = this.consume(TokenType.StringLiteral, "Expected string literal for export source");
        source = sourceToken.value;
      }
    } else {
      // export declaration
      if (this.check(TokenType.Function)) {
        declaration = this.parseFunctionDeclaration();
      } else if (this.check(TokenType.Class)) {
        declaration = this.parseClassDeclaration();
      } else if (this.checkAny(TokenType.Let, TokenType.Const, TokenType.Var)) {
        declaration = this.parseVariableDeclaration();
      } else {
        throw new Error("Unexpected token after export");
      }
    }

    this.consume(TokenType.Semicolon, "Expected ';' after export declaration");

    return {
      type: AST.NodeType.ExportDeclaration,
      declaration,
      specifiers,
      source,
    };
  }

  private skipInterfaceDeclaration(): void {
    this.advance(); // interface
    this.consume(TokenType.Identifier, "Expected interface name");
    this.parseTypeParameters();
    if (this.match(TokenType.Extends)) {
      while (!this.check(TokenType.LeftBrace) && !this.isAtEnd()) {
        this.advance();
      }
    }
    this.consume(TokenType.LeftBrace, "Expected '{' after interface declaration");
    this.skipToMatchingBrace();
  }

  private skipTypeAliasDeclaration(): void {
    this.advance(); // type
    this.consume(TokenType.Identifier, "Expected type alias name");
    this.parseTypeParameters();
    this.consume(TokenType.Equal, "Expected '=' in type alias");
    this.skipType();
    this.consume(TokenType.Semicolon, "Expected ';' after type alias");
  }

  private skipType(): void {
    // Skip type annotation (simplified)
    let depth = 0;
    while (!this.isAtEnd()) {
      const token = this.peek();
      if (token.type === TokenType.Semicolon && depth === 0) break;
      if (token.type === TokenType.LeftBrace) depth++;
      if (token.type === TokenType.RightBrace) depth--;
      if (token.type === TokenType.LeftParen) depth++;
      if (token.type === TokenType.RightParen) depth--;
      if (token.type === TokenType.LeftBracket) depth++;
      if (token.type === TokenType.RightBracket) depth--;
      if (depth < 0) break;
      this.advance();
    }
  }

  private parseEnumDeclaration(): AST.EnumDeclaration {
    const isConst = this.previous().value === "const";
    const id = this.parseIdentifier();
    const members: AST.EnumMember[] = [];

    this.consume(TokenType.LeftBrace, "Expected '{' after enum name");

    while (!this.check(TokenType.RightBrace) && !this.isAtEnd()) {
      const memberId = this.parseIdentifier();
      let init: string | number | undefined;

      if (this.match(TokenType.Equal)) {
        const token = this.peek();
        if (this.match(TokenType.StringLiteral)) {
          init = token.value;
        } else if (this.match(TokenType.NumericLiteral)) {
          init = parseFloat(token.value);
        } else {
          throw new Error("Expected string or number literal for enum value");
        }
      }

      members.push({ id: memberId, init });

      if (!this.match(TokenType.Comma)) break;
    }

    this.consume(TokenType.RightBrace, "Expected '}' after enum members");

    return {
      type: AST.NodeType.EnumDeclaration,
      id,
      members,
      isConst,
    };
  }

  private parseVariableDeclaration(): AST.VariableDeclaration {
    const kindToken = this.previous();
    const kind = kindToken.value as "var" | "let" | "const";
    const declarations: AST.VariableDeclarator[] = [];

    do {
      const id = this.parseIdentifier();

      // Skip type annotation
      if (this.match(TokenType.Colon)) {
        this.skipType();
      }

      let init: AST.Expression | undefined;
      if (this.match(TokenType.Equal)) {
        init = this.parseExpression();
      }

      declarations.push({ id, init });
    } while (this.match(TokenType.Comma));

    this.consume(TokenType.Semicolon, "Expected ';' after variable declaration");

    return {
      type: AST.NodeType.VariableDeclaration,
      kind,
      declarations,
    };
  }

  private parseFunctionDeclaration(): AST.FunctionDeclaration {
    const isAsync = this.previous().value === "async";
    if (isAsync) {
      this.consume(TokenType.Function, "Expected 'function' after 'async'");
    }

    const id = this.parseIdentifier();
    const typeParameters = this.parseTypeParameters();

    this.consume(TokenType.LeftParen, "Expected '(' after function name");
    const params = this.parseParameters();
    this.consume(TokenType.RightParen, "Expected ')' after function parameters");

    // Skip return type annotation
    if (this.match(TokenType.Colon)) {
      this.skipType();
    }

    const body = this.parseBlockStatement();

    return {
      type: AST.NodeType.FunctionDeclaration,
      id,
      params,
      body,
      isAsync,
      isGenerator: false,
      typeParameters,
    };
  }

  private parseClassDeclaration(): AST.ClassDeclaration {
    const id = this.parseIdentifier();
    let superClass: AST.Identifier | undefined;
    const implements_: AST.TypeReference[] = [];

    // Check for extends
    if (this.match(TokenType.Extends)) {
      superClass = this.parseIdentifier();
    }

    // Check for implements (skip - not used in JS)
    if (this.match(TokenType.Implements)) {
      while (!this.check(TokenType.LeftBrace) && !this.isAtEnd()) {
        this.parseTypeReference();
        if (!this.match(TokenType.Comma)) break;
      }
    }

    this.consume(TokenType.LeftBrace, "Expected '{' after class declaration");
    const body = this.parseClassBody();
    this.consume(TokenType.RightBrace, "Expected '}' after class body");

    return {
      type: AST.NodeType.ClassDeclaration,
      id,
      superClass,
      body,
      implements: implements_,
    };
  }

  private parseClassBody(): AST.ClassElement[] {
    const elements: AST.ClassElement[] = [];

    while (!this.check(TokenType.RightBrace) && !this.isAtEnd()) {
      // Skip decorators
      while (this.match(TokenType.At)) {
        this.skipToNextLine();
      }

      const isStatic = this.match(TokenType.Static);
      let accessModifier: "public" | "private" | "protected" | undefined;

      if (this.match(TokenType.Public)) accessModifier = "public";
      else if (this.match(TokenType.Private)) accessModifier = "private";
      else if (this.match(TokenType.Protected)) accessModifier = "protected";

      // Check for constructor
      if (this.match(TokenType.Constructor)) {
        this.consume(TokenType.LeftParen, "Expected '(' after 'constructor'");
        const params = this.parseParameters();
        this.consume(TokenType.RightParen, "Expected ')' after constructor parameters");
        const body = this.parseBlockStatement();

        elements.push({
          type: AST.NodeType.ConstructorDefinition,
          params,
          body,
          accessModifier,
        });
        continue;
      }

      // Check for method or property
      const isAsync = this.match(TokenType.Async);
      const isGenerator = this.match(TokenType.Asterisk);

      const key = this.parseIdentifier();

      // Check if it's a method
      if (this.match(TokenType.LeftParen)) {
        const params = this.parseParameters();
        this.consume(TokenType.RightParen, "Expected ')' after method parameters");

        // Skip return type
        if (this.match(TokenType.Colon)) {
          this.skipType();
        }

        const body = this.parseBlockStatement();

        elements.push({
          type: AST.NodeType.MethodDefinition,
          key: key.name,
          value: {
            type: AST.NodeType.FunctionExpression,
            params,
            body,
            isAsync,
            isGenerator,
          },
          kind: "method",
          isStatic: isStatic || false,
          accessModifier,
        });
      } else {
        // Property
        let typeAnnotation: AST.TypeNode | undefined;
        let value: AST.Expression | undefined;

        // Skip type annotation
        if (this.match(TokenType.Colon)) {
          typeAnnotation = this.parseType();
        }

        if (this.match(TokenType.Equal)) {
          value = this.parseExpression();
        }

        this.consume(TokenType.Semicolon, "Expected ';' after class property");

        elements.push({
          type: AST.NodeType.PropertyDefinition,
          key,
          value,
          typeAnnotation,
          isStatic: isStatic || false,
          accessModifier,
        });
      }
    }

    return elements;
  }

  private parseParameters(): AST.Parameter[] {
    const params: AST.Parameter[] = [];

    if (!this.check(TokenType.RightParen)) {
      do {
        // Check for rest parameter
        const isRest = this.match(TokenType.Ellipsis);

        // Check for access modifier (constructor parameter properties)
        let accessModifier: "public" | "private" | "protected" | "readonly" | undefined;
        if (this.match(TokenType.Public)) accessModifier = "public";
        else if (this.match(TokenType.Private)) accessModifier = "private";
        else if (this.match(TokenType.Protected)) accessModifier = "protected";
        else if (this.match(TokenType.Readonly)) accessModifier = "readonly";

        const id = this.parseIdentifier();

        // Skip type annotation
        if (this.match(TokenType.Colon)) {
          this.skipType();
        }

        let initializer: AST.Expression | undefined;
        let isOptional = false;

        if (this.match(TokenType.Equal)) {
          initializer = this.parseExpression();
        } else if (this.match(TokenType.Question)) {
          isOptional = true;
        }

        params.push({
          id,
          isRest,
          isOptional,
          initializer,
          accessModifier,
        });
      } while (this.match(TokenType.Comma));
    }

    return params;
  }

  private parseReturnStatement(): AST.ReturnStatement {
    let argument: AST.Expression | undefined;

    if (!this.check(TokenType.Semicolon)) {
      argument = this.parseExpression();
    }

    this.consume(TokenType.Semicolon, "Expected ';' after return statement");

    return {
      type: AST.NodeType.ReturnStatement,
      argument,
    };
  }

  private parseIfStatement(): AST.IfStatement {
    this.consume(TokenType.LeftParen, "Expected '(' after 'if'");
    const test = this.parseExpression();
    this.consume(TokenType.RightParen, "Expected ')' after if condition");

    const consequent = this.parseStatement()!;
    let alternate: AST.Statement | undefined;

    if (this.match(TokenType.Else)) {
      alternate = this.parseStatement() || undefined;
    }

    return {
      type: AST.NodeType.IfStatement,
      test,
      consequent,
      alternate,
    };
  }

  private parseForStatement(): AST.ForStatement {
    this.consume(TokenType.LeftParen, "Expected '(' after 'for'");

    let init: AST.VariableDeclaration | AST.Expression | undefined;

    if (this.match(TokenType.Semicolon)) {
      // No init
    } else if (this.checkAny(TokenType.Let, TokenType.Const, TokenType.Var)) {
      init = this.parseVariableDeclaration();
    } else {
      init = this.parseExpression();
      this.consume(TokenType.Semicolon, "Expected ';' after for loop init");
    }

    let test: AST.Expression | undefined;
    if (!this.check(TokenType.Semicolon)) {
      test = this.parseExpression();
    }
    this.consume(TokenType.Semicolon, "Expected ';' after for loop test");

    let update: AST.Expression | undefined;
    if (!this.check(TokenType.RightParen)) {
      update = this.parseExpression();
    }
    this.consume(TokenType.RightParen, "Expected ')' after for loop update");

    const body = this.parseStatement()!;

    return {
      type: AST.NodeType.ForStatement,
      init,
      test,
      update,
      body,
    };
  }

  private parseWhileStatement(): AST.WhileStatement {
    this.consume(TokenType.LeftParen, "Expected '(' after 'while'");
    const test = this.parseExpression();
    this.consume(TokenType.RightParen, "Expected ')' after while condition");

    const body = this.parseStatement()!;

    return {
      type: AST.NodeType.WhileStatement,
      test,
      body,
    };
  }

  private parseBlockStatement(): AST.BlockStatement {
    this.consume(TokenType.LeftBrace, "Expected '{' for block statement");
    const body: AST.Statement[] = [];

    while (!this.check(TokenType.RightBrace) && !this.isAtEnd()) {
      const stmt = this.parseStatement();
      if (stmt) {
        body.push(stmt);
      }
    }

    this.consume(TokenType.RightBrace, "Expected '}' after block statement");

    return {
      type: AST.NodeType.BlockStatement,
      body,
    };
  }

  private parseExpressionStatement(): AST.ExpressionStatement {
    const expression = this.parseExpression();
    this.consume(TokenType.Semicolon, "Expected ';' after expression");
    return {
      type: AST.NodeType.ExpressionStatement,
      expression,
    };
  }

  private parseExpression(): AST.Expression {
    return this.parseAssignment();
  }

  private parseAssignment(): AST.Expression {
    const expr = this.parseConditional();

    if (this.match(
      TokenType.Equal,
      TokenType.PlusEqual,
      TokenType.MinusEqual,
      TokenType.AsteriskEqual,
      TokenType.SlashEqual,
      TokenType.PercentEqual,
      TokenType.CaretEqual,
      TokenType.AmpersandEqual,
      TokenType.PipeEqual
    )) {
      const operator = this.previous().value;
      const right = this.parseAssignment();

      return {
        type: AST.NodeType.AssignmentExpression,
        operator,
        left: expr,
        right,
      };
    }

    return expr;
  }

  private parseConditional(): AST.Expression {
    const expr = this.parseLogicalOr();

    if (this.match(TokenType.Question)) {
      const consequent = this.parseExpression();
      this.consume(TokenType.Colon, "Expected ':' in conditional expression");
      const alternate = this.parseConditional();

      return {
        type: AST.NodeType.ConditionalExpression,
        test: expr,
        consequent,
        alternate,
      };
    }

    return expr;
  }

  private parseLogicalOr(): AST.Expression {
    let expr = this.parseLogicalAnd();

    while (this.match(TokenType.PipePipe)) {
      const operator = this.previous().value;
      const right = this.parseLogicalAnd();
      expr = {
        type: AST.NodeType.BinaryExpression,
        operator,
        left: expr,
        right,
      };
    }

    return expr;
  }

  private parseLogicalAnd(): AST.Expression {
    let expr = this.parseEquality();

    while (this.match(TokenType.AmpersandAmpersand)) {
      const operator = this.previous().value;
      const right = this.parseEquality();
      expr = {
        type: AST.NodeType.BinaryExpression,
        operator,
        left: expr,
        right,
      };
    }

    return expr;
  }

  private parseEquality(): AST.Expression {
    let expr = this.parseRelational();

    while (this.match(TokenType.EqualEqual, TokenType.ExclamationEqual, TokenType.EqualEqualEqual, TokenType.ExclamationEqualEqual)) {
      const operator = this.previous().value;
      const right = this.parseRelational();
      expr = {
        type: AST.NodeType.BinaryExpression,
        operator,
        left: expr,
        right,
      };
    }

    return expr;
  }

  private parseRelational(): AST.Expression {
    let expr = this.parseShift();

    while (this.match(TokenType.LessThan, TokenType.GreaterThan, TokenType.LessThanEqual, TokenType.GreaterThanEqual)) {
      const operator = this.previous().value;
      const right = this.parseShift();
      expr = {
        type: AST.NodeType.BinaryExpression,
        operator,
        left: expr,
        right,
      };
    }

    return expr;
  }

  private parseShift(): AST.Expression {
    let expr = this.parseAdditive();

    while (this.match(TokenType.LessThanLessThan, TokenType.GreaterThanGreaterThan)) {
      const operator = this.previous().value;
      const right = this.parseAdditive();
      expr = {
        type: AST.NodeType.BinaryExpression,
        operator,
        left: expr,
        right,
      };
    }

    return expr;
  }

  private parseAdditive(): AST.Expression {
    let expr = this.parseMultiplicative();

    while (this.match(TokenType.Plus, TokenType.Minus)) {
      const operator = this.previous().value;
      const right = this.parseMultiplicative();
      expr = {
        type: AST.NodeType.BinaryExpression,
        operator,
        left: expr,
        right,
      };
    }

    return expr;
  }

  private parseMultiplicative(): AST.Expression {
    let expr = this.parseUnary();

    while (this.match(TokenType.Asterisk, TokenType.Slash, TokenType.Percent)) {
      const operator = this.previous().value;
      const right = this.parseUnary();
      expr = {
        type: AST.NodeType.BinaryExpression,
        operator,
        left: expr,
        right,
      };
    }

    return expr;
  }

  private parseUnary(): AST.Expression {
    if (this.match(TokenType.Exclamation, TokenType.Minus, TokenType.Plus, TokenType.Tilde, TokenType.Typeof)) {
      const operator = this.previous().value;
      const argument = this.parseUnary();

      return {
        type: AST.NodeType.UnaryExpression,
        operator,
        argument,
        prefix: true,
      };
    }

    return this.parsePostfix();
  }

  private parsePostfix(): AST.Expression {
    let expr = this.parseCall();

    while (true) {
      if (this.match(TokenType.PlusPlus, TokenType.MinusMinus)) {
        const operator = this.previous().value;
        expr = {
          type: AST.NodeType.UnaryExpression,
          operator,
          argument: expr,
          prefix: false,
        };
      } else {
        break;
      }
    }

    return expr;
  }

  private parseCall(): AST.Expression {
    let expr = this.parseMember();

    while (true) {
      if (this.match(TokenType.LeftParen)) {
        const args: AST.Expression[] = [];
        while (!this.check(TokenType.RightParen) && !this.isAtEnd()) {
          args.push(this.parseExpression());
          if (!this.match(TokenType.Comma)) break;
        }
        this.consume(TokenType.RightParen, "Expected ')' after call arguments");

        expr = {
          type: AST.NodeType.CallExpression,
          callee: expr,
          arguments: args,
        };
      } else {
        break;
      }
    }

    return expr;
  }

  private parseMember(): AST.Expression {
    let expr = this.parsePrimary();

    while (true) {
      if (this.match(TokenType.Dot)) {
        const property = this.parseIdentifier();
        expr = {
          type: AST.NodeType.MemberExpression,
          object: expr,
          property,
          computed: false,
        };
      } else if (this.match(TokenType.LeftBracket)) {
        const property = this.parseExpression();
        this.consume(TokenType.RightBracket, "Expected ']' after computed property");
        expr = {
          type: AST.NodeType.MemberExpression,
          object: expr,
          property,
          computed: true,
        };
      } else {
        break;
      }
    }

    return expr;
  }

  private parsePrimary(): AST.Expression {
    // Literals
    if (this.match(TokenType.NumericLiteral)) {
      return {
        type: AST.NodeType.Literal,
        value: parseFloat(this.previous().value),
      };
    }

    if (this.match(TokenType.StringLiteral)) {
      return {
        type: AST.NodeType.Literal,
        value: this.previous().value,
      };
    }

    if (this.match(TokenType.BooleanLiteral)) {
      return {
        type: AST.NodeType.Literal,
        value: this.previous().value === "true",
      };
    }

    if (this.match(TokenType.NullLiteral)) {
      return {
        type: AST.NodeType.Literal,
        value: null,
      };
    }

    // This expression
    if (this.match(TokenType.This)) {
      return {
        type: AST.NodeType.ThisExpression,
      };
    }

    // Identifier
    if (this.match(TokenType.Identifier)) {
      return this.parseIdentifier();
    }

    // Array expression
    if (this.match(TokenType.LeftBracket)) {
      const elements: (AST.Expression | null)[] = [];

      while (!this.check(TokenType.RightBracket) && !this.isAtEnd()) {
        if (this.check(TokenType.Comma)) {
          elements.push(null);
        } else {
          elements.push(this.parseExpression());
        }
        if (!this.match(TokenType.Comma)) break;
      }

      this.consume(TokenType.RightBracket, "Expected ']' after array elements");

      return {
        type: AST.NodeType.ArrayExpression,
        elements,
      };
    }

    // Object expression
    if (this.match(TokenType.LeftBrace)) {
      const properties: AST.Property[] = [];

      while (!this.check(TokenType.RightBrace) && !this.isAtEnd()) {
        const key = this.parseIdentifier();

        let value: AST.Expression;
        if (this.match(TokenType.Colon)) {
          value = this.parseExpression();
        } else {
          // Shorthand property
          value = {
            type: AST.NodeType.Identifier,
            name: key.name,
          };
        }

        properties.push({
          key: key.name,
          value,
          kind: "init",
          computed: false,
          shorthand: false,
        });

        if (!this.match(TokenType.Comma)) break;
      }

      this.consume(TokenType.RightBrace, "Expected '}' after object properties");

      return {
        type: AST.NodeType.ObjectExpression,
        properties,
      };
    }

    // Parenthesized expression
    if (this.match(TokenType.LeftParen)) {
      const expr = this.parseExpression();
      this.consume(TokenType.RightParen, "Expected ')' after parenthesized expression");
      return expr;
    }

    // New expression
    if (this.match(TokenType.New)) {
      const callee = this.parsePrimary();

      this.consume(TokenType.LeftParen, "Expected '(' after new");
      const args: AST.Expression[] = [];
      while (!this.check(TokenType.RightParen) && !this.isAtEnd()) {
        args.push(this.parseExpression());
        if (!this.match(TokenType.Comma)) break;
      }
      this.consume(TokenType.RightParen, "Expected ')' after new arguments");

      return {
        type: AST.NodeType.NewExpression,
        callee,
        arguments: args,
      };
    }

    // Arrow function
    if (this.match(TokenType.Async)) {
      return this.parseArrowFunction(true);
    }

    throw new Error(`Unexpected token ${this.peek().type} at line ${this.peek().loc.start.line}`);
  }

  private parseIdentifier(): AST.Identifier {
    const token = this.previous();
    return {
      type: AST.NodeType.Identifier,
      name: token.value,
    };
  }

  private parseArrowFunction(isAsync: boolean = false): AST.ArrowFunctionExpression {
    const startPos = this.pos;

    // Try to parse as arrow function
    let params: AST.Parameter[] = [];

    if (this.check(TokenType.LeftParen)) {
      this.advance();
      params = this.parseParameters();
      this.consume(TokenType.RightParen, "Expected ')' after arrow function parameters");
    } else {
      const id = this.parseIdentifier();
      params = [{
        id,
        isRest: false,
        isOptional: false,
      }];
    }

    if (!this.match(TokenType.Arrow)) {
      // Not an arrow function, reset
      this.pos = startPos;
      return this.parsePrimary() as any;
    }

    // Skip return type
    if (this.match(TokenType.Colon)) {
      this.skipType();
    }

    let body: AST.BlockStatement | AST.Expression;
    if (this.check(TokenType.LeftBrace)) {
      body = this.parseBlockStatement();
    } else {
      body = this.parseExpression();
    }

    return {
      type: AST.NodeType.ArrowFunctionExpression,
      params,
      body,
      isAsync,
    };
  }

  private parseTypeParameters(): AST.TypeParameter[] | undefined {
    if (!this.match(TokenType.LessThan)) {
      return undefined;
    }

    const params: AST.TypeParameter[] = [];

    while (!this.check(TokenType.GreaterThan) && !this.isAtEnd()) {
      const name = this.peek().value;
      this.advance();

      let constraint: AST.TypeNode | undefined;
      let default_: AST.TypeNode | undefined;

      if (this.match(TokenType.Extends)) {
        constraint = this.parseType();
      }

      if (this.match(TokenType.Equal)) {
        default_ = this.parseType();
      }

      params.push({ name, constraint, default: default_ });

      if (!this.match(TokenType.Comma)) break;
    }

    this.consume(TokenType.GreaterThan, "Expected '>' after type parameters");

    return params;
  }

  private parseType(): AST.TypeNode {
    // Simplified type parsing - just skip to next meaningful token
    let depth = 0;
    const start = this.pos;

    while (!this.isAtEnd()) {
      const token = this.peek();

      if (token.type === TokenType.Semicolon && depth === 0) break;
      if (token.type === TokenType.Equal && depth === 0) break;
      if (token.type === TokenType.Comma && depth === 0) break;
      if (token.type === TokenType.RightParen && depth === 0) break;
      if (token.type === TokenType.RightBrace && depth === 0) break;
      if (token.type === TokenType.RightBracket && depth === 0) break;

      if (token.type === TokenType.LeftParen) depth++;
      if (token.type === TokenType.RightParen) depth--;
      if (token.type === TokenType.LeftBrace) depth++;
      if (token.type === TokenType.RightBrace) depth--;
      if (token.type === TokenType.LeftBracket) depth++;
      if (token.type === TokenType.RightBracket) depth--;
      if (token.type === TokenType.LessThan) depth++;
      if (token.type === TokenType.GreaterThan) depth--;

      if (depth < 0) break;

      this.advance();
    }

    // Return a dummy AnyType
    return {
      type: AST.NodeType.AnyType,
    };
  }

  private parseTypeReference(): AST.TypeReference {
    const typeName = this.peek().value;
    this.advance();

    // Skip type arguments
    if (this.match(TokenType.LessThan)) {
      let depth = 1;
      while (depth > 0 && !this.isAtEnd()) {
        if (this.match(TokenType.LessThan)) depth++;
        else if (this.match(TokenType.GreaterThan)) depth--;
        else this.advance();
      }
    }

    return {
      type: AST.NodeType.TypeReference,
      typeName,
    };
  }

  private skipToMatchingBrace(): void {
    let depth = 1;
    while (depth > 0 && !this.isAtEnd()) {
      if (this.match(TokenType.LeftBrace)) depth++;
      else if (this.match(TokenType.RightBrace)) depth--;
      else this.advance();
    }
  }

  private skipToNextLine(): void {
    while (!this.isAtEnd() && this.peek().loc.start.line === this.previous().loc.start.line) {
      this.advance();
    }
  }
}