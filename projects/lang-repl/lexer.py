"""
Lexer for the LangRePL programming language.
Tokenizes source code into tokens for parsing.
"""

import re
from enum import Enum
from dataclasses import dataclass
from typing import List, Optional


class TokenType(Enum):
    # Literals
    NUMBER = "NUMBER"
    STRING = "STRING"
    BOOLEAN = "BOOLEAN"
    
    # Identifiers and Keywords
    IDENTIFIER = "IDENTIFIER"
    
    # Keywords
    LET = "LET"
    FN = "FN"
    IF = "IF"
    ELSE = "ELSE"
    WHILE = "WHILE"
    FOR = "FOR"
    RETURN = "RETURN"
    TRUE = "TRUE"
    FALSE = "FALSE"
    NIL = "NIL"
    PRINT = "PRINT"
    
    # Operators
    PLUS = "PLUS"
    MINUS = "MINUS"
    STAR = "STAR"
    SLASH = "SLASH"
    PERCENT = "PERCENT"
    
    # Comparison
    EQUAL = "EQUAL"
    EQUAL_EQUAL = "EQUAL_EQUAL"
    BANG = "BANG"
    BANG_EQUAL = "BANG_EQUAL"
    LESS = "LESS"
    LESS_EQUAL = "LESS_EQUAL"
    GREATER = "GREATER"
    GREATER_EQUAL = "GREATER_EQUAL"
    
    # Logical
    AND = "AND"
    OR = "OR"
    
    # Assignment
    ASSIGN = "ASSIGN"
    PLUS_ASSIGN = "PLUS_ASSIGN"
    MINUS_ASSIGN = "MINUS_ASSIGN"
    
    # Delimiters
    LPAREN = "LPAREN"
    RPAREN = "RPAREN"
    LBRACE = "LBRACE"
    RBRACE = "RBRACE"
    LBRACKET = "LBRACKET"
    RBRACKET = "RBRACKET"
    COMMA = "COMMA"
    SEMICOLON = "SEMICOLON"
    COLON = "COLON"
    DOT = "DOT"
    
    # Special
    EOF = "EOF"
    NEWLINE = "NEWLINE"


@dataclass
class Token:
    type: TokenType
    lexeme: str
    literal: Optional[object]
    line: int
    
    def __repr__(self):
        return f"Token({self.type}, '{self.lexeme}', {self.literal}, line {self.line})"


class Lexer:
    """Lexical analyzer for LangRePL."""
    
    KEYWORDS = {
        "let": TokenType.LET,
        "fn": TokenType.FN,
        "if": TokenType.IF,
        "else": TokenType.ELSE,
        "while": TokenType.WHILE,
        "for": TokenType.FOR,
        "return": TokenType.RETURN,
        "true": TokenType.TRUE,
        "false": TokenType.FALSE,
        "nil": TokenType.NIL,
        "print": TokenType.PRINT,
        "and": TokenType.AND,
        "or": TokenType.OR,
    }
    
    def __init__(self, source: str):
        self.source = source
        self.tokens: List[Token] = []
        self.start = 0
        self.current = 0
        self.line = 1
    
    def tokenize(self) -> List[Token]:
        """Tokenize the entire source code."""
        while not self._at_end():
            self.start = self.current
            self._scan_token()
        
        self.tokens.append(Token(TokenType.EOF, "", None, self.line))
        return self.tokens
    
    def _at_end(self) -> bool:
        return self.current >= len(self.source)
    
    def _scan_token(self):
        """Scan and identify a single token."""
        char = self._advance()
        
        # Single character tokens
        if char == '(':
            self._add_token(TokenType.LPAREN)
        elif char == ')':
            self._add_token(TokenType.RPAREN)
        elif char == '{':
            self._add_token(TokenType.LBRACE)
        elif char == '}':
            self._add_token(TokenType.RBRACE)
        elif char == '[':
            self._add_token(TokenType.LBRACKET)
        elif char == ']':
            self._add_token(TokenType.RBRACKET)
        elif char == ',':
            self._add_token(TokenType.COMMA)
        elif char == ';':
            self._add_token(TokenType.SEMICOLON)
        elif char == ':':
            self._add_token(TokenType.COLON)
        elif char == '.':
            self._add_token(TokenType.DOT)
        
        # Operators (potentially multi-character)
        elif char == '+':
            if self._match('='):
                self._add_token(TokenType.PLUS_ASSIGN)
            else:
                self._add_token(TokenType.PLUS)
        elif char == '-':
            if self._match('='):
                self._add_token(TokenType.MINUS_ASSIGN)
            else:
                self._add_token(TokenType.MINUS)
        elif char == '*':
            self._add_token(TokenType.STAR)
        elif char == '/':
            if self._match('/'):
                # Comment - consume until end of line
                while self._peek() != '\n' and not self._at_end():
                    self._advance()
            else:
                self._add_token(TokenType.SLASH)
        elif char == '%':
            self._add_token(TokenType.PERCENT)
        
        # Comparison operators
        elif char == '=':
            if self._match('='):
                self._add_token(TokenType.EQUAL_EQUAL)
            else:
                self._add_token(TokenType.ASSIGN)
        elif char == '!':
            if self._match('='):
                self._add_token(TokenType.BANG_EQUAL)
            else:
                self._add_token(TokenType.BANG)
        elif char == '<':
            if self._match('='):
                self._add_token(TokenType.LESS_EQUAL)
            else:
                self._add_token(TokenType.LESS)
        elif char == '>':
            if self._match('='):
                self._add_token(TokenType.GREATER_EQUAL)
            else:
                self._add_token(TokenType.GREATER)
        
        # Whitespace
        elif char == ' ' or char == '\r' or char == '\t':
            pass  # Ignore whitespace
        elif char == '\n':
            self.line += 1
            self._add_token(TokenType.NEWLINE)
        
        # String literals
        elif char == '"':
            self._string()
        
        # Number literals or identifiers
        elif self._is_digit(char):
            self._number()
        elif self._is_alpha(char):
            self._identifier()
        else:
            raise SyntaxError(f"Unexpected character '{char}' at line {self.line}")
    
    def _advance(self) -> str:
        """Consume and return current character."""
        char = self.source[self.current]
        self.current += 1
        return char
    
    def _peek(self) -> str:
        """Look at current character without consuming."""
        if self._at_end():
            return '\0'
        return self.source[self.current]
    
    def _peek_next(self) -> str:
        """Look at next character without consuming."""
        if self.current + 1 >= len(self.source):
            return '\0'
        return self.source[self.current + 1]
    
    def _match(self, expected: str) -> bool:
        """Check if current character matches expected."""
        if self._at_end():
            return False
        if self.source[self.current] != expected:
            return False
        self.current += 1
        return True
    
    def _string(self):
        """Parse a string literal."""
        while self._peek() != '"' and not self._at_end():
            if self._peek() == '\n':
                self.line += 1
            self._advance()
        
        if self._at_end():
            raise SyntaxError(f"Unterminated string at line {self.line}")
        
        self._advance()  # Consume closing quote
        
        # Extract string value (without quotes)
        value = self.source[self.start + 1:self.current - 1]
        self._add_token(TokenType.STRING, value)
    
    def _number(self):
        """Parse a number literal."""
        while self._is_digit(self._peek()):
            self._advance()
        
        # Check for decimal point
        if self._peek() == '.' and self._is_digit(self._peek_next()):
            self._advance()  # Consume decimal point
            while self._is_digit(self._peek()):
                self._advance()
        
        value = self.source[self.start:self.current]
        if '.' in value:
            self._add_token(TokenType.NUMBER, float(value))
        else:
            self._add_token(TokenType.NUMBER, int(value))
    
    def _identifier(self):
        """Parse an identifier or keyword."""
        while self._is_alphanumeric(self._peek()):
            self._advance()
        
        text = self.source[self.start:self.current]
        token_type = self.KEYWORDS.get(text, TokenType.IDENTIFIER)
        
        # Set literal value for boolean and nil
        literal = None
        if token_type == TokenType.TRUE:
            literal = True
        elif token_type == TokenType.FALSE:
            literal = False
        elif token_type == TokenType.NIL:
            literal = None
        
        self._add_token(token_type, literal)
    
    def _is_digit(self, char: str) -> bool:
        return char >= '0' and char <= '9'
    
    def _is_alpha(self, char: str) -> bool:
        return (char >= 'a' and char <= 'z') or (char >= 'A' and char <= 'Z') or char == '_'
    
    def _is_alphanumeric(self, char: str) -> bool:
        return self._is_alpha(char) or self._is_digit(char)
    
    def _add_token(self, token_type: TokenType, literal: Optional[object] = None):
        lexeme = self.source[self.start:self.current]
        self.tokens.append(Token(token_type, lexeme, literal, self.line))