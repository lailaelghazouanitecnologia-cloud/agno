"""
REPL (Read-Eval-Print Loop) for the LangRePL programming language.
Provides an interactive shell for running LangRePL code.
"""

import sys
import readline
from typing import Optional
from lexer import Lexer, TokenType
from parser import Parser
from interpreter import Interpreter, RuntimeError


class REPL:
    """Interactive REPL for LangRePL."""
    
    def __init__(self):
        self.interpreter = Interpreter()
        self.running = True
        self.multiline = False
        self.multiline_buffer = []
        self.prompt = ">>> "
        self.multiline_prompt = "... "
    
    def run(self):
        """Start the REPL loop."""
        self._print_banner()
        
        while self.running:
            try:
                line = self._read_line()
                
                if line is None:
                    break  # EOF
                
                line = line.strip()
                
                if not line:
                    continue
                
                # Handle special commands
                if line.startswith(":"):
                    self._handle_command(line)
                    continue
                
                # Handle multiline input
                if self.multiline:
                    self.multiline_buffer.append(line)
                    
                    # Check if block is complete
                    if self._is_complete_block("\n".join(self.multiline_buffer)):
                        self.multiline = False
                        source = "\n".join(self.multiline_buffer)
                        self.multiline_buffer = []
                        self._execute(source)
                    continue
                
                # Check if this line starts a block
                if self._starts_block(line):
                    self.multiline = True
                    self.multiline_buffer = [line]
                    continue
                
                # Execute single line
                self._execute(line)
                
            except KeyboardInterrupt:
                print("\nKeyboardInterrupt")
                self.multiline = False
                self.multiline_buffer = []
                self.prompt = ">>> "
            except EOFError:
                print("\nGoodbye!")
                break
            except Exception as e:
                print(f"Error: {e}")
    
    def _print_banner(self):
        """Print the REPL banner."""
        print("=" * 50)
        print("LangRePL v1.0 - A Simple Programming Language")
        print("=" * 50)
        print("Type :help for available commands")
        print("Type :quit or Ctrl-D to exit")
        print()
    
    def _read_line(self) -> Optional[str]:
        """Read a line of input from the user."""
        try:
            prompt = self.multiline_prompt if self.multiline else self.prompt
            return input(prompt)
        except EOFError:
            return None
    
    def _handle_command(self, command: str):
        """Handle special REPL commands."""
        cmd = command.lower()
        
        if cmd == ":quit" or cmd == ":exit" or cmd == ":q":
            self.running = False
            print("Goodbye!")
        
        elif cmd == ":help" or cmd == ":h":
            self._print_help()
        
        elif cmd == ":clear" or cmd == ":c":
            # Clear screen
            print("\033[2J\033[H", end="")
        
        elif cmd == ":vars" or cmd == ":v":
            self._print_variables()
        
        elif cmd == ":reset" or cmd == ":r":
            self.interpreter = Interpreter()
            print("Environment reset")
        
        elif cmd.startswith(":load "):
            filename = command[6:].strip()
            self._load_file(filename)
        
        else:
            print(f"Unknown command: {command}")
            print("Type :help for available commands")
    
    def _print_help(self):
        """Print help information."""
        print()
        print("Available commands:")
        print("  :quit, :exit, :q    - Exit the REPL")
        print("  :help, :h           - Show this help message")
        print("  :clear, :c          - Clear the screen")
        print("  :vars, :v           - List all defined variables")
        print("  :reset, :r          - Reset the environment")
        print("  :load <file>        - Load and execute a file")
        print()
        print("Language features:")
        print("  - Variables: let x = 10;")
        print("  - Functions: fn add(a, b) { return a + b; }")
        print("  - Conditionals: if (x > 5) { print(x); }")
        print("  - Loops: while (x > 0) { print(x); x = x - 1; }")
        print("  - Arrays: let arr = [1, 2, 3];")
        print("  - Built-ins: print(), clock(), len(), type(), str(), number()")
        print()
    
    def _print_variables(self):
        """Print all defined variables in the current environment."""
        print()
        print("Defined variables:")
        env = self.interpreter.environment
        
        while env:
            for name, value in env.values.items():
                if not callable(value):
                    print(f"  {name} = {self.interpreter._stringify(value)}")
            
            env = env.enclosing
        
        print()
    
    def _load_file(self, filename: str):
        """Load and execute a file."""
        try:
            with open(filename, 'r') as f:
                source = f.read()
            
            print(f"Loading {filename}...")
            self._execute(source)
            print(f"Loaded {filename}")
        
        except FileNotFoundError:
            print(f"Error: File not found: {filename}")
        except Exception as e:
            print(f"Error loading file: {e}")
    
    def _starts_block(self, line: str) -> bool:
        """Check if a line starts a code block."""
        # Simple heuristic: check for unclosed braces/parentheses
        open_braces = line.count('{')
        close_braces = line.count('}')
        open_parens = line.count('(')
        close_parens = line.count(')')
        
        return (open_braces > close_braces or 
                open_parens > close_parens or
                line.endswith('{') or
                line.endswith('('))
    
    def _is_complete_block(self, source: str) -> bool:
        """Check if a block of code is complete."""
        open_braces = source.count('{')
        close_braces = source.count('}')
        open_parens = source.count('(')
        close_parens = source.count(')')
        
        return (open_braces == close_braces and 
                open_parens == close_parens)
    
    def _execute(self, source: str):
        """Execute a line or block of source code."""
        try:
            # Tokenize
            lexer = Lexer(source)
            tokens = lexer.tokenize()
            
            # Parse
            parser = Parser(tokens)
            statements = parser.parse()
            
            # Interpret
            self.interpreter.interpret(statements)
        
        except SyntaxError as e:
            print(f"Syntax Error: {e}")
        except RuntimeError as e:
            print(f"Runtime Error: {e.message}")
            if e.token:
                print(f"  at line {e.token.line}")
        except Exception as e:
            print(f"Error: {e}")


def main():
    """Main entry point for the REPL."""
    repl = REPL()
    repl.run()


if __name__ == "__main__":
    main()