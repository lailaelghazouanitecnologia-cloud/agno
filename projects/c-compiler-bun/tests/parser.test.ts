/**
 * Parser tests
 */

import { describe, it, expect } from "bun:test";
import { Lexer } from "../src/lexer/lexer.js";
import { Parser } from "../src/parser/parser.js";

describe("Parser", () => {
  function parse(source: string) {
    const lexer = new Lexer(source);
    const tokens = lexer.tokenize();
    const parser = new Parser(tokens);
    return parser.parse();
  }

  it("should parse a simple program", () => {
    const ast = parse("int main() { return 0; }");

    expect(ast.type).toBe("Program");
    expect(ast.declarations).toHaveLength(1);
    expect(ast.declarations[0].type).toBe("Function");
  });

  it("should parse variable declarations", () => {
    const ast = parse("int x; float y = 3.14;");

    expect(ast.declarations).toHaveLength(2);
    expect(ast.declarations[0].type).toBe("Declaration");
    expect((ast.declarations[0] as any).name).toBe("x");
    expect((ast.declarations[1] as any).name).toBe("y");
  });

  it("should parse function definitions", () => {
    const ast = parse("int add(int a, int b) { return a + b; }");

    expect(ast.declarations).toHaveLength(1);
    const func = ast.declarations[0] as any;
    expect(func.type).toBe("Function");
    expect(func.name).toBe("add");
    expect(func.parameters).toHaveLength(2);
    expect(func.parameters[0].name).toBe("a");
    expect(func.parameters[1].name).toBe("b");
  });

  it("should parse if statements", () => {
    const ast = parse("int main() { if (x > 0) { return 1; } }");

    const func = ast.declarations[0] as any;
    const stmt = func.body.statements[0];
    expect(stmt.type).toBe("IfStatement");
    expect(stmt.condition.type).toBe("BinaryOp");
  });

  it("should parse if-else statements", () => {
    const ast = parse("int main() { if (x > 0) { return 1; } else { return 0; } }");

    const func = ast.declarations[0] as any;
    const stmt = func.body.statements[0];
    expect(stmt.type).toBe("IfStatement");
    expect(stmt.elseBranch).toBeDefined();
  });

  it("should parse while statements", () => {
    const ast = parse("int main() { while (x < 10) { x = x + 1; } }");

    const func = ast.declarations[0] as any;
    const stmt = func.body.statements[0];
    expect(stmt.type).toBe("WhileStatement");
  });

  it("should parse for statements", () => {
    const ast = parse("int main() { for (int i = 0; i < 10; i = i + 1) { } }");

    const func = ast.declarations[0] as any;
    const stmt = func.body.statements[0];
    expect(stmt.type).toBe("ForStatement");
    expect(stmt.init).toBeDefined();
    expect(stmt.condition).toBeDefined();
    expect(stmt.increment).toBeDefined();
  });

  it("should parse return statements", () => {
    const ast = parse("int main() { return 42; }");

    const func = ast.declarations[0] as any;
    const stmt = func.body.statements[0];
    expect(stmt.type).toBe("ReturnStatement");
    expect(stmt.value.type).toBe("Number");
  });

  it("should parse binary operations", () => {
    const ast = parse("int main() { int x = 1 + 2 * 3; }");

    const func = ast.declarations[0] as any;
    const decl = func.body.statements[0];
    const expr = (decl as any).initialValue;
    expect(expr.type).toBe("BinaryOp");
    expect(expr.operator).toBe("+");
  });

  it("should parse comparison operations", () => {
    const ast = parse("int main() { if (x == y) { } }");

    const func = ast.declarations[0] as any;
    const ifStmt = func.body.statements[0];
    expect((ifStmt as any).condition.operator).toBe("==");
  });

  it("should parse function calls", () => {
    const ast = parse("int main() { printf(\"hello\"); }");

    const func = ast.declarations[0] as any;
    const stmt = func.body.statements[0];
    const expr = (stmt as any).expression;
    expect(expr.type).toBe("FunctionCall");
    expect(expr.functionName).toBe("printf");
  });

  it("should parse array declarations", () => {
    const ast = parse("int arr[10];");

    expect(ast.declarations).toHaveLength(1);
    const decl = ast.declarations[0] as any;
    expect(decl.varType.isArray).toBe(true);
    expect(decl.varType.arraySize).toBe(10);
  });

  it("should parse pointer types", () => {
    const ast = parse("int* ptr;");

    expect(ast.declarations).toHaveLength(1);
    const decl = ast.declarations[0] as any;
    expect(decl.varType.isPointer).toBe(true);
  });

  it("should parse address-of operator", () => {
    const ast = parse("int main() { int* ptr = &x; }");

    const func = ast.declarations[0] as any;
    const decl = func.body.statements[0];
    const init = (decl as any).initialValue;
    expect(init.type).toBe("UnaryOp");
    expect(init.operator).toBe("&");
  });

  it("should parse dereference operator", () => {
    const ast = parse("int main() { int x = *ptr; }");

    const func = ast.declarations[0] as any;
    const decl = func.body.statements[0];
    const init = (decl as any).initialValue;
    expect(init.type).toBe("UnaryOp");
    expect(init.operator).toBe("*");
  });

  it("should parse array access", () => {
    const ast = parse("int main() { int x = arr[5]; }");

    const func = ast.declarations[0] as any;
    const decl = func.body.statements[0];
    const init = (decl as any).initialValue;
    expect(init.type).toBe("ArrayAccess");
  });

  it("should parse assignment expressions", () => {
    const ast = parse("int main() { x = 42; }");

    const func = ast.declarations[0] as any;
    const stmt = func.body.statements[0];
    const expr = (stmt as any).expression;
    expect(expr.type).toBe("Assignment");
  });

  it("should handle multiple statements in a block", () => {
    const ast = parse("int main() { int x; int y; x = 1; y = 2; }");

    const func = ast.declarations[0] as any;
    expect(func.body.statements).toHaveLength(4);
  });

  it("should parse void functions", () => {
    const ast = parse("void foo() { }");

    expect(ast.declarations).toHaveLength(1);
    const func = ast.declarations[0] as any;
    expect(func.returnType.base).toBe("void");
  });
});