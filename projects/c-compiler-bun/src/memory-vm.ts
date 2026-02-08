/**
 * MemoryVM - Handles variables, arrays, pointers, and scoping with stack frames
 */

import {
  MicroVM,
  ASTNode,
  ExecutionContext,
  RuntimeValue,
  ValueType,
  IntValue,
  FloatValue,
  CharValue,
  StringValue,
  PointerValue,
  ArrayValue,
  MemoryManager,
  StackFrame,
  VarDeclNode,
  ArrayDeclNode,
  IdentifierNode,
  ArrayAccessNode,
  NodeType,
  NumberNode,
  StringNode
} from "./interfaces.js";

export class MemoryVM implements MicroVM {
  name = "MemoryVM";

  private memory: MemoryManager = {
    stackFrames: [],
    globals: new Map(),
    heap: new Map(),

    currentFrame(): StackFrame {
      if (this.stackFrames.length === 0) {
        // Create global frame
        const globalFrame: StackFrame = {
          functionName: "global",
          variables: this.globals
        };
        this.stackFrames.push(globalFrame);
      }
      return this.stackFrames[this.stackFrames.length - 1];
    },

    pushFrame(functionName: string): StackFrame {
      const frame: StackFrame = {
        functionName,
        variables: new Map()
      };
      this.stackFrames.push(frame);
      return frame;
    },

    popFrame(): StackFrame | null {
      if (this.stackFrames.length <= 1) return null;
      return this.stackFrames.pop() || null;
    },

    getVariable(name: string): RuntimeValue | undefined {
      // Search from innermost to outermost
      for (let i = this.stackFrames.length - 1; i >= 0; i--) {
        const frame = this.stackFrames[i];
        if (frame.variables.has(name)) {
          return frame.variables.get(name);
        }
      }
      return undefined;
    },

    setVariable(name: string, value: RuntimeValue): void {
      // Search from innermost to outermost
      for (let i = this.stackFrames.length - 1; i >= 0; i--) {
        const frame = this.stackFrames[i];
        if (frame.variables.has(name)) {
          frame.variables.set(name, value);
          return;
        }
      }
      // If not found, set in current frame
      this.currentFrame().variables.set(name, value);
    },

    declareVariable(name: string, value: RuntimeValue): void {
      this.currentFrame().variables.set(name, value);
    }
  };

  /**
   * Execute a memory-related node
   */
  public execute(node: ASTNode, context: ExecutionContext): RuntimeValue | void {
    switch (node.type) {
      case NodeType.VAR_DECL:
        return this.executeVarDecl(node as VarDeclNode, context);
      case NodeType.ARRAY_DECL:
        return this.executeArrayDecl(node as ArrayDeclNode, context);
      case NodeType.IDENTIFIER:
        return this.executeIdentifier(node as IdentifierNode, context);
      case NodeType.ARRAY_ACCESS:
        return this.executeArrayAccess(node as ArrayAccessNode, context);
      default:
        return undefined;
    }
  }

  /**
   * Execute variable declaration
   */
  private executeVarDecl(node: VarDeclNode, context: ExecutionContext): RuntimeValue {
    let value: RuntimeValue;

    if (node.init) {
      // Evaluate initialization expression
      // This would be handled by the compiler orchestrator
      // For now, we'll create a default value
      value = this.createDefaultValue(node.varType);
    } else {
      value = this.createDefaultValue(node.varType);
    }

    this.memory.declareVariable(node.name, value);
    return value;
  }

  /**
   * Execute array declaration
   */
  private executeArrayDecl(node: ArrayDeclNode, context: ExecutionContext): RuntimeValue {
    // Evaluate array size
    let size = 10; // Default size
    if (node.size.type === NodeType.NUMBER) {
      size = Math.floor((node.size as NumberNode).value);
    }

    // Create array with default values
    const elementType = this.typeStringToValueType(node.elementType);
    const defaultValue = this.createRuntimeValue(elementType, 0);

    const arrayValue: ArrayValue = {
      type: ValueType.ARRAY,
      value: Array(size).fill(defaultValue),
      elementType
    };

    this.memory.declareVariable(node.name, arrayValue);
    return arrayValue;
  }

  /**
   * Execute identifier access
   */
  private executeIdentifier(node: IdentifierNode, context: ExecutionContext): RuntimeValue {
    const value = this.memory.getVariable(node.name);
    if (value === undefined) {
      throw new Error(`Undefined variable: ${node.name}`);
    }
    return value;
  }

  /**
   * Execute array access
   */
  private executeArrayAccess(node: ArrayAccessNode, context: ExecutionContext): RuntimeValue {
    const arrayVar = this.memory.getVariable(node.array);
    if (!arrayVar || arrayVar.type !== ValueType.ARRAY) {
      throw new Error(`${node.array} is not an array`);
    }

    const array = arrayVar as ArrayValue;
    
    // Evaluate index (simplified - would need full expression evaluation)
    let index = 0;
    if (node.index.type === NodeType.NUMBER) {
      index = Math.floor((node.index as NumberNode).value);
    }

    if (index < 0 || index >= array.value.length) {
      throw new Error(`Array index out of bounds: ${index}`);
    }

    return array.value[index];
  }

  /**
   * Set a variable's value
   */
  public setVariable(name: string, value: RuntimeValue): void {
    this.memory.setVariable(name, value);
  }

  /**
   * Set an array element's value
   */
  public setArrayElement(arrayName: string, index: number, value: RuntimeValue): void {
    const arrayVar = this.memory.getVariable(arrayName);
    if (!arrayVar || arrayVar.type !== ValueType.ARRAY) {
      throw new Error(`${arrayName} is not an array`);
    }

    const array = arrayVar as ArrayValue;
    if (index < 0 || index >= array.value.length) {
      throw new Error(`Array index out of bounds: ${index}`);
    }

    array.value[index] = value;
  }

  /**
   * Get a variable's value
   */
  public getVariable(name: string): RuntimeValue | undefined {
    return this.memory.getVariable(name);
  }

  /**
   * Push a new stack frame for function call
   */
  public pushFrame(functionName: string): StackFrame {
    return this.memory.pushFrame(functionName);
  }

  /**
   * Pop the current stack frame
   */
  public popFrame(): StackFrame | null {
    return this.memory.popFrame();
  }

  /**
   * Get current stack frame
   */
  public currentFrame(): StackFrame {
    return this.memory.currentFrame();
  }

  /**
   * Create a default value for a type
   */
  private createDefaultValue(type: string): RuntimeValue {
    const valueType = this.typeStringToValueType(type);
    return this.createRuntimeValue(valueType, 0);
  }

  /**
   * Convert type string to ValueType enum
   */
  private typeStringToValueType(type: string): ValueType {
    switch (type) {
      case "int":
        return ValueType.INT;
      case "float":
        return ValueType.FLOAT;
      case "char":
        return ValueType.CHAR;
      default:
        return ValueType.INT;
    }
  }

  /**
   * Create a runtime value
   */
  private createRuntimeValue(type: ValueType, value: any): RuntimeValue {
    switch (type) {
      case ValueType.INT:
        return { type: ValueType.INT, value: Math.floor(value) };
      case ValueType.FLOAT:
        return { type: ValueType.FLOAT, value: Number(value) };
      case ValueType.CHAR:
        return { type: ValueType.CHAR, value: String(value)[0] || "\0" };
      case ValueType.STRING:
        return { type: ValueType.STRING, value: String(value) };
      default:
        return { type: ValueType.VOID, value: undefined };
    }
  }

  /**
   * Get the memory manager (for other VMs to access)
   */
  public getMemory(): MemoryManager {
    return this.memory;
  }

  /**
   * Reset memory (for testing)
   */
  public reset(): void {
    this.memory.stackFrames = [];
    this.memory.globals.clear();
    this.memory.heap.clear();
  }
}