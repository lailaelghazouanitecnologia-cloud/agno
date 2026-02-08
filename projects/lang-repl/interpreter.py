"""
Interpreter for the LangRePL programming language.
Evaluates the AST and executes the program.
"""

import time
from typing import Any, Dict, List, Optional
from lexer import Token, TokenType
from parser import (
    Expr, Stmt, Binary, Unary, Literal, Grouping, Variable, Assignment,
    Call, Array, Index, ExpressionStmt, PrintStmt, VarStmt, BlockStmt,
    IfStmt, WhileStmt, ForStmt, FunctionStmt, ReturnStmt
)


class RuntimeError(Exception):
    def __init__(self, message: str, token: Optional[Token] = None):
        self.message = message
        self.token = token
        super().__init__(message)


class ReturnException(Exception):
    """Exception used to handle return statements."""
    def __init__(self, value: Any):
        self.value = value


class LangFunction:
    """Represents a user-defined function."""
    
    def __init__(self, declaration: FunctionStmt, closure: 'Environment'):
        self.declaration = declaration
        self.closure = closure
    
    def call(self, interpreter: 'Interpreter', arguments: List[Any]) -> Any:
        environment = Environment(self.closure)
        
        # Bind parameters to arguments
        for i, param in enumerate(self.declaration.params):
            environment.define(param.lexeme, arguments[i] if i < len(arguments) else None)
        
        # Execute function body
        try:
            interpreter.execute_block(self.declaration.body, environment)
        except ReturnException as ret:
            return ret.value
        
        return None
    
    def arity(self) -> int:
        return len(self.declaration.params)
    
    def __repr__(self):
        return f"<fn {self.declaration.name.lexeme}>"


class Environment:
    """Lexical scope environment for variable storage."""
    
    def __init__(self, enclosing: Optional['Environment'] = None):
        self.enclosing = enclosing
        self.values: Dict[str, Any] = {}
    
    def define(self, name: str, value: Any):
        """Define a new variable in the current scope."""
        self.values[name] = value
    
    def get(self, name: Token) -> Any:
        """Get the value of a variable."""
        if name.lexeme in self.values:
            return self.values[name.lexeme]
        
        if self.enclosing:
            return self.enclosing.get(name)
        
        raise RuntimeError(f"Undefined variable '{name.lexeme}'", name)
    
    def assign(self, name: Token, value: Any):
        """Assign a value to an existing variable."""
        if name.lexeme in self.values:
            self.values[name.lexeme] = value
            return
        
        if self.enclosing:
            self.enclosing.assign(name, value)
            return
        
        raise RuntimeError(f"Undefined variable '{name.lexeme}'", name)


class Interpreter:
    """Interpreter that evaluates the AST."""
    
    def __init__(self):
        self.globals = Environment()
        self.environment = self.globals
        self.locals: Dict[Expr, int] = {}
        
        # Define built-in functions
        self._define_builtins()
    
    def _define_builtins(self):
        """Define built-in functions and variables."""
        # Clock function
        def clock_fn(*args):
            return time.time()
        
        self.globals.define("clock", clock_fn)
        
        # Length function
        def length_fn(obj):
            if isinstance(obj, (str, list)):
                return len(obj)
            raise RuntimeError("Object has no length")
        
        self.globals.define("len", length_fn)
        
        # Type function
        def type_fn(obj):
            if obj is None:
                return "nil"
            elif isinstance(obj, bool):
                return "boolean"
            elif isinstance(obj, (int, float)):
                return "number"
            elif isinstance(obj, str):
                return "string"
            elif isinstance(obj, list):
                return "array"
            elif isinstance(obj, LangFunction):
                return "function"
            return "unknown"
        
        self.globals.define("type", type_fn)
        
        # String conversion
        def str_fn(obj):
            return self._stringify(obj)
        
        self.globals.define("str", str_fn)
        
        # Number conversion
        def number_fn(obj):
            if isinstance(obj, (int, float)):
                return obj
            if isinstance(obj, str):
                try:
                    return float(obj) if '.' in obj else int(obj)
                except ValueError:
                    return None
            return None
        
        self.globals.define("number", number_fn)
    
    def interpret(self, statements: List[Stmt]):
        """Interpret a list of statements."""
        try:
            for statement in statements:
                self.execute(statement)
        except RuntimeError as e:
            print(f"Runtime Error: {e.message}")
            if e.token:
                print(f"  at line {e.token.line}")
    
    def execute(self, stmt: Stmt):
        """Execute a statement."""
        if isinstance(stmt, ExpressionStmt):
            self._execute_expression_stmt(stmt)
        elif isinstance(stmt, PrintStmt):
            self._execute_print_stmt(stmt)
        elif isinstance(stmt, VarStmt):
            self._execute_var_stmt(stmt)
        elif isinstance(stmt, BlockStmt):
            self._execute_block_stmt(stmt)
        elif isinstance(stmt, IfStmt):
            self._execute_if_stmt(stmt)
        elif isinstance(stmt, WhileStmt):
            self._execute_while_stmt(stmt)
        elif isinstance(stmt, ForStmt):
            self._execute_for_stmt(stmt)
        elif isinstance(stmt, FunctionStmt):
            self._execute_function_stmt(stmt)
        elif isinstance(stmt, ReturnStmt):
            self._execute_return_stmt(stmt)
        else:
            raise RuntimeError(f"Unknown statement type: {type(stmt)}")
    
    def execute_block(self, statements: List[Stmt], environment: Environment):
        """Execute a block of statements in a new environment."""
        previous = self.environment
        try:
            self.environment = environment
            for statement in statements:
                self.execute(statement)
        finally:
            self.environment = previous
    
    def evaluate(self, expr: Expr) -> Any:
        """Evaluate an expression."""
        if isinstance(expr, Binary):
            return self._evaluate_binary(expr)
        elif isinstance(expr, Unary):
            return self._evaluate_unary(expr)
        elif isinstance(expr, Literal):
            return self._evaluate_literal(expr)
        elif isinstance(expr, Grouping):
            return self._evaluate_grouping(expr)
        elif isinstance(expr, Variable):
            return self._evaluate_variable(expr)
        elif isinstance(expr, Assignment):
            return self._evaluate_assignment(expr)
        elif isinstance(expr, Call):
            return self._evaluate_call(expr)
        elif isinstance(expr, Array):
            return self._evaluate_array(expr)
        elif isinstance(expr, Index):
            return self._evaluate_index(expr)
        else:
            raise RuntimeError(f"Unknown expression type: {type(expr)}")
    
    # Statement execution methods
    def _execute_expression_stmt(self, stmt: ExpressionStmt):
        self.evaluate(stmt.expression)
    
    def _execute_print_stmt(self, stmt: PrintStmt):
        value = self.evaluate(stmt.expression)
        print(self._stringify(value))
    
    def _execute_var_stmt(self, stmt: VarStmt):
        value = None
        if stmt.initializer:
            value = self.evaluate(stmt.initializer)
        
        self.environment.define(stmt.name.lexeme, value)
    
    def _execute_block_stmt(self, stmt: BlockStmt):
        self.execute_block(stmt.statements, Environment(self.environment))
    
    def _execute_if_stmt(self, stmt: IfStmt):
        if self._is_truthy(self.evaluate(stmt.condition)):
            self.execute(stmt.then_branch)
        elif stmt.else_branch:
            self.execute(stmt.else_branch)
    
    def _execute_while_stmt(self, stmt: WhileStmt):
        while self._is_truthy(self.evaluate(stmt.condition)):
            self.execute(stmt.body)
    
    def _execute_for_stmt(self, stmt: ForStmt):
        # For statements are desugared to while statements in the parser
        self.execute(stmt.body)
    
    def _execute_function_stmt(self, stmt: FunctionStmt):
        function = LangFunction(stmt, self.environment)
        self.environment.define(stmt.name.lexeme, function)
    
    def _execute_return_stmt(self, stmt: ReturnStmt):
        value = None
        if stmt.value:
            value = self.evaluate(stmt.value)
        
        raise ReturnException(value)
    
    # Expression evaluation methods
    def _evaluate_binary(self, expr: Binary) -> Any:
        left = self.evaluate(expr.left)
        right = self.evaluate(expr.right)
        
        token_type = expr.operator.type
        
        # Arithmetic operations
        if token_type == TokenType.PLUS:
            if isinstance(left, (int, float)) and isinstance(right, (int, float)):
                return left + right
            if isinstance(left, str) and isinstance(right, str):
                return left + right
            if isinstance(left, list) and isinstance(right, list):
                return left + right
            raise RuntimeError("Operands must be numbers or strings", expr.operator)
        
        elif token_type == TokenType.MINUS:
            self._check_number_operands(expr.operator, left, right)
            return left - right
        
        elif token_type == TokenType.STAR:
            if isinstance(left, (int, float)) and isinstance(right, (int, float)):
                return left * right
            if isinstance(left, str) and isinstance(right, (int, float)):
                return left * int(right)
            if isinstance(left, (int, float)) and isinstance(right, str):
                return int(left) * right
            raise RuntimeError("Operands must be numbers", expr.operator)
        
        elif token_type == TokenType.SLASH:
            self._check_number_operands(expr.operator, left, right)
            if right == 0:
                raise RuntimeError("Division by zero", expr.operator)
            return left / right
        
        elif token_type == TokenType.PERCENT:
            self._check_number_operands(expr.operator, left, right)
            return left % right
        
        # Comparison operations
        elif token_type == TokenType.GREATER:
            self._check_number_operands(expr.operator, left, right)
            return left > right
        
        elif token_type == TokenType.GREATER_EQUAL:
            self._check_number_operands(expr.operator, left, right)
            return left >= right
        
        elif token_type == TokenType.LESS:
            self._check_number_operands(expr.operator, left, right)
            return left < right
        
        elif token_type == TokenType.LESS_EQUAL:
            self._check_number_operands(expr.operator, left, right)
            return left <= right
        
        # Equality operations
        elif token_type == TokenType.EQUAL_EQUAL:
            return left == right
        
        elif token_type == TokenType.BANG_EQUAL:
            return left != right
        
        # Logical operations
        elif token_type == TokenType.AND:
            return self._is_truthy(left) and self._is_truthy(right)
        
        elif token_type == TokenType.OR:
            return self._is_truthy(left) or self._is_truthy(right)
        
        raise RuntimeError(f"Unknown binary operator: {token_type}", expr.operator)
    
    def _evaluate_unary(self, expr: Unary) -> Any:
        right = self.evaluate(expr.right)
        
        token_type = expr.operator.type
        
        if token_type == TokenType.MINUS:
            self._check_number_operand(expr.operator, right)
            return -right
        
        elif token_type == TokenType.BANG:
            return not self._is_truthy(right)
        
        raise RuntimeError(f"Unknown unary operator: {token_type}", expr.operator)
    
    def _evaluate_literal(self, expr: Literal) -> Any:
        return expr.value
    
    def _evaluate_grouping(self, expr: Grouping) -> Any:
        return self.evaluate(expr.expression)
    
    def _evaluate_variable(self, expr: Variable) -> Any:
        return self.environment.get(expr.name)
    
    def _evaluate_assignment(self, expr: Assignment) -> Any:
        value = self.evaluate(expr.value)
        self.environment.assign(expr.name, value)
        return value
    
    def _evaluate_call(self, expr: Call) -> Any:
        callee = self.evaluate(expr.callee)
        
        arguments = []
        for arg in expr.arguments:
            arguments.append(self.evaluate(arg))
        
        if not isinstance(callee, LangFunction) and not callable(callee):
            raise RuntimeError("Can only call functions", expr.paren)
        
        if isinstance(callee, LangFunction):
            if len(arguments) != callee.arity():
                raise RuntimeError(
                    f"Expected {callee.arity()} arguments but got {len(arguments)}",
                    expr.paren
                )
            return callee.call(self, arguments)
        else:
            # Built-in function
            return callee(*arguments)
    
    def _evaluate_array(self, expr: Array) -> Any:
        elements = []
        for element in expr.elements:
            elements.append(self.evaluate(element))
        return elements
    
    def _evaluate_index(self, expr: Index) -> Any:
        array = self.evaluate(expr.array)
        index = self.evaluate(expr.index)
        
        if not isinstance(array, list):
            raise RuntimeError("Can only index arrays", expr.array)
        
        if not isinstance(index, int):
            raise RuntimeError("Array index must be an integer")
        
        if index < 0 or index >= len(array):
            raise RuntimeError("Array index out of bounds")
        
        return array[index]
    
    # Helper methods
    def _is_truthy(self, value: Any) -> bool:
        """Determine if a value is truthy."""
        if value is None:
            return False
        if isinstance(value, bool):
            return value
        if isinstance(value, (int, float)):
            return value != 0
        if isinstance(value, (str, list)):
            return len(value) > 0
        return True
    
    def _check_number_operands(self, operator: Token, *operands: Any):
        """Check if all operands are numbers."""
        for operand in operands:
            self._check_number_operand(operator, operand)
    
    def _check_number_operand(self, operator: Token, operand: Any):
        """Check if an operand is a number."""
        if not isinstance(operand, (int, float)):
            raise RuntimeError("Operand must be a number", operator)
    
    def _stringify(self, value: Any) -> str:
        """Convert a value to its string representation."""
        if value is None:
            return "nil"
        if isinstance(value, bool):
            return "true" if value else "false"
        if isinstance(value, list):
            elements = [self._stringify(e) for e in value]
            return "[" + ", ".join(elements) + "]"
        return str(value)