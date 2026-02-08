/**
 * Parser Tests
 */

const Lexer = require('../src/lexer');
const Parser = require('../src/parser');

function run(runner) {
  // Test 1: Variable declaration
  runner.test('Variable declaration', () => {
    const source = 'int x = 42;';
    const lexer = new Lexer(source);
    const tokens = lexer.tokenize();
    const parser = new Parser(tokens);
    const ast = parser.parse();
    
    runner.assertEqual(ast.declarations.length, 1, 'one declaration');
    runner.assertEqual(ast.declarations[0].type, 'VariableDeclaration', 'variable declaration');
    runner.assertEqual(ast.declarations[0].varType, 'INT', 'int type');
    runner.assertEqual(ast.declarations[0].name, 'x', 'variable name');
    runner.assertEqual(ast.declarations[0].init.value, 42, 'initial value');
  });
  
  // Test 2: Function declaration
  runner.test('Function declaration', () => {
    const source = 'int add(int a, int b) { return a + b; }';
    const lexer = new Lexer(source);
    const tokens = lexer.tokenize();
    const parser = new Parser(tokens);
    const ast = parser.parse();
    
    runner.assertEqual(ast.declarations.length, 1, 'one declaration');
    runner.assertEqual(ast.declarations[0].type, 'FunctionDeclaration', 'function declaration');
    runner.assertEqual(ast.declarations[0].returnType, 'INT', 'return type');
    runner.assertEqual(ast.declarations[0].name, 'add', 'function name');
    runner.assertEqual(ast.declarations[0].params.length, 2, 'two parameters');
  });
  
  // Test 3: If statement
  runner.test('If statement', () => {
    const source = 'void test() { if (x > 0) { return; } }';
    const lexer = new Lexer(source);
    const tokens = lexer.tokenize();
    const parser = new Parser(tokens);
    const ast = parser.parse();
    
    const func = ast.declarations[0];
    const stmt = func.body.statements[0];
    runner.assertEqual(stmt.type, 'IfStatement', 'if statement');
    runner.assertEqual(stmt.condition.type, 'Binary', 'binary condition');
  });
  
  // Test 4: While loop
  runner.test('While loop', () => {
    const source = 'void test() { while (x < 10) { x = x + 1; } }';
    const lexer = new Lexer(source);
    const tokens = lexer.tokenize();
    const parser = new Parser(tokens);
    const ast = parser.parse();
    
    const func = ast.declarations[0];
    const stmt = func.body.statements[0];
    runner.assertEqual(stmt.type, 'WhileStatement', 'while statement');
  });
  
  // Test 5: For loop
  runner.test('For loop', () => {
    const source = 'void test() { for (int i = 0; i < 10; i = i + 1) { } }';
    const lexer = new Lexer(source);
    const tokens = lexer.tokenize();
    const parser = new Parser(tokens);
    const ast = parser.parse();
    
    const func = ast.declarations[0];
    const stmt = func.body.statements[0];
    runner.assertEqual(stmt.type, 'ForStatement', 'for statement');
    runner.assertEqual(stmt.init.type, 'VariableDeclaration', 'for init');
  });
  
  // Test 6: Binary expressions
  runner.test('Binary expressions', () => {
    const source = 'int x = a + b * c;';
    const lexer = new Lexer(source);
    const tokens = lexer.tokenize();
    const parser = new Parser(tokens);
    const ast = parser.parse();
    
    const decl = ast.declarations[0];
    runner.assertEqual(decl.init.type, 'Binary', 'binary expression');
    runner.assertEqual(decl.init.operator, '+', 'plus operator');
  });
  
  // Test 7: Function call
  runner.test('Function call', () => {
    const source = 'void test() { printf("hello"); }';
    const lexer = new Lexer(source);
    const tokens = lexer.tokenize();
    const parser = new Parser(tokens);
    const ast = parser.parse();
    
    const func = ast.declarations[0];
    const stmt = func.body.statements[0];
    runner.assertEqual(stmt.expression.type, 'Call', 'function call');
    runner.assertEqual(stmt.expression.callee.name, 'printf', 'printf call');
  });
  
  // Test 8: Array declaration
  runner.test('Array declaration', () => {
    const source = 'int arr[10];';
    const lexer = new Lexer(source);
    const tokens = lexer.tokenize();
    const parser = new Parser(tokens);
    const ast = parser.parse();
    
    runner.assertEqual(ast.declarations.length, 1, 'one declaration');
    runner.assertEqual(ast.declarations[0].type, 'VariableDeclaration', 'variable declaration');
    runner.assertEqual(ast.declarations[0].isArray, true, 'is array');
    runner.assertEqual(ast.declarations[0].arraySize.value, 10, 'array size');
  });
  
  // Test 9: Array access
  runner.test('Array access', () => {
    const source = 'void test() { x = arr[5]; }';
    const lexer = new Lexer(source);
    const tokens = lexer.tokenize();
    const parser = new Parser(tokens);
    const ast = parser.parse();
    
    const func = ast.declarations[0];
    const stmt = func.body.statements[0];
    runner.assertEqual(stmt.expression.value.type, 'ArrayAccess', 'array access');
  });
  
  // Test 10: Pointer operations
  runner.test('Pointer operations', () => {
    const source = 'void test() { int* p = &x; y = *p; }';
    const lexer = new Lexer(source);
    const tokens = lexer.tokenize();
    const parser = new Parser(tokens);
    const ast = parser.parse();
    
    const func = ast.declarations[0];
    runner.assertEqual(func.body.statements.length, 2, 'two statements');
  });
}

module.exports = { run };