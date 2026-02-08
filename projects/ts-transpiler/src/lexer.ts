/**
 * Lexer for TypeScript - tokenizes source code
 */

export enum TokenType {
  // Literals
  Identifier = "Identifier",
  NumericLiteral = "NumericLiteral",
  StringLiteral = "StringLiteral",
  BooleanLiteral = "BooleanLiteral",
  NullLiteral = "NullLiteral",
  TemplateLiteral = "TemplateLiteral",

  // Keywords
  Let = "Let",
  Const = "Const",
  Var = "Var",
  Function = "Function",
  Class = "Class",
  Interface = "Interface",
  Type = "Type",
  Enum = "Enum",
  Extends = "Extends",
  Implements = "Implements",
  Import = "Import",
  Export = "Export",
  From = "From",
  As = "As",
  Default = "Default",
  Return = "Return",
  If = "If",
  Else = "Else",
  For = "For",
  While = "While",
  Do = "Do",
  Switch = "Switch",
  Case = "Case",
  Break = "Break",
  Continue = "Continue",
  New = "New",
  This = "This",
  Super = "Super",
  Async = "Async",
  Await = "Await",
  Yield = "Yield",
  Try = "Try",
  Catch = "Catch",
  Finally = "Finally",
  Throw = "Throw",
  Typeof = "Typeof",
  Instanceof = "Instanceof",
  In = "In",
  Of = "Of",
  Void = "Void",
  Delete = "Delete",
  Constructor = "Constructor",

  // TypeScript keywords
  Readonly = "Readonly",
  Public = "Public",
  Private = "Private",
  Protected = "Protected",
  Abstract = "Abstract",
  Static = "Static",
  Namespace = "Namespace",
  Module = "Module",
  Declare = "Declare",
  Keyof = "Keyof",
  Infer = "Infer",
  Is = "Is",
  Never = "Never",
  Unknown = "Unknown",
  Any = "Any",
  String = "String",
  Number = "Number",
  Boolean = "Boolean",
  Symbol = "Symbol",
  Object = "Object",
  Undefined = "Undefined",
  Null = "Null",

  // Operators
  Plus = "Plus",
  Minus = "Minus",
  Asterisk = "Asterisk",
  Slash = "Slash",
  Percent = "Percent",
  Caret = "Caret",
  Ampersand = "Ampersand",
  Pipe = "Pipe",
  Tilde = "Tilde",
  Exclamation = "Exclamation",
  Equal = "Equal",
  LessThan = "LessThan",
  GreaterThan = "GreaterThan",
  Question = "Question",
  Colon = "Colon",
  Semicolon = "Semicolon",
  Comma = "Comma",
  Dot = "Dot",
  At = "At",
  PlusPlus = "PlusPlus",
  MinusMinus = "MinusMinus",

  // Compound operators
  PlusEqual = "PlusEqual",
  MinusEqual = "MinusEqual",
  AsteriskEqual = "AsteriskEqual",
  SlashEqual = "SlashEqual",
  PercentEqual = "PercentEqual",
  CaretEqual = "CaretEqual",
  AmpersandEqual = "AmpersandEqual",
  PipeEqual = "PipeEqual",
  LessThanEqual = "LessThanEqual",
  GreaterThanEqual = "GreaterThanEqual",
  EqualEqual = "EqualEqual",
  ExclamationEqual = "ExclamationEqual",
  EqualEqualEqual = "EqualEqualEqual",
  ExclamationEqualEqual = "ExclamationEqualEqual",
  AmpersandAmpersand = "AmpersandAmpersand",
  PipePipe = "PipePipe",
  LessThanLessThan = "LessThanLessThan",
  GreaterThanGreaterThan = "GreaterThanGreaterThan",
  LessThanLessThanEqual = "LessThanLessThanEqual",
  GreaterThanGreaterThanEqual = "GreaterThanGreaterThanEqual",
  Arrow = "Arrow",
  QuestionDot = "QuestionDot",
  QuestionQuestion = "QuestionQuestion",
  EqualGreaterThan = "EqualGreaterThan",

  // Punctuation
  LeftParen = "LeftParen",
  RightParen = "RightParen",
  LeftBrace = "LeftBrace",
  RightBrace = "RightBrace",
  LeftBracket = "LeftBracket",
  RightBracket = "RightBracket",
  Bar = "Bar",
  Ellipsis = "Ellipsis",

  // Special
  EOF = "EOF",
  Comment = "Comment",
}

export interface Token {
  type: TokenType;
  value: string;
  loc: {
    start: { line: number; column: number };
    end: { line: number; column: number };
  };
}

export class Lexer {
  private source: string;
  private pos: number = 0;
  private line: number = 1;
  private column: number = 1;
  private tokens: Token[] = [];

  private static readonly KEYWORDS: Map<string, TokenType> = new Map([
    ["let", TokenType.Let],
    ["const", TokenType.Const],
    ["var", TokenType.Var],
    ["function", TokenType.Function],
    ["class", TokenType.Class],
    ["interface", TokenType.Interface],
    ["type", TokenType.Type],
    ["enum", TokenType.Enum],
    ["extends", TokenType.Extends],
    ["implements", TokenType.Implements],
    ["import", TokenType.Import],
    ["export", TokenType.Export],
    ["from", TokenType.From],
    ["as", TokenType.As],
    ["default", TokenType.Default],
    ["return", TokenType.Return],
    ["if", TokenType.If],
    ["else", TokenType.Else],
    ["for", TokenType.For],
    ["while", TokenType.While],
    ["do", TokenType.Do],
    ["switch", TokenType.Switch],
    ["case", TokenType.Case],
    ["break", TokenType.Break],
    ["continue", TokenType.Continue],
    ["new", TokenType.New],
    ["this", TokenType.This],
    ["super", TokenType.Super],
    ["async", TokenType.Async],
    ["await", TokenType.Await],
    ["yield", TokenType.Yield],
    ["try", TokenType.Try],
    ["catch", TokenType.Catch],
    ["finally", TokenType.Finally],
    ["throw", TokenType.Throw],
    ["typeof", TokenType.Typeof],
    ["instanceof", TokenType.Instanceof],
    ["in", TokenType.In],
    ["of", TokenType.Of],
    ["void", TokenType.Void],
    ["delete", TokenType.Delete],
    ["constructor", TokenType.Constructor],
    ["readonly", TokenType.Readonly],
    ["public", TokenType.Public],
    ["private", TokenType.Private],
    ["protected", TokenType.Protected],
    ["abstract", TokenType.Abstract],
    ["static", TokenType.Static],
    ["namespace", TokenType.Namespace],
    ["module", TokenType.Module],
    ["declare", TokenType.Declare],
    ["keyof", TokenType.Keyof],
    ["infer", TokenType.Infer],
    ["is", TokenType.Is],
    ["never", TokenType.Never],
    ["unknown", TokenType.Unknown],
    ["any", TokenType.Any],
    ["string", TokenType.String],
    ["number", TokenType.Number],
    ["boolean", TokenType.Boolean],
    ["symbol", TokenType.Symbol],
    ["object", TokenType.Object],
    ["undefined", TokenType.Undefined],
    ["null", TokenType.Null],
    ["true", TokenType.BooleanLiteral],
    ["false", TokenType.BooleanLiteral],
  ]);

  constructor(source: string) {
    this.source = source;
  }

  public tokenize(): Token[] {
    this.tokens = [];
    this.pos = 0;
    this.line = 1;
    this.column = 1;

    while (this.pos < this.source.length) {
      this.skipWhitespace();

      if (this.pos >= this.source.length) {
        break;
      }

      const char = this.source[this.pos];

      // Comments
      if (char === "/" && this.source[this.pos + 1] === "/") {
        this.readLineComment();
        continue;
      }

      if (char === "/" && this.source[this.pos + 1] === "*") {
        this.readBlockComment();
        continue;
      }

      // String literals
      if (char === '"' || char === "'" || char === "`") {
        this.tokens.push(this.readStringLiteral());
        continue;
      }

      // Numeric literals
      if (this.isDigit(char)) {
        this.tokens.push(this.readNumericLiteral());
        continue;
      }

      // Identifiers and keywords
      if (this.isIdentifierStart(char)) {
        this.tokens.push(this.readIdentifier());
        continue;
      }

      // Operators and punctuation
      const token = this.readOperatorOrPunctuation();
      if (token) {
        this.tokens.push(token);
        continue;
      }

      // Unknown character
      throw new Error(
        `Unexpected character '${char}' at line ${this.line}, column ${this.column}`
      );
    }

    // Add EOF token
    this.tokens.push({
      type: TokenType.EOF,
      value: "",
      loc: {
        start: { line: this.line, column: this.column },
        end: { line: this.line, column: this.column },
      },
    });

    return this.tokens;
  }

  private skipWhitespace(): void {
    while (this.pos < this.source.length) {
      const char = this.source[this.pos];
      if (char === " " || char === "\t" || char === "\r") {
        this.advance();
      } else if (char === "\n") {
        this.line++;
        this.column = 1;
        this.pos++;
      } else {
        break;
      }
    }
  }

  private advance(): void {
    this.pos++;
    this.column++;
  }

  private peek(offset: number = 0): string {
    return this.source[this.pos + offset] || "";
  }

  private isDigit(char: string): boolean {
    return char >= "0" && char <= "9";
  }

  private isIdentifierStart(char: string): boolean {
    return (
      (char >= "a" && char <= "z") ||
      (char >= "A" && char <= "Z") ||
      char === "_" ||
      char === "$"
    );
  }

  private isIdentifierPart(char: string): boolean {
    return (
      this.isIdentifierStart(char) || this.isDigit(char)
    );
  }

  private readLineComment(): void {
    const startLine = this.line;
    const startColumn = this.column;
    this.advance(); // /
    this.advance(); // /

    while (this.pos < this.source.length && this.source[this.pos] !== "\n") {
      this.advance();
    }
  }

  private readBlockComment(): void {
    this.advance(); // /
    this.advance(); // *

    while (this.pos < this.source.length) {
      if (this.source[this.pos] === "*" && this.peek(1) === "/") {
        this.advance();
        this.advance();
        return;
      }
      if (this.source[this.pos] === "\n") {
        this.line++;
        this.column = 1;
        this.pos++;
      } else {
        this.advance();
      }
    }

    throw new Error("Unterminated block comment");
  }

  private readStringLiteral(): Token {
    const quote = this.source[this.pos];
    const startLine = this.line;
    const startColumn = this.column;
    this.advance();

    let value = "";

    while (this.pos < this.source.length) {
      const char = this.source[this.pos];

      if (char === quote) {
        this.advance();
        return {
          type: quote === "`" ? TokenType.TemplateLiteral : TokenType.StringLiteral,
          value,
          loc: {
            start: { line: startLine, column: startColumn },
            end: { line: this.line, column: this.column },
          },
        };
      }

      if (char === "\\") {
        this.advance();
        const escaped = this.source[this.pos];
        value += this.parseEscapeSequence(escaped);
        this.advance();
        continue;
      }

      if (char === "\n") {
        this.line++;
        this.column = 1;
        this.pos++;
        value += "\n";
        continue;
      }

      value += char;
      this.advance();
    }

    throw new Error("Unterminated string literal");
  }

  private parseEscapeSequence(char: string): string {
    switch (char) {
      case "n": return "\n";
      case "r": return "\r";
      case "t": return "\t";
      case "b": return "\b";
      case "f": return "\f";
      case "v": return "\v";
      case "0": return "\0";
      case "'": return "'";
      case '"': return '"';
      case "\\": return "\\";
      default: return char;
    }
  }

  private readNumericLiteral(): Token {
    const startLine = this.line;
    const startColumn = this.column;
    let value = "";

    while (this.pos < this.source.length && this.isDigit(this.source[this.pos])) {
      value += this.source[this.pos];
      this.advance();
    }

    // Decimal part
    if (this.source[this.pos] === ".") {
      value += ".";
      this.advance();

      while (this.pos < this.source.length && this.isDigit(this.source[this.pos])) {
        value += this.source[this.pos];
        this.advance();
      }
    }

    // Exponent part
    if (this.source[this.pos] === "e" || this.source[this.pos] === "E") {
      value += this.source[this.pos];
      this.advance();

      if (this.source[this.pos] === "+" || this.source[this.pos] === "-") {
        value += this.source[this.pos];
        this.advance();
      }

      while (this.pos < this.source.length && this.isDigit(this.source[this.pos])) {
        value += this.source[this.pos];
        this.advance();
      }
    }

    return {
      type: TokenType.NumericLiteral,
      value,
      loc: {
        start: { line: startLine, column: startColumn },
        end: { line: this.line, column: this.column },
      },
    };
  }

  private readIdentifier(): Token {
    const startLine = this.line;
    const startColumn = this.column;
    let value = "";

    while (this.pos < this.source.length && this.isIdentifierPart(this.source[this.pos])) {
      value += this.source[this.pos];
      this.advance();
    }

    // Check if it's a keyword
    const keywordType = Lexer.KEYWORDS.get(value);
    const type = keywordType || TokenType.Identifier;

    return {
      type,
      value,
      loc: {
        start: { line: startLine, column: startColumn },
        end: { line: this.line, column: this.column },
      },
    };
  }

  private readOperatorOrPunctuation(): Token | null {
    const startLine = this.line;
    const startColumn = this.column;
    const char = this.source[this.pos];

    // Multi-character operators
    const twoChar = char + this.peek(1);
    const threeChar = char + this.peek(1) + this.peek(2);

    switch (threeChar) {
      case "...":
        this.advance();
        this.advance();
        this.advance();
        return {
          type: TokenType.Ellipsis,
          value: "...",
          loc: {
            start: { line: startLine, column: startColumn },
            end: { line: this.line, column: this.column },
          },
        };
      case "===":
        this.advance();
        this.advance();
        this.advance();
        return {
          type: TokenType.EqualEqualEqual,
          value: "===",
          loc: {
            start: { line: startLine, column: startColumn },
            end: { line: this.line, column: this.column },
          },
        };
      case "!==":
        this.advance();
        this.advance();
        this.advance();
        return {
          type: TokenType.ExclamationEqualEqual,
          value: "!==",
          loc: {
            start: { line: startLine, column: startColumn },
            end: { line: this.line, column: this.column },
          },
        };
    }

    switch (twoChar) {
      case "=>":
        this.advance();
        this.advance();
        return {
          type: TokenType.Arrow,
          value: "=>",
          loc: {
            start: { line: startLine, column: startColumn },
            end: { line: this.line, column: this.column },
          },
        };
      case "==":
        this.advance();
        this.advance();
        return {
          type: TokenType.EqualEqual,
          value: "==",
          loc: {
            start: { line: startLine, column: startColumn },
            end: { line: this.line, column: this.column },
          },
        };
      case "!=":
        this.advance();
        this.advance();
        return {
          type: TokenType.ExclamationEqual,
          value: "!=",
          loc: {
            start: { line: startLine, column: startColumn },
            end: { line: this.line, column: this.column },
          },
        };
      case "&&":
        this.advance();
        this.advance();
        return {
          type: TokenType.AmpersandAmpersand,
          value: "&&",
          loc: {
            start: { line: startLine, column: startColumn },
            end: { line: this.line, column: this.column },
          },
        };
      case "||":
        this.advance();
        this.advance();
        return {
          type: TokenType.PipePipe,
          value: "||",
          loc: {
            start: { line: startLine, column: startColumn },
            end: { line: this.line, column: this.column },
          },
        };
      case "??":
        this.advance();
        this.advance();
        return {
          type: TokenType.QuestionQuestion,
          value: "??",
          loc: {
            start: { line: startLine, column: startColumn },
            end: { line: this.line, column: this.column },
          },
        };
      case "?.": {
        this.advance();
        this.advance();
        return {
          type: TokenType.QuestionDot,
          value: "?.",
          loc: {
            start: { line: startLine, column: startColumn },
            end: { line: this.line, column: this.column },
          },
        };
      }
      case "<=":
        this.advance();
        this.advance();
        return {
          type: TokenType.LessThanEqual,
          value: "<=",
          loc: {
            start: { line: startLine, column: startColumn },
            end: { line: this.line, column: this.column },
          },
        };
      case ">=":
        this.advance();
        this.advance();
        return {
          type: TokenType.GreaterThanEqual,
          value: ">=",
          loc: {
            start: { line: startLine, column: startColumn },
            end: { line: this.line, column: this.column },
          },
        };
      case "<<":
        this.advance();
        this.advance();
        return {
          type: TokenType.LessThanLessThan,
          value: "<<",
          loc: {
            start: { line: startLine, column: startColumn },
            end: { line: this.line, column: this.column },
          },
        };
      case ">>":
        this.advance();
        this.advance();
        return {
          type: TokenType.GreaterThanGreaterThan,
          value: ">>",
          loc: {
            start: { line: startLine, column: startColumn },
            end: { line: this.line, column: this.column },
          },
        };
      case "+=":
        this.advance();
        this.advance();
        return {
          type: TokenType.PlusEqual,
          value: "+=",
          loc: {
            start: { line: startLine, column: startColumn },
            end: { line: this.line, column: this.column },
          },
        };
      case "-=":
        this.advance();
        this.advance();
        return {
          type: TokenType.MinusEqual,
          value: "-=",
          loc: {
            start: { line: startLine, column: startColumn },
            end: { line: this.line, column: this.column },
          },
        };
      case "*=":
        this.advance();
        this.advance();
        return {
          type: TokenType.AsteriskEqual,
          value: "*=",
          loc: {
            start: { line: startLine, column: startColumn },
            end: { line: this.line, column: this.column },
          },
        };
      case "/=":
        this.advance();
        this.advance();
        return {
          type: TokenType.SlashEqual,
          value: "/=",
          loc: {
            start: { line: startLine, column: startColumn },
            end: { line: this.line, column: this.column },
          },
        };
      case "%=":
        this.advance();
        this.advance();
        return {
          type: TokenType.PercentEqual,
          value: "%=",
          loc: {
            start: { line: startLine, column: startColumn },
            end: { line: this.line, column: this.column },
          },
        };
      case "^=":
        this.advance();
        this.advance();
        return {
          type: TokenType.CaretEqual,
          value: "^=",
          loc: {
            start: { line: startLine, column: startColumn },
            end: { line: this.line, column: this.column },
          },
        };
      case "&=":
        this.advance();
        this.advance();
        return {
          type: TokenType.AmpersandEqual,
          value: "&=",
          loc: {
            start: { line: startLine, column: startColumn },
            end: { line: this.line, column: this.column },
          },
        };
      case "|=":
        this.advance();
        this.advance();
        return {
          type: TokenType.PipeEqual,
          value: "|=",
          loc: {
            start: { line: startLine, column: startColumn },
            end: { line: this.line, column: this.column },
          },
        };
      case "<<=":
        this.advance();
        this.advance();
        this.advance();
        return {
          type: TokenType.LessThanLessThanEqual,
          value: "<<=",
          loc: {
            start: { line: startLine, column: startColumn },
            end: { line: this.line, column: this.column },
          },
        };
      case ">>=":
        this.advance();
        this.advance();
        this.advance();
        return {
          type: TokenType.GreaterThanGreaterThanEqual,
          value: ">>=",
          loc: {
            start: { line: startLine, column: startColumn },
            end: { line: this.line, column: this.column },
          },
        };
      case "++":
        this.advance();
        this.advance();
        return {
          type: TokenType.PlusPlus,
          value: "++",
          loc: {
            start: { line: startLine, column: startColumn },
            end: { line: this.line, column: this.column },
          },
        };
      case "--":
        this.advance();
        this.advance();
        return {
          type: TokenType.MinusMinus,
          value: "--",
          loc: {
            start: { line: startLine, column: startColumn },
            end: { line: this.line, column: this.column },
          },
        };
    }

    // Single-character operators and punctuation
    this.advance();
    let type: TokenType;

    switch (char) {
      case "+": type = TokenType.Plus; break;
      case "-": type = TokenType.Minus; break;
      case "*": type = TokenType.Asterisk; break;
      case "/": type = TokenType.Slash; break;
      case "%": type = TokenType.Percent; break;
      case "^": type = TokenType.Caret; break;
      case "&": type = TokenType.Ampersand; break;
      case "|": type = TokenType.Pipe; break;
      case "~": type = TokenType.Tilde; break;
      case "!": type = TokenType.Exclamation; break;
      case "=": type = TokenType.Equal; break;
      case "<": type = TokenType.LessThan; break;
      case ">": type = TokenType.GreaterThan; break;
      case "?": type = TokenType.Question; break;
      case ":": type = TokenType.Colon; break;
      case ";": type = TokenType.Semicolon; break;
      case ",": type = TokenType.Comma; break;
      case ".": type = TokenType.Dot; break;
      case "@": type = TokenType.At; break;
      case "(": type = TokenType.LeftParen; break;
      case ")": type = TokenType.RightParen; break;
      case "{": type = TokenType.LeftBrace; break;
      case "}": type = TokenType.RightBrace; break;
      case "[": type = TokenType.LeftBracket; break;
      case "]": type = TokenType.RightBracket; break;
      default: return null;
    }

    return {
      type,
      value: char,
      loc: {
        start: { line: startLine, column: startColumn },
        end: { line: this.line, column: this.column },
      },
    };
  }
}