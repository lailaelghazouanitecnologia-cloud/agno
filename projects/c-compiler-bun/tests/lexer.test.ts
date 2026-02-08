/**
 * Lexer tests
 */

import { describe, it, expect } from "bun:test";
import { Lexer } from "../src/lexer/lexer.js";
import { TokenType } from "../src/core/interfaces.js";

describe("Lexer", () => {
  it("should tokenize numbers", () => {
    const lexer = new Lexer("42 3.14");
    const tokens = lexer.tokenize();

    expect(tokens[0].type).toBe(TokenType.NUMBER);
    expect(tokens[0].value).toBe("42");
    expect(tokens[1].type).toBe(TokenType.NUMBER);
    expect(tokens[1].value).toBe("3.14");
  });

  it("should tokenize identifiers", () => {
    const lexer = new Lexer("foo bar_baz");
    const tokens = lexer.tokenize();

    expect(tokens[0].type).toBe(TokenType.IDENTIFIER);
    expect(tokens[0].value).toBe("foo");
    expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
    expect(tokens[1].value).toBe("bar_baz");
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
    const lexer = new Lexer("+ - * / % < > <= >= == !=");
    const tokens = lexer.tokenize();

    expect(tokens[0].type).toBe(TokenType.PLUS);
    expect(tokens[1].type).toBe(TokenType.MINUS);
    expect(tokens[2].type).toBe(TokenType.STAR);
    expect(tokens[3].type).toBe(TokenType.SLASH);
    expect(tokens[4].type).toBe(TokenType.PERCENT);
    expect(tokens[5].type).toBe(TokenType.LESS);
    expect(tokens[6].type).toBe(TokenType.GREATER);
    expect(tokens[7].type).toBe(TokenType.LESS_EQUAL);
    expect(tokens[8].type).toBe(TokenType.GREATER_EQUAL);
    expect(tokens[9].type).toBe(TokenType.EQUAL);
    expect(tokens[10].type).toBe(TokenType.NOT_EQUAL);
  });

  it("should tokenize punctuation", () => {
    const lexer = new Lexer("; , ( ) { } [ ]");
    const tokens = lexer.tokenize();

    expect(tokens[0].type).toBe(TokenType.SEMICOLON);
    expect(tokens[1].type).toBe(TokenType.COMMA);
    expect(tokens[2].type).toBe(TokenType.LPAREN);
    expect(tokens[3].type).toBe(TokenType.RPAREN);
    expect(tokens[4].type).toBe(TokenType.LBRACE);
    expect(tokens[5].type).toBe(TokenType.RBRACE);
    expect(tokens[6].type).toBe(TokenType.LBRACKET);
    expect(tokens[7].type).toBe(TokenType.RBRACKET);
  });

  it("should tokenize string literals", () => {
    const lexer = new Lexer('"hello world"');
    const tokens = lexer.tokenize();

    expect(tokens[0].type).toBe(TokenType.STRING);
    expect(tokens[0].value).toBe("hello world");
  });

  it("should tokenize character literals", () => {
    const lexer = new Lexer("'a' '\\n'");
    const tokens = lexer.tokenize();

    expect(tokens[0].type).toBe(TokenType.NUMBER);
    expect(tokens[0].value).toBe("97"); // 'a' = 97
    expect(tokens[1].type).toBe(TokenType.NUMBER);
    expect(tokens[1].value).toBe("10"); // '\n' = 10
  });

  it("should handle line comments", () => {
    const lexer = new Lexer("42 // this is a comment\n43");
    const tokens = lexer.tokenize();

    expect(tokens[0].type).toBe(TokenType.NUMBER);
    expect(tokens[0].value).toBe("42");
    expect(tokens[1].type).toBe(TokenType.NUMBER);
    expect(tokens[1].value).toBe("43");
  });

  it("should handle block comments", () => {
    const lexer = new Lexer("42 /* block\ncomment */ 43");
    const tokens = lexer.tokenize();

    expect(tokens[0].type).toBe(TokenType.NUMBER);
    expect(tokens[0].value).toBe("42");
    expect(tokens[1].type).toBe(TokenType.NUMBER);
    expect(tokens[1].value).toBe("43");
  });

  it("should track line and column numbers", () => {
    const lexer = new Lexer("int x;\nfloat y;");
    const tokens = lexer.tokenize();

    expect(tokens[0].line).toBe(1);
    expect(tokens[0].column).toBe(1);
    expect(tokens[4].line).toBe(2);
    expect(tokens[4].column).toBe(1);
  });

  it("should add EOF token at the end", () => {
    const lexer = new Lexer("42");
    const tokens = lexer.tokenize();

    expect(tokens[tokens.length - 1].type).toBe(TokenType.EOF);
  });
});