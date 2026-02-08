/**
 * MemoryVM - Handles variables, arrays, pointers, and scoping with stack frames
 */

import { BaseMicroVM } from './base-vm.js';
import { ExecutionContext, ExecutionResult, VMInstruction, Memory, CallStackFrame } from '../core/interfaces.js';
import { VMOperation, RuntimeValue, RuntimeType } from '../core/types.js';

export class MemoryVM extends BaseMicroVM implements Memory {
  readonly name = 'MemoryVM';

  private heap: Map<number, RuntimeValue> = new Map();
  private stack: RuntimeValue[] = [];
  private frames: CallStackFrame[] = [];
  private nextAddress = 1000;

  constructor() {
    super();
    this.registerOperation(VMOperation.LOAD);
    this.registerOperation(VMOperation.STORE);
    this.registerOperation(VMOperation.ALLOCATE);
    this.registerOperation(VMOperation.PUSH);
    this.registerOperation(VMOperation.POP);
  }

  protected executeInstruction(
    instruction: VMInstruction,
    context: ExecutionContext
  ): ExecutionResult {
    const { operation, operands } = instruction;

    switch (operation) {
      case VMOperation.LOAD:
        return this.executeLoad(operands, context);

      case VMOperation.STORE:
        return this.executeStore(operands, context);

      case VMOperation.ALLOCATE:
        return this.executeAllocate(operands, context);

      case VMOperation.PUSH:
        return this.executePush(operands, context);

      case VMOperation.POP:
        return this.executePop(context);

      default:
        return {
          success: false,
          error: `Unknown memory operation: ${operation}`,
        };
    }
  }

  private executeLoad(operands: unknown[], context: ExecutionContext): ExecutionResult {
    if (operands.length < 1) {
      return {
        success: false,
        error: 'LOAD operation requires an address or variable name',
      };
    }

    const addressOrName = operands[0];
    let value: RuntimeValue | undefined;

    if (typeof addressOrName === 'number') {
      value = this.get(addressOrName);
    } else if (typeof addressOrName === 'string') {
      value = this.getVariable(addressOrName);
    }

    if (!value) {
      return {
        success: false,
        error: `Cannot load from address: ${addressOrName}`,
      };
    }

    return {
      success: true,
      value,
    };
  }

  private executeStore(operands: unknown[], context: ExecutionContext): ExecutionResult {
    if (operands.length < 2) {
      return {
        success: false,
        error: 'STORE operation requires an address and value',
      };
    }

    const addressOrName = operands[0];
    const value = this.getRuntimeValue(operands[1]);

    if (!value) {
      return {
        success: false,
        error: 'Invalid value for STORE operation',
      };
    }

    if (typeof addressOrName === 'number') {
      this.set(addressOrName, value);
    } else if (typeof addressOrName === 'string') {
      this.setVariable(addressOrName, value);
    }

    return {
      success: true,
    };
  }

  private executeAllocate(operands: unknown[], context: ExecutionContext): ExecutionResult {
    if (operands.length < 2) {
      return {
        success: false,
        error: 'ALLOCATE operation requires a name and size',
      };
    }

    const name = operands[0] as string;
    const type = operands[1] as string;
    const size = typeof operands[2] === 'number' ? operands[2] : 1;

    const address = this.allocate(name, type, size);

    return {
      success: true,
      value: {
        type: RuntimeType.POINTER,
        value: address,
      },
    };
  }

  private executePush(operands: unknown[], context: ExecutionContext): ExecutionResult {
    if (operands.length < 1) {
      return {
        success: false,
        error: 'PUSH operation requires a value',
      };
    }

    const value = this.getRuntimeValue(operands[0]);
    if (!value) {
      return {
        success: false,
        error: 'Invalid value for PUSH operation',
      };
    }

    this.push(value);

    return {
      success: true,
    };
  }

  private executePop(context: ExecutionContext): ExecutionResult {
    const value = this.pop();

    if (!value) {
      return {
        success: false,
        error: 'Cannot pop from empty stack',
      };
    }

    return {
      success: true,
      value,
    };
  }

  // Memory interface implementation

  allocate(name: string, type: string, size: number): number {
    const address = this.nextAddress;
    this.nextAddress += size * 4; // Assume 4 bytes per element

    // Store in current frame's locals
    const currentFrame = this.getCurrentFrame();
    if (currentFrame) {
      currentFrame.locals.set(name, {
        type: this.mapTypeToRuntimeType(type),
        value: address,
      });
    }

    return address;
  }

  get(address: number): RuntimeValue | undefined {
    return this.heap.get(address);
  }

  set(address: number, value: RuntimeValue): void {
    this.heap.set(address, value);
  }

  getVariable(name: string): RuntimeValue | undefined {
    // Check current frame first
    const currentFrame = this.getCurrentFrame();
    if (currentFrame) {
      if (currentFrame.locals.has(name)) {
        return currentFrame.locals.get(name);
      }
      if (currentFrame.params.has(name)) {
        return currentFrame.params.get(name);
      }
    }

    // Check other frames (for closures)
    for (let i = this.frames.length - 1; i >= 0; i--) {
      const frame = this.frames[i];
      if (frame.locals.has(name)) {
        return frame.locals.get(name);
      }
      if (frame.params.has(name)) {
        return frame.params.get(name);
      }
    }

    return undefined;
  }

  setVariable(name: string, value: RuntimeValue): void {
    const currentFrame = this.getCurrentFrame();
    if (currentFrame) {
      currentFrame.locals.set(name, value);
    }
  }

  push(value: RuntimeValue): void {
    this.stack.push(value);
  }

  pop(): RuntimeValue | undefined {
    return this.stack.pop();
  }

  peek(): RuntimeValue | undefined {
    return this.stack[this.stack.length - 1];
  }

  pushFrame(frameName: string): void {
    const frame: CallStackFrame = {
      functionName: frameName,
      returnAddress: 0,
      locals: new Map(),
      params: new Map(),
      baseAddress: this.nextAddress,
    };
    this.frames.push(frame);
  }

  popFrame(): void {
    this.frames.pop();
  }

  getCurrentFrame(): CallStackFrame | undefined {
    return this.frames[this.frames.length - 1];
  }

  reset(): void {
    this.heap.clear();
    this.stack = [];
    this.frames = [];
    this.nextAddress = 1000;
  }

  private getRuntimeValue(operand: unknown): RuntimeValue | undefined {
    if (typeof operand === 'object' && operand !== null && 'type' in operand && 'value' in operand) {
      return operand as RuntimeValue;
    }
    return undefined;
  }

  private mapTypeToRuntimeType(type: string): RuntimeType {
    switch (type) {
      case 'int':
        return RuntimeType.INT;
      case 'float':
        return RuntimeType.FLOAT;
      case 'char':
        return RuntimeType.CHAR;
      case 'pointer':
        return RuntimeType.POINTER;
      case 'array':
        return RuntimeType.ARRAY;
      default:
        return RuntimeType.VOID;
    }
  }
}