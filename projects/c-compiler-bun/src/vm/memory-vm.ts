/**
 * MemoryVM - Handles variables, arrays, pointers, and scoping with stack frames
 */

import { BaseVM } from './base-vm.js';
import { ASTNode, ExecutionContext, ExecutionResult, ValueType, MemoryState, StackFrame, Variable } from '../core/interfaces.js';
import { NodeType } from '../core/types.js';

/**
 * MemoryVM - handles memory and variable management
 */
export class MemoryVM extends BaseVM {
  readonly name = 'memory';

  private memory: MemoryState;

  constructor() {
    super();
    this.memory = this.createMemoryState();
  }

  initialize?(context?: unknown): void {
    this.memory = this.createMemoryState();
  }

  execute(node: ASTNode, context: ExecutionContext): ExecutionResult {
    try {
      switch (node.type) {
        case NodeType.VARIABLE_DECL:
          return this.declareVariable(node, context);

        case NodeType.ASSIGN_EXPR:
          return this.assignVariable(node, context);

        case NodeType.IDENTIFIER_EXPR:
          return this.accessVariable(node, context);

        case NodeType.ARRAY_ACCESS_EXPR:
          return this.accessArray(node, context);

        case NodeType.ADDRESS_EXPR:
          return this.getAddress(node, context);

        case NodeType.DEREFERENCE_EXPR:
          return this.dereference(node, context);

        default:
          return {
            success: false,
            error: `MemoryVM cannot handle node type: ${node.type}`,
          };
      }
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  canHandle(node: ASTNode): boolean {
    return (
      node.type === NodeType.VARIABLE_DECL ||
      node.type === NodeType.ASSIGN_EXPR ||
      node.type === NodeType.IDENTIFIER_EXPR ||
      node.type === NodeType.ARRAY_ACCESS_EXPR ||
      node.type === NodeType.ADDRESS_EXPR ||
      node.type === NodeType.DEREFERENCE_EXPR
    );
  }

  private declareVariable(node: ASTNode, context: ExecutionContext): ExecutionResult {
    const decl = node as {
      varType: string;
      name: string;
      isArray?: boolean;
      arraySize?: ASTNode;
      init?: ASTNode | null;
    };

    let initValue: ValueType = null;
    if (decl.init) {
      const initResult = this.evaluateExpression(decl.init, context);
      if (!initResult.success) {
        return initResult;
      }
      initValue = initResult.value!;
    } else {
      // Default initialization based on type
      initValue = this.getDefaultValue(decl.varType);
    }

    // Handle array declaration
    if (decl.isArray) {
      let arraySize = 0;
      if (decl.arraySize) {
        const sizeResult = this.evaluateExpression(decl.arraySize, context);
        if (!sizeResult.success) {
          return sizeResult;
        }
        arraySize = Math.floor(this.toNumber(sizeResult.value!));
      }

      // Create array with default values
      const arrayValue: ValueType[] = [];
      for (let i = 0; i < arraySize; i++) {
        arrayValue.push(this.getDefaultValue(decl.varType));
      }

      context.memory.allocate(decl.name, decl.varType + '[]', arrayValue);
    } else {
      context.memory.allocate(decl.name, decl.varType, initValue);
    }

    return { success: true };
  }

  private assignVariable(node: ASTNode, context: ExecutionContext): ExecutionResult {
    const assign = node as {
      operator: string;
      left: ASTNode;
      right: ASTNode;
    };

    // Evaluate right-hand side
    const rightResult = this.evaluateExpression(assign.right, context);
    if (!rightResult.success) {
      return rightResult;
    }
    let value = rightResult.value!;

    // Handle compound assignment
    if (assign.operator !== '=') {
      const leftResult = this.evaluateExpression(assign.left, context);
      if (!leftResult.success) {
        return leftResult;
      }
      const leftValue = leftResult.value!;

      value = this.applyCompoundAssignment(assign.operator, leftValue, value);
    }

    // Assign to left-hand side
    if (assign.left.type === NodeType.IDENTIFIER_EXPR) {
      const name = (assign.left as { name: string }).name;
      context.memory.set(name, value);
    } else if (assign.left.type === NodeType.ARRAY_ACCESS_EXPR) {
      const arrayAccess = assign.left as { array: ASTNode; index: ASTNode };
      const arrayResult = this.evaluateExpression(arrayAccess.array, context);
      const indexResult = this.evaluateExpression(arrayAccess.index, context);

      if (!arrayResult.success || !indexResult.success) {
        return arrayResult.success ? indexResult : arrayResult;
      }

      const array = arrayResult.value! as ValueType[];
      const index = Math.floor(this.toNumber(indexResult.value!));

      if (!Array.isArray(array)) {
        return {
          success: false,
          error: 'Cannot index non-array value',
        };
      }

      if (index < 0 || index >= array.length) {
        return {
          success: false,
          error: `Array index out of bounds: ${index}`,
        };
      }

      array[index] = value;
    } else if (assign.left.type === NodeType.DEREFERENCE_EXPR) {
      // Handle pointer dereference assignment
      const deref = assign.left as { operand: ASTNode };
      const ptrResult = this.evaluateExpression(deref.operand, context);

      if (!ptrResult.success) {
        return ptrResult;
      }

      const address = ptrResult.value! as string;
      // In a real implementation, we'd look up the address in memory
      // For now, this is a placeholder
      return {
        success: false,
        error: 'Pointer dereference assignment not fully implemented',
      };
    } else {
      return {
        success: false,
        error: 'Invalid assignment target',
      };
    }

    return { success: true, value };
  }

  private accessVariable(node: ASTNode, context: ExecutionContext): ExecutionResult {
    const ident = node as { name: string };
    const value = context.memory.get(ident.name);

    if (value === undefined) {
      return {
        success: false,
        error: `Undefined variable: ${ident.name}`,
      };
    }

    return {
      success: true,
      value,
    };
  }

  private accessArray(node: ASTNode, context: ExecutionContext): ExecutionResult {
    const arrayAccess = node as { array: ASTNode; index: ASTNode };

    const arrayResult = this.evaluateExpression(arrayAccess.array, context);
    const indexResult = this.evaluateExpression(arrayAccess.index, context);

    if (!arrayResult.success) {
      return arrayResult;
    }
    if (!indexResult.success) {
      return indexResult;
    }

    const array = arrayResult.value!;
    const index = Math.floor(this.toNumber(indexResult.value!));

    if (!Array.isArray(array)) {
      return {
        success: false,
        error: 'Cannot index non-array value',
      };
    }

    if (index < 0 || index >= array.length) {
      return {
        success: false,
        error: `Array index out of bounds: ${index}`,
      };
    }

    return {
      success: true,
      value: array[index],
    };
  }

  private getAddress(node: ASTNode, context: ExecutionContext): ExecutionResult {
    const addrExpr = node as { operand: ASTNode };

    if (addrExpr.operand.type !== NodeType.IDENTIFIER_EXPR) {
      return {
        success: false,
        error: 'Can only take address of identifiers',
      };
    }

    const name = (addrExpr.operand as { name: string }).name;
    const value = context.memory.get(name);

    if (value === undefined) {
      return {
        success: false,
        error: `Undefined variable: ${name}`,
      };
    }

    // Return a mock address
    const address = `&${name}`;

    return {
      success: true,
      value: address,
    };
  }

  private dereference(node: ASTNode, context: ExecutionContext): ExecutionResult {
    const derefExpr = node as { operand: ASTNode };

    const ptrResult = this.evaluateExpression(derefExpr.operand, context);
    if (!ptrResult.success) {
      return ptrResult;
    }

    const address = ptrResult.value! as string;

    // In a real implementation, we'd look up the address in memory
    // For now, extract the variable name from the address
    if (typeof address === 'string' && address.startsWith('&')) {
      const name = address.substring(1);
      const value = context.memory.get(name);

      if (value === undefined) {
        return {
          success: false,
          error: `Invalid pointer address: ${address}`,
        };
      }

      return {
        success: true,
        value,
      };
    }

    return {
      success: false,
      error: 'Invalid pointer address',
    };
  }

  private evaluateExpression(node: ASTNode, context: ExecutionContext): ExecutionResult {
    // Handle literals
    if (node.type === NodeType.INTEGER_LITERAL) {
      return { success: true, value: (node as { value: number }).value };
    }
    if (node.type === NodeType.FLOAT_LITERAL) {
      return { success: true, value: (node as { value: number }).value };
    }
    if (node.type === NodeType.CHAR_LITERAL) {
      return { success: true, value: (node as { value: string }).value };
    }
    if (node.type === NodeType.STRING_LITERAL) {
      return { success: true, value: (node as { value: string }).value };
    }

    // Handle identifiers
    if (node.type === NodeType.IDENTIFIER_EXPR) {
      return this.accessVariable(node, context);
    }

    // For other expressions, we'd need to delegate to appropriate VMs
    // This is a placeholder
    return {
      success: false,
      error: `Cannot evaluate expression node type: ${node.type}`,
    };
  }

  private getDefaultValue(type: string): ValueType {
    if (type === 'int' || type === 'float') {
      return 0;
    }
    if (type === 'char') {
      return '\0';
    }
    return null;
  }

  private toNumber(value: ValueType): number {
    if (typeof value === 'number') {
      return value;
    }
    if (typeof value === 'string') {
      const num = parseFloat(value);
      if (isNaN(num)) {
        throw new Error(`Cannot convert string to number: ${value}`);
      }
      return num;
    }
    if (typeof value === 'boolean') {
      return value ? 1 : 0;
    }
    if (value === null) {
      return 0;
    }
    throw new Error(`Cannot convert to number: ${typeof value}`);
  }

  private applyCompoundAssignment(operator: string, left: ValueType, right: ValueType): ValueType {
    const leftNum = this.toNumber(left);
    const rightNum = this.toNumber(right);

    switch (operator) {
      case '+=':
        return leftNum + rightNum;
      case '-=':
        return leftNum - rightNum;
      case '*=':
        return leftNum * rightNum;
      case '/=':
        return leftNum / rightNum;
      default:
        throw new Error(`Unknown compound assignment operator: ${operator}`);
    }
  }

  private createMemoryState(): MemoryState {
    const frames: StackFrame[] = [];
    const heap = new Map<string, ValueType>();

    // Create global frame
    frames.push({
      variables: new Map(),
      type: 'global',
    });

    return {
      frames,
      currentFrame: 0,
      heap,

      get(name: string): ValueType | undefined {
        // Search from current frame backwards
        for (let i = this.currentFrame; i >= 0; i--) {
          const frame = this.frames[i];
          const variable = frame.variables.get(name);
          if (variable) {
            return variable.value;
          }
        }
        return undefined;
      },

      set(name: string, value: ValueType): void {
        // Search from current frame backwards
        for (let i = this.currentFrame; i >= 0; i--) {
          const frame = this.frames[i];
          const variable = frame.variables.get(name);
          if (variable) {
            variable.value = value;
            return;
          }
        }
        // If not found, create in current frame
        this.getCurrentFrame().variables.set(name, {
          name,
          type: 'auto',
          value,
          isArray: false,
          isPointer: false,
        });
      },

      allocate(name: string, type: string, value?: ValueType): void {
        const frame = this.getCurrentFrame();
        frame.variables.set(name, {
          name,
          type,
          value: value !== undefined ? value : null,
          isArray: type.endsWith('[]'),
          isPointer: type.includes('*'),
        });
      },

      pushFrame(): void {
        this.frames.push({
          variables: new Map(),
          type: 'block',
          parent: this.currentFrame,
        });
        this.currentFrame = this.frames.length - 1;
      },

      popFrame(): void {
        if (this.currentFrame > 0) {
          const frame = this.frames[this.currentFrame];
          this.currentFrame = frame.parent ?? this.currentFrame - 1;
          this.frames.pop();
        }
      },

      getCurrentFrame(): StackFrame {
        return this.frames[this.currentFrame];
      },
    };
  }

  /**
   * Get the current memory state
   */
  getMemory(): MemoryState {
    return this.memory;
  }

  /**
   * Set the memory state
   */
  setMemory(memory: MemoryState): void {
    this.memory = memory;
  }
}