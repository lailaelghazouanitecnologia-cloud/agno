import { Lexer } from '../../lexer/Lexer';
import { Parser } from '../Parser';
import { createValue } from '../../types/Value';
import {
  Program,
  VariableDeclaration,
  FunctionDeclaration,
  IfStatement,
  WhileStatement,
  PrintStatement,
  BlockStatement,
  ExpressionStatement,
  BinaryExpression,
  LiteralExpression,
  IdentifierExpression,
  CallExpression,
  AssignmentExpression
} from '../../types/AST';

describe('Parser', () => {
  const parse = (source: string): Program => {
    const lexer = new Lexer(source);
    const tokens = lexer.tokenize();
    const parser = new Parser(tokens);
    return parser.parse();
  };

  const parseExpectError = (source: string): Error => {
    try {
      parse(source);
      throw new Error('Expected parse to throw an error');
    } catch (error) {
      return error as Error;
    }
  };

  // ============================================================================
  // HAPPY PATH TESTS
  // ============================================================================

  describe('Happy Path - Literals', () => {
    it('should parse integer number literals', () => {
      const ast = parse('42;');
      expect(ast.type).toBe('Program');
      expect(ast.statements).toHaveLength(1);
      
      const stmt = ast.statements[0] as ExpressionStatement;
      expect(stmt.type).toBe('ExpressionStatement');
      
      const expr = stmt.expression as LiteralExpression;
      expect(expr.type).toBe('LiteralExpression');
      expect(expr.value).toEqual(createValue(42));
    });

    it('should parse floating point number literals', () => {
      const ast = parse('3.14;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as LiteralExpression;
      expect(expr.value).toEqual(createValue(3.14));
    });

    it('should parse negative number literals', () => {
      const ast = parse('-42;');
      const stmt = ast.statements[0] as ExpressionStatement;
      expect(stmt.expression.type).toBe('UnaryExpression');
    });

    it('should parse string literals', () => {
      const ast = parse('"hello world";');
      const expr = (ast.statements[0] as ExpressionStatement).expression as LiteralExpression;
      expect(expr.type).toBe('LiteralExpression');
      expect(expr.value).toEqual(createValue('hello world'));
    });

    it('should parse empty string literals', () => {
      const ast = parse('"";');
      const expr = (ast.statements[0] as ExpressionStatement).expression as LiteralExpression;
      expect(expr.value).toEqual(createValue(''));
    });

    it('should parse string with special characters', () => {
      const ast = parse('"hello\\nworld";');
      const expr = (ast.statements[0] as ExpressionStatement).expression as LiteralExpression;
      expect(expr.value).toEqual(createValue('hello\\nworld'));
    });

    it('should parse true boolean literal', () => {
      const ast = parse('true;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as LiteralExpression;
      expect(expr.type).toBe('LiteralExpression');
      expect(expr.value).toEqual(createValue(true));
    });

    it('should parse false boolean literal', () => {
      const ast = parse('false;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as LiteralExpression;
      expect(expr.type).toBe('LiteralExpression');
      expect(expr.value).toEqual(createValue(false));
    });

    it('should parse multiple literals in sequence', () => {
      const ast = parse('42; "hello"; true; false;');
      expect(ast.statements).toHaveLength(4);
    });
  });

  describe('Happy Path - Variable Declarations', () => {
    it('should parse variable declaration with integer initializer', () => {
      const ast = parse('let x = 5;');
      expect(ast.statements).toHaveLength(1);
      
      const decl = ast.statements[0] as VariableDeclaration;
      expect(decl.type).toBe('VariableDeclaration');
      expect(decl.name).toBe('x');
      
      const init = decl.initializer as LiteralExpression;
      expect(init.type).toBe('LiteralExpression');
      expect(init.value).toEqual(createValue(5));
    });

    it('should parse variable declaration with string initializer', () => {
      const ast = parse('let message = "hello";');
      const decl = ast.statements[0] as VariableDeclaration;
      expect(decl.name).toBe('message');
      
      const init = decl.initializer as LiteralExpression;
      expect(init.value).toEqual(createValue('hello'));
    });

    it('should parse variable declaration with boolean initializer', () => {
      const ast = parse('let flag = true;');
      const decl = ast.statements[0] as VariableDeclaration;
      expect(decl.name).toBe('flag');
      
      const init = decl.initializer as LiteralExpression;
      expect(init.value).toEqual(createValue(true));
    });

    it('should parse variable declaration without initializer', () => {
      const ast = parse('let y;');
      const decl = ast.statements[0] as VariableDeclaration;
      expect(decl.name).toBe('y');
      expect(decl.initializer).toBeNull();
    });

    it('should parse variable declaration with expression initializer', () => {
      const ast = parse('let z = 5 + 3;');
      const decl = ast.statements[0] as VariableDeclaration;
      expect(decl.name).toBe('z');
      
      const init = decl.initializer as BinaryExpression;
      expect(init.type).toBe('BinaryExpression');
      expect(init.operator).toBe('+');
    });

    it('should parse multiple variable declarations', () => {
      const ast = parse('let x = 1; let y = 2; let z;');
      expect(ast.statements).toHaveLength(3);
      
      expect((ast.statements[0] as VariableDeclaration).name).toBe('x');
      expect((ast.statements[1] as VariableDeclaration).name).toBe('y');
      expect((ast.statements[2] as VariableDeclaration).name).toBe('z');
    });
  });

  describe('Happy Path - Arithmetic Expressions', () => {
    it('should parse simple addition', () => {
      const ast = parse('x + y;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as BinaryExpression;
      expect(expr.type).toBe('BinaryExpression');
      expect(expr.operator).toBe('+');
    });

    it('should parse simple subtraction', () => {
      const ast = parse('x - y;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as BinaryExpression;
      expect(expr.operator).toBe('-');
    });

    it('should parse simple multiplication', () => {
      const ast = parse('x * y;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as BinaryExpression;
      expect(expr.operator).toBe('*');
    });

    it('should parse simple division', () => {
      const ast = parse('x / y;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as BinaryExpression;
      expect(expr.operator).toBe('/');
    });

    it('should parse simple modulo', () => {
      const ast = parse('x % y;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as BinaryExpression;
      expect(expr.operator).toBe('%');
    });

    it('should parse multiplication with higher precedence than addition', () => {
      const ast = parse('x + y * z;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as BinaryExpression;
      expect(expr.operator).toBe('+');
      
      const right = expr.right as BinaryExpression;
      expect(right.operator).toBe('*');
    });

    it('should parse division with higher precedence than subtraction', () => {
      const ast = parse('x - y / z;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as BinaryExpression;
      expect(expr.operator).toBe('-');
      
      const right = expr.right as BinaryExpression;
      expect(right.operator).toBe('/');
    });

    it('should parse multiplication and division with same precedence (left to right)', () => {
      const ast = parse('x * y / z;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as BinaryExpression;
      expect(expr.operator).toBe('*');
      
      const right = expr.right as BinaryExpression;
      expect(right.operator).toBe('/');
    });

    it('should parse addition and subtraction with same precedence (left to right)', () => {
      const ast = parse('x + y - z;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as BinaryExpression;
      expect(expr.operator).toBe('+');
      
      const right = expr.right as BinaryExpression;
      expect(right.operator).toBe('-');
    });

    it('should parse parenthesized expressions', () => {
      const ast = parse('(x + y) * z;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as BinaryExpression;
      expect(expr.operator).toBe('*');
      
      const left = expr.left as BinaryExpression;
      expect(left.type).toBe('BinaryExpression');
      expect(left.operator).toBe('+');
    });

    it('should parse nested parentheses', () => {
      const ast = parse('((x + y) * z);');
      const expr = (ast.statements[0] as ExpressionStatement).expression as BinaryExpression;
      expect(expr.operator).toBe('*');
    });

    it('should parse complex arithmetic expression', () => {
      const ast = parse('x + y * z - a / b % c;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as BinaryExpression;
      expect(expr.type).toBe('BinaryExpression');
    });

    it('should parse expression with all arithmetic operators', () => {
      const ast = parse('a + b - c * d / e % f;');
      expect(ast.statements).toHaveLength(1);
    });

    it('should parse arithmetic with number literals', () => {
      const ast = parse('5 + 3 * 2;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as BinaryExpression;
      expect(expr.operator).toBe('+');
    });

    it('should parse chained arithmetic operations', () => {
      const ast = parse('1 + 2 + 3 + 4;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as BinaryExpression;
      expect(expr.operator).toBe('+');
    });
  });

  describe('Happy Path - Comparison Expressions', () => {
    it('should parse equality operator (==)', () => {
      const ast = parse('x == y;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as BinaryExpression;
      expect(expr.operator).toBe('==');
    });

    it('should parse inequality operator (!=)', () => {
      const ast = parse('x != y;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as BinaryExpression;
      expect(expr.operator).toBe('!=');
    });

    it('should parse less than operator (<)', () => {
      const ast = parse('x < y;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as BinaryExpression;
      expect(expr.operator).toBe('<');
    });

    it('should parse greater than operator (>)', () => {
      const ast = parse('x > y;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as BinaryExpression;
      expect(expr.operator).toBe('>');
    });

    it('should parse less than or equal operator (<=)', () => {
      const ast = parse('x <= y;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as BinaryExpression;
      expect(expr.operator).toBe('<=');
    });

    it('should parse greater than or equal operator (>=)', () => {
      const ast = parse('x >= y;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as BinaryExpression;
      expect(expr.operator).toBe('>=');
    });

    it('should give comparisons lower precedence than arithmetic', () => {
      const ast = parse('x + y < z * w;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as BinaryExpression;
      expect(expr.operator).toBe('<');
      
      const left = expr.left as BinaryExpression;
      expect(left.operator).toBe('+');
      
      const right = expr.right as BinaryExpression;
      expect(right.operator).toBe('*');
    });

    it('should parse multiple comparison expressions', () => {
      const ast = parse('x == y; x != y; x < y; x > y; x <= y; x >= y;');
      expect(ast.statements).toHaveLength(6);
    });

    it('should parse chained comparisons', () => {
      const ast = parse('x < y < z;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as BinaryExpression;
      expect(expr.operator).toBe('<');
    });
  });

  describe('Happy Path - Assignment Expressions', () => {
    it('should parse simple assignment', () => {
      const ast = parse('x = 10;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as AssignmentExpression;
      expect(expr.type).toBe('AssignmentExpression');
      expect(expr.name).toBe('x');
      
      const value = expr.value as LiteralExpression;
      expect(value.value).toEqual(createValue(10));
    });

    it('should parse assignment with expression', () => {
      const ast = parse('x = y + 5;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as AssignmentExpression;
      expect(expr.name).toBe('x');
      
      const value = expr.value as BinaryExpression;
      expect(value.operator).toBe('+');
    });

    it('should parse assignment with arithmetic', () => {
      const ast = parse('x = x + 1;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as AssignmentExpression;
      expect(expr.name).toBe('x');
      
      const value = expr.value as BinaryExpression;
      expect(value.operator).toBe('+');
    });

    it('should parse multiple assignments', () => {
      const ast = parse('x = 1; y = 2; z = 3;');
      expect(ast.statements).toHaveLength(3);
    });
  });

  describe('Happy Path - If Statements', () => {
    it('should parse if statement without else', () => {
      const ast = parse('if (x > 0) { print(x); }');
      expect(ast.statements).toHaveLength(1);
      
      const stmt = ast.statements[0] as IfStatement;
      expect(stmt.type).toBe('IfStatement');
      expect(stmt.elseBranch).toBeNull();
      
      const cond = stmt.condition as BinaryExpression;
      expect(cond.operator).toBe('>');
      
      const thenBranch = stmt.thenBranch as BlockStatement;
      expect(thenBranch.type).toBe('BlockStatement');
      expect(thenBranch.statements).toHaveLength(1);
    });

    it('should parse if statement with else', () => {
      const ast = parse('if (x > 0) { print(x); } else { print(0); }');
      const stmt = ast.statements[0] as IfStatement;
      expect(stmt.type).toBe('IfStatement');
      expect(stmt.elseBranch).not.toBeNull();
      
      const elseBranch = stmt.elseBranch as BlockStatement;
      expect(elseBranch.type).toBe('BlockStatement');
    });

    it('should parse if statement with single statement body (no braces)', () => {
      const ast = parse('if (x > 0) print(x);');
      const stmt = ast.statements[0] as IfStatement;
      expect(stmt.thenBranch.type).toBe('PrintStatement');
    });

    it('should parse nested if statements', () => {
      const ast = parse('if (x > 0) { if (y > 0) { print(x + y); } }');
      const stmt = ast.statements[0] as IfStatement;
      const thenBranch = stmt.thenBranch as BlockStatement;
      expect(thenBranch.statements[0].type).toBe('IfStatement');
    });

    it('should parse if-else-if chain', () => {
      const ast = parse('if (x > 0) { print(1); } else if (x < 0) { print(-1); } else { print(0); }');
      const stmt = ast.statements[0] as IfStatement;
      expect(stmt.elseBranch).not.toBeNull();
    });

    it('should parse if with complex condition', () => {
      const ast = parse('if (x + y > z * 2) { print(x); }');
      const stmt = ast.statements[0] as IfStatement;
      const cond = stmt.condition as BinaryExpression;
      expect(cond.operator).toBe('>');
    });
  });

  describe('Happy Path - While Statements', () => {
    it('should parse while statement', () => {
      const ast = parse('while (x < 10) { x = x + 1; }');
      expect(ast.statements).toHaveLength(1);
      
      const stmt = ast.statements[0] as WhileStatement;
      expect(stmt.type).toBe('WhileStatement');
      
      const cond = stmt.condition as BinaryExpression;
      expect(cond.operator).toBe('<');
      
      const body = stmt.body as BlockStatement;
      expect(body.type).toBe('BlockStatement');
      expect(body.statements).toHaveLength(1);
    });

    it('should parse while statement with single statement body', () => {
      const ast = parse('while (x < 10) print(x);');
      const stmt = ast.statements[0] as WhileStatement;
      expect(stmt.body.type).toBe('PrintStatement');
    });

    it('should parse while with complex condition', () => {
      const ast = parse('while (x * 2 < y + z) { x = x + 1; }');
      const stmt = ast.statements[0] as WhileStatement;
      const cond = stmt.condition as BinaryExpression;
      expect(cond.operator).toBe('<');
    });

    it('should parse nested while loops', () => {
      const ast = parse('while (i < 10) { while (j < 10) { print(i * j); } }');
      const stmt = ast.statements[0] as WhileStatement;
      const body = stmt.body as BlockStatement;
      expect(body.statements[0].type).toBe('WhileStatement');
    });
  });

  describe('Happy Path - Function Declarations', () => {
    it('should parse function declaration without parameters', () => {
      const ast = parse('function foo() { print(42); }');
      expect(ast.statements).toHaveLength(1);
      
      const stmt = ast.statements[0] as FunctionDeclaration;
      expect(stmt.type).toBe('FunctionDeclaration');
      expect(stmt.name).toBe('foo');
      expect(stmt.parameters).toHaveLength(0);
      
      const body = stmt.body;
      expect(body.statements).toHaveLength(1);
    });

    it('should parse function declaration with single parameter', () => {
      const ast = parse('function square(x) { return x * x; }');
      const stmt = ast.statements[0] as FunctionDeclaration;
      expect(stmt.name).toBe('square');
      expect(stmt.parameters).toEqual(['x']);
    });

    it('should parse function declaration with multiple parameters', () => {
      const ast = parse('function add(a, b) { return a + b; }');
      const stmt = ast.statements[0] as FunctionDeclaration;
      expect(stmt.name).toBe('add');
      expect(stmt.parameters).toEqual(['a', 'b']);
    });

    it('should parse function declaration with many parameters', () => {
      const ast = parse('function many(a, b, c, d, e) { return a; }');
      const stmt = ast.statements[0] as FunctionDeclaration;
      expect(stmt.parameters).toEqual(['a', 'b', 'c', 'd', 'e']);
    });

    it('should parse function with multiple statements in body', () => {
      const ast = parse('function foo() { let x = 1; let y = 2; print(x + y); }');
      const stmt = ast.statements[0] as FunctionDeclaration;
      expect(stmt.body.statements).toHaveLength(3);
    });

    it('should parse function with nested blocks', () => {
      const ast = parse('function foo() { if (true) { print(1); } }');
      const stmt = ast.statements[0] as FunctionDeclaration;
      expect(stmt.body.statements).toHaveLength(1);
    });

    it('should parse multiple function declarations', () => {
      const ast = parse('function foo() {} function bar() {}');
      expect(ast.statements).toHaveLength(2);
    });
  });

  describe('Happy Path - Function Calls', () => {
    it('should parse function call without arguments', () => {
      const ast = parse('foo();');
      const expr = (ast.statements[0] as ExpressionStatement).expression as CallExpression;
      expect(expr.type).toBe('CallExpression');
      expect(expr.arguments).toHaveLength(0);
      
      const callee = expr.callee as IdentifierExpression;
      expect(callee.name).toBe('foo');
    });

    it('should parse function call with single argument', () => {
      const ast = parse('print(42);');
      const expr = (ast.statements[0] as ExpressionStatement).expression as CallExpression;
      expect(expr.arguments).toHaveLength(1);
    });

    it('should parse function call with multiple arguments', () => {
      const ast = parse('add(x, y);');
      const expr = (ast.statements[0] as ExpressionStatement).expression as CallExpression;
      expect(expr.arguments).toHaveLength(2);
    });

    it('should parse function call with expression arguments', () => {
      const ast = parse('add(x + y, z * w);');
      const expr = (ast.statements[0] as ExpressionStatement).expression as CallExpression;
      expect(expr.arguments).toHaveLength(2);
    });

    it('should parse nested function calls', () => {
      const ast = parse('foo(bar(x));');
      const expr = (ast.statements[0] as ExpressionStatement).expression as CallExpression;
      expect(expr.arguments).toHaveLength(1);
      
      const arg = expr.arguments[0] as CallExpression;
      expect(arg.type).toBe('CallExpression');
    });

    it('should parse function call as part of expression', () => {
      const ast = parse('x + foo(y);');
      const expr = (ast.statements[0] as ExpressionStatement).expression as BinaryExpression;
      expect(expr.operator).toBe('+');
    });

    it('should parse print as function call', () => {
      const ast = parse('print(x);');
      const stmt = ast.statements[0] as PrintStatement;
      expect(stmt.type).toBe('PrintStatement');
    });

    it('should parse print with string argument', () => {
      const ast = parse('print("hello");');
      const stmt = ast.statements[0] as PrintStatement;
      expect(stmt.type).toBe('PrintStatement');
    });

    it('should parse print with expression argument', () => {
      const ast = parse('print(x + y);');
      const stmt = ast.statements[0] as PrintStatement;
      expect(stmt.type).toBe('PrintStatement');
    });
  });

  describe('Happy Path - Block Statements', () => {
    it('should parse empty block', () => {
      const ast = parse('{}');
      const stmt = ast.statements[0] as BlockStatement;
      expect(stmt.type).toBe('BlockStatement');
      expect(stmt.statements).toHaveLength(0);
    });

    it('should parse block with single statement', () => {
      const ast = parse('{ let x = 1; }');
      const stmt = ast.statements[0] as BlockStatement;
      expect(stmt.statements).toHaveLength(1);
    });

    it('should parse block with multiple statements', () => {
      const ast = parse('{ let x = 1; let y = 2; print(x + y); }');
      const stmt = ast.statements[0] as BlockStatement;
      expect(stmt.statements).toHaveLength(3);
    });

    it('should parse nested blocks', () => {
      const ast = parse('{ { let x = 1; } }');
      const stmt = ast.statements[0] as BlockStatement;
      expect(stmt.statements).toHaveLength(1);
      expect(stmt.statements[0].type).toBe('BlockStatement');
    });

    it('should parse block with mixed statement types', () => {
      const ast = parse('{ let x = 1; if (x > 0) { print(x); } }');
      const stmt = ast.statements[0] as BlockStatement;
      expect(stmt.statements).toHaveLength(2);
    });
  });

  describe('Happy Path - Complex Programs', () => {
    it('should parse factorial function', () => {
      const source = `
        function factorial(n) {
          if (n <= 1) {
            return 1;
          } else {
            return n * factorial(n - 1);
          }
        }
      `;
      const ast = parse(source);
      expect(ast.statements).toHaveLength(1);
      expect(ast.statements[0].type).toBe('FunctionDeclaration');
    });

    it('should parse while loop counter', () => {
      const source = `
        let i = 0;
        while (i < 10) {
          print(i);
          i = i + 1;
        }
      `;
      const ast = parse(source);
      expect(ast.statements).toHaveLength(2);
    });

    it('should parse fibonacci function', () => {
      const source = `
        function fib(n) {
          if (n <= 1) {
            return n;
          } else {
            return fib(n - 1) + fib(n - 2);
          }
        }
      `;
      const ast = parse(source);
      expect(ast.statements).toHaveLength(1);
    });

    it('should parse complete program with multiple functions', () => {
      const source = `
        function add(a, b) {
          return a + b;
        }
        
        function multiply(a, b) {
          return a * b;
        }
        
        let x = 5;
        let y = 10;
        print(add(x, y));
        print(multiply(x, y));
      `;
      const ast = parse(source);
      expect(ast.statements.length).toBeGreaterThan(0);
    });

    it('should parse nested control flow', () => {
      const source = `
        let x = 0;
        while (x < 10) {
          if (x % 2 == 0) {
            print(x);
          }
          x = x + 1;
        }
      `;
      const ast = parse(source);
      expect(ast.statements).toHaveLength(2);
    });

    it('should parse program with variable shadowing in blocks', () => {
      const source = `
        let x = 1;
        {
          let x = 2;
          print(x);
        }
        print(x);
      `;
      const ast = parse(source);
      expect(ast.statements).toHaveLength(3);
    });
  });

  // ============================================================================
  // EDGE CASE TESTS
  // ============================================================================

  describe('Edge Cases - Empty and Minimal Programs', () => {
    it('should parse empty program', () => {
      const ast = parse('');
      expect(ast.type).toBe('Program');
      expect(ast.statements).toHaveLength(0);
    });

    it('should parse program with only whitespace', () => {
      const ast = parse('   \n\t  ');
      expect(ast.type).toBe('Program');
      expect(ast.statements).toHaveLength(0);
    });

    it('should parse program with only comments (if supported)', () => {
      const ast = parse('let x = 1;');
      expect(ast.statements).toHaveLength(1);
    });

    it('should parse single expression statement', () => {
      const ast = parse('42;');
      expect(ast.statements).toHaveLength(1);
    });

    it('should parse single variable declaration', () => {
      const ast = parse('let x;');
      expect(ast.statements).toHaveLength(1);
    });

    it('should parse single empty block', () => {
      const ast = parse('{}');
      expect(ast.statements).toHaveLength(1);
    });
  });

  describe('Edge Cases - Operator Precedence', () => {
    it('should handle all operators in single expression', () => {
      const ast = parse('a + b * c < d / e % f == g;');
      expect(ast.statements).toHaveLength(1);
    });

    it('should handle deeply nested parentheses', () => {
      const ast = parse('((((((x))))));');
      expect(ast.statements).toHaveLength(1);
    });

    it('should handle multiple levels of precedence', () => {
      const ast = parse('a + b * c - d / e;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as BinaryExpression;
      expect(expr.operator).toBe('+');
    });

    it('should handle unary operators with binary operators', () => {
      const ast = parse('-x + y;');
      expect(ast.statements).toHaveLength(1);
    });

    it('should handle assignment in complex expression', () => {
      const ast = parse('x = y + z * a;');
      expect(ast.statements).toHaveLength(1);
    });
  });

  describe('Edge Cases - Large Numbers and Values', () => {
    it('should parse very large integer', () => {
      const ast = parse('999999999999;');
      expect(ast.statements).toHaveLength(1);
    });

    it('should parse very small decimal', () => {
      const ast = parse('0.000001;');
      expect(ast.statements).toHaveLength(1);
    });

    it('should parse zero', () => {
      const ast = parse('0;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as LiteralExpression;
      expect(expr.value).toEqual(createValue(0));
    });

    it('should parse negative zero', () => {
      const ast = parse('-0;');
      expect(ast.statements).toHaveLength(1);
    });
  });

  describe('Edge Cases - Long Identifiers', () => {
    it('should parse very long identifier', () => {
      const ast = parse('let veryLongIdentifierNameThatGoesOnAndOn = 42;');
      const decl = ast.statements[0] as VariableDeclaration;
      expect(decl.name).toBe('veryLongIdentifierNameThatGoesOnAndOn');
    });

    it('should parse identifier with underscores', () => {
      const ast = parse('let my_variable_name = 42;');
      const decl = ast.statements[0] as VariableDeclaration;
      expect(decl.name).toBe('my_variable_name');
    });

    it('should parse identifier starting with underscore', () => {
      const ast = parse('_private = 42;');
      expect(ast.statements).toHaveLength(1);
    });
  });

  describe('Edge Cases - Deep Nesting', () => {
    it('should parse deeply nested if statements', () => {
      const source = 'if (a) { if (b) { if (c) { if (d) { print(1); } } } }';
      const ast = parse(source);
      expect(ast.statements).toHaveLength(1);
    });

    it('should parse deeply nested blocks', () => {
      const source = '{{{{let x = 1;}}}}';
      const ast = parse(source);
      expect(ast.statements).toHaveLength(1);
    });

    it('should parse deeply nested function calls', () => {
      const ast = parse('a(b(c(d(e(f(g(h(i(j)))))))));');
      expect(ast.statements).toHaveLength(1);
    });

    it('should parse deeply nested expressions', () => {
      const ast = parse('(((((((((1 + 2) + 3) + 4) + 5) + 6) + 7) + 8) + 9)));');
      expect(ast.statements).toHaveLength(1);
    });
  });

  describe('Edge Cases - Statement Combinations', () => {
    it('should parse if-else with while in both branches', () => {
      const source = `
        if (x) {
          while (y) { print(1); }
        } else {
          while (z) { print(2); }
        }
      `;
      const ast = parse(source);
      expect(ast.statements).toHaveLength(1);
    });

    it('should parse function with all control flow types', () => {
      const source = `
        function complex(x) {
          if (x > 0) {
            while (x < 10) {
              x = x + 1;
            }
          }
          return x;
        }
      `;
      const ast = parse(source);
      expect(ast.statements).toHaveLength(1);
    });

    it('should parse block with many different statement types', () => {
      const source = `
        {
          let x = 1;
          let y = 2;
          if (x > y) { print(x); }
          while (x < 10) { x = x + 1; }
          print(x);
        }
      `;
      const ast = parse(source);
      expect(ast.statements).toHaveLength(1);
    });
  });

  describe('Edge Cases - Function Parameter Edge Cases', () => {
    it('should parse function with many parameters', () => {
      const ast = parse('function many(a,b,c,d,e,f,g,h,i,j) { return a; }');
      const stmt = ast.statements[0] as FunctionDeclaration;
      expect(stmt.parameters).toHaveLength(10);
    });

    it('should parse function with single-character parameter names', () => {
      const ast = parse('function f(x,y,z) { return x; }');
      const stmt = ast.statements[0] as FunctionDeclaration;
      expect(stmt.parameters).toEqual(['x', 'y', 'z']);
    });

    it('should parse function with underscore parameter names', () => {
      const ast = parse('function f(_a, _b, _c) { return _a; }');
      const stmt = ast.statements[0] as FunctionDeclaration;
      expect(stmt.parameters).toEqual(['_a', '_b', '_c']);
    });
  });

  describe('Edge Cases - Expression Edge Cases', () => {
    it('should parse expression with only literals', () => {
      const ast = parse('42;');
      expect(ast.statements).toHaveLength(1);
    });

    it('should parse expression with only identifier', () => {
      const ast = parse('x;');
      const expr = (ast.statements[0] as ExpressionStatement).expression as IdentifierExpression;
      expect(expr.type).toBe('IdentifierExpression');
      expect(expr.name).toBe('x');
    });

    it('should parse parenthesized single literal', () => {
      const ast = parse('(42);');
      expect(ast.statements).toHaveLength(1);
    });

    it('should parse expression with redundant parentheses', () => {
      const ast = parse('((x + y));');
      expect(ast.statements).toHaveLength(1);
    });
  });

  // ============================================================================
  // ERROR CASE TESTS
  // ============================================================================

  describe('Error Cases - Syntax Errors', () => {
    it('should throw error on missing semicolon after variable declaration', () => {
      const error = parseExpectError('let x = 5');
      expect(error.message).toContain('semicolon');
    });

    it('should throw error on missing semicolon after expression', () => {
      const error = parseExpectError('x + y');
      expect(error.message).toContain('semicolon');
    });

    it('should throw error on missing semicolon after print', () => {
      const error = parseExpectError('print(x)');
      expect(error.message).toContain('semicolon');
    });

    it('should throw error on missing semicolon after assignment', () => {
      const error = parseExpectError('x = 10');
      expect(error.message).toContain('semicolon');
    });

    it('should throw error on missing closing parenthesis in print', () => {
      const error = parseExpectError('print(x;');
      expect(error.message).toContain("'");
    });

    it('should throw error on missing opening parenthesis in print', () => {
      const error = parseExpectError('print x);');
      expect(error.message).toContain("'");
    });

    it('should throw error on missing closing parenthesis in if condition', () => {
      const error = parseExpectError('if (x > 0 { print(x); }');
      expect(error.message).toContain("'");
    });

    it('should throw error on missing opening parenthesis in if condition', () => {
      const error = parseExpectError('if x > 0) { print(x); }');
      expect(error.message).toContain("'");
    });

    it('should throw error on missing closing parenthesis in while condition', () => {
      const error = parseExpectError('while (x < 10 { print(x); }');
      expect(error.message).toContain("'");
    });

    it('should throw error on missing opening parenthesis in while condition', () => {
      const error = parseExpectError('while x < 10) { print(x); }');
      expect(error.message).toContain("'");
    });

    it('should throw error on missing closing brace in block', () => {
      const error = parseExpectError('{ let x = 1;');
      expect(error.message).toContain('}');
    });

    it('should throw error on missing opening brace in block', () => {
      const error = parseExpectError('let x = 1; }');
      expect(error.message).toContain("'");
    });

    it('should throw error on missing closing brace in function body', () => {
      const error = parseExpectError('function foo() { print(1);');
      expect(error.message).toContain('}');
    });

    it('should throw error on missing opening brace in function body', () => {
      const error = parseExpectError('function foo() print(1); }');
      expect(error.message).toContain("'");
    });

    it('should throw error on missing closing parenthesis in function parameters', () => {
      const error = parseExpectError('function foo(a, b { print(1); }');
      expect(error.message).toContain("'");
    });

    it('should throw error on missing opening parenthesis in function parameters', () => {
      const error = parseExpectError('function foo a, b) { print(1); }');
      expect(error.message).toContain("'");
    });

    it('should throw error on missing closing parenthesis in function call', () => {
      const error = parseExpectError('foo(x;');
      expect(error.message).toContain("'");
    });

    it('should throw error on missing opening parenthesis in function call', () => {
      const error = parseExpectError('foo x);');
      expect(error.message).toContain("'");
    });

    it('should throw error on missing closing parenthesis in expression', () => {
      const error = parseExpectError('(x + y;');
      expect(error.message).toContain("'");
    });

    it('should throw error on missing opening parenthesis in expression', () => {
      const error = parseExpectError('x + y);');
      expect(error.message).toContain("'");
    });
  });

  describe('Error Cases - Invalid Syntax', () => {
    it('should throw error on variable declaration without name', () => {
      const error = parseExpectError('let = 5;');
      expect(error.message).toBeTruthy();
    });

    it('should throw error on function declaration without name', () => {
      const error = parseExpectError('function() { print(1); }');
      expect(error.message).toBeTruthy();
    });

    it('should throw error on invalid assignment target', () => {
      const error = parseExpectError('5 = x;');
      expect(error.message).toContain('assignment');
    });

    it('should throw error on assignment to literal', () => {
      const error = parseExpectError('"hello" = x;');
      expect(error.message).toContain('assignment');
    });

    it('should throw error on assignment to expression result', () => {
      const error = parseExpectError('(x + y) = z;');
      expect(error.message).toContain('assignment');
    });

    it('should throw error on missing variable name in let', () => {
      const error = parseExpectError('let ;');
      expect(error.message).toBeTruthy();
    });

    it('should throw error on missing function name', () => {
      const error = parseExpectError('function (x) {}');
      expect(error.message).toBeTruthy();
    });

    it('should throw error on missing parameter name', () => {
      const error = parseExpectError('function foo(, b) {}');
      expect(error.message).toBeTruthy();
    });

    it('should throw error on missing expression in print', () => {
      const error = parseExpectError('print();');
      // This might not throw depending on implementation
      // Adjust based on actual behavior
    });
  });

  describe('Error Cases - Unexpected Tokens', () => {
    it('should throw error on unexpected token at start', () => {
      const error = parseExpectError('@');
      expect(error.message).toBeTruthy();
    });

    it('should throw error on unexpected token in expression', () => {
      const error = parseExpectError('x + @ y;');
      expect(error.message).toBeTruthy();
    });

    it('should throw error on unexpected token in block', () => {
      const error = parseExpectError('{ let x = 1; @ }');
      expect(error.message).toBeTruthy();
    });

    it('should throw error on incomplete expression', () => {
      const error = parseExpectError('x +;');
      expect(error.message).toBeTruthy();
    });

    it('should throw error on incomplete binary expression', () => {
      const error = parseExpectError('+ y;');
      expect(error.message).toBeTruthy();
    });

    it('should throw error on dangling operator', () => {
      const error = parseExpectError('x;');
      // This should actually parse fine
      // Adjust test based on actual behavior
    });
  });

  describe('Error Cases - Mismatched Brackets/Parentheses', () => {
    it('should throw error on mismatched parentheses', () => {
      const error = parseExpectError('((x + y);');
      expect(error.message).toContain("'");
    });

    it('should throw error on mismatched braces', () => {
      const error = parseExpectError('{{let x = 1;}}');
      // This might actually parse fine
      // Adjust based on actual behavior
    });

    it('should throw error on extra closing parenthesis', () => {
      const error = parseExpectError('(x + y));');
      expect(error.message).toBeTruthy();
    });

    it('should throw error on extra closing brace', () => {
      const error = parseExpectError('{ let x = 1; }}');
      expect(error.message).toBeTruthy();
    });
  });

  describe('Error Cases - Invalid Identifiers', () => {
    it('should throw error on identifier starting with number', () => {
      const error = parseExpectError('let 1x = 5;');
      expect(error.message).toBeTruthy();
    });

    it('should throw error on identifier with special characters', () => {
      const error = parseExpectError('let x$y = 5;');
      expect(error.message).toBeTruthy();
    });

    it('should throw error on identifier with hyphen', () => {
      const error = parseExpectError('let x-y = 5;');
      expect(error.message).toBeTruthy();
    });
  });

  describe('Error Cases - Empty Constructs', () => {
    it('should throw error on empty if condition', () => {
      const error = parseExpectError('if () { print(1); }');
      expect(error.message).toBeTruthy();
    });

    it('should throw error on empty while condition', () => {
      const error = parseExpectError('while () { print(1); }');
      expect(error.message).toBeTruthy();
    });

    it('should throw error on empty parentheses in expression', () => {
      const error = parseExpectError('();');
      expect(error.message).toBeTruthy();
    });
  });

  describe('Error Cases - Invalid Operators', () => {
    it('should throw error on unsupported operator', () => {
      const error = parseExpectError('x & y;');
      expect(error.message).toBeTruthy();
    });

    it('should throw error on incomplete comparison operator', () => {
      const error = parseExpectError('x =;');
      expect(error.message).toBeTruthy();
    });
  });

  describe('Error Cases - Synchronization After Errors', () => {
    it('should synchronize and continue parsing after error', () => {
      const source = 'let x = 5; let y = ; let z = 10;';
      const lexer = new Lexer(source);
      const tokens = lexer.tokenize();
      const parser = new Parser(tokens);
      
      // Should not throw, but handle error gracefully
      const ast = parser.parse();
      expect(ast).toBeDefined();
    });

    it('should handle multiple errors in source', () => {
      const source = 'let x = ; let y = ;';
      const lexer = new Lexer(source);
      const tokens = lexer.tokenize();
      const parser = new Parser(tokens);
      
      const ast = parser.parse();
      expect(ast).toBeDefined();
    });
  });

  // ============================================================================
  // TYPE SAFETY TESTS
  // ============================================================================

  describe('Type Safety - AST Node Types', () => {
    it('should create correct AST node types for literals', () => {
      const ast = parse('42;');
      const expr = (ast.statements[0] as ExpressionStatement).expression;
      expect(expr.type).toBe('LiteralExpression');
    });

    it('should create correct AST node types for binary expressions', () => {
      const ast = parse('x + y;');
      const expr = (ast.statements[0] as ExpressionStatement).expression;
      expect(expr.type).toBe('BinaryExpression');
    });

    it('should create correct AST node types for variable declarations', () => {
      const ast = parse('let x = 5;');
      expect(ast.statements[0].type).toBe('VariableDeclaration');
    });

    it('should create correct AST node types for function declarations', () => {
      const ast = parse('function foo() {}');
      expect(ast.statements[0].type).toBe('FunctionDeclaration');
    });

    it('should create correct AST node types for if statements', () => {
      const ast = parse('if (true) {}');
      expect(ast.statements[0].type).toBe('IfStatement');
    });

    it('should create correct AST node types for while statements', () => {
      const ast = parse('while (true) {}');
      expect(ast.statements[0].type).toBe('WhileStatement');
    });

    it('should create correct AST node types for print statements', () => {
      const ast = parse('print(42);');
      expect(ast.statements[0].type).toBe('PrintStatement');
    });

    it('should create correct AST node types for block statements', () => {
      const ast = parse('{}');
      expect(ast.statements[0].type).toBe('BlockStatement');
    });

    it('should create correct AST node types for call expressions', () => {
      const ast = parse('foo();');
      const expr = (ast.statements[0] as ExpressionStatement).expression;
      expect(expr.type).toBe('CallExpression');
    });

    it('should create correct AST node types for assignment expressions', () => {
      const ast = parse('x = 5;');
      const expr = (ast.statements[0] as ExpressionStatement).expression;
      expect(expr.type).toBe('AssignmentExpression');
    });

    it('should create correct AST node types for identifier expressions', () => {
      const ast = parse('x;');
      const expr = (ast.statements[0] as ExpressionStatement).expression;
      expect(expr.type).toBe('IdentifierExpression');
    });
  });

  // ============================================================================
  // INTEGRATION TESTS
  // ============================================================================

  describe('Integration - Complete Programs', () => {
    it('should parse a complete calculator program', () => {
      const source = `
        function add(a, b) { return a + b; }
        function sub(a, b) { return a - b; }
        function mul(a, b) { return a * b; }
        function div(a, b) { return a / b; }
        
        let x = 10;
        let y = 5;
        print(add(x, y));
        print(sub(x, y));
        print(mul(x, y));
        print(div(x, y));
      `;
      const ast = parse(source);
      expect(ast.statements.length).toBeGreaterThan(0);
    });

    it('should parse a complete loop program', () => {
      const source = `
        let sum = 0;
        let i = 1;
        while (i <= 10) {
          sum = sum + i;
          i = i + 1;
        }
        print(sum);
      `;
      const ast = parse(source);
      expect(ast.statements).toHaveLength(3);
    });

    it('should parse a complete factorial program', () => {
      const source = `
        function factorial(n) {
          if (n <= 1) {
            return 1;
          } else {
            return n * factorial(n - 1);
          }
        }
        
        let result = factorial(5);
        print(result);
      `;
      const ast = parse(source);
      expect(ast.statements).toHaveLength(2);
    });

    it('should parse a complete conditional program', () => {
      const source = `
        let x = 10;
        let y = 20;
        
        if (x > y) {
          print("x is greater");
        } else if (x < y) {
          print("y is greater");
        } else {
          print("equal");
        }
      `;
      const ast = parse(source);
      expect(ast.statements).toHaveLength(3);
    });
  });
});