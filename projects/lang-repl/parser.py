"""
Parser for the LangRePL programming language.
Builds an Abstract Syntax Tree (AST) from tokens.
"""

from typing import List, Optional, Union
from lexer import Token, TokenType


# AST Node Types
class Expr:
    """Base class for expression nodes."""
    pass


class Binary(Expr):
    def __init__(self, left: Expr, operator: Token, right: Expr):
        self.left = left
        self.operator = operator
        self.right = right
    
    def __repr__(self):
        return f"Binary({self.left}, {self.operator.lexeme}, {self.right})"


class Unary(Expr):
    def __init__(self, operator: Token, right: Expr):
        self.operator = operator
        self.right = right
    
    def __repr__(self):
        return f"Unary({self.operator.lexeme}, {self.right})"


class Literal(Expr):
    def __init__(self, value: object):
        self.value = value
    
    def __repr__(self):
        return f"Literal({self.value})"


class Grouping(Expr):
    def __init__(self, expression: Expr):
        self.expression = expression
    
    def __repr__(self):
        return f"Grouping({self.expression})"


class Variable(Expr):
    def __init__(self, name: Token):
        self.name = name
    
    def __repr__(self):
        return f"Variable({self.name.lexeme})"


class Assignment(Expr):
    def __init__(self, name: Token, value: Expr):
        self.name = name
        self.value = value
    
    def __repr__(self):
        return f"Assignment({self.name.lexeme}, {self.value})"


class Call(Expr):
    def __init__(self, callee: Expr, paren: Token, arguments: List[Expr]):
        self.callee = callee
        self.paren = paren
        self.arguments = arguments
    
    def __repr__(self):
        return f"Call({self.callee}, {self.arguments})"


class Get(Expr):
    def __init__(self, object: Expr, name: Token):
        self.object = object
        self.name = name
    
    def __repr__(self):
        return f"Get({self.object}, {self.name.lexeme})"


class Array(Expr):
    def __init__(self, elements: List[Expr]):
        self.elements = elements
    
    def __repr__(self):
        return f"Array({self.elements})"


class Index(Expr):
    def __init__(self, array: Expr, index: Expr):
        self.array = array
        self.index = index
    
    def __repr__(self):
        return f"Index({self.array}, {self.index})"


# Statement types
class Stmt:
    """Base class for statement nodes."""
    pass


class ExpressionStmt(Stmt):
    def __init__(self, expression: Expr):
        self.expression = expression
    
    def __repr__(self):
        return f"ExpressionStmt({self.expression})"


class PrintStmt(Stmt):
    def __init__(self, expression: Expr):
        self.expression = expression
    
    def __repr__(self):
        return f"PrintStmt({self.expression})"


class VarStmt(Stmt):
    def __init__(self, name: Token, initializer: Optional[Expr]):
        self.name = name
        self.initializer = initializer
    
    def __repr__(self):
        return f"VarStmt({self.name.lexeme}, {self.initializer})"


class BlockStmt(Stmt):
    def __init__(self, statements: List[Stmt]):
        self.statements = statements
    
    def __repr__(self):
        return f"BlockStmt({self.statements})"


class IfStmt(Stmt):
    def __init__(self, condition: Expr, then_branch: Stmt, else_branch: Optional[Stmt]):
        self.condition = condition
        self.then_branch = then_branch
        self.else_branch = else_branch
    
    def __repr__(self):
        return f"IfStmt({self.condition}, {self.then_branch}, {self.else_branch})"


class WhileStmt(Stmt):
    def __init__(self, condition: Expr, body: Stmt):
        self.condition = condition
        self.body = body
    
    def __repr__(self):
        return f"WhileStmt({self.condition}, {self.body})"


class ForStmt(Stmt):
    def __init__(self, initializer: Optional[Stmt], condition: Optional[Expr], 
                 increment: Optional[Expr], body: Stmt):
        self.initializer = initializer
        self.condition = condition
        self.increment = increment
        self.body = body
    
    def __repr__(self):
        return f"ForStmt({self.initializer}, {self.condition}, {self.increment}, {self.body})"


class FunctionStmt(Stmt):
    def __init__(self, name: Token, params: List[Token], body: List[Stmt]):
        self.name = name
        self.params = params
        self.body = body
    
    def __repr__(self):
        return f"FunctionStmt({self.name.lexeme}, {[p.lexeme for p in self.params]}, {self.body})"


class ReturnStmt(Stmt):
    def __init__(self, keyword: Token, value: Optional[Expr]):
        self.keyword = keyword
        self.value = value
    
    def __repr__(self):
        return f"ReturnStmt({self.value})"


class Parser:
    """Recursive descent parser for LangRePL."""
    
    def __init__(self, tokens: List[Token]):
        self.tokens = tokens
        self.current = 0
    
    def parse(self) -> List[Stmt]:
        """Parse the entire token list into statements."""
        statements = []
        
        while not self._at_end():
            if self._check(TokenType.NEWLINE):
                self._advance()
                continue
            
            stmt = self._declaration()
            if stmt:
                statements.append(stmt)
        
        return statements
    
    def _at_end(self) -> bool:
        return self._peek().type == TokenType.EOF
    
    def _peek(self) -> Token:
        return self.tokens[self.current]
    
    def _previous(self) -> Token:
        return self.tokens[self.current - 1]
    
    def _advance(self) -> Token:
        if not self._at_end():
            self.current += 1
        return self._previous()
    
    def _check(self, token_type: TokenType) -> bool:
        if self._at_end():
            return False
        return self._peek().type == token_type
    
    def _match(self, *token_types: TokenType) -> bool:
        for token_type in token_types:
            if self._check(token_type):
                self._advance()
                return True
        return False
    
    def _consume(self, token_type: TokenType, message: str) -> Token:
        if self._check(token_type):
            return self._advance()
        
        raise SyntaxError(f"{message} at line {self._peek().line}")
    
    def _synchronize(self):
        """Synchronize parser after an error."""
        self._advance()
        
        while not self._at_end():
            if self._previous().type == TokenType.SEMICOLON:
                return
            
            if self._peek().type in (TokenType.CLASS, TokenType.FN, TokenType.LET, 
                                     TokenType.IF, TokenType.WHILE, TokenType.FOR,
                                     TokenType.PRINT, TokenType.RETURN):
                return
            
            self._advance()
    
    # Declaration parsing
    def _declaration(self) -> Optional[Stmt]:
        try:
            if self._match(TokenType.FN):
                return self._function("function")
            if self._match(TokenType.LET):
                return self._var_declaration()
            
            return self._statement()
        except SyntaxError as e:
            self._synchronize()
            raise e
    
    def _function(self, kind: str) -> FunctionStmt:
        name = self._consume(TokenType.IDENTIFIER, f"Expect {kind} name")
        
        self._consume(TokenType.LPAREN, f"Expect '(' after {kind} name")
        params = []
        
        if not self._check(TokenType.RPAREN):
            params.append(self._consume(TokenType.IDENTIFIER, "Expect parameter name"))
            while self._match(TokenType.COMMA):
                params.append(self._consume(TokenType.IDENTIFIER, "Expect parameter name"))
        
        self._consume(TokenType.RPAREN, "Expect ')' after parameters")
        self._consume(TokenType.LBRACE, f"Expect '{{' before {kind} body")
        
        body = self._block()
        return FunctionStmt(name, params, body)
    
    def _var_declaration(self) -> VarStmt:
        name = self._consume(TokenType.IDENTIFIER, "Expect variable name")
        
        initializer = None
        if self._match(TokenType.ASSIGN):
            initializer = self._expression()
        
        self._consume(TokenType.SEMICOLON, "Expect ';' after variable declaration")
        return VarStmt(name, initializer)
    
    # Statement parsing
    def _statement(self) -> Stmt:
        if self._match(TokenType.IF):
            return self._if_statement()
        if self._match(TokenType.WHILE):
            return self._while_statement()
        if self._match(TokenType.FOR):
            return self._for_statement()
        if self._match(TokenType.PRINT):
            return self._print_statement()
        if self._match(TokenType.RETURN):
            return self._return_statement()
        if self._match(TokenType.LBRACE):
            return BlockStmt(self._block())
        
        return self._expression_statement()
    
    def _if_statement(self) -> IfStmt:
        self._consume(TokenType.LPAREN, "Expect '(' after 'if'")
        condition = self._expression()
        self._consume(TokenType.RPAREN, "Expect ')' after if condition")
        
        then_branch = self._statement()
        else_branch = None
        
        if self._match(TokenType.ELSE):
            else_branch = self._statement()
        
        return IfStmt(condition, then_branch, else_branch)
    
    def _while_statement(self) -> WhileStmt:
        self._consume(TokenType.LPAREN, "Expect '(' after 'while'")
        condition = self._expression()
        self._consume(TokenType.RPAREN, "Expect ')' after while condition")
        
        body = self._statement()
        return WhileStmt(condition, body)
    
    def _for_statement(self) -> ForStmt:
        self._consume(TokenType.LPAREN, "Expect '(' after 'for'")
        
        initializer = None
        if self._match(TokenType.SEMICOLON):
            pass
        elif self._match(TokenType.LET):
            initializer = self._var_declaration()
        else:
            initializer = self._expression_statement()
        
        condition = None
        if not self._check(TokenType.SEMICOLON):
            condition = self._expression()
        self._consume(TokenType.SEMICOLON, "Expect ';' after loop condition")
        
        increment = None
        if not self._check(TokenType.RPAREN):
            increment = self._expression()
        self._consume(TokenType.RPAREN, "Expect ')' after for clauses")
        
        body = self._statement()
        
        # Desugar for loop to while loop
        # for (init; cond; incr) body -> { init; while (cond) { body; incr; } }
        if increment:
            body = BlockStmt([body, ExpressionStmt(increment)])
        
        if condition is None:
            condition = Literal(True)
        
        body = WhileStmt(condition, body)
        
        if initializer:
            body = BlockStmt([initializer, body])
        
        return body
    
    def _print_statement(self) -> PrintStmt:
        value = self._expression()
        self._consume(TokenType.SEMICOLON, "Expect ';' after value")
        return PrintStmt(value)
    
    def _return_statement(self) -> ReturnStmt:
        keyword = self._previous()
        value = None
        
        if not self._check(TokenType.SEMICOLON):
            value = self._expression()
        
        self._consume(TokenType.SEMICOLON, "Expect ';' after return value")
        return ReturnStmt(keyword, value)
    
    def _block(self) -> List[Stmt]:
        statements = []
        
        while not self._check(TokenType.RBRACE) and not self._at_end():
            if self._check(TokenType.NEWLINE):
                self._advance()
                continue
            
            stmt = self._declaration()
            if stmt:
                statements.append(stmt)
        
        self._consume(TokenType.RBRACE, "Expect '}' after block")
        return statements
    
    def _expression_statement(self) -> ExpressionStmt:
        expr = self._expression()
        self._consume(TokenType.SEMICOLON, "Expect ';' after expression")
        return ExpressionStmt(expr)
    
    # Expression parsing (precedence climbing)
    def _expression(self) -> Expr:
        return self._assignment()
    
    def _assignment(self) -> Expr:
        expr = self._or()
        
        if self._match(TokenType.ASSIGN, TokenType.PLUS_ASSIGN, TokenType.MINUS_ASSIGN):
            operator = self._previous()
            value = self._assignment()
            
            if isinstance(expr, Variable):
                if operator.type == TokenType.PLUS_ASSIGN:
                    value = Binary(expr, Token(TokenType.PLUS, "+", None, operator.line), value)
                elif operator.type == TokenType.MINUS_ASSIGN:
                    value = Binary(expr, Token(TokenType.MINUS, "-", None, operator.line), value)
                return Assignment(expr.name, value)
            
            raise SyntaxError(f"Invalid assignment target at line {operator.line}")
        
        return expr
    
    def _or(self) -> Expr:
        expr = self._and()
        
        while self._match(TokenType.OR):
            operator = self._previous()
            right = self._and()
            expr = Binary(expr, operator, right)
        
        return expr
    
    def _and(self) -> Expr:
        expr = self._equality()
        
        while self._match(TokenType.AND):
            operator = self._previous()
            right = self._equality()
            expr = Binary(expr, operator, right)
        
        return expr
    
    def _equality(self) -> Expr:
        expr = self._comparison()
        
        while self._match(TokenType.BANG_EQUAL, TokenType.EQUAL_EQUAL):
            operator = self._previous()
            right = self._comparison()
            expr = Binary(expr, operator, right)
        
        return expr
    
    def _comparison(self) -> Expr:
        expr = self._term()
        
        while self._match(TokenType.GREATER, TokenType.GREATER_EQUAL, 
                          TokenType.LESS, TokenType.LESS_EQUAL):
            operator = self._previous()
            right = self._term()
            expr = Binary(expr, operator, right)
        
        return expr
    
    def _term(self) -> Expr:
        expr = self._factor()
        
        while self._match(TokenType.MINUS, TokenType.PLUS):
            operator = self._previous()
            right = self._factor()
            expr = Binary(expr, operator, right)
        
        return expr
    
    def _factor(self) -> Expr:
        expr = self._unary()
        
        while self._match(TokenType.SLASH, TokenType.STAR, TokenType.PERCENT):
            operator = self._previous()
            right = self._unary()
            expr = Binary(expr, operator, right)
        
        return expr
    
    def _unary(self) -> Expr:
        if self._match(TokenType.BANG, TokenType.MINUS):
            operator = self._previous()
            right = self._unary()
            return Unary(operator, right)
        
        return self._call()
    
    def _call(self) -> Expr:
        expr = self._primary()
        
        while True:
            if self._match(TokenType.LPAREN):
                expr = self._finish_call(expr)
            elif self._match(TokenType.LBRACKET):
                index = self._expression()
                self._consume(TokenType.RBRACKET, "Expect ']' after index")
                expr = Index(expr, index)
            else:
                break
        
        return expr
    
    def _finish_call(self, callee: Expr) -> Expr:
        arguments = []
        
        if not self._check(TokenType.RPAREN):
            arguments.append(self._expression())
            while self._match(TokenType.COMMA):
                arguments.append(self._expression())
        
        self._consume(TokenType.RPAREN, "Expect ')' after arguments")
        return Call(callee, self._previous(), arguments)
    
    def _primary(self) -> Expr:
        if self._match(TokenType.TRUE):
            return Literal(True)
        if self._match(TokenType.FALSE):
            return Literal(False)
        if self._match(TokenType.NIL):
            return Literal(None)
        
        if self._match(TokenType.NUMBER):
            return Literal(self._previous().literal)
        
        if self._match(TokenType.STRING):
            return Literal(self._previous().literal)
        
        if self._match(TokenType.LBRACKET):
            return self._array()
        
        if self._match(TokenType.LPAREN):
            expr = self._expression()
            self._consume(TokenType.RPAREN, "Expect ')' after expression")
            return Grouping(expr)
        
        if self._match(TokenType.IDENTIFIER):
            return Variable(self._previous())
        
        raise SyntaxError(f"Expect expression at line {self._peek().line}")
    
    def _array(self) -> Expr:
        elements = []
        
        if not self._check(TokenType.RBRACKET):
            elements.append(self._expression())
            while self._match(TokenType.COMMA):
                elements.append(self._expression())
        
        self._consume(TokenType.RBRACKET, "Expect ']' after array elements")
        return Array(elements)