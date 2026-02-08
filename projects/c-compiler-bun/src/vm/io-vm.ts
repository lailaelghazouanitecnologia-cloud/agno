/**
 * IOVM - Handles printf and scanf I/O operations
 */

import { BaseMicroVM } from './base-vm.js';
import { ExecutionContext, ExecutionResult, VMInstruction, IOHandler } from '../core/interfaces.js';
import { VMOperation, RuntimeValue, RuntimeType } from '../core/types.js';

export class IOVM extends BaseMicroVM implements IOHandler {
  readonly name = 'IOVM';

  private outputBuffer: string[] = [];
  private inputBuffer: string[] = [];
  private inputIndex = 0;

  constructor() {
    super();
    this.registerOperation(VMOperation.PRINT);
    this.registerOperation(VMOperation.READ);
  }

  protected executeInstruction(
    instruction: VMInstruction,
    context: ExecutionContext
  ): ExecutionResult {
    const { operation, operands } = instruction;

    switch (operation) {
      case VMOperation.PRINT:
        return this.executePrint(operands, context);

      case VMOperation.READ:
        return this.executeRead(operands, context);

      default:
        return {
          success: false,
          error: `Unknown I/O operation: ${operation}`,
        };
    }
  }

  private executePrint(operands: unknown[], context: ExecutionContext): ExecutionResult {
    if (operands.length < 1) {
      return {
        success: false,
        error: 'PRINT operation requires a value',
      };
    }

    const value = this.getRuntimeValue(operands[0]);
    if (!value) {
      return {
        success: false,
        error: 'Invalid value for PRINT operation',
      };
    }

    this.print(value);

    return {
      success: true,
    };
  }

  private executeRead(operands: unknown[], context: ExecutionContext): ExecutionResult {
    const value = this.read();

    if (!value) {
      return {
        success: false,
        error: 'No input available for READ operation',
      };
    }

    return {
      success: true,
      value,
    };
  }

  // IOHandler interface implementation

  print(value: RuntimeValue): void {
    let output: string;
    switch (value.type) {
      case RuntimeType.INT:
        output = Math.trunc(Number(value.value)).toString();
        break;
      case RuntimeType.FLOAT:
        output = Number(value.value).toString();
        break;
      case RuntimeType.CHAR:
        if (typeof value.value === 'string') {
          output = value.value;
        } else {
          output = String.fromCharCode(Number(value.value));
        }
        break;
      case RuntimeType.POINTER:
        output = `0x${Number(value.value).toString(16)}`;
        break;
      default:
        output = String(value.value);
    }
    this.outputBuffer.push(output);
  }

  read(): RuntimeValue | undefined {
    if (this.inputIndex >= this.inputBuffer.length) {
      return undefined;
    }

    const input = this.inputBuffer[this.inputIndex++];
    const parsed = parseInt(input, 10);

    if (isNaN(parsed)) {
      return {
        type: RuntimeType.CHAR,
        value: input,
      };
    }

    return {
      type: RuntimeType.INT,
      value: parsed,
    };
  }

  getOutput(): string[] {
    return [...this.outputBuffer];
  }

  clearOutput(): void {
    this.outputBuffer = [];
  }

  setInput(input: string[]): void {
    this.inputBuffer = input;
    this.inputIndex = 0;
  }

  reset(): void {
    this.outputBuffer = [];
    this.inputBuffer = [];
    this.inputIndex = 0;
  }

  private getRuntimeValue(operand: unknown): RuntimeValue | undefined {
    if (typeof operand === 'object' && operand !== null && 'type' in operand && 'value' in operand) {
      return operand as RuntimeValue;
    }
    return undefined;
  }
}