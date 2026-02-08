# LangRePL

A simple programming language with REPL (Read-Eval-Print Loop) built from scratch in Python.

## Features

- **Variables**: `let x = 10;`
- **Functions**: First-class functions with closures
- **Control Flow**: `if`, `else`, `while`, `for` loops
- **Data Types**: Numbers, strings, booleans, nil, arrays
- **Operators**: Arithmetic, comparison, logical operators
- **Built-in Functions**: `print()`, `clock()`, `len()`, `type()`, `str()`, `number()`
- **REPL**: Interactive shell with multiline support
- **File Execution**: Run `.lrepl` source files

## Installation

No installation required! Just Python 3.6+.

```bash
git clone <repository>
cd lang-repl
```

## Usage

### Start the REPL

```bash
python langrepl.py
```

### Run a source file

```bash
python langrepl.py program.lrepl
```

### REPL Commands

- `:quit`, `:exit`, `:q` - Exit the REPL
- `:help`, `:h` - Show help
- `:clear`, `:c` - Clear the screen
- `:vars`, `:v` - List all variables
- `:reset`, `:r` - Reset the environment
- `:load <file>` - Load and execute a file

## Language Reference

### Variables

```lrepl
let x = 10;
let name = "Alice";
let flag = true;
let nothing = nil;
```

### Functions

```lrepl
fn add(a, b) {
    return a + b;
}

let result = add(5, 3);
print(result);  // Output: 8
```

### Conditionals

```lrepl
if (x > 5) {
    print("x is greater than 5");
} else {
    print("x is less than or equal to 5");
}
```

### Loops

```lrepl
// While loop
let i = 0;
while (i < 5) {
    print(i);
    i = i + 1;
}

// For loop
for (let j = 0; j < 3; j = j + 1) {
    print(j);
}
```

### Arrays

```lrepl
let arr = [1, 2, 3, 4, 5];
print(arr[0]);  // Output: 1
print(len(arr)); // Output: 5
```

### Operators

**Arithmetic**: `+`, `-`, `*`, `/`, `%`

**Comparison**: `==`, `!=`, `<`, `>`, `<=`, `>=`

**Logical**: `and`, `or`, `!`

**Assignment**: `=`, `+=`, `-=`

### Built-in Functions

- `print(value)` - Print a value
- `clock()` - Return current time in seconds
- `len(object)` - Return length of string or array
- `type(value)` - Return type name as string
- `str(value)` - Convert value to string
- `number(value)` - Convert value to number

## Examples

### Fibonacci Sequence

```lrepl
fn fib(n) {
    if (n <= 1) {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}

for (let i = 0; i < 10; i = i + 1) {
    print(fib(i));
}
```

### Factorial

```lrepl
fn factorial(n) {
    if (n <= 1) {
        return 1;
    }
    return n * factorial(n - 1);
}

print(factorial(5));  // Output: 120
```

### Array Operations

```lrepl
let numbers = [1, 2, 3, 4, 5];
let sum = 0;

for (let i = 0; i < len(numbers); i = i + 1) {
    sum = sum + numbers[i];
}

print(sum);  // Output: 15
```

## Architecture

The language is implemented with a classic interpreter pipeline:

1. **Lexer** (`lexer.py`): Tokenizes source code
2. **Parser** (`parser.py`): Builds an Abstract Syntax Tree (AST)
3. **Interpreter** (`interpreter.py`): Evaluates the AST
4. **REPL** (`repl.py`): Interactive shell

## Project Structure

```
lang-repl/
├── lexer.py          # Lexical analyzer
├── parser.py         # Recursive descent parser
├── interpreter.py    # AST interpreter
├── repl.py           # Interactive REPL
├── langrepl.py       # Main entry point
├── examples/         # Example programs
└── README.md         # This file
```

## License

MIT License - Feel free to use and modify!

## Contributing

Contributions are welcome! Feel free to submit issues or pull requests.