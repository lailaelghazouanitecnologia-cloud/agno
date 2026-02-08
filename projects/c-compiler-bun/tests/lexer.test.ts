/**
 * Comprehensive Lexer tests
 * Covers: happy path, edge cases, error cases
 */

import { describe, it, expect } from "bun:test";
import { Lexer } from "../src/lexer/lexer.js";
import { TokenType } from "../src/core/types.js";

describe("Lexer Module", () => {
  // =========================================================================
  // HAPPY PATH TESTS
  // =========================================================================

  describe("Happy Path - Basic Tokenization", () => {
    it("should tokenize empty source", () => {
      const lexer = new Lexer("");
      const tokens = lexer.tokenize();

      expect(tokens).toHaveLength(1);
      expect(tokens[0].type).toBe(TokenType.EOF);
    });

    it("should tokenize whitespace only", () => {
      const lexer = new Lexer("   \t\n\r   ");
      const tokens = lexer.tokenize();

      expect(tokens).toHaveLength(1);
      expect(tokens[0].type).toBe(TokenType.EOF);
    });

    it("should tokenize single integer", () => {
      const lexer = new Lexer("42");
      const tokens = lexer.tokenize();

      expect(tokens).toHaveLength(2); // INTEGER + EOF
      expect(tokens[0].type).toBe(TokenType.INTEGER);
      expect(tokens[0].value).toBe("42");
      expect(tokens[0].line).toBe(1);
      expect(tokens[0].column).toBe(1);
    });

    it("should tokenize multiple integers", () => {
      const lexer = new Lexer("42 100 0 999");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.INTEGER);
      expect(tokens[0].value).toBe("42");
      expect(tokens[1].type).toBe(TokenType.INTEGER);
      expect(tokens[1].value).toBe("100");
      expect(tokens[2].type).toBe(TokenType.INTEGER);
      expect(tokens[2].value).toBe("0");
      expect(tokens[3].type).toBe(TokenType.INTEGER);
      expect(tokens[3].value).toBe("999");
    });

    it("should tokenize float literals", () => {
      const lexer = new Lexer("3.14 2.5 0.5 1.0");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.FLOAT_LITERAL);
      expect(tokens[0].value).toBe("3.14");
      expect(tokens[1].type).toBe(TokenType.FLOAT_LITERAL);
      expect(tokens[1].value).toBe("2.5");
      expect(tokens[2].type).toBe(TokenType.FLOAT_LITERAL);
      expect(tokens[2].value).toBe("0.5");
      expect(tokens[3].type).toBe(TokenType.FLOAT_LITERAL);
      expect(tokens[3].value).toBe("1.0");
    });

    it("should tokenize simple identifiers", () => {
      const lexer = new Lexer("foo bar baz");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[0].value).toBe("foo");
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].value).toBe("bar");
      expect(tokens[2].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[2].value).toBe("baz");
    });

    it("should tokenize identifiers with underscores", () => {
      const lexer = new Lexer("foo_bar _baz _private");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[0].value).toBe("foo_bar");
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].value).toBe("_baz");
      expect(tokens[2].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[2].value).toBe("_private");
    });

    it("should tokenize identifiers with numbers", () => {
      const lexer = new Lexer("var1 value2 temp3");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[0].value).toBe("var1");
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].value).toBe("value2");
      expect(tokens[2].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[2].value).toBe("temp3");
    });

    it("should tokenize all type keywords", () => {
      const lexer = new Lexer("int char float void");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.INT);
      expect(tokens[1].type).toBe(TokenType.CHAR);
      expect(tokens[2].type).toBe(TokenType.FLOAT);
      expect(tokens[3].type).toBe(TokenType.VOID);
    });

    it("should tokenize all control flow keywords", () => {
      const lexer = new Lexer("if else while for return break continue");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.IF);
      expect(tokens[1].type).toBe(TokenType.ELSE);
      expect(tokens[2].type).toBe(TokenType.WHILE);
      expect(tokens[3].type).toBe(TokenType.FOR);
      expect(tokens[4].type).toBe(TokenType.RETURN);
      expect(tokens[5].type).toBe(TokenType.BREAK);
      expect(tokens[6].type).toBe(TokenType.CONTINUE);
    });

    it("should tokenize arithmetic operators", () => {
      const lexer = new Lexer("+ - * / %");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.PLUS);
      expect(tokens[0].value).toBe("+");
      expect(tokens[1].type).toBe(TokenType.MINUS);
      expect(tokens[1].value).toBe("-");
      expect(tokens[2].type).toBe(TokenType.MULTIPLY);
      expect(tokens[2].value).toBe("*");
      expect(tokens[3].type).toBe(TokenType.DIVIDE);
      expect(tokens[3].value).toBe("/");
      expect(tokens[4].type).toBe(TokenType.MODULO);
      expect(tokens[4].value).toBe("%");
    });

    it("should tokenize comparison operators", () => {
      const lexer = new Lexer("< > <= >= == !=");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.LESS);
      expect(tokens[0].value).toBe("<");
      expect(tokens[1].type).toBe(TokenType.GREATER);
      expect(tokens[1].value).toBe(">");
      expect(tokens[2].type).toBe(TokenType.LESS_EQUAL);
      expect(tokens[2].value).toBe("<=");
      expect(tokens[3].type).toBe(TokenType.GREATER_EQUAL);
      expect(tokens[3].value).toBe(">=");
      expect(tokens[4].type).toBe(TokenType.EQUAL);
      expect(tokens[4].value).toBe("==");
      expect(tokens[5].type).toBe(TokenType.NOT_EQUAL);
      expect(tokens[5].value).toBe("!=");
    });

    it("should tokenize logical operators", () => {
      const lexer = new Lexer("&& || !");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.AND);
      expect(tokens[0].value).toBe("&&");
      expect(tokens[1].type).toBe(TokenType.OR);
      expect(tokens[1].value).toBe("||");
      expect(tokens[2].type).toBe(TokenType.NOT);
      expect(tokens[2].value).toBe("!");
    });

    it("should tokenize assignment operators", () => {
      const lexer = new Lexer("= += -= *= /=");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.ASSIGN);
      expect(tokens[0].value).toBe("=");
      expect(tokens[1].type).toBe(TokenType.PLUS_ASSIGN);
      expect(tokens[1].value).toBe("+=");
      expect(tokens[2].type).toBe(TokenType.MINUS_ASSIGN);
      expect(tokens[2].value).toBe("-=");
      expect(tokens[3].type).toBe(TokenType.MULTIPLY_ASSIGN);
      expect(tokens[3].value).toBe("*=");
      expect(tokens[4].type).toBe(TokenType.DIVIDE_ASSIGN);
      expect(tokens[4].value).toBe("/=");
    });

    it("should tokenize pointer operators", () => {
      const lexer = new Lexer("& * ->");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.ADDRESS);
      expect(tokens[0].value).toBe("&");
      expect(tokens[1].type).toBe(TokenType.MULTIPLY);
      expect(tokens[1].value).toBe("*");
      expect(tokens[2].type).toBe(TokenType.ARROW);
      expect(tokens[2].value).toBe("->");
    });

    it("should tokenize increment/decrement operators", () => {
      const lexer = new Lexer("++ --");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.PLUS);
      expect(tokens[0].value).toBe("++");
      expect(tokens[1].type).toBe(TokenType.MINUS);
      expect(tokens[1].value).toBe("--");
    });

    it("should tokenize punctuation", () => {
      const lexer = new Lexer("; , .");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.SEMICOLON);
      expect(tokens[0].value).toBe(";");
      expect(tokens[1].type).toBe(TokenType.COMMA);
      expect(tokens[1].value).toBe(",");
      expect(tokens[2].type).toBe(TokenType.DOT);
      expect(tokens[2].value).toBe(".");
    });

    it("should tokenize brackets and braces", () => {
      const lexer = new Lexer("( ) { } [ ]");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.LEFT_PAREN);
      expect(tokens[0].value).toBe("(");
      expect(tokens[1].type).toBe(TokenType.RIGHT_PAREN);
      expect(tokens[1].value).toBe(")");
      expect(tokens[2].type).toBe(TokenType.LEFT_BRACE);
      expect(tokens[2].value).toBe("{");
      expect(tokens[3].type).toBe(TokenType.RIGHT_BRACE);
      expect(tokens[3].value).toBe("}");
      expect(tokens[4].type).toBe(TokenType.LEFT_BRACKET);
      expect(tokens[4].value).toBe("[");
      expect(tokens[5].type).toBe(TokenType.RIGHT_BRACKET);
      expect(tokens[5].value).toBe("]");
    });

    it("should tokenize simple string literal", () => {
      const lexer = new Lexer('"hello"');
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.STRING);
      expect(tokens[0].value).toBe("hello");
    });

    it("should tokenize string literal with spaces", () => {
      const lexer = new Lexer('"hello world"');
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.STRING);
      expect(tokens[0].value).toBe("hello world");
    });

    it("should tokenize empty string literal", () => {
      const lexer = new Lexer('""');
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.STRING);
      expect(tokens[0].value).toBe("");
    });

    it("should tokenize simple character literal", () => {
      const lexer = new Lexer("'a'");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.CHARACTER);
      expect(tokens[0].value).toBe("a");
    });

    it("should tokenize character literal with escape sequence", () => {
      const lexer = new Lexer("'\\n'");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.CHARACTER);
      expect(tokens[0].value).toBe("\n");
    });

    it("should tokenize simple variable declaration", () => {
      const lexer = new Lexer("int x;");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.INT);
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].value).toBe("x");
      expect(tokens[2].type).toBe(TokenType.SEMICOLON);
    });

    it("should tokenize variable declaration with initialization", () => {
      const lexer = new Lexer("int x = 42;");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.INT);
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].value).toBe("x");
      expect(tokens[2].type).toBe(TokenType.ASSIGN);
      expect(tokens[3].type).toBe(TokenType.INTEGER);
      expect(tokens[3].value).toBe("42");
      expect(tokens[4].type).toBe(TokenType.SEMICOLON);
    });

    it("should tokenize function call", () => {
      const lexer = new Lexer("foo();");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[0].value).toBe("foo");
      expect(tokens[1].type).toBe(TokenType.LEFT_PAREN);
      expect(tokens[2].type).toBe(TokenType.RIGHT_PAREN);
      expect(tokens[3].type).toBe(TokenType.SEMICOLON);
    });

    it("should tokenize function call with arguments", () => {
      const lexer = new Lexer("foo(x, y);");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[0].value).toBe("foo");
      expect(tokens[1].type).toBe(TokenType.LEFT_PAREN);
      expect(tokens[2].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[2].value).toBe("x");
      expect(tokens[3].type).toBe(TokenType.COMMA);
      expect(tokens[4].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[4].value).toBe("y");
      expect(tokens[5].type).toBe(TokenType.RIGHT_PAREN);
      expect(tokens[6].type).toBe(TokenType.SEMICOLON);
    });

    it("should tokenize if statement", () => {
      const lexer = new Lexer("if (x > 0) { return x; }");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.IF);
      expect(tokens[1].type).toBe(TokenType.LEFT_PAREN);
      expect(tokens[2].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[2].value).toBe("x");
      expect(tokens[3].type).toBe(TokenType.GREATER);
      expect(tokens[4].type).toBe(TokenType.INTEGER);
      expect(tokens[4].value).toBe("0");
      expect(tokens[5].type).toBe(TokenType.RIGHT_PAREN);
      expect(tokens[6].type).toBe(TokenType.LEFT_BRACE);
      expect(tokens[7].type).toBe(TokenType.RETURN);
      expect(tokens[8].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[8].value).toBe("x");
      expect(tokens[9].type).toBe(TokenType.SEMICOLON);
      expect(tokens[10].type).toBe(TokenType.RIGHT_BRACE);
    });

    it("should tokenize while loop", () => {
      const lexer = new Lexer("while (i < 10) { i++; }");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.WHILE);
      expect(tokens[1].type).toBe(TokenType.LEFT_PAREN);
      expect(tokens[2].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[2].value).toBe("i");
      expect(tokens[3].type).toBe(TokenType.LESS);
      expect(tokens[4].type).toBe(TokenType.INTEGER);
      expect(tokens[4].value).toBe("10");
      expect(tokens[5].type).toBe(TokenType.RIGHT_PAREN);
      expect(tokens[6].type).toBe(TokenType.LEFT_BRACE);
      expect(tokens[7].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[7].value).toBe("i");
      expect(tokens[8].type).toBe(TokenType.PLUS);
      expect(tokens[8].value).toBe("++");
      expect(tokens[9].type).toBe(TokenType.SEMICOLON);
      expect(tokens[10].type).toBe(TokenType.RIGHT_BRACE);
    });

    it("should tokenize for loop", () => {
      const lexer = new Lexer("for (int i = 0; i < 10; i++) { }");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.FOR);
      expect(tokens[1].type).toBe(TokenType.LEFT_PAREN);
      expect(tokens[2].type).toBe(TokenType.INT);
      expect(tokens[3].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[3].value).toBe("i");
      expect(tokens[4].type).toBe(TokenType.ASSIGN);
      expect(tokens[5].type).toBe(TokenType.INTEGER);
      expect(tokens[5].value).toBe("0");
      expect(tokens[6].type).toBe(TokenType.SEMICOLON);
      expect(tokens[7].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[7].value).toBe("i");
      expect(tokens[8].type).toBe(TokenType.LESS);
      expect(tokens[9].type).toBe(TokenType.INTEGER);
      expect(tokens[9].value).toBe("10");
      expect(tokens[10].type).toBe(TokenType.SEMICOLON);
      expect(tokens[11].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[11].value).toBe("i");
      expect(tokens[12].type).toBe(TokenType.PLUS);
      expect(tokens[12].value).toBe("++");
      expect(tokens[13].type).toBe(TokenType.RIGHT_PAREN);
      expect(tokens[14].type).toBe(TokenType.LEFT_BRACE);
      expect(tokens[15].type).toBe(TokenType.RIGHT_BRACE);
    });

    it("should tokenize function definition", () => {
      const lexer = new Lexer("int add(int a, int b) { return a + b; }");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.INT);
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].value).toBe("add");
      expect(tokens[2].type).toBe(TokenType.LEFT_PAREN);
      expect(tokens[3].type).toBe(TokenType.INT);
      expect(tokens[4].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[4].value).toBe("a");
      expect(tokens[5].type).toBe(TokenType.COMMA);
      expect(tokens[6].type).toBe(TokenType.INT);
      expect(tokens[7].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[7].value).toBe("b");
      expect(tokens[8].type).toBe(TokenType.RIGHT_PAREN);
      expect(tokens[9].type).toBe(TokenType.LEFT_BRACE);
      expect(tokens[10].type).toBe(TokenType.RETURN);
      expect(tokens[11].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[11].value).toBe("a");
      expect(tokens[12].type).toBe(TokenType.PLUS);
      expect(tokens[13].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[13].value).toBe("b");
      expect(tokens[14].type).toBe(TokenType.SEMICOLON);
      expect(tokens[15].type).toBe(TokenType.RIGHT_BRACE);
    });

    it("should tokenize array declaration", () => {
      const lexer = new Lexer("int arr[10];");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.INT);
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].value).toBe("arr");
      expect(tokens[2].type).toBe(TokenType.LEFT_BRACKET);
      expect(tokens[3].type).toBe(TokenType.INTEGER);
      expect(tokens[3].value).toBe("10");
      expect(tokens[4].type).toBe(TokenType.RIGHT_BRACKET);
      expect(tokens[5].type).toBe(TokenType.SEMICOLON);
    });

    it("should tokenize array access", () => {
      const lexer = new Lexer("arr[0] = 42;");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[0].value).toBe("arr");
      expect(tokens[1].type).toBe(TokenType.LEFT_BRACKET);
      expect(tokens[2].type).toBe(TokenType.INTEGER);
      expect(tokens[2].value).toBe("0");
      expect(tokens[3].type).toBe(TokenType.RIGHT_BRACKET);
      expect(tokens[4].type).toBe(TokenType.ASSIGN);
      expect(tokens[5].type).toBe(TokenType.INTEGER);
      expect(tokens[5].value).toBe("42");
      expect(tokens[6].type).toBe(TokenType.SEMICOLON);
    });

    it("should tokenize pointer declaration", () => {
      const lexer = new Lexer("int *ptr;");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.INT);
      expect(tokens[1].type).toBe(TokenType.MULTIPLY);
      expect(tokens[2].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[2].value).toBe("ptr");
      expect(tokens[3].type).toBe(TokenType.SEMICOLON);
    });

    it("should tokenize address-of operator", () => {
      const lexer = new Lexer("ptr = &x;");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[0].value).toBe("ptr");
      expect(tokens[1].type).toBe(TokenType.ASSIGN);
      expect(tokens[2].type).toBe(TokenType.ADDRESS);
      expect(tokens[3].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[3].value).toBe("x");
      expect(tokens[4].type).toBe(TokenType.SEMICOLON);
    });

    it("should tokenize pointer dereference", () => {
      const lexer = new Lexer("*ptr = 42;");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.MULTIPLY);
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].value).toBe("ptr");
      expect(tokens[2].type).toBe(TokenType.ASSIGN);
      expect(tokens[3].type).toBe(TokenType.INTEGER);
      expect(tokens[3].value).toBe("42");
      expect(tokens[4].type).toBe(TokenType.SEMICOLON);
    });

    it("should tokenize arrow operator", () => {
      const lexer = new Lexer("ptr->value = 42;");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[0].value).toBe("ptr");
      expect(tokens[1].type).toBe(TokenType.ARROW);
      expect(tokens[2].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[2].value).toBe("value");
      expect(tokens[3].type).toBe(TokenType.ASSIGN);
      expect(tokens[4].type).toBe(TokenType.INTEGER);
      expect(tokens[4].value).toBe("42");
      expect(tokens[5].type).toBe(TokenType.SEMICOLON);
    });

    it("should always add EOF token at the end", () => {
      const lexer = new Lexer("int x;");
      const tokens = lexer.tokenize();

      expect(tokens[tokens.length - 1].type).toBe(TokenType.EOF);
      expect(tokens[tokens.length - 1].value).toBe("");
    });
  });

  // =========================================================================
  // EDGE CASE TESTS
  // =========================================================================

  describe("Edge Cases - String Literals", () => {
    it("should tokenize string with escape sequences", () => {
      const lexer = new Lexer('"hello\\nworld\\t!"');
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.STRING);
      expect(tokens[0].value).toBe("hello\nworld\t!");
    });

    it("should tokenize string with escaped quote", () => {
      const lexer = new Lexer('"He said \\"hello\\""');
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.STRING);
      expect(tokens[0].value).toBe('He said "hello"');
    });

    it("should tokenize string with escaped backslash", () => {
      const lexer = new Lexer('"path\\\\to\\\\file"');
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.STRING);
      expect(tokens[0].value).toBe("path\\to\\file");
    });

    it("should tokenize string with escaped apostrophe", () => {
      const lexer = new Lexer(`"It\\'s a test"`);
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.STRING);
      expect(tokens[0].value).toBe("It's a test");
    });

    it("should tokenize string with carriage return", () => {
      const lexer = new Lexer('"line1\\rline2"');
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.STRING);
      expect(tokens[0].value).toBe("line1\rline2");
    });

    it("should tokenize string with multiple escape sequences", () => {
      const lexer = new Lexer(`"\\\\n\\\\t\\\\r\\\\\\\\\\\"\\'"`);
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.STRING);
      expect(tokens[0].value).toBe("\n\t\r\\\"'");
    });

    it("should tokenize string with special characters", () => {
      const lexer = new Lexer('"!@#$%^&*()"');
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.STRING);
      expect(tokens[0].value).toBe("!@#$%^&*()");
    });

    it("should tokenize string with numbers", () => {
      const lexer = new Lexer('"12345"');
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.STRING);
      expect(tokens[0].value).toBe("12345");
    });

    it("should tokenize string with mixed content", () => {
      const lexer = new Lexer('"Value: 42, Status: OK"');
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.STRING);
      expect(tokens[0].value).toBe("Value: 42, Status: OK");
    });
  });

  describe("Edge Cases - Character Literals", () => {
    it("should tokenize character with escape sequences", () => {
      const lexer = new Lexer("'\\n' '\\t' '\\r' '\\\\' '\\\"' '\\''");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.CHARACTER);
      expect(tokens[0].value).toBe("\n");
      expect(tokens[1].type).toBe(TokenType.CHARACTER);
      expect(tokens[1].value).toBe("\t");
      expect(tokens[2].type).toBe(TokenType.CHARACTER);
      expect(tokens[2].value).toBe("\r");
      expect(tokens[3].type).toBe(TokenType.CHARACTER);
      expect(tokens[3].value).toBe("\\");
      expect(tokens[4].type).toBe(TokenType.CHARACTER);
      expect(tokens[4].value).toBe('"');
      expect(tokens[5].type).toBe(TokenType.CHARACTER);
      expect(tokens[5].value).toBe("'");
    });

    it("should tokenize null character", () => {
      const lexer = new Lexer("'\\0'");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.CHARACTER);
      expect(tokens[0].value).toBe("\0");
    });

    it("should tokenize digit characters", () => {
      const lexer = new Lexer("'0' '1' '9'");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.CHARACTER);
      expect(tokens[0].value).toBe("0");
      expect(tokens[1].type).toBe(TokenType.CHARACTER);
      expect(tokens[1].value).toBe("1");
      expect(tokens[2].type).toBe(TokenType.CHARACTER);
      expect(tokens[2].value).toBe("9");
    });

    it("should tokenize special characters", () => {
      const lexer = new Lexer("'!' '@' '#' '$'");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.CHARACTER);
      expect(tokens[0].value).toBe("!");
      expect(tokens[1].type).toBe(TokenType.CHARACTER);
      expect(tokens[1].value).toBe("@");
      expect(tokens[2].type).toBe(TokenType.CHARACTER);
      expect(tokens[2].value).toBe("#");
      expect(tokens[3].type).toBe(TokenType.CHARACTER);
      expect(tokens[3].value).toBe("$");
    });
  });

  describe("Edge Cases - Numbers", () => {
    it("should tokenize large integers", () => {
      const lexer = new Lexer("999999999");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.INTEGER);
      expect(tokens[0].value).toBe("999999999");
    });

    it("should tokenize zero", () => {
      const lexer = new Lexer("0");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.INTEGER);
      expect(tokens[0].value).toBe("0");
    });

    it("should tokenize floats starting with zero", () => {
      const lexer = new Lexer("0.5 0.25 0.125");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.FLOAT_LITERAL);
      expect(tokens[0].value).toBe("0.5");
      expect(tokens[1].type).toBe(TokenType.FLOAT_LITERAL);
      expect(tokens[1].value).toBe("0.25");
      expect(tokens[2].type).toBe(TokenType.FLOAT_LITERAL);
      expect(tokens[2].value).toBe("0.125");
    });

    it("should tokenize floats with many decimal places", () => {
      const lexer = new Lexer("3.14159265");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.FLOAT_LITERAL);
      expect(tokens[0].value).toBe("3.14159265");
    });

    it("should tokenize floats without leading zero", () => {
      const lexer = new Lexer(".5 .25");
      const tokens = lexer.tokenize();

      // Note: This might be handled differently depending on implementation
      // The lexer should handle this as DOT followed by INTEGER
      expect(tokens[0].type).toBe(TokenType.DOT);
      expect(tokens[1].type).toBe(TokenType.INTEGER);
    });
  });

  describe("Edge Cases - Identifiers", () => {
    it("should tokenize single character identifier", () => {
      const lexer = new Lexer("x");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[0].value).toBe("x");
    });

    it("should tokenize identifier starting with underscore", () => {
      const lexer = new Lexer("_x _y _z");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[0].value).toBe("_x");
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].value).toBe("_y");
      expect(tokens[2].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[2].value).toBe("_z");
    });

    it("should tokenize identifier with only underscores", () => {
      const lexer = new Lexer("_");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[0].value).toBe("_");
    });

    it("should tokenize identifier with multiple underscores", () => {
      const lexer = new Lexer("__x __y __private");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[0].value).toBe("__x");
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].value).toBe("__y");
      expect(tokens[2].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[2].value).toBe("__private");
    });

    it("should tokenize identifier with mixed case", () => {
      const lexer = new Lexer("MyVariable someOtherName");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[0].value).toBe("MyVariable");
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].value).toBe("someOtherName");
    });

    it("should tokenize identifier with trailing numbers", () => {
      const lexer = new Lexer("value1 temp2 var3");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[0].value).toBe("value1");
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].value).toBe("temp2");
      expect(tokens[2].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[2].value).toBe("var3");
    });

    it("should tokenize identifier with numbers in middle", () => {
      const lexer = new Lexer("var123name test456value");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[0].value).toBe("var123name");
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].value).toBe("test456value");
    });

    it("should not tokenize numbers as identifiers", () => {
      const lexer = new Lexer("123 456");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.INTEGER);
      expect(tokens[0].value).toBe("123");
      expect(tokens[1].type).toBe(TokenType.INTEGER);
      expect(tokens[1].value).toBe("456");
    });
  });

  describe("Edge Cases - Comments", () => {
    it("should handle single line comment at start", () => {
      const lexer = new Lexer("// comment\nint x;");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.INT);
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].value).toBe("x");
    });

    it("should handle single line comment at end", () => {
      const lexer = new Lexer("int x; // comment");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.INT);
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].value).toBe("x");
      expect(tokens[2].type).toBe(TokenType.SEMICOLON);
    });

    it("should handle single line comment in middle", () => {
      const lexer = new Lexer("int x; // comment\nint y;");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.INT);
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].value).toBe("x");
      expect(tokens[2].type).toBe(TokenType.SEMICOLON);
      expect(tokens[3].type).toBe(TokenType.INT);
      expect(tokens[4].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[4].value).toBe("y");
    });

    it("should handle multiple single line comments", () => {
      const lexer = new Lexer("// comment1\n// comment2\nint x;");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.INT);
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].value).toBe("x");
    });

    it("should handle block comment on single line", () => {
      const lexer = new Lexer("int x; /* comment */ int y;");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.INT);
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].value).toBe("x");
      expect(tokens[2].type).toBe(TokenType.SEMICOLON);
      expect(tokens[3].type).toBe(TokenType.INT);
      expect(tokens[4].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[4].value).toBe("y");
    });

    it("should handle multi-line block comment", () => {
      const lexer = new Lexer("int x; /* multi\nline\ncomment */ int y;");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.INT);
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].value).toBe("x");
      expect(tokens[2].type).toBe(TokenType.SEMICOLON);
      expect(tokens[3].type).toBe(TokenType.INT);
      expect(tokens[4].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[4].value).toBe("y");
    });

    it("should handle block comment at start", () => {
      const lexer = new Lexer("/* comment */ int x;");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.INT);
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].value).toBe("x");
    });

    it("should handle block comment at end", () => {
      const lexer = new Lexer("int x; /* comment */");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.INT);
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].value).toBe("x");
      expect(tokens[2].type).toBe(TokenType.SEMICOLON);
    });

    it("should handle empty block comment", () => {
      const lexer = new Lexer("int x; /**/ int y;");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.INT);
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].value).toBe("x");
      expect(tokens[2].type).toBe(TokenType.SEMICOLON);
      expect(tokens[3].type).toBe(TokenType.INT);
      expect(tokens[4].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[4].value).toBe("y");
    });

    it("should handle nested block comment style (not actually nested)", () => {
      const lexer = new Lexer("int x; /* outer /* inner */ outer */ int y;");
      const tokens = lexer.tokenize();

      // The first */ closes the comment, so "outer */" becomes code
      expect(tokens[0].type).toBe(TokenType.INT);
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].value).toBe("x");
      expect(tokens[2].type).toBe(TokenType.SEMICOLON);
      // "outer" would be tokenized as identifier
      expect(tokens[3].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[3].value).toBe("outer");
    });

    it("should handle comment-like content in string", () => {
      const lexer = new Lexer('"// not a comment"');
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.STRING);
      expect(tokens[0].value).toBe("// not a comment");
    });

    it("should handle block comment markers in string", () => {
      const lexer = new Lexer('"/* not a comment */"');
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.STRING);
      expect(tokens[0].value).toBe("/* not a comment */");
    });
  });

  describe("Edge Cases - Position Tracking", () => {
    it("should track line numbers correctly", () => {
      const lexer = new Lexer("int x;\nfloat y;\nchar z;");
      const tokens = lexer.tokenize();

      expect(tokens[0].line).toBe(1);
      expect(tokens[4].line).toBe(2);
      expect(tokens[8].line).toBe(3);
    });

    it("should track column numbers correctly", () => {
      const lexer = new Lexer("int x;");
      const tokens = lexer.tokenize();

      expect(tokens[0].column).toBe(1); // int
      expect(tokens[1].column).toBe(5); // x
      expect(tokens[2].column).toBe(6); // ;
    });

    it("should track position after whitespace", () => {
      const lexer = new Lexer("  int  x  ;");
      const tokens = lexer.tokenize();

      expect(tokens[0].column).toBe(3); // int
      expect(tokens[1].column).toBe(8); // x
      expect(tokens[2].column).toBe(11); // ;
    });

    it("should track position after newlines", () => {
      const lexer = new Lexer("\n\nint x;");
      const tokens = lexer.tokenize();

      expect(tokens[0].line).toBe(3);
      expect(tokens[0].column).toBe(1);
    });

    it("should track position after tabs", () => {
      const lexer = new Lexer("\t\tint x;");
      const tokens = lexer.tokenize();

      expect(tokens[0].column).toBe(3);
    });

    it("should track position in multi-line statement", () => {
      const lexer = new Lexer("int x =\n  42;");
      const tokens = lexer.tokenize();

      expect(tokens[0].line).toBe(1);
      expect(tokens[0].column).toBe(1);
      expect(tokens[4].line).toBe(2);
      expect(tokens[4].column).toBe(3);
    });

    it("should track position through comments", () => {
      const lexer = new Lexer("int x; /* comment */\nfloat y;");
      const tokens = lexer.tokenize();

      expect(tokens[0].line).toBe(1);
      expect(tokens[3].line).toBe(2);
    });
  });

  describe("Edge Cases - Whitespace", () => {
    it("should handle spaces", () => {
      const lexer = new Lexer("int x;");
      const tokens = lexer.tokenize();

      expect(tokens).toHaveLength(4); // INT, IDENTIFIER, SEMICOLON, EOF
    });

    it("should handle tabs", () => {
      const lexer = new Lexer("int\tx;");
      const tokens = lexer.tokenize();

      expect(tokens).toHaveLength(4);
    });

    it("should handle newlines", () => {
      const lexer = new Lexer("int\nx;");
      const tokens = lexer.tokenize();

      expect(tokens).toHaveLength(4);
    });

    it("should handle carriage returns", () => {
      const lexer = new Lexer("int\rx;");
      const tokens = lexer.tokenize();

      expect(tokens).toHaveLength(4);
    });

    it("should handle mixed whitespace", () => {
      const lexer = new Lexer("int \t\n\r x;");
      const tokens = lexer.tokenize();

      expect(tokens).toHaveLength(4);
    });

    it("should handle multiple spaces", () => {
      const lexer = new Lexer("int    x;");
      const tokens = lexer.tokenize();

      expect(tokens).toHaveLength(4);
    });

    it("should handle multiple tabs", () => {
      const lexer = new Lexer("int\t\t\tx;");
      const tokens = lexer.tokenize();

      expect(tokens).toHaveLength(4);
    });

    it("should handle multiple newlines", () => {
      const lexer = new Lexer("int\n\n\nx;");
      const tokens = lexer.tokenize();

      expect(tokens).toHaveLength(4);
    });
  });

  describe("Edge Cases - Complex Programs", () => {
    it("should tokenize complete function", () => {
      const source = `
int factorial(int n) {
    if (n <= 1) {
        return 1;
    }
    return n * factorial(n - 1);
}
`;
      const lexer = new Lexer(source);
      const tokens = lexer.tokenize();

      expect(tokens.length).toBeGreaterThan(20);
      expect(tokens[0].type).toBe(TokenType.INT);
      expect(tokens[tokens.length - 1].type).toBe(TokenType.EOF);
    });

    it("should tokenize main function", () => {
      const source = `
int main() {
    printf("Hello, World!\\n");
    return 0;
}
`;
      const lexer = new Lexer(source);
      const tokens = lexer.tokenize();

      expect(tokens.length).toBeGreaterThan(15);
      expect(tokens[0].type).toBe(TokenType.INT);
      expect(tokens[tokens.length - 1].type).toBe(TokenType.EOF);
    });

    it("should tokenize for loop with complex body", () => {
      const source = `
for (int i = 0; i < 10; i++) {
    if (i % 2 == 0) {
        printf("%d\\n", i);
    }
}
`;
      const lexer = new Lexer(source);
      const tokens = lexer.tokenize();

      expect(tokens.length).toBeGreaterThan(20);
      expect(tokens[0].type).toBe(TokenType.FOR);
      expect(tokens[tokens.length - 1].type).toBe(TokenType.EOF);
    });

    it("should tokenize array operations", () => {
      const source = `
int arr[5] = {1, 2, 3, 4, 5};
for (int i = 0; i < 5; i++) {
    arr[i] = arr[i] * 2;
}
`;
      const lexer = new Lexer(source);
      const tokens = lexer.tokenize();

      expect(tokens.length).toBeGreaterThan(30);
      expect(tokens[0].type).toBe(TokenType.INT);
      expect(tokens[tokens.length - 1].type).toBe(TokenType.EOF);
    });

    it("should tokenize pointer operations", () => {
      const source = `
int x = 42;
int *ptr = &x;
*ptr = 100;
`;
      const lexer = new Lexer(source);
      const tokens = lexer.tokenize();

      expect(tokens.length).toBeGreaterThan(10);
      expect(tokens[0].type).toBe(TokenType.INT);
      expect(tokens[tokens.length - 1].type).toBe(TokenType.EOF);
    });
  });

  // =========================================================================
  // ERROR CASE TESTS
  // =========================================================================

  describe("Error Cases - Unterminated Literals", () => {
    it("should throw error for unterminated string literal", () => {
      const lexer = new Lexer('"unterminated');

      expect(() => {
        lexer.tokenize();
      }).toThrow("Unterminated string literal");
    });

    it("should throw error for unterminated string literal with content", () => {
      const lexer = new Lexer('"hello world');

      expect(() => {
        lexer.tokenize();
      }).toThrow("Unterminated string literal");
    });

    it("should throw error for unterminated string literal after newline", () => {
      const lexer = new Lexer('"unterminated\nint x;');

      expect(() => {
        lexer.tokenize();
      }).toThrow("Unterminated string literal");
    });

    it("should throw error for unterminated character literal", () => {
      const lexer = new Lexer("'unterminated");

      expect(() => {
        lexer.tokenize();
      }).toThrow("Unterminated character literal");
    });

    it("should throw error for unterminated character literal with escape", () => {
      const lexer = new Lexer("'\\n");

      expect(() => {
        lexer.tokenize();
      }).toThrow("Unterminated character literal");
    });

    it("should throw error for unterminated character literal after newline", () => {
      const lexer = new Lexer("'a\nint x;");

      expect(() => {
        lexer.tokenize();
      }).toThrow("Unterminated character literal");
    });
  });

  describe("Error Cases - Unterminated Block Comment", () => {
    it("should handle unterminated block comment gracefully", () => {
      const lexer = new Lexer("int x; /* unterminated comment");

      // The lexer should not throw, but may not produce correct tokens
      // This is an edge case where behavior may vary
      const tokens = lexer.tokenize();

      // Should at least have the tokens before the comment
      expect(tokens[0].type).toBe(TokenType.INT);
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].value).toBe("x");
      expect(tokens[2].type).toBe(TokenType.SEMICOLON);
    });

    it("should handle unterminated block comment at end", () => {
      const lexer = new Lexer("/* unterminated");

      const tokens = lexer.tokenize();

      // Should only have EOF
      expect(tokens).toHaveLength(1);
      expect(tokens[0].type).toBe(TokenType.EOF);
    });
  });

  describe("Error Cases - Unknown Characters", () => {
    it("should tokenize unknown character as UNKNOWN token", () => {
      const lexer = new Lexer("int $x;");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.INT);
      expect(tokens[1].type).toBe(TokenType.UNKNOWN);
      expect(tokens[1].value).toBe("$");
      expect(tokens[2].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[2].value).toBe("x");
    });

    it("should tokenize multiple unknown characters", () => {
      const lexer = new Lexer("@ # $ % ^");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.UNKNOWN);
      expect(tokens[0].value).toBe("@");
      expect(tokens[1].type).toBe(TokenType.UNKNOWN);
      expect(tokens[1].value).toBe("#");
      expect(tokens[2].type).toBe(TokenType.UNKNOWN);
      expect(tokens[2].value).toBe("$");
      expect(tokens[3].type).toBe(TokenType.UNKNOWN);
      expect(tokens[3].value).toBe("%");
      expect(tokens[4].type).toBe(TokenType.UNKNOWN);
      expect(tokens[4].value).toBe("^");
    });

    it("should tokenize backtick as unknown", () => {
      const lexer = new Lexer("`");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.UNKNOWN);
      expect(tokens[0].value).toBe("`");
    });

    it("should tokenize tilde as unknown", () => {
      const lexer = new Lexer("~");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.UNKNOWN);
      expect(tokens[0].value).toBe("~");
    });

    it("should tokenize backslash outside string as unknown", () => {
      const lexer = new Lexer("\\");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.UNKNOWN);
      expect(tokens[0].value).toBe("\\");
    });
  });

  describe("Error Cases - Ambiguous Tokens", () => {
    it("should handle dot followed by number correctly", () => {
      const lexer = new Lexer(".5");
      const tokens = lexer.tokenize();

      // Should be DOT followed by INTEGER
      expect(tokens[0].type).toBe(TokenType.DOT);
      expect(tokens[1].type).toBe(TokenType.INTEGER);
      expect(tokens[1].value).toBe("5");
    });

    it("should handle multiple dots", () => {
      const lexer = new Lexer("...");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.DOT);
      expect(tokens[1].type).toBe(TokenType.DOT);
      expect(tokens[2].type).toBe(TokenType.DOT);
    });

    it("should handle multiple asterisks", () => {
      const lexer = new Lexer("***");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.MULTIPLY);
      expect(tokens[1].type).toBe(TokenType.MULTIPLY);
      expect(tokens[2].type).toBe(TokenType.MULTIPLY);
    });

    it("should handle multiple ampersands", () => {
      const lexer = new Lexer("&&&");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.AND);
      expect(tokens[0].value).toBe("&&");
      expect(tokens[1].type).toBe(TokenType.ADDRESS);
      expect(tokens[1].value).toBe("&");
    });

    it("should handle multiple pipes", () => {
      const lexer = new Lexer("|||");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.OR);
      expect(tokens[0].value).toBe("||");
      expect(tokens[1].type).toBe(TokenType.OR);
      expect(tokens[1].value).toBe("|");
    });

    it("should handle multiple equals", () => {
      const lexer = new Lexer("===");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.EQUAL);
      expect(tokens[0].value).toBe("==");
      expect(tokens[1].type).toBe(TokenType.ASSIGN);
      expect(tokens[1].value).toBe("=");
    });

    it("should handle multiple less than", () => {
      const lexer = new Lexer("<<<");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.LESS);
      expect(tokens[0].value).toBe("<<");
      expect(tokens[1].type).toBe(TokenType.LESS);
      expect(tokens[1].value).toBe("<");
    });

    it("should handle multiple greater than", () => {
      const lexer = new Lexer(">>>");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.GREATER);
      expect(tokens[0].value).toBe(">>");
      expect(tokens[1].type).toBe(TokenType.GREATER);
      expect(tokens[1].value).toBe(">");
    });

    it("should handle plus followed by equals", () => {
      const lexer = new Lexer("+=");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.PLUS_ASSIGN);
      expect(tokens[0].value).toBe("+=");
    });

    it("should handle minus followed by equals", () => {
      const lexer = new Lexer("-=");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.MINUS_ASSIGN);
      expect(tokens[0].value).toBe("-=");
    });
  });

  // =========================================================================
  // LEXER STATE TESTS
  // =========================================================================

  describe("Lexer State Management", () => {
    it("should reset lexer state", () => {
      const lexer = new Lexer("int x;");
      lexer.tokenize();

      lexer.reset();
      const tokens = lexer.tokenize("float y;");

      expect(tokens[0].type).toBe(TokenType.FLOAT);
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].value).toBe("y");
    });

    it("should tokenize new source after reset", () => {
      const lexer = new Lexer("int x;");
      lexer.tokenize();

      lexer.reset();
      const tokens = lexer.tokenize("char z;");

      expect(tokens[0].type).toBe(TokenType.CHAR);
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].value).toBe("z");
    });

    it("should get current position", () => {
      const lexer = new Lexer("int x;");
      const position = lexer.getPosition();

      expect(position.line).toBe(1);
      expect(position.column).toBe(1);
    });

    it("should update position after tokenization", () => {
      const lexer = new Lexer("int x;\nfloat y;");
      lexer.tokenize();
      const position = lexer.getPosition();

      expect(position.line).toBeGreaterThan(1);
    });

    it("should tokenize with new source parameter", () => {
      const lexer = new Lexer("int x;");
      const tokens1 = lexer.tokenize();

      expect(tokens1[0].type).toBe(TokenType.INT);

      const tokens2 = lexer.tokenize("float y;");

      expect(tokens2[0].type).toBe(TokenType.FLOAT);
    });

    it("should tokenize without new source parameter", () => {
      const lexer = new Lexer("int x;");
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.INT);
    });

    it("should handle multiple tokenizations", () => {
      const lexer = new Lexer("int x;");

      const tokens1 = lexer.tokenize();
      const tokens2 = lexer.tokenize();

      expect(tokens1).toEqual(tokens2);
    });
  });

  // =========================================================================
  // REAL-WORLD C CODE EXAMPLES
  // =========================================================================

  describe("Real-World C Code", () => {
    it("should tokenize hello world program", () => {
      const source = `
#include <stdio.h>

int main() {
    printf("Hello, World!\\n");
    return 0;
}
`;
      const lexer = new Lexer(source);
      const tokens = lexer.tokenize();

      expect(tokens.length).toBeGreaterThan(20);
      expect(tokens[tokens.length - 1].type).toBe(TokenType.EOF);
    });

    it("should tokenize variable declarations", () => {
      const source = `
int age = 25;
float height = 5.9;
char grade = 'A';
`;
      const lexer = new Lexer(source);
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.INT);
      expect(tokens[4].type).toBe(TokenType.FLOAT);
      expect(tokens[8].type).toBe(TokenType.CHAR);
    });

    it("should tokenize arithmetic expressions", () => {
      const source = `
int result = (a + b) * (c - d) / e;
`;
      const lexer = new Lexer(source);
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.INT);
      expect(tokens[6].type).toBe(TokenType.PLUS);
      expect(tokens[10].type).toBe(TokenType.MINUS);
      expect(tokens[14].type).toBe(TokenType.MULTIPLY);
      expect(tokens[18].type).toBe(TokenType.DIVIDE);
    });

    it("should tokenize conditional statements", () => {
      const source = `
if (score >= 90) {
    grade = 'A';
} else if (score >= 80) {
    grade = 'B';
} else {
    grade = 'C';
}
`;
      const lexer = new Lexer(source);
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.IF);
      expect(tokens[12].type).toBe(TokenType.ELSE);
    });

    it("should tokenize loop structures", () => {
      const source = `
while (count < 10) {
    sum += count;
    count++;
}
`;
      const lexer = new Lexer(source);
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.WHILE);
      expect(tokens[8].type).toBe(TokenType.PLUS_ASSIGN);
      expect(tokens[13].type).toBe(TokenType.PLUS);
    });

    it("should tokenize function with parameters", () => {
      const source = `
int calculate(int a, int b, int c) {
    return (a + b + c) / 3;
}
`;
      const lexer = new Lexer(source);
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.INT);
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].value).toBe("calculate");
      expect(tokens[3].type).toBe(TokenType.INT);
      expect(tokens[5].type).toBe(TokenType.COMMA);
    });

    it("should tokenize array initialization", () => {
      const source = `
int numbers[5] = {10, 20, 30, 40, 50};
`;
      const lexer = new Lexer(source);
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.INT);
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].value).toBe("numbers");
      expect(tokens[3].type).toBe(TokenType.INTEGER);
      expect(tokens[3].value).toBe("5");
    });

    it("should tokenize pointer operations", () => {
      const source = `
int value = 42;
int *ptr = &value;
*ptr = 100;
printf("Value: %d\\n", *ptr);
`;
      const lexer = new Lexer(source);
      const tokens = lexer.tokenize();

      expect(tokens[5].type).toBe(TokenType.MULTIPLY);
      expect(tokens[9].type).toBe(TokenType.ADDRESS);
      expect(tokens[12].type).toBe(TokenType.MULTIPLY);
    });
  });
});