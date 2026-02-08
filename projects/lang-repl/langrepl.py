#!/usr/bin/env python3
"""
LangRePL - A simple programming language with REPL
"""

import sys
import os
from lexer import Lexer
from parser import Parser
from interpreter import Interpreter


def run_file(filename: str):
    """Run a LangRePL source file."""
    try:
        with open(filename, 'r') as f:
            source = f.read()
        
        run_source(source)
    
    except FileNotFoundError:
        print(f"Error: File not found: {filename}")
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)


def run_source(source: str):
    """Run LangRePL source code."""
    try:
        # Tokenize
        lexer = Lexer(source)
        tokens = lexer.tokenize()
        
        # Parse
        parser = Parser(tokens)
        statements = parser.parse()
        
        # Interpret
        interpreter = Interpreter()
        interpreter.interpret(statements)
    
    except SyntaxError as e:
        print(f"Syntax Error: {e}")
        sys.exit(1)
    except Exception as e:
        print(f"Runtime Error: {e}")
        sys.exit(1)


def main():
    """Main entry point."""
    if len(sys.argv) > 1:
        # Run a file
        filename = sys.argv[1]
        run_file(filename)
    else:
        # Start REPL
        from repl import REPL
        repl = REPL()
        repl.run()


if __name__ == "__main__":
    main()