/**
 * Comprehensive tests for Core Interfaces
 * Tests cover: happy paths, edge cases, error cases
 */

import { describe, it, expect, beforeEach } from 'bun:test';
import {
  // Interfaces
  MicroVM,
  ExecutionContext,
  MemoryState,
  StackFrame,
  Variable,
  CallStack,
  CallFrame,
  ExecutionResult,
  MemoryChange,
  Lexer,
  Parser,
  Compiler,
  CompilationResult,
  CompilationError,
  CompilationWarning,
  VMRegistry,
  TypeChecker,
  IOHandler,
} from '../src/core/interfaces.js';
import {
  // Types
  TokenType,
  Token,
  NodeType,
  ASTNode,
  ValueType,
  BinaryOp,
  UnaryOp,
  CType,
  TypeInfo,
  FunctionSignature,
  ParameterInfo,
  SymbolEntry,
  Scope,
  SymbolTable,
} from '../src/core/types.js';

describe('Core Interfaces - Type Definitions', () => {
  describe('TokenType Enum', () => {
    it('should have all keyword tokens', () => {
      expect(TokenType.INT).toBe('INT');
      expect(TokenType.CHAR).toBe('CHAR');
      expect(TokenType.FLOAT).toBe('FLOAT');
      expect(TokenType.VOID).toBe('VOID');
      expect(TokenType.IF).toBe('IF');
      expect(TokenType.ELSE).toBe('ELSE');
      expect(TokenType.ELSE_IF).toBe('ELSE_IF');
      expect(TokenType.WHILE).toBe('WHILE');
      expect(TokenType.FOR).toBe('FOR');
      expect(TokenType.RETURN).toBe('RETURN');
      expect(TokenType.BREAK).toBe('BREAK');
      expect(TokenType.CONTINUE).toBe('CONTINUE');
    });

    it('should have all literal tokens', () => {
      expect(TokenType.INTEGER).toBe('INTEGER');
      expect(TokenType.FLOAT).toBe('FLOAT');
      expect(TokenType.CHARACTER).toBe('CHARACTER');
      expect(TokenType.STRING).toBe('STRING');
    });

    it('should have all operator tokens', () => {
      expect(TokenType.PLUS).toBe('PLUS');
      expect(TokenType.MINUS).toBe('MINUS');
      expect(TokenType.MULTIPLY).toBe('MULTIPLY');
      expect(TokenType.DIVIDE).toBe('DIVIDE');
      expect(TokenType.MODULO).toBe('MODULO');
    });

    it('should have all comparison operator tokens', () => {
      expect(TokenType.LESS).toBe('LESS');
      expect(TokenType.GREATER).toBe('GREATER');
      expect(TokenType.LESS_EQUAL).toBe('LESS_EQUAL');
      expect(TokenType.GREATER_EQUAL).toBe('GREATER_EQUAL');
      expect(TokenType.EQUAL).toBe('EQUAL');
      expect(TokenType.NOT_EQUAL).toBe('NOT_EQUAL');
    });

    it('should have all logical operator tokens', () => {
      expect(TokenType.AND).toBe('AND');
      expect(TokenType.OR).toBe('OR');
      expect(TokenType.NOT).toBe('NOT');
    });

    it('should have all assignment tokens', () => {
      expect(TokenType.ASSIGN).toBe('ASSIGN');
      expect(TokenType.PLUS_ASSIGN).toBe('PLUS_ASSIGN');
      expect(TokenType.MINUS_ASSIGN).toBe('MINUS_ASSIGN');
      expect(TokenType.MULTIPLY_ASSIGN).toBe('MULTIPLY_ASSIGN');
      expect(TokenType.DIVIDE_ASSIGN).toBe('DIVIDE_ASSIGN');
    });

    it('should have pointer operator tokens', () => {
      expect(TokenType.ADDRESS).toBe('ADDRESS');
      expect(TokenType.DEREFERENCE).toBe('DEREFERENCE');
    });

    it('should have all punctuation tokens', () => {
      expect(TokenType.SEMICOLON).toBe('SEMICOLON');
      expect(TokenType.COMMA).toBe('COMMA');
      expect(TokenType.DOT).toBe('DOT');
      expect(TokenType.ARROW).toBe('ARROW');
    });

    it('should have all bracket tokens', () => {
      expect(TokenType.LEFT_PAREN).toBe('LEFT_PAREN');
      expect(TokenType.RIGHT_PAREN).toBe('RIGHT_PAREN');
      expect(TokenType.LEFT_BRACE).toBe('LEFT_BRACE');
      expect(TokenType.RIGHT_BRACE).toBe('RIGHT_BRACE');
      expect(TokenType.LEFT_BRACKET).toBe('LEFT_BRACKET');
      expect(TokenType.RIGHT_BRACKET).toBe('RIGHT_BRACKET');
    });

    it('should have special tokens', () => {
      expect(TokenType.EOF).toBe('EOF');
      expect(TokenType.UNKNOWN).toBe('UNKNOWN');
    });
  });

  describe('Token Interface', () => {
    it('should create a valid token', () => {
      const token: Token = {
        type: TokenType.INTEGER,
        value: '42',
        line: 1,
        column: 0,
      };
      expect(token.type).toBe(TokenType.INTEGER);
      expect(token.value).toBe('42');
      expect(token.line).toBe(1);
      expect(token.column).toBe(0);
    });

    it('should handle string tokens', () => {
      const token: Token = {
        type: TokenType.STRING,
        value: 'Hello, World!',
        line: 5,
        column: 10,
      };
      expect(token.type).toBe(TokenType.STRING);
      expect(token.value).toBe('Hello, World!');
    });

    it('should handle identifier tokens', () => {
      const token: Token = {
        type: TokenType.IDENTIFIER,
        value: 'myVariable',
        line: 3,
        column: 4,
      };
      expect(token.type).toBe(TokenType.IDENTIFIER);
      expect(token.value).toBe('myVariable');
    });
  });

  describe('NodeType Enum', () => {
    it('should have program node type', () => {
      expect(NodeType.PROGRAM).toBe('PROGRAM');
    });

    it('should have all declaration node types', () => {
      expect(NodeType.FUNCTION_DECL).toBe('FUNCTION_DECL');
      expect(NodeType.FUNCTION_DEF).toBe('FUNCTION_DEF');
      expect(NodeType.VARIABLE_DECL).toBe('VARIABLE_DECL');
      expect(NodeType.PARAM_DECL).toBe('PARAM_DECL');
    });

    it('should have all statement node types', () => {
      expect(NodeType.COMPOUND_STMT).toBe('COMPOUND_STMT');
      expect(NodeType.IF_STMT).toBe('IF_STMT');
      expect(NodeType.WHILE_STMT).toBe('WHILE_STMT');
      expect(NodeType.FOR_STMT).toBe('FOR_STMT');
      expect(NodeType.RETURN_STMT).toBe('RETURN_STMT');
      expect(NodeType.BREAK_STMT).toBe('BREAK_STMT');
      expect(NodeType.CONTINUE_STMT).toBe('CONTINUE_STMT');
      expect(NodeType.EXPR_STMT).toBe('EXPR_STMT');
    });

    it('should have all expression node types', () => {
      expect(NodeType.BINARY_EXPR).toBe('BINARY_EXPR');
      expect(NodeType.UNARY_EXPR).toBe('UNARY_EXPR');
      expect(NodeType.ASSIGN_EXPR).toBe('ASSIGN_EXPR');
      expect(NodeType.CALL_EXPR).toBe('CALL_EXPR');
      expect(NodeType.ARRAY_ACCESS_EXPR).toBe('ARRAY_ACCESS_EXPR');
      expect(NodeType.MEMBER_EXPR).toBe('MEMBER_EXPR');
    });

    it('should have all literal node types', () => {
      expect(NodeType.INTEGER_LITERAL).toBe('INTEGER_LITERAL');
      expect(NodeType.FLOAT_LITERAL).toBe('FLOAT_LITERAL');
      expect(NodeType.CHAR_LITERAL).toBe('CHAR_LITERAL');
      expect(NodeType.STRING_LITERAL).toBe('STRING_LITERAL');
    });

    it('should have all reference node types', () => {
      expect(NodeType.IDENTIFIER_EXPR).toBe('IDENTIFIER_EXPR');
      expect(NodeType.ADDRESS_EXPR).toBe('ADDRESS_EXPR');
      expect(NodeType.DEREFERENCE_EXPR).toBe('DEREFERENCE_EXPR');
    });

    it('should have type specifier node type', () => {
      expect(NodeType.TYPE_SPECIFIER).toBe('TYPE_SPECIFIER');
    });
  });

  describe('ASTNode Interface', () => {
    it('should create a valid AST node', () => {
      const node: ASTNode = {
        type: NodeType.INTEGER_LITERAL,
        line: 1,
        column: 0,
        value: 42,
      };
      expect(node.type).toBe(NodeType.INTEGER_LITERAL);
      expect(node.line).toBe(1);
      expect(node.column).toBe(0);
      expect(node.value).toBe(42);
    });

    it('should allow additional properties', () => {
      const node: ASTNode = {
        type: NodeType.BINARY_EXPR,
        line: 2,
        column: 5,
        operator: BinaryOp.ADD,
        left: { type: NodeType.INTEGER_LITERAL, line: 2, column: 5, value: 10 },
        right: { type: NodeType.INTEGER_LITERAL, line: 2, column: 9, value: 20 },
      };
      expect(node.operator).toBe(BinaryOp.ADD);
      expect(node.left).toBeDefined();
      expect(node.right).toBeDefined();
    });
  });

  describe('ValueType', () => {
    it('should accept number values', () => {
      const value: ValueType = 42;
      expect(typeof value).toBe('number');
    });

    it('should accept string values', () => {
      const value: ValueType = 'hello';
      expect(typeof value).toBe('string');
    });

    it('should accept boolean values', () => {
      const value: ValueType = true;
      expect(typeof value).toBe('boolean');
    });

    it('should accept null values', () => {
      const value: ValueType = null;
      expect(value).toBeNull();
    });

    it('should accept array values', () => {
      const value: ValueType = [1, 2, 3];
      expect(Array.isArray(value)).toBe(true);
    });

    it('should accept nested array values', () => {
      const value: ValueType = [[1, 2], [3, 4]];
      expect(Array.isArray(value)).toBe(true);
    });
  });

  describe('BinaryOp Enum', () => {
    it('should have arithmetic operators', () => {
      expect(BinaryOp.ADD).toBe('+');
      expect(BinaryOp.SUB).toBe('-');
      expect(BinaryOp.MUL).toBe('*');
      expect(BinaryOp.DIV).toBe('/');
      expect(BinaryOp.MOD).toBe('%');
    });

    it('should have comparison operators', () => {
      expect(BinaryOp.LT).toBe('<');
      expect(BinaryOp.GT).toBe('>');
      expect(BinaryOp.LTE).toBe('<=');
      expect(BinaryOp.GTE).toBe('>=');
      expect(BinaryOp.EQ).toBe('==');
      expect(BinaryOp.NEQ).toBe('!=');
    });

    it('should have logical operators', () => {
      expect(BinaryOp.AND).toBe('&&');
      expect(BinaryOp.OR).toBe('||');
    });

    it('should have assignment operators', () => {
      expect(BinaryOp.ASSIGN).toBe('=');
      expect(BinaryOp.ADD_ASSIGN).toBe('+=');
      expect(BinaryOp.SUB_ASSIGN).toBe('-=');
      expect(BinaryOp.MUL_ASSIGN).toBe('*=');
      expect(BinaryOp.DIV_ASSIGN).toBe('/=');
    });
  });

  describe('UnaryOp Enum', () => {
    it('should have arithmetic unary operators', () => {
      expect(UnaryOp.POS).toBe('+');
      expect(UnaryOp.NEG).toBe('-');
    });

    it('should have logical unary operators', () => {
      expect(UnaryOp.NOT).toBe('!');
      expect(UnaryOp.BIT_NOT).toBe('~');
    });

    it('should have pointer operators', () => {
      expect(UnaryOp.ADDRESS).toBe('&');
      expect(UnaryOp.DEREFERENCE).toBe('*');
    });

    it('should have increment/decrement operators', () => {
      expect(UnaryOp.PRE_INC).toBe('++');
      expect(UnaryOp.PRE_DEC).toBe('--');
      expect(UnaryOp.POST_INC).toBe('++');
      expect(UnaryOp.POST_DEC).toBe('--');
    });
  });

  describe('CType Enum', () => {
    it('should have basic types', () => {
      expect(CType.INT).toBe('int');
      expect(CType.CHAR).toBe('char');
      expect(CType.FLOAT).toBe('float');
      expect(CType.VOID).toBe('void');
    });

    it('should have composite types', () => {
      expect(CType.POINTER).toBe('pointer');
      expect(CType.ARRAY).toBe('array');
    });
  });

  describe('TypeInfo Interface', () => {
    it('should create basic type info', () => {
      const typeInfo: TypeInfo = {
        base: CType.INT,
        isPointer: false,
        isArray: false,
      };
      expect(typeInfo.base).toBe(CType.INT);
      expect(typeInfo.isPointer).toBe(false);
      expect(typeInfo.isArray).toBe(false);
    });

    it('should create pointer type info', () => {
      const typeInfo: TypeInfo = {
        base: CType.INT,
        isPointer: true,
        isArray: false,
        pointerDepth: 1,
      };
      expect(typeInfo.isPointer).toBe(true);
      expect(typeInfo.pointerDepth).toBe(1);
    });

    it('should create array type info', () => {
      const typeInfo: TypeInfo = {
        base: CType.INT,
        isPointer: false,
        isArray: true,
        arrayDimensions: [10],
      };
      expect(typeInfo.isArray).toBe(true);
      expect(typeInfo.arrayDimensions).toEqual([10]);
    });

    it('should create multi-dimensional array type info', () => {
      const typeInfo: TypeInfo = {
        base: CType.INT,
        isPointer: false,
        isArray: true,
        arrayDimensions: [10, 20],
      };
      expect(typeInfo.arrayDimensions).toEqual([10, 20]);
    });
  });

  describe('FunctionSignature Interface', () => {
    it('should create function signature', () => {
      const signature: FunctionSignature = {
        name: 'main',
        returnType: { base: CType.INT, isPointer: false, isArray: false },
        parameters: [],
        isVariadic: false,
      };
      expect(signature.name).toBe('main');
      expect(signature.returnType.base).toBe(CType.INT);
      expect(signature.parameters).toEqual([]);
      expect(signature.isVariadic).toBe(false);
    });

    it('should create function signature with parameters', () => {
      const signature: FunctionSignature = {
        name: 'add',
        returnType: { base: CType.INT, isPointer: false, isArray: false },
        parameters: [
          { name: 'a', type: { base: CType.INT, isPointer: false, isArray: false } },
          { name: 'b', type: { base: CType.INT, isPointer: false, isArray: false } },
        ],
        isVariadic: false,
      };
      expect(signature.parameters.length).toBe(2);
      expect(signature.parameters[0].name).toBe('a');
      expect(signature.parameters[1].name).toBe('b');
    });

    it('should create variadic function signature', () => {
      const signature: FunctionSignature = {
        name: 'printf',
        returnType: { base: CType.INT, isPointer: false, isArray: false },
        parameters: [
          { name: 'format', type: { base: CType.CHAR, isPointer: true, isArray: false } },
        ],
        isVariadic: true,
      };
      expect(signature.isVariadic).toBe(true);
    });
  });

  describe('ParameterInfo Interface', () => {
    it('should create parameter info', () => {
      const param: ParameterInfo = {
        name: 'x',
        type: { base: CType.INT, isPointer: false, isArray: false },
      };
      expect(param.name).toBe('x');
      expect(param.type.base).toBe(CType.INT);
    });
  });

  describe('SymbolEntry Interface', () => {
    it('should create variable symbol entry', () => {
      const entry: SymbolEntry = {
        name: 'myVar',
        type: { base: CType.INT, isPointer: false, isArray: false },
        kind: 'variable',
        scope: 0,
        value: 42,
      };
      expect(entry.name).toBe('myVar');
      expect(entry.kind).toBe('variable');
      expect(entry.scope).toBe(0);
      expect(entry.value).toBe(42);
    });

    it('should create function symbol entry', () => {
      const entry: SymbolEntry = {
        name: 'myFunc',
        type: { base: CType.VOID, isPointer: false, isArray: false },
        kind: 'function',
        scope: 0,
        functionSignature: {
          name: 'myFunc',
          returnType: { base: CType.VOID, isPointer: false, isArray: false },
          parameters: [],
          isVariadic: false,
        },
      };
      expect(entry.kind).toBe('function');
      expect(entry.functionSignature).toBeDefined();
    });

    it('should create parameter symbol entry', () => {
      const entry: SymbolEntry = {
        name: 'param',
        type: { base: CType.INT, isPointer: false, isArray: false },
        kind: 'parameter',
        scope: 1,
      };
      expect(entry.kind).toBe('parameter');
    });
  });

  describe('Scope Interface', () => {
    it('should create global scope', () => {
      const scope: Scope = {
        id: 0,
        symbols: new Map(),
      };
      expect(scope.id).toBe(0);
      expect(scope.parent).toBeUndefined();
      expect(scope.symbols).toBeInstanceOf(Map);
    });

    it('should create nested scope', () => {
      const scope: Scope = {
        id: 1,
        parent: 0,
        symbols: new Map(),
      };
      expect(scope.id).toBe(1);
      expect(scope.parent).toBe(0);
    });

    it('should add symbols to scope', () => {
      const scope: Scope = {
        id: 0,
        symbols: new Map(),
      };
      const entry: SymbolEntry = {
        name: 'x',
        type: { base: CType.INT, isPointer: false, isArray: false },
        kind: 'variable',
        scope: 0,
      };
      scope.symbols.set('x', entry);
      expect(scope.symbols.has('x')).toBe(true);
    });
  });

  describe('SymbolTable Interface', () => {
    let symbolTable: SymbolTable;

    beforeEach(() => {
      symbolTable = {
        scopes: [],
        currentScope: -1,
        enterScope() {
          const newScope: Scope = {
            id: this.scopes.length,
            parent: this.currentScope >= 0 ? this.currentScope : undefined,
            symbols: new Map(),
          };
          this.scopes.push(newScope);
          this.currentScope = newScope.id;
        },
        exitScope() {
          if (this.currentScope >= 0) {
            const current = this.scopes[this.currentScope];
            this.currentScope = current.parent ?? -1;
          }
        },
        addSymbol(entry: SymbolEntry) {
          if (this.currentScope >= 0) {
            this.scopes[this.currentScope].symbols.set(entry.name, entry);
          }
        },
        lookup(name: string) {
          for (let i = this.currentScope; i >= 0; i--) {
            const symbol = this.scopes[i].symbols.get(name);
            if (symbol) return symbol;
          }
          return undefined;
        },
        lookupLocal(name: string) {
          if (this.currentScope >= 0) {
            return this.scopes[this.currentScope].symbols.get(name);
          }
          return undefined;
        },
      };
    });

    it('should enter and exit scopes', () => {
      expect(symbolTable.currentScope).toBe(-1);
      symbolTable.enterScope();
      expect(symbolTable.currentScope).toBe(0);
      symbolTable.enterScope();
      expect(symbolTable.currentScope).toBe(1);
      symbolTable.exitScope();
      expect(symbolTable.currentScope).toBe(0);
      symbolTable.exitScope();
      expect(symbolTable.currentScope).toBe(-1);
    });

    it('should add and lookup symbols', () => {
      symbolTable.enterScope();
      const entry: SymbolEntry = {
        name: 'x',
        type: { base: CType.INT, isPointer: false, isArray: false },
        kind: 'variable',
        scope: 0,
        value: 42,
      };
      symbolTable.addSymbol(entry);
      const found = symbolTable.lookup('x');
      expect(found).toBeDefined();
      expect(found?.name).toBe('x');
    });

    it('should lookup symbols in parent scopes', () => {
      symbolTable.enterScope();
      const entry: SymbolEntry = {
        name: 'x',
        type: { base: CType.INT, isPointer: false, isArray: false },
        kind: 'variable',
        scope: 0,
        value: 42,
      };
      symbolTable.addSymbol(entry);
      symbolTable.enterScope();
      const found = symbolTable.lookup('x');
      expect(found).toBeDefined();
      expect(found?.scope).toBe(0);
    });

    it('should lookup only in local scope', () => {
      symbolTable.enterScope();
      const entry: SymbolEntry = {
        name: 'x',
        type: { base: CType.INT, isPointer: false, isArray: false },
        kind: 'variable',
        scope: 0,
        value: 42,
      };
      symbolTable.addSymbol(entry);
      symbolTable.enterScope();
      const found = symbolTable.lookupLocal('x');
      expect(found).toBeUndefined();
    });

    it('should handle shadowing', () => {
      symbolTable.enterScope();
      const entry1: SymbolEntry = {
        name: 'x',
        type: { base: CType.INT, isPointer: false, isArray: false },
        kind: 'variable',
        scope: 0,
        value: 42,
      };
      symbolTable.addSymbol(entry1);
      symbolTable.enterScope();
      const entry2: SymbolEntry = {
        name: 'x',
        type: { base: CType.FLOAT, isPointer: false, isArray: false },
        kind: 'variable',
        scope: 1,
        value: 3.14,
      };
      symbolTable.addSymbol(entry2);
      const found = symbolTable.lookup('x');
      expect(found?.scope).toBe(1);
      expect(found?.value).toBe(3.14);
    });
  });
});

describe('Core Interfaces - VM Interfaces', () => {
  describe('MicroVM Interface', () => {
    it('should create a minimal MicroVM implementation', () => {
      const vm: MicroVM = {
        name: 'TestVM',
        execute(node: ASTNode, context: ExecutionContext): ExecutionResult {
          return { success: true };
        },
        canHandle(node: ASTNode): boolean {
          return node.type === NodeType.INTEGER_LITERAL;
        },
      };
      expect(vm.name).toBe('TestVM');
      expect(vm.canHandle({ type: NodeType.INTEGER_LITERAL, line: 1, column: 0 })).toBe(true);
      expect(vm.canHandle({ type: NodeType.STRING_LITERAL, line: 1, column: 0 })).toBe(false);
    });

    it('should support optional initialize method', () => {
      let initialized = false;
      const vm: MicroVM = {
        name: 'TestVM',
        initialize(context?: unknown): void {
          initialized = true;
        },
        execute(node: ASTNode, context: ExecutionContext): ExecutionResult {
          return { success: true };
        },
        canHandle(node: ASTNode): boolean {
          return true;
        },
      };
      vm.initialize();
      expect(initialized).toBe(true);
    });

    it('should support optional reset method', () => {
      let resetCount = 0;
      const vm: MicroVM = {
        name: 'TestVM',
        execute(node: ASTNode, context: ExecutionContext): ExecutionResult {
          return { success: true };
        },
        canHandle(node: ASTNode): boolean {
          return true;
        },
        reset(): void {
          resetCount++;
        },
      };
      vm.reset();
      expect(resetCount).toBe(1);
    });

    it('should support optional getState and setState methods', () => {
      const vm: MicroVM = {
        name: 'TestVM',
        execute(node: ASTNode, context: ExecutionContext): ExecutionResult {
          return { success: true };
        },
        canHandle(node: ASTNode): boolean {
          return true;
        },
        getState(): unknown {
          return { counter: 42 };
        },
        setState(state: unknown): void {
          expect(state).toEqual({ counter: 42 });
        },
      };
      const state = vm.getState();
      vm.setState(state);
    });

    it('should execute with context and return result', () => {
      const vm: MicroVM = {
        name: 'TestVM',
        execute(node: ASTNode, context: ExecutionContext): ExecutionResult {
          return {
            success: true,
            value: 42,
            sideEffects: {
              output: ['test'],
              memoryChanges: [{ variable: 'x', newValue: 42 }],
            },
          };
        },
        canHandle(node: ASTNode): boolean {
          return true;
        },
      };
      const context: ExecutionContext = createMockExecutionContext();
      const result = vm.execute(
        { type: NodeType.INTEGER_LITERAL, line: 1, column: 0, value: 42 },
        context
      );
      expect(result.success).toBe(true);
      expect(result.value).toBe(42);
      expect(result.sideEffects?.output).toEqual(['test']);
    });
  });

  describe('ExecutionContext Interface', () => {
    it('should create execution context', () => {
      const context: ExecutionContext = {
        memory: createMockMemoryState(),
        callStack: createMockCallStack(),
        output: [],
        input: [],
        scopeDepth: 0,
        inLoop: false,
        breakFlag: false,
        continueFlag: false,
        hasReturned: false,
      };
      expect(context.scopeDepth).toBe(0);
      expect(context.inLoop).toBe(false);
      expect(context.breakFlag).toBe(false);
      expect(context.continueFlag).toBe(false);
      expect(context.hasReturned).toBe(false);
    });

    it('should support return value', () => {
      const context: ExecutionContext = {
        memory: createMockMemoryState(),
        callStack: createMockCallStack(),
        output: [],
        input: [],
        scopeDepth: 0,
        inLoop: false,
        breakFlag: false,
        continueFlag: false,
        returnValue: 42,
        hasReturned: true,
      };
      expect(context.returnValue).toBe(42);
      expect(context.hasReturned).toBe(true);
    });

    it('should support loop context', () => {
      const context: ExecutionContext = {
        memory: createMockMemoryState(),
        callStack: createMockCallStack(),
        output: [],
        input: [],
        scopeDepth: 1,
        inLoop: true,
        breakFlag: false,
        continueFlag: false,
        hasReturned: false,
      };
      expect(context.inLoop).toBe(true);
      expect(context.scopeDepth).toBe(1);
    });

    it('should support break and continue flags', () => {
      const context: ExecutionContext = {
        memory: createMockMemoryState(),
        callStack: createMockCallStack(),
        output: [],
        input: [],
        scopeDepth: 1,
        inLoop: true,
        breakFlag: true,
        continueFlag: false,
        hasReturned: false,
      };
      expect(context.breakFlag).toBe(true);
      expect(context.continueFlag).toBe(false);
    });
  });

  describe('MemoryState Interface', () => {
    it('should create memory state', () => {
      const memory: MemoryState = {
        frames: [],
        currentFrame: -1,
        heap: new Map(),
        get(name: string): ValueType | undefined {
          return undefined;
        },
        set(name: string, value: ValueType): void {},
        allocate(name: string, type: string, value?: ValueType): void {},
        pushFrame(): void {},
        popFrame(): void {},
        getCurrentFrame(): StackFrame {
          return { variables: new Map(), type: 'global' };
        },
      };
      expect(memory.frames).toEqual([]);
      expect(memory.currentFrame).toBe(-1);
      expect(memory.heap).toBeInstanceOf(Map);
    });

    it('should get and set variables', () => {
      const variables = new Map<string, ValueType>();
      const memory: MemoryState = {
        frames: [{ variables: new Map(), type: 'global' }],
        currentFrame: 0,
        heap: new Map(),
        get(name: string): ValueType | undefined {
          return variables.get(name);
        },
        set(name: string, value: ValueType): void {
          variables.set(name, value);
        },
        allocate(name: string, type: string, value?: ValueType): void {
          variables.set(name, value ?? 0);
        },
        pushFrame(): void {},
        popFrame(): void {},
        getCurrentFrame(): StackFrame {
          return { variables: new Map(), type: 'global' };
        },
      };
      memory.allocate('x', 'int', 42);
      expect(memory.get('x')).toBe(42);
      memory.set('x', 100);
      expect(memory.get('x')).toBe(100);
    });

    it('should handle undefined variables', () => {
      const memory: MemoryState = {
        frames: [],
        currentFrame: -1,
        heap: new Map(),
        get(name: string): ValueType | undefined {
          return undefined;
        },
        set(name: string, value: ValueType): void {},
        allocate(name: string, type: string, value?: ValueType): void {},
        pushFrame(): void {},
        popFrame(): void {},
        getCurrentFrame(): StackFrame {
          return { variables: new Map(), type: 'global' };
        },
      };
      expect(memory.get('nonexistent')).toBeUndefined();
    });

    it('should push and pop frames', () => {
      const frames: StackFrame[] = [];
      const memory: MemoryState = {
        frames,
        currentFrame: -1,
        heap: new Map(),
        get(name: string): ValueType | undefined {
          return undefined;
        },
        set(name: string, value: ValueType): void {},
        allocate(name: string, type: string, value?: ValueType): void {},
        pushFrame(): void {
          frames.push({ variables: new Map(), type: 'block' });
          this.currentFrame = frames.length - 1;
        },
        popFrame(): void {
          if (frames.length > 0) {
            frames.pop();
            this.currentFrame = frames.length - 1;
          }
        },
        getCurrentFrame(): StackFrame {
          return frames[this.currentFrame] ?? { variables: new Map(), type: 'global' };
        },
      };
      expect(memory.currentFrame).toBe(-1);
      memory.pushFrame();
      expect(memory.currentFrame).toBe(0);
      memory.pushFrame();
      expect(memory.currentFrame).toBe(1);
      memory.popFrame();
      expect(memory.currentFrame).toBe(0);
      memory.popFrame();
      expect(memory.currentFrame).toBe(-1);
    });

    it('should handle heap allocations', () => {
      const memory: MemoryState = {
        frames: [],
        currentFrame: -1,
        heap: new Map(),
        get(name: string): ValueType | undefined {
          return this.heap.get(name);
        },
        set(name: string, value: ValueType): void {
          this.heap.set(name, value);
        },
        allocate(name: string, type: string, value?: ValueType): void {
          this.heap.set(name, value ?? 0);
        },
        pushFrame(): void {},
        popFrame(): void {},
        getCurrentFrame(): StackFrame {
          return { variables: new Map(), type: 'global' };
        },
      };
      memory.allocate('heap_var', 'int', 99);
      expect(memory.heap.get('heap_var')).toBe(99);
    });
  });

  describe('StackFrame Interface', () => {
    it('should create global stack frame', () => {
      const frame: StackFrame = {
        variables: new Map(),
        type: 'global',
      };
      expect(frame.type).toBe('global');
      expect(frame.parent).toBeUndefined();
    });

    it('should create function stack frame', () => {
      const frame: StackFrame = {
        variables: new Map(),
        type: 'function',
        parent: 0,
      };
      expect(frame.type).toBe('function');
      expect(frame.parent).toBe(0);
    });

    it('should create block stack frame', () => {
      const frame: StackFrame = {
        variables: new Map(),
        type: 'block',
        parent: 1,
      };
      expect(frame.type).toBe('block');
      expect(frame.parent).toBe(1);
    });

    it('should store variables', () => {
      const frame: StackFrame = {
        variables: new Map(),
        type: 'global',
      };
      const variable: Variable = {
        name: 'x',
        type: 'int',
        value: 42,
        isArray: false,
        isPointer: false,
      };
      frame.variables.set('x', variable);
      expect(frame.variables.has('x')).toBe(true);
      expect(frame.variables.get('x')?.value).toBe(42);
    });
  });

  describe('Variable Interface', () => {
    it('should create simple variable', () => {
      const variable: Variable = {
        name: 'x',
        type: 'int',
        value: 42,
        isArray: false,
        isPointer: false,
      };
      expect(variable.name).toBe('x');
      expect(variable.type).toBe('int');
      expect(variable.value).toBe(42);
      expect(variable.isArray).toBe(false);
      expect(variable.isPointer).toBe(false);
    });

    it('should create array variable', () => {
      const variable: Variable = {
        name: 'arr',
        type: 'int',
        value: [1, 2, 3, 4, 5],
        isArray: true,
        dimensions: [5],
        isPointer: false,
      };
      expect(variable.isArray).toBe(true);
      expect(variable.dimensions).toEqual([5]);
      expect(Array.isArray(variable.value)).toBe(true);
    });

    it('should create multi-dimensional array variable', () => {
      const variable: Variable = {
        name: 'matrix',
        type: 'int',
        value: [[1, 2], [3, 4]],
        isArray: true,
        dimensions: [2, 2],
        isPointer: false,
      };
      expect(variable.dimensions).toEqual([2, 2]);
    });

    it('should create pointer variable', () => {
      const variable: Variable = {
        name: 'ptr',
        type: 'int',
        value: 1000,
        isArray: false,
        isPointer: true,
        address: '0x3e8',
      };
      expect(variable.isPointer).toBe(true);
      expect(variable.address).toBe('0x3e8');
    });

    it('should create pointer to array', () => {
      const variable: Variable = {
        name: 'arr_ptr',
        type: 'int',
        value: 2000,
        isArray: false,
        isPointer: true,
        address: '0x7d0',
      };
      expect(variable.isPointer).toBe(true);
    });
  });

  describe('CallStack Interface', () => {
    it('should create empty call stack', () => {
      const callStack: CallStack = {
        frames: [],
        currentFrame: -1,
        push(frame: CallFrame): void {},
        pop(): CallFrame | undefined {
          return undefined;
        },
        getCurrent(): CallFrame | undefined {
          return undefined;
        },
        peek(): CallFrame | undefined {
          return undefined;
        },
      };
      expect(callStack.frames).toEqual([]);
      expect(callStack.currentFrame).toBe(-1);
    });

    it('should push and pop frames', () => {
      const frames: CallFrame[] = [];
      const callStack: CallStack = {
        frames,
        currentFrame: -1,
        push(frame: CallFrame): void {
          frames.push(frame);
          this.currentFrame = frames.length - 1;
        },
        pop(): CallFrame | undefined {
          if (frames.length > 0) {
            this.currentFrame = frames.length - 2;
            return frames.pop();
          }
          return undefined;
        },
        getCurrent(): CallFrame | undefined {
          return frames[this.currentFrame];
        },
        peek(): CallFrame | undefined {
          return frames[frames.length - 1];
        },
      };
      const frame1: CallFrame = {
        functionName: 'main',
        locals: new Map(),
        parameters: new Map(),
      };
      callStack.push(frame1);
      expect(callStack.currentFrame).toBe(0);
      expect(callStack.getCurrent()?.functionName).toBe('main');

      const frame2: CallFrame = {
        functionName: 'helper',
        locals: new Map(),
        parameters: new Map(),
      };
      callStack.push(frame2);
      expect(callStack.currentFrame).toBe(1);
      expect(callStack.getCurrent()?.functionName).toBe('helper');

      const popped = callStack.pop();
      expect(popped?.functionName).toBe('helper');
      expect(callStack.currentFrame).toBe(0);
    });

    it('should peek without popping', () => {
      const frames: CallFrame[] = [];
      const callStack: CallStack = {
        frames,
        currentFrame: -1,
        push(frame: CallFrame): void {
          frames.push(frame);
          this.currentFrame = frames.length - 1;
        },
        pop(): CallFrame | undefined {
          return frames.pop();
        },
        getCurrent(): CallFrame | undefined {
          return frames[this.currentFrame];
        },
        peek(): CallFrame | undefined {
          return frames[frames.length - 1];
        },
      };
      const frame: CallFrame = {
        functionName: 'test',
        locals: new Map(),
        parameters: new Map(),
      };
      callStack.push(frame);
      const peeked = callStack.peek();
      expect(peeked?.functionName).toBe('test');
      expect(callStack.frames.length).toBe(1);
    });

    it('should handle empty stack pop', () => {
      const callStack: CallStack = {
        frames: [],
        currentFrame: -1,
        push(frame: CallFrame): void {},
        pop(): CallFrame | undefined {
          return undefined;
        },
        getCurrent(): CallFrame | undefined {
          return undefined;
        },
        peek(): CallFrame | undefined {
          return undefined;
        },
      };
      expect(callStack.pop()).toBeUndefined();
    });
  });

  describe('CallFrame Interface', () => {
    it('should create call frame', () => {
      const frame: CallFrame = {
        functionName: 'main',
        locals: new Map(),
        parameters: new Map(),
      };
      expect(frame.functionName).toBe('main');
      expect(frame.locals).toBeInstanceOf(Map);
      expect(frame.parameters).toBeInstanceOf(Map);
    });

    it('should create call frame with return address', () => {
      const frame: CallFrame = {
        functionName: 'helper',
        returnAddress: 42,
        locals: new Map(),
        parameters: new Map(),
      };
      expect(frame.returnAddress).toBe(42);
    });

    it('should create call frame with locals', () => {
      const locals = new Map<string, ValueType>();
      locals.set('x', 10);
      locals.set('y', 20);
      const frame: CallFrame = {
        functionName: 'func',
        locals,
        parameters: new Map(),
      };
      expect(frame.locals.get('x')).toBe(10);
      expect(frame.locals.get('y')).toBe(20);
    });

    it('should create call frame with parameters', () => {
      const parameters = new Map<string, ValueType>();
      parameters.set('a', 5);
      parameters.set('b', 10);
      const frame: CallFrame = {
        functionName: 'add',
        locals: new Map(),
        parameters,
      };
      expect(frame.parameters.get('a')).toBe(5);
      expect(frame.parameters.get('b')).toBe(10);
    });

    it('should create call frame with return value', () => {
      const frame: CallFrame = {
        functionName: 'multiply',
        locals: new Map(),
        parameters: new Map(),
        returnValue: 42,
      };
      expect(frame.returnValue).toBe(42);
    });
  });

  describe('ExecutionResult Interface', () => {
    it('should create successful result', () => {
      const result: ExecutionResult = {
        success: true,
        value: 42,
      };
      expect(result.success).toBe(true);
      expect(result.value).toBe(42);
    });

    it('should create failed result', () => {
      const result: ExecutionResult = {
        success: false,
        error: 'Division by zero',
      };
      expect(result.success).toBe(false);
      expect(result.error).toBe('Division by zero');
    });

    it('should create result with side effects', () => {
      const result: ExecutionResult = {
        success: true,
        value: 42,
        sideEffects: {
          output: ['Hello, World!'],
          memoryChanges: [
            { variable: 'x', oldValue: 10, newValue: 20 },
            { variable: 'y', newValue: 30 },
          ],
        },
      };
      expect(result.sideEffects?.output).toEqual(['Hello, World!']);
      expect(result.sideEffects?.memoryChanges?.length).toBe(2);
    });

    it('should create result with only output side effect', () => {
      const result: ExecutionResult = {
        success: true,
        sideEffects: {
          output: ['Line 1', 'Line 2'],
        },
      };
      expect(result.sideEffects?.output).toEqual(['Line 1', 'Line 2']);
      expect(result.sideEffects?.memoryChanges).toBeUndefined();
    });
  });

  describe('MemoryChange Interface', () => {
    it('should create memory change with old value', () => {
      const change: MemoryChange = {
        variable: 'x',
        oldValue: 10,
        newValue: 20,
      };
      expect(change.variable).toBe('x');
      expect(change.oldValue).toBe(10);
      expect(change.newValue).toBe(20);
    });

    it('should create memory change without old value', () => {
      const change: MemoryChange = {
        variable: 'y',
        newValue: 30,
      };
      expect(change.variable).toBe('y');
      expect(change.oldValue).toBeUndefined();
      expect(change.newValue).toBe(30);
    });
  });
});

describe('Core Interfaces - Compiler Interfaces', () => {
  describe('Lexer Interface', () => {
    it('should tokenize source code', () => {
      const lexer: Lexer = {
        tokenize(source: string): Token[] {
          return [
            { type: TokenType.INT, value: 'int', line: 1, column: 0 },
            { type: TokenType.IDENTIFIER, value: 'x', line: 1, column: 4 },
            { type: TokenType.ASSIGN, value: '=', line: 1, column: 6 },
            { type: TokenType.INTEGER, value: '42', line: 1, column: 8 },
            { type: TokenType.SEMICOLON, value: ';', line: 1, column: 10 },
          ];
        },
        getPosition(): { line: number; column: number } {
          return { line: 1, column: 11 };
        },
        reset(): void {},
      };
      const tokens = lexer.tokenize('int x = 42;');
      expect(tokens.length).toBe(5);
      expect(tokens[0].type).toBe(TokenType.INT);
      expect(tokens[4].type).toBe(TokenType.SEMICOLON);
    });

    it('should get current position', () => {
      const lexer: Lexer = {
        tokenize(source: string): Token[] {
          return [];
        },
        getPosition(): { line: number; column: number } {
          return { line: 5, column: 10 };
        },
        reset(): void {},
      };
      const pos = lexer.getPosition();
      expect(pos.line).toBe(5);
      expect(pos.column).toBe(10);
    });

    it('should reset lexer state', () => {
      let resetCalled = false;
      const lexer: Lexer = {
        tokenize(source: string): Token[] {
          return [];
        },
        getPosition(): { line: number; column: number } {
          return { line: 0, column: 0 };
        },
        reset(): void {
          resetCalled = true;
        },
      };
      lexer.reset();
      expect(resetCalled).toBe(true);
    });

    it('should handle empty source', () => {
      const lexer: Lexer = {
        tokenize(source: string): Token[] {
          return [];
        },
        getPosition(): { line: number; column: number } {
          return { line: 0, column: 0 };
        },
        reset(): void {},
      };
      const tokens = lexer.tokenize('');
      expect(tokens).toEqual([]);
    });
  });

  describe('Parser Interface', () => {
    it('should parse tokens into AST', () => {
      const parser: Parser = {
        parse(tokens: Token[]): ASTNode {
          return {
            type: NodeType.PROGRAM,
            line: 1,
            column: 0,
            body: [],
          };
        },
        getCurrentToken(): Token | undefined {
          return undefined;
        },
        reset(): void {},
      };
      const ast = parser.parse([]);
      expect(ast.type).toBe(NodeType.PROGRAM);
    });

    it('should get current token', () => {
      const parser: Parser = {
        parse(tokens: Token[]): ASTNode {
          return { type: NodeType.PROGRAM, line: 1, column: 0 };
        },
        getCurrentToken(): Token | undefined {
          return { type: TokenType.INT, value: 'int', line: 1, column: 0 };
        },
        reset(): void {},
      };
      const token = parser.getCurrentToken();
      expect(token?.type).toBe(TokenType.INT);
    });

    it('should return undefined when no current token', () => {
      const parser: Parser = {
        parse(tokens: Token[]): ASTNode {
          return { type: NodeType.PROGRAM, line: 1, column: 0 };
        },
        getCurrentToken(): Token | undefined {
          return undefined;
        },
        reset(): void {},
      };
      const token = parser.getCurrentToken();
      expect(token).toBeUndefined();
    });

    it('should reset parser state', () => {
      let resetCalled = false;
      const parser: Parser = {
        parse(tokens: Token[]): ASTNode {
          return { type: NodeType.PROGRAM, line: 1, column: 0 };
        },
        getCurrentToken(): Token | undefined {
          return undefined;
        },
        reset(): void {
          resetCalled = true;
        },
      };
      parser.reset();
      expect(resetCalled).toBe(true);
    });
  });

  describe('Compiler Interface', () => {
    it('should compile source code', () => {
      const compiler: Compiler = {
        compile(source: string): CompilationResult {
          return {
            success: true,
            ast: { type: NodeType.PROGRAM, line: 1, column: 0 },
            errors: [],
            warnings: [],
          };
        },
        run(ast: ASTNode, input?: string[]): ExecutionResult {
          return { success: true };
        },
        compileAndRun(source: string, input?: string[]): ExecutionResult {
          return { success: true };
        },
      };
      const result = compiler.compile('int main() { return 0; }');
      expect(result.success).toBe(true);
      expect(result.ast).toBeDefined();
    });

    it('should run compiled AST', () => {
      const compiler: Compiler = {
        compile(source: string): CompilationResult {
          return { success: true, errors: [], warnings: [] };
        },
        run(ast: ASTNode, input?: string[]): ExecutionResult {
          return {
            success: true,
            value: 0,
            sideEffects: { output: ['Program exited'] },
          };
        },
        compileAndRun(source: string, input?: string[]): ExecutionResult {
          return { success: true };
        },
      };
      const ast: ASTNode = { type: NodeType.PROGRAM, line: 1, column: 0 };
      const result = compiler.run(ast);
      expect(result.success).toBe(true);
      expect(result.value).toBe(0);
    });

    it('should compile and run in one step', () => {
      const compiler: Compiler = {
        compile(source: string): CompilationResult {
          return { success: true, errors: [], warnings: [] };
        },
        run(ast: ASTNode, input?: string[]): ExecutionResult {
          return { success: true };
        },
        compileAndRun(source: string, input?: string[]): ExecutionResult {
          return {
            success: true,
            value: 42,
            sideEffects: { output: ['Result: 42'] },
          };
        },
      };
      const result = compiler.compileAndRun('int main() { return 42; }');
      expect(result.success).toBe(true);
      expect(result.value).toBe(42);
    });

    it('should handle compilation errors', () => {
      const compiler: Compiler = {
        compile(source: string): CompilationResult {
          return {
            success: false,
            errors: [
              { message: 'Syntax error', line: 1, column: 5, type: 'syntax' },
            ],
            warnings: [],
          };
        },
        run(ast: ASTNode, input?: string[]): ExecutionResult {
          return { success: false, error: 'Compilation failed' };
        },
        compileAndRun(source: string, input?: string[]): ExecutionResult {
          return { success: false, error: 'Compilation failed' };
        },
      };
      const result = compiler.compile('int main {');
      expect(result.success).toBe(false);
      expect(result.errors.length).toBe(1);
    });
  });

  describe('CompilationResult Interface', () => {
    it('should create successful compilation result', () => {
      const result: CompilationResult = {
        success: true,
        ast: { type: NodeType.PROGRAM, line: 1, column: 0 },
        errors: [],
        warnings: [],
      };
      expect(result.success).toBe(true);
      expect(result.ast).toBeDefined();
      expect(result.errors).toEqual([]);
      expect(result.warnings).toEqual([]);
    });

    it('should create failed compilation result', () => {
      const result: CompilationResult = {
        success: false,
        errors: [
          { message: 'Unexpected token', line: 1, column: 10, type: 'syntax' },
        ],
        warnings: [],
      };
      expect(result.success).toBe(false);
      expect(result.errors.length).toBe(1);
    });

    it('should include warnings', () => {
      const result: CompilationResult = {
        success: true,
        ast: { type: NodeType.PROGRAM, line: 1, column: 0 },
        errors: [],
        warnings: [
          { message: 'Unused variable', line: 5, column: 8 },
          { message: 'Implicit conversion', line: 10, column: 15 },
        ],
      };
      expect(result.warnings.length).toBe(2);
    });
  });

  describe('CompilationError Interface', () => {
    it('should create syntax error', () => {
      const error: CompilationError = {
        message: 'Missing semicolon',
        line: 10,
        column: 20,
        type: 'syntax',
      };
      expect(error.message).toBe('Missing semicolon');
      expect(error.line).toBe(10);
      expect(error.column).toBe(20);
      expect(error.type).toBe('syntax');
    });

    it('should create semantic error', () => {
      const error: CompilationError = {
        message: 'Undefined variable',
        line: 5,
        column: 10,
        type: 'semantic',
      };
      expect(error.type).toBe('semantic');
    });

    it('should create type error', () => {
      const error: CompilationError = {
        message: 'Type mismatch',
        line: 15,
        column: 25,
        type: 'type',
      };
      expect(error.type).toBe('type');
    });
  });

  describe('CompilationWarning Interface', () => {
    it('should create compilation warning', () => {
      const warning: CompilationWarning = {
        message: 'Unused variable "x"',
        line: 3,
        column: 5,
      };
      expect(warning.message).toBe('Unused variable "x"');
      expect(warning.line).toBe(3);
      expect(warning.column).toBe(5);
    });
  });
});

describe('Core Interfaces - Registry and Utility Interfaces', () => {
  describe('VMRegistry Interface', () => {
    let registry: VMRegistry;
    let testVM: MicroVM;

    beforeEach(() => {
      const vms = new Map<string, MicroVM>();
      registry = {
        register(vm: MicroVM): void {
          vms.set(vm.name, vm);
        },
        unregister(name: string): void {
          vms.delete(name);
        },
        get(name: string): MicroVM | undefined {
          return vms.get(name);
        },
        find(node: ASTNode): MicroVM | undefined {
          for (const vm of vms.values()) {
            if (vm.canHandle(node)) return vm;
          }
          return undefined;
        },
        getAll(): MicroVM[] {
          return Array.from(vms.values());
        },
        clear(): void {
          vms.clear();
        },
      };

      testVM = {
        name: 'TestVM',
        execute(node: ASTNode, context: ExecutionContext): ExecutionResult {
          return { success: true };
        },
        canHandle(node: ASTNode): boolean {
          return node.type === NodeType.INTEGER_LITERAL;
        },
      };
    });

    it('should register a VM', () => {
      registry.register(testVM);
      const retrieved = registry.get('TestVM');
      expect(retrieved).toBeDefined();
      expect(retrieved?.name).toBe('TestVM');
    });

    it('should unregister a VM', () => {
      registry.register(testVM);
      expect(registry.get('TestVM')).toBeDefined();
      registry.unregister('TestVM');
      expect(registry.get('TestVM')).toBeUndefined();
    });

    it('should find VM for node', () => {
      registry.register(testVM);
      const node: ASTNode = { type: NodeType.INTEGER_LITERAL, line: 1, column: 0 };
      const found = registry.find(node);
      expect(found).toBeDefined();
      expect(found?.name).toBe('TestVM');
    });

    it('should return undefined when no VM can handle node', () => {
      registry.register(testVM);
      const node: ASTNode = { type: NodeType.STRING_LITERAL, line: 1, column: 0 };
      const found = registry.find(node);
      expect(found).toBeUndefined();
    });

    it('should get all registered VMs', () => {
      registry.register(testVM);
      const vm2: MicroVM = {
        name: 'TestVM2',
        execute(node: ASTNode, context: ExecutionContext): ExecutionResult {
          return { success: true };
        },
        canHandle(node: ASTNode): boolean {
          return node.type === NodeType.STRING_LITERAL;
        },
      };
      registry.register(vm2);
      const all = registry.getAll();
      expect(all.length).toBe(2);
      expect(all.map((vm) => vm.name)).toContain('TestVM');
      expect(all.map((vm) => vm.name)).toContain('TestVM2');
    });

    it('should clear all VMs', () => {
      registry.register(testVM);
      expect(registry.getAll().length).toBe(1);
      registry.clear();
      expect(registry.getAll().length).toBe(0);
    });

    it('should handle getting non-existent VM', () => {
      const retrieved = registry.get('NonExistent');
      expect(retrieved).toBeUndefined();
    });
  });

  describe('TypeChecker Interface', () => {
    it('should check type of expression', () => {
      const typeChecker: TypeChecker = {
        checkType(node: ASTNode, context: ExecutionContext): string {
          if (node.type === NodeType.INTEGER_LITERAL) return 'int';
          if (node.type === NodeType.FLOAT_LITERAL) return 'float';
          return 'unknown';
        },
        areCompatible(type1: string, type2: string): boolean {
          return type1 === type2;
        },
        cast(value: ValueType, fromType: string, toType: string): ValueType {
          if (fromType === 'int' && toType === 'float') {
            return Number(value);
          }
          return value;
        },
      };
      const node: ASTNode = { type: NodeType.INTEGER_LITERAL, line: 1, column: 0 };
      const context = createMockExecutionContext();
      const type = typeChecker.checkType(node, context);
      expect(type).toBe('int');
    });

    it('should check type compatibility', () => {
      const typeChecker: TypeChecker = {
        checkType(node: ASTNode, context: ExecutionContext): string {
          return 'int';
        },
        areCompatible(type1: string, type2: string): boolean {
          const compatibleTypes = ['int', 'char', 'float'];
          return compatibleTypes.includes(type1) && compatibleTypes.includes(type2);
        },
        cast(value: ValueType, fromType: string, toType: string): ValueType {
          return value;
        },
      };
      expect(typeChecker.areCompatible('int', 'float')).toBe(true);
      expect(typeChecker.areCompatible('int', 'int')).toBe(true);
      expect(typeChecker.areCompatible('int', 'pointer')).toBe(false);
    });

    it('should perform type casting', () => {
      const typeChecker: TypeChecker = {
        checkType(node: ASTNode, context: ExecutionContext): string {
          return 'int';
        },
        areCompatible(type1: string, type2: string): boolean {
          return true;
        },
        cast(value: ValueType, fromType: string, toType: string): ValueType {
          if (fromType === 'int' && toType === 'float') {
            return Number(value);
          }
          if (fromType === 'float' && toType === 'int') {
            return Math.floor(Number(value));
          }
          return value;
        },
      };
      expect(typeChecker.cast(42, 'int', 'float')).toBe(42);
      expect(typeChecker.cast(3.7, 'float', 'int')).toBe(3);
    });

    it('should handle incompatible types', () => {
      const typeChecker: TypeChecker = {
        checkType(node: ASTNode, context: ExecutionContext): string {
          return 'int';
        },
        areCompatible(type1: string, type2: string): boolean {
          return type1 === type2;
        },
        cast(value: ValueType, fromType: string, toType: string): ValueType {
          return value;
        },
      };
      expect(typeChecker.areCompatible('int', 'function')).toBe(false);
    });
  });

  describe('IOHandler Interface', () => {
    let outputBuffer: string[];
    let inputBuffer: string[];

    beforeEach(() => {
      outputBuffer = [];
      inputBuffer = ['42', 'hello'];
    });

    it('should write output', () => {
      const io: IOHandler = {
        write(format: string, args: ValueType[]): void {
          let message = format;
          args.forEach((arg, i) => {
            message = message.replace(`%${i + 1}`, String(arg));
          });
          outputBuffer.push(message);
        },
        read(format: string): ValueType[] {
          return inputBuffer.splice(0, 1);
        },
        getOutput(): string[] {
          return [...outputBuffer];
        },
        clearOutput(): void {
          outputBuffer = [];
        },
        setInput(input: string[]): void {
          inputBuffer = [...input];
        },
      };
      io.write('Value: %1', [42]);
      expect(outputBuffer).toEqual(['Value: 42']);
    });

    it('should write multiple values', () => {
      const io: IOHandler = {
        write(format: string, args: ValueType[]): void {
          let message = format;
          args.forEach((arg, i) => {
            message = message.replace(`%${i + 1}`, String(arg));
          });
          outputBuffer.push(message);
        },
        read(format: string): ValueType[] {
          return [];
        },
        getOutput(): string[] {
          return [...outputBuffer];
        },
        clearOutput(): void {
          outputBuffer = [];
        },
        setInput(input: string[]): void {},
      };
      io.write('Name: %1, Age: %2', ['John', 30]);
      expect(outputBuffer).toEqual(['Name: John, Age: 30']);
    });

    it('should read input', () => {
      const io: IOHandler = {
        write(format: string, args: ValueType[]): void {},
        read(format: string): ValueType[] {
          return inputBuffer.splice(0, 1);
        },
        getOutput(): string[] {
          return [];
        },
        clearOutput(): void {},
        setInput(input: string[]): void {
          inputBuffer = [...input];
        },
      };
      const value = io.read('%d');
      expect(value).toEqual(['42']);
      expect(inputBuffer).toEqual(['hello']);
    });

    it('should get output buffer', () => {
      const io: IOHandler = {
        write(format: string, args: ValueType[]): void {
          outputBuffer.push(format);
        },
        read(format: string): ValueType[] {
          return [];
        },
        getOutput(): string[] {
          return [...outputBuffer];
        },
        clearOutput(): void {
          outputBuffer = [];
        },
        setInput(input: string[]): void {},
      };
      io.write('Line 1', []);
      io.write('Line 2', []);
      const output = io.getOutput();
      expect(output).toEqual(['Line 1', 'Line 2']);
    });

    it('should clear output buffer', () => {
      const io: IOHandler = {
        write(format: string, args: ValueType[]): void {
          outputBuffer.push(format);
        },
        read(format: string): ValueType[] {
          return [];
        },
        getOutput(): string[] {
          return [...outputBuffer];
        },
        clearOutput(): void {
          outputBuffer = [];
        },
        setInput(input: string[]): void {},
      };
      io.write('Line 1', []);
      io.write('Line 2', []);
      expect(outputBuffer.length).toBe(2);
      io.clearOutput();
      expect(outputBuffer.length).toBe(0);
    });

    it('should set input buffer', () => {
      const io: IOHandler = {
        write(format: string, args: ValueType[]): void {},
        read(format: string): ValueType[] {
          return inputBuffer.splice(0, 1);
        },
        getOutput(): string[] {
          return [];
        },
        clearOutput(): void {},
        setInput(input: string[]): void {
          inputBuffer = [...input];
        },
      };
      io.setInput(['100', '200', '300']);
      expect(inputBuffer).toEqual(['100', '200', '300']);
      const val1 = io.read('');
      const val2 = io.read('');
      expect(val1).toEqual(['100']);
      expect(val2).toEqual(['200']);
      expect(inputBuffer).toEqual(['300']);
    });
  });
});

// Helper functions to create mock objects
function createMockExecutionContext(): ExecutionContext {
  return {
    memory: createMockMemoryState(),
    callStack: createMockCallStack(),
    output: [],
    input: [],
    scopeDepth: 0,
    inLoop: false,
    breakFlag: false,
    continueFlag: false,
    hasReturned: false,
  };
}

function createMockMemoryState(): MemoryState {
  return {
    frames: [],
    currentFrame: -1,
    heap: new Map(),
    get(name: string): ValueType | undefined {
      return undefined;
    },
    set(name: string, value: ValueType): void {},
    allocate(name: string, type: string, value?: ValueType): void {},
    pushFrame(): void {},
    popFrame(): void {},
    getCurrentFrame(): StackFrame {
      return { variables: new Map(), type: 'global' };
    },
  };
}

function createMockCallStack(): CallStack {
  return {
    frames: [],
    currentFrame: -1,
    push(frame: CallFrame): void {},
    pop(): CallFrame | undefined {
      return undefined;
    },
    getCurrent(): CallFrame | undefined {
      return undefined;
    },
    peek(): CallFrame | undefined {
      return undefined;
    },
  };
}