/**
 * Code Generator Tests
 */

const Compiler = require('../src/compiler');

function run(runner) {
  // Test 1: Simple variable declaration
  runner.test('Simple variable declaration', () => {
    const source = 'int x = 42;';
    const compiler = new Compiler();
    const js = compiler.compile(source);
    
    runner.assertContains(js, 'let x = 42;', 'variable declaration');
  });
  
  // Test 2: Function declaration
  runner.test('Function declaration', () => {
    const source = 'int add(int a, int b) { return a + b; }';
    const compiler = new Compiler();
    const js = compiler.compile(source);
    
    runner.assertContains(js, 'function add(a, b)', 'function declaration');
    runner.assertContains(js, 'return (a + b);', 'return statement');
  });
  
  // Test 3: If statement
  runner.test('If statement', () => {
    const source = 'void test() { if (x > 0) { return; } }';
    const compiler = new Compiler();
    const js = compiler.compile(source);
    
    runner.assertContains(js, 'if ((x > 0))', 'if condition');
  });
  
  // Test 4: While loop
  runner.test('While loop', () => {
    const source = 'void test() { while (x < 10) { x = x + 1; } }';
    const compiler = new Compiler();
    const js = compiler.compile(source);
    
    runner.assertContains(js, 'while ((x < 10))', 'while condition');
  });
  
  // Test 5: For loop
  runner.test('For loop', () => {
    const source = 'void test() { for (int i = 0; i < 10; i = i + 1) { } }';
    const compiler = new Compiler();
    const js = compiler.compile(source);
    
    runner.assertContains(js, 'for (let i = 0; (i < 10); i = (i + 1))', 'for loop');
  });
  
  // Test 6: Arithmetic operations
  runner.test('Arithmetic operations', () => {
    const source = 'int x = a + b * c - d / e % f;';
    const compiler = new Compiler();
    const js = compiler.compile(source);
    
    runner.assertContains(js, 'let x = ((a + (b * c)) - (d / e)) % f;', 'arithmetic expression');
  });
  
  // Test 7: Comparison operators
  runner.test('Comparison operators', () => {
    const source = 'void test() { if (x == y && z != w) { } }';
    const compiler = new Compiler();
    const js = compiler.compile(source);
    
    runner.assertContains(js, 'if (((x == y) && (z != w)))', 'comparison operators');
  });
  
  // Test 8: Printf with string
  runner.test('Printf with string', () => {
    const source = 'void test() { printf("hello world"); }';
    const compiler = new Compiler();
    const js = compiler.compile(source);
    
    runner.assertContains(js, 'print(`hello world`)', 'printf call');
  });
  
  // Test 9: Printf with format
  runner.test('Printf with format', () => {
    const source = 'void test() { printf("value: %d", x); }';
    const compiler = new Compiler();
    const js = compiler.compile(source);
    
    runner.assertContains(js, 'print(`value: ${x}`)', 'printf with format');
  });
  
  // Test 10: Array declaration
  runner.test('Array declaration', () => {
    const source = 'int arr[10];';
    const compiler = new Compiler();
    const js = compiler.compile(source);
    
    runner.assertContains(js, 'let arr = new Array(10).fill(0);', 'array declaration');
  });
  
  // Test 11: Array access
  runner.test('Array access', () => {
    const source = 'void test() { x = arr[5]; }';
    const compiler = new Compiler();
    const js = compiler.compile(source);
    
    runner.assertContains(js, 'x = arr[5];', 'array access');
  });
  
  // Test 12: Array assignment
  runner.test('Array assignment', () => {
    const source = 'void test() { arr[5] = 10; }';
    const compiler = new Compiler();
    const js = compiler.compile(source);
    
    runner.assertContains(js, 'arr[5] = 10;', 'array assignment');
  });
  
  // Test 13: Character literal
  runner.test('Character literal', () => {
    const source = 'int x = \'a\';';
    const compiler = new Compiler();
    const js = compiler.compile(source);
    
    runner.assertContains(js, 'let x = 97;', 'character literal (ASCII)');
  });
  
  // Test 14: Multiple declarations
  runner.test('Multiple declarations', () => {
    const source = 'int x = 1; int y = 2; void test() { }';
    const compiler = new Compiler();
    const js = compiler.compile(source);
    
    runner.assertContains(js, 'let x = 1;', 'first variable');
    runner.assertContains(js, 'let y = 2;', 'second variable');
    runner.assertContains(js, 'function test()', 'function');
  });
  
  // Test 15: Increment/decrement
  runner.test('Increment/decrement', () => {
    const source = 'void test() { x++; y--; }';
    const compiler = new Compiler();
    const js = compiler.compile(source);
    
    runner.assertContains(js, 'x++;', 'increment');
    runner.assertContains(js, 'y--;', 'decrement');
  });
}

module.exports = { run };