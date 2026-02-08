/**
 * Lexer Tests
 */

const Lexer = require('../src/lexer');

function run(runner) {
  // Test 1: Simple tokens
  runner.test('Simple tokens', () => {
    const source = 'int x = 42;';
    const lexer = new Lexer(source);
    const tokens = lexer.tokenize();
    
    runner.assertEqual(tokens[0].type, 'INT', 'INT token');
    runner.assertEqual(tokens[1].type, 'IDENTIFIER', 'IDENTIFIER token');
    runner.assertEqual(tokens[2].type, 'ASSIGN', 'ASSIGN token');
    runner.assertEqual(tokens[3].type, 'NUMBER', 'NUMBER token');
    runner.assertEqual(tokens[4].type, 'SEMICOLON', 'SEMICOLON token');
  });
  
  // Test 2: Keywords
  runner.test('Keywords', () => {
    const source = 'if else while for return void char';
    const lexer = new Lexer(source);
    const tokens = lexer.tokenize();
    
    runner.assertEqual(tokens[0].type, 'IF', 'IF keyword');
    runner.assertEqual(tokens[1].type, 'ELSE', 'ELSE keyword');
    runner.assertEqual(tokens[2].type, 'WHILE', 'WHILE keyword');
    runner.assertEqual(tokens[3].type, 'FOR', 'FOR keyword');
    runner.assertEqual(tokens[4].type, 'RETURN', 'RETURN keyword');
    runner.assertEqual(tokens[5].type, 'VOID', 'VOID keyword');
    runner.assertEqual(tokens[6].type, 'CHAR', 'CHAR keyword');
  });
  
  // Test 3: Operators
  runner.test('Operators', () => {
    const source = '+ - * / % == != < > <= >= && ||';
    const lexer = new Lexer(source);
    const tokens = lexer.tokenize();
    
    runner.assertEqual(tokens[0].type, 'PLUS', 'PLUS operator');
    runner.assertEqual(tokens[1].type, 'MINUS', 'MINUS operator');
    runner.assertEqual(tokens[2].type, 'STAR', 'STAR operator');
    runner.assertEqual(tokens[3].type, 'SLASH', 'SLASH operator');
    runner.assertEqual(tokens[4].type, 'PERCENT', 'PERCENT operator');
    runner.assertEqual(tokens[5].type, 'EQ', 'EQ operator');
    runner.assertEqual(tokens[6].type, 'NEQ', 'NEQ operator');
    runner.assertEqual(tokens[7].type, 'LT', 'LT operator');
    runner.assertEqual(tokens[8].type, 'GT', 'GT operator');
    runner.assertEqual(tokens[9].type, 'LTE', 'LTE operator');
    runner.assertEqual(tokens[10].type, 'GTE', 'GTE operator');
    runner.assertEqual(tokens[11].type, 'AND', 'AND operator');
    runner.assertEqual(tokens[12].type, 'OR', 'OR operator');
  });
  
  // Test 4: String literals
  runner.test('String literals', () => {
    const source = '"hello" "world\\n"';
    const lexer = new Lexer(source);
    const tokens = lexer.tokenize();
    
    runner.assertEqual(tokens[0].type, 'STRING', 'STRING token');
    runner.assertEqual(tokens[0].value, 'hello', 'string value');
    runner.assertEqual(tokens[1].type, 'STRING', 'STRING token 2');
    runner.assertEqual(tokens[1].value, 'world\n', 'escaped string value');
  });
  
  // Test 5: Character literals
  runner.test('Character literals', () => {
    const source = "'a' '\\n'";
    const lexer = new Lexer(source);
    const tokens = lexer.tokenize();
    
    runner.assertEqual(tokens[0].type, 'CHAR', 'CHAR token');
    runner.assertEqual(tokens[0].value, 'a', 'char value');
    runner.assertEqual(tokens[1].type, 'CHAR', 'CHAR token 2');
    runner.assertEqual(tokens[1].value, 'n', 'escaped char value');
  });
  
  // Test 6: Comments
  runner.test('Comments', () => {
    const source = '// line comment\nint x; /* block comment */ int y;';
    const lexer = new Lexer(source);
    const tokens = lexer.tokenize();
    
    runner.assertEqual(tokens[0].type, 'INT', 'INT after line comment');
    runner.assertEqual(tokens[1].type, 'IDENTIFIER', 'IDENTIFIER after line comment');
    runner.assertEqual(tokens[2].type, 'SEMICOLON', 'SEMICOLON after line comment');
    runner.assertEqual(tokens[3].type, 'INT', 'INT after block comment');
    runner.assertEqual(tokens[4].type, 'IDENTIFIER', 'IDENTIFIER after block comment');
  });
  
  // Test 7: Pointers
  runner.test('Pointer operators', () => {
    const source = '&x *p';
    const lexer = new Lexer(source);
    const tokens = lexer.tokenize();
    
    runner.assertEqual(tokens[0].type, 'AMP', 'AMP operator');
    runner.assertEqual(tokens[1].type, 'IDENTIFIER', 'IDENTIFIER after AMP');
    runner.assertEqual(tokens[2].type, 'STAR', 'STAR operator');
    runner.assertEqual(tokens[3].type, 'IDENTIFIER', 'IDENTIFIER after STAR');
  });
  
  // Test 8: Arrays
  runner.test('Array syntax', () => {
    const source = 'int arr[10];';
    const lexer = new Lexer(source);
    const tokens = lexer.tokenize();
    
    runner.assertEqual(tokens[0].type, 'INT', 'INT token');
    runner.assertEqual(tokens[1].type, 'IDENTIFIER', 'IDENTIFIER token');
    runner.assertEqual(tokens[2].type, 'LBRACKET', 'LBRACKET token');
    runner.assertEqual(tokens[3].type, 'NUMBER', 'NUMBER token');
    runner.assertEqual(tokens[4].type, 'RBRACKET', 'RBRACKET token');
    runner.assertEqual(tokens[5].type, 'SEMICOLON', 'SEMICOLON token');
  });
}

module.exports = { run };