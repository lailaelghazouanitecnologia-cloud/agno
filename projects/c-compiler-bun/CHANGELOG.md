# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2024-01-XX

### Added
- Initial project scaffold
- Core interfaces and types
- Lexer implementation
- Parser implementation
- Micro-VM architecture with:
  - ArithmeticVM
  - ComparisonVM
  - ControlFlowVM
  - FunctionVM
  - MemoryVM
  - IOVM
  - TypeVM
- Main compiler orchestrator
- CLI entry point
- Comprehensive test suite
- Example C programs

### Features
- Variable declarations (int, char, float)
- Arithmetic operations (+, -, *, /, %)
- Comparison operations (<, >, <=, >=, ==, !=)
- Control flow (if/else/else-if, while, for)
- Functions with parameters and return values
- Arrays
- Basic pointer operations (&, *)
- printf/scanf I/O
- String literals