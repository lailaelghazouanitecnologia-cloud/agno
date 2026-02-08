import { Lexer } from '../Lexer';
import { TokenType } from '../../types/Token';

describe('Lexer', () => {
  describe('Basic tokenization', () => {
    it('should tokenize numbers', () => {
      const lexer = new Lexer('42');
      const tokens = lexer.tokenize();

      expect(tokens).toHaveLength(2); // NUMBER + EOF
      expect(tokens[0].type).toBe(TokenType.NUMBER);
      expect(tokens[0].literal).toBe(42);
    });

    it('should tokenize decimal numbers', () => {
      const lexer = new Lexer('3.14');
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.NUMBER);
      expect(tokens[0].literal).toBe(3.14);
    });

    it('should tokenize strings', () => {
      const lexer = new Lexer('"hello world"');
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.STRING);
      expect(tokens[0].literal).toBe('hello world');
    });

    it('should tokenize identifiers', () => {
      const lexer = new Lexer('myVariable');
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[0].lexeme).toBe('myVariable');
    });
  });

  describe('Keywords', () => {
    it('should tokenize all keywords', () => {
      const source = 'let function if else while print true false';
      const lexer = new Lexer(source);
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.LET);
      expect(tokens[1].type).toBe(TokenType.FUNCTION);
      expect(tokens[2].type).toBe(TokenType.IF);
      expect(tokens[3].type).toBe(TokenType.ELSE);
      expect(tokens[4].type).toBe(TokenType.WHILE);
      expect(tokens[5].type).toBe(TokenType.PRINT);
      expect(tokens[6].type).toBe(TokenType.TRUE);
      expect(tokens[7].type).toBe(TokenType.FALSE);
    });
  });

  describe('Operators', () => {
    it('should tokenize arithmetic operators', () => {
      const lexer = new Lexer('+ - * / %');
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.PLUS);
      expect(tokens[1].type).toBe(TokenType.MINUS);
      expect(tokens[2].type).toBe(TokenType.STAR);
      expect(tokens[3].type).toBe(TokenType.SLASH);
      expect(tokens[4].type).toBe(TokenType.PERCENT);
    });

    it('should tokenize comparison operators', () => {
      const lexer = new Lexer('== != < > <= >=');
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.EQUAL_EQUAL);
      expect(tokens[1].type).toBe(TokenType.BANG_EQUAL);
      expect(tokens[2].type).toBe(TokenType.LESS);
      expect(tokens[3].type).toBe(TokenType.GREATER);
      expect(tokens[4].type).toBe(TokenType.LESS_EQUAL);
      expect(tokens[5].type).toBe(TokenType.GREATER_EQUAL);
    });

    it('should tokenize assignment operator', () => {
      const lexer = new Lexer('=');
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.EQUAL);
    });
  });

  describe('Punctuation', () => {
    it('should tokenize punctuation', () => {
      const lexer = new Lexer('() {} , ;');
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.LEFT_PAREN);
      expect(tokens[1].type).toBe(TokenType.RIGHT_PAREN);
      expect(tokens[2].type).toBe(TokenType.LEFT_BRACE);
      expect(tokens[3].type).toBe(TokenType.RIGHT_BRACE);
      expect(tokens[4].type).toBe(TokenType.COMMA);
      expect(tokens[5].type).toBe(TokenType.SEMICOLON);
    });
  });

  describe('Complex expressions', () => {
    it('should tokenize a variable declaration', () => {
      const lexer = new Lexer('let x = 5;');
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.LET);
      expect(tokens[1].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[2].type).toBe(TokenType.EQUAL);
      expect(tokens[3].type).toBe(TokenType.NUMBER);
      expect(tokens[4].type).toBe(TokenType.SEMICOLON);
    });

    it('should tokenize an arithmetic expression', () => {
      const lexer = new Lexer('x + y * 10');
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[1].type).toBe(TokenType.PLUS);
      expect(tokens[2].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[3].type).toBe(TokenType.STAR);
      expect(tokens[4].type).toBe(TokenType.NUMBER);
    });

    it('should tokenize a function call', () => {
      const lexer = new Lexer('print(x);');
      const tokens = lexer.tokenize();

      expect(tokens[0].type).toBe(TokenType.PRINT);
      expect(tokens[1].type).toBe(TokenType.LEFT_PAREN);
      expect(tokens[2].type).toBe(TokenType.IDENTIFIER);
      expect(tokens[3].type).toBe(TokenType.RIGHT_PAREN);
      expect(tokens[4].type).toBe(TokenType.SEMICOLON);
    });
  });

  describe('Line and column tracking', () => {
    it('should track line numbers correctly', () => {
      const lexer = new Lexer('x\ny\nz');
      const tokens = lexer.tokenize();

      expect(tokens[0].line).toBe(1);
      expect(tokens[1].line).toBe(2);
      expect(tokens[2].line).toBe(3);
    });

    it('should track column numbers correctly', () => {
      const lexer = new Lexer('x + y');
      const tokens = lexer.tokenize();

      expect(tokens[0].column).toBe(1);
      expect(tokens[1].column).toBe(3);
      expect(tokens[2].column).toBe(5);
    });
  });

  describe('Error handling', () => {
    it('should throw error on unexpected character', () => {
      const lexer = new Lexer('x @ y');

      expect(() => lexer.tokenize()).toThrow();
    });

    it('should throw error on unterminated string', () => {
      const lexer = new Lexer('"unterminated');

      expect(() => lexer.tokenize()).toThrow();
    });
  });
});