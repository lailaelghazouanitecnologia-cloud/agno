/**
 * ArithmeticVM - Handles arithmetic operations: +, -, *, /, %
 */

import { BaseMicroVM } from './base-vm.js';
import { ExecutionContext, ExecutionResult, VMInstruction } from '../core/interfaces.js';
import { VMOperation, RuntimeValue, RuntimeType } from '../core/types.js';

export class ArithmeticVM extends BaseMicroVM {
  readonly name = 'ArithmeticVM';

  constructor() {
    super();
    this.registerOperation(VMOperation.ADD);
    this.registerOperation(VMOperation.SUB);
    this.registerOperation(VMOperation.MUL);
    this.registerOperation(VMOperation.DIV);
    this.registerOperation(VMOperation.MOD);
  }

  protected executeInstruction(
    instruction: VMInstruction,
    context: ExecutionContext
  ): ExecutionResult {
    const { operation, operands } = instruction;

    if (operands.length < 2) {
      return {
        success: false,
        error: `Arithmetic operation requires 2 operands, got ${operands.length}`,
      };
    }

    const left = this.getRuntimeValue(operands[0]);
    const right = this.getRuntimeValue(operands[1]);

    if (!left || !right) {
      return {
        success: false,
        error: 'Invalid operands for arithmetic operation',
      };
    }

    let result: number;
    let resultType: RuntimeType;

    // Determine result type (float if either operand is float)
    if (left.type === RuntimeType.FLOAT || right.type === RuntimeType.FLOAT) {
      resultType = RuntimeType.FLOAT;
    } else {
      resultType = RuntimeType.INT;
    }

    const leftVal = this.toNumber(left);
    const rightVal = this.toNumber(right);

    switch (operation) {
      case VMOperation.ADD:
        result = leftVal + rightVal;
        break;
      case VMOperation.SUB:
        result = leftVal - rightVal;
        break;
      case VMOperation.MUL:
        result = leftVal * rightVal;
        break;
      case VMOperation.DIV:
        if (rightVal === 0) {
          return {
            success: false,
            error: 'Division by zero',
          };
        }
        result = leftVal / rightVal;
        break;
      case VMOperation.MOD:
        if (rightVal === 0) {
          return {
            success: false,
            error: 'Modulo by zero',
          };
        }
        result = leftVal % rightVal;
        break;
      default:
        return {
          success: false,
          error: `Unknown arithmetic operation: ${operation}`,
        };
    }

    return {
      success: true,
      value: {
        type: resultType,
        value: result,
      },
    };
  }

  private getRuntimeValue(operand: unknown): RuntimeValue | undefined {
    if (typeof operand === 'object' && operand !== null && 'type' in operand && 'value' in operand) {
      return operand as RuntimeValue;
    }
    return undefined;
  }

  private toNumber(value: RuntimeValue): number {
    if (typeof value.value === 'number') {
      return value.value;
    }
    if (typeof value.value === 'string') {
      return parseFloat(value.value);
    }
    return 0;
  }
}