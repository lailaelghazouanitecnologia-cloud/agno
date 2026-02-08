/**
 * Comprehensive Tests for C Compiler using bun:test
 */

import { describe, it, expect, beforeAll, afterAll } from "bun:test";
import { CCompiler } from "./compiler.js";
import { Lexer } from "./lexer.js";
import { Parser } from "./parser.js";
import { TokenType } from "./interfaces.js";

describe("C Compiler", () => {
  let compiler: CCompiler;

  beforeAll(() => {
    compiler = new CCompiler({ debug: false });
  });

  afterAll(() => {
    compiler.reset();
  });

  describe("Lexer", () => {
    it("should tokenize integers", () => {
      const lexer = new Lexer("42");
      const tokens = lexer.tokenize();
      expect(tokens[0].type).toBe(TokenType.NUMBER);
      expect(tokens[0].value).toBe("42");
    });

    it("should tokenize floats", () => {
      const lexer = new Lexer("3.14");
      const tokens = lexer.tokenize();
      expect(tokens[0].type).toBe(TokenType.NUMBER);
      expect(tokens[0].value).toBe("3.14");
    });

    it("should tokenize identifiers", () => {
      const lexer = new Lexer("myVariable");
      const tokens = lexer.tokenize();
      expect(tokens[0].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[0].value).toBe("myVariable");
    });

    it("should tokenize keywords", () => {
      const lexer = new Lexer("int if while return");
      const tokens = lexer.tokenize();
      expect(tokens[0].type).toBe(TokenType.INT);
      expect(tokens[1].type).toBe(TokenType.IF);
      expect(tokens[2].type).toBe(TokenType.WHILE);
      expect(tokens[3].type).toBe(TokenType.RETURN);
    });

    it("should tokenize operators", () => {
      const lexer = new Lexer("+ - * / %");
      const tokens = lexer.tokenize();
      expect(tokens[0].type).toBe(TokenType.PLUS);
      expect(tokens[1].type).toBe(TokenType.MINUS);
      expect(tokens[2].type).toBe(TokenType.STAR);
      expect(tokens[3].type).toBe(TokenType.SLASH);
      expect(tokens[4].type).toBe(TokenType.PERCENT);
    });

    it("should tokenize comparison operators", () => {
      const lexer = new Lexer("< > <= >= == !=");
      const tokens = lexer.tokenize();
      expect(tokens[0].type).toBe(TokenType.LESS);
      expect(tokens[1].type).toBe(TokenType.GREATER);
      expect(tokens[2].type).toBe(TokenType.LESS_EQUAL);
      expect(tokens[3].type).toBe(TokenType.GREATER_EQUAL);
      expect(tokens[4].type).toBe(TokenType.EQUAL);
      expect(tokens[5].type).toBe(TokenType.NOT_EQUAL);
    });

    it("should tokenize string literals", () => {
      const lexer = new Lexer('"Hello, World!"');
      const tokens = lexer.tokenize();
      expect(tokens[0].type).toBe(TokenType.STRING);
      expect(tokens[0].value).toBe("Hello, World!");
    });

    it("should tokenize punctuation", () => {
      const lexer = new Lexer("( ) { } [ ] ; ,");
      const tokens = lexer.tokenize();
      expect(tokens[0].type).toBe(TokenType.LPAREN);
      expect(tokens[1].type).toBe(TokenType.RPAREN);
      expect(tokens[2].type).toBe(TokenType.LBRACE);
      expect(tokens[3].type).toBe(TokenType.RBRACE);
      expect(tokens[4].type).toBe(TokenType.LBRACKET);
      expect(tokens[5].type).toBe(TokenType.RBRACKET);
      expect(tokens[6].type).toBe(TokenType.SEMICOLON);
      expect(tokens[7].type).toBe(TokenType.COMMA);
    });

    it("should handle comments", () => {
      const lexer = new Lexer("42 // This is a comment\n24");
      const tokens = lexer.tokenize();
      expect(tokens[0].type).toBe(TokenType.NUMBER);
      expect(tokens[0].value).toBe("42");
      expect(tokens[1].type).toBe(TokenType.NUMBER);
      expect(tokens[1].value).toBe("24");
    });

    it("should track line and column numbers", () => {
      const lexer = new Lexer("int x;\nfloat y;");
      const tokens = lexer.tokenize();
      expect(tokens[0].line).toBe(1);
      expect(tokens[4].line).toBe(2);
    });
  });

  describe("Parser", () => {
    it("should parse a simple program", () => {
      const source = `
        int main() {
          return 0;
        }
      `;
      const lexer = new Lexer(source);
      const tokens = lexer.tokenize();
      const parser = new Parser(tokens);
      const ast = parser.parse();

      expect(ast.type).toBe("PROGRAM");
      expect(ast.declarations).toHaveLength(1);
      expect(ast.declarations[0].type).toBe("FUNCTION_DECL");
      expect(ast.declarations[0].name).toBe("main");
    });

    it("should parse variable declarations", () => {
      const source = `
        int main() {
          int x;
          float y;
          char c;
          return 0;
        }
      `;
      const lexer = new Lexer(source);
      const tokens = lexer.tokenize();
      const parser = new Parser(tokens);
      const ast = parser.parse();

      const mainFunc = ast.declarations[0];
      expect(mainFunc.body.statements).toHaveLength(4);
    });

    it("should parse variable initialization", () => {
      const source = `
        int main() {
          int x = 42;
          return 0;
        }
      `;
      const lexer = new Lexer(source);
      const tokens = lexer.tokenize();
      const parser = new Parser(tokens);
      const ast = parser.parse();

      const mainFunc = ast.declarations[0];
      const varDecl = mainFunc.body.statements[0];
      expect(varDecl.type).toBe("VAR_DECL");
      expect(varDecl.name).toBe("x");
    });

    it("should parse if statements", () => {
      const source = `
        int main() {
          if (x > 0) {
            return 1;
          }
          return 0;
        }
      `;
      const lexer = new Lexer(source);
      const tokens = lexer.tokenize();
      const parser = new Parser(tokens);
      const ast = parser.parse();

      const mainFunc = ast.declarations[0];
      const ifStmt = mainFunc.body.statements[0];
      expect(ifStmt.type).toBe("IF_STMT");
    });

    it("should parse if-else statements", () => {
      const source = `
        int main() {
          if (x > 0) {
            return 1;
          } else {
            return -1;
          }
        }
      `;
      const lexer = new Lexer(source);
      const tokens = lexer.tokenize();
      const parser = new Parser(tokens);
      const ast = parser.parse();

      const mainFunc = ast.declarations[0];
      const ifStmt = mainFunc.body.statements[0];
      expect(ifStmt.type).toBe("IF_STMT");
      expect(ifStmt.elseBranch).toBeDefined();
    });

    it("should parse while loops", () => {
      const source = `
        int main() {
          while (x < 10) {
            x = x + 1;
          }
          return 0;
        }
      `;
      const lexer = new Lexer(source);
      const tokens = lexer.tokenize();
      const parser = new Parser(tokens);
      const ast = parser.parse();

      const mainFunc = ast.declarations[0];
      const whileStmt = mainFunc.body.statements[0];
      expect(whileStmt.type).toBe("WHILE_STMT");
    });

    it("should parse for loops", () => {
      const source = `
        int main() {
          for (int i = 0; i < 10; i = i + 1) {
            x = x + i;
          }
          return 0;
        }
      `;
      const lexer = new Lexer(source);
      const tokens = lexer.tokenize();
      const parser = new Parser(tokens);
      const ast = parser.parse();

      const mainFunc = ast.declarations[0];
      const forStmt = mainFunc.body.statements[0];
      expect(forStmt.type).toBe("FOR_STMT");
    });

    it("should parse function declarations with parameters", () => {
      const source = `
        int add(int a, int b) {
          return a + b;
        }

        int main() {
          return 0;
        }
      `;
      const lexer = new Lexer(source);
      const tokens = lexer.tokenize();
      const parser = new Parser(tokens);
      const ast = parser.parse();

      expect(ast.declarations).toHaveLength(2);
      const addFunc = ast.declarations[0];
      expect(addFunc.name).toBe("add");
      expect(addFunc.params).toHaveLength(2);
    });

    it("should parse function calls", () => {
      const source = `
        int main() {
          int x = foo(42, 24);
          return 0;
        }
      `;
      const lexer = new Lexer(source);
      const tokens = lexer.tokenize();
      const parser = new Parser(tokens);
      const ast = parser.parse();

      const mainFunc = ast.declarations[0];
      const varDecl = mainFunc.body.statements[0];
      expect(varDecl.type).toBe("VAR_DECL");
    });

    it("should parse binary expressions", () => {
      const source = `
        int main() {
          int x = 1 + 2 * 3;
          return 0;
        }
      `;
      const lexer = new Lexer(source);
      const tokens = lexer.tokenize();
      const parser = new Parser(tokens);
      const ast = parser.parse();

      const mainFunc = ast.declarations[0];
      expect(mainFunc.body.statements).toHaveLength(2);
    });
  });

  describe("Compiler - Basic Programs", () => {
    it("should compile and run a simple main function", () => {
      const source = `
        int main() {
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(true);
      expect(result.errors).toHaveLength(0);
    });

    it("should handle variable declarations", () => {
      const source = `
        int main() {
          int x;
          int y;
          int z;
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(true);
    });

    it("should handle variable initialization", () => {
      const source = `
        int main() {
          int x = 42;
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(true);
    });
  });

  describe("Compiler - Arithmetic Operations", () => {
    it("should handle addition", () => {
      const source = `
        int main() {
          int x = 10 + 20;
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(true);
    });

    it("should handle subtraction", () => {
      const source = `
        int main() {
          int x = 20 - 10;
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(true);
    });

    it("should handle multiplication", () => {
      const source = `
        int main() {
          int x = 5 * 6;
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(true);
    });

    it("should handle division", () => {
      const source = `
        int main() {
          int x = 20 / 4;
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(true);
    });

    it("should handle modulo", () => {
      const source = `
        int main() {
          int x = 17 % 5;
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(true);
    });

    it("should handle operator precedence", () => {
      const source = `
        int main() {
          int x = 1 + 2 * 3;
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(true);
    });

    it("should handle unary negation", () => {
      const source = `
        int main() {
          int x = -42;
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(true);
    });
  });

  describe("Compiler - Comparison Operations", () => {
    it("should handle less than", () => {
      const source = `
        int main() {
          int x = 5 < 10;
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(true);
    });

    it("should handle greater than", () => {
      const source = `
        int main() {
          int x = 10 > 5;
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(true);
    });

    it("should handle equality", () => {
      const source = `
        int main() {
          int x = 5 == 5;
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(true);
    });

    it("should handle inequality", () => {
      const source = `
        int main() {
          int x = 5 != 10;
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(true);
    });
  });

  describe("Compiler - Control Flow", () => {
    it("should handle if statements", () => {
      const source = `
        int main() {
          int x = 10;
          if (x > 5) {
            x = 20;
          }
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(true);
    });

    it("should handle if-else statements", () => {
      const source = `
        int main() {
          int x = 10;
          if (x > 20) {
            x = 5;
          } else {
            x = 15;
          }
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(true);
    });

    it("should handle while loops", () => {
      const source = `
        int main() {
          int x = 0;
          while (x < 5) {
            x = x + 1;
          }
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(true);
    });

    it("should handle for loops", () => {
      const source = `
        int main() {
          int x = 0;
          for (int i = 0; i < 5; i = i + 1) {
            x = x + 1;
          }
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(true);
    });
  });

  describe("Compiler - Functions", () => {
    it("should declare and call functions", () => {
      const source = `
        int add(int a, int b) {
          return a + b;
        }

        int main() {
          int x = add(5, 10);
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(true);
    });

    it("should handle functions with multiple parameters", () => {
      const source = `
        int compute(int a, int b, int c) {
          return a + b + c;
        }

        int main() {
          int x = compute(1, 2, 3);
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(true);
    });

    it("should handle recursive function calls", () => {
      const source = `
        int factorial(int n) {
          if (n <= 1) {
            return 1;
          }
          return n * factorial(n - 1);
        }

        int main() {
          int x = factorial(5);
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(true);
    });
  });

  describe("Compiler - I/O Operations", () => {
    it("should handle printf with string", () => {
      const source = `
        int main() {
          printf("Hello, World!\\n");
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(true);
    });

    it("should handle printf with integer", () => {
      const source = `
        int main() {
          int x = 42;
          printf("Value: %d\\n", x);
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(true);
    });

    it("should handle printf with multiple arguments", () => {
      const source = `
        int main() {
          int x = 10;
          int y = 20;
          printf("x=%d, y=%d\\n", x, y);
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(true);
    });
  });

  describe("Compiler - Complex Programs", () => {
    it("should handle a factorial program", () => {
      const source = `
        int factorial(int n) {
          if (n <= 1) {
            return 1;
          }
          return n * factorial(n - 1);
        }

        int main() {
          int result = factorial(5);
          printf("Factorial: %d\\n", result);
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(true);
    });

    it("should handle a fibonacci program", () => {
      const source = `
        int fibonacci(int n) {
          if (n <= 1) {
            return n;
          }
          return fibonacci(n - 1) + fibonacci(n - 2);
        }

        int main() {
          int result = fibonacci(10);
          printf("Fibonacci: %d\\n", result);
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(true);
    });

    it("should handle a sum program", () => {
      const source = `
        int sum(int n) {
          int total = 0;
          int i = 0;
          while (i <= n) {
            total = total + i;
            i = i + 1;
          }
          return total;
        }

        int main() {
          int result = sum(10);
          printf("Sum: %d\\n", result);
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(true);
    });
  });

  describe("Compiler - Error Handling", () => {
    it("should report missing main function", () => {
      const source = `
        int foo() {
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(false);
      expect(result.errors.length).toBeGreaterThan(0);
    });

    it("should handle syntax errors gracefully", () => {
      const source = `
        int main() {
          int x = ;
          return 0;
        }
      `;

      const result = compiler.compileAndRun(source);
      expect(result.success).toBe(false);
    });
  });

  describe("MicroVM Registry", () => {
    it("should have all required VMs registered", () => {
      const registry = compiler.getRegistry();
      const vms = registry.getAllVMs();

      const vmNames = vms.map(vm => vm.name);
      expect(vmNames).toContain("MemoryVM");
      expect(vmNames).toContain("TypeVM");
      expect(vmNames).toContain("ArithmeticVM");
      expect(vmNames).toContain("ComparisonVM");
      expect(vmNames).toContain("ControlFlowVM");
      expect(vmNames).toContain("FunctionVM");
      expect(vmNames).toContain("IOVM");
    });

    it("should provide VM capabilities", () => {
      const registry = compiler.getRegistry();
      const capabilities = registry.getCapabilities();

      expect(capabilities.get("ArithmeticVM")).toContain("+");
      expect(capabilities.get("ArithmeticVM")).toContain("-");
      expect(capabilities.get("ComparisonVM")).toContain("==");
      expect(capabilities.get("ComparisonVM")).toContain("<");
      expect(capabilities.get("ControlFlowVM")).toContain("if");
      expect(capabilities.get("ControlFlowVM")).toContain("while");
      expect(capabilities.get("FunctionVM")).toContain("function");
      expect(capabilities.get("IOVM")).toContain("printf");
    });
  });
});