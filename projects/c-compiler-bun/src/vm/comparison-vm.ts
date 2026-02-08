/**
 * ComparisonVM - Handles comparison operations: <, >, <=, >=, ==, !=
 */

import { BaseMicroVM } from './base-vm.js';
import { ExecutionContext, ExecutionResult, VMInstruction } from '../core/interfaces.js';
import { VMOperation, RuntimeValue, RuntimeType } from '../core/types.js';

export class ComparisonVM extends BaseMicroVM {
  readonly name = 'ComparisonVM';

  constructor() {
    super();
    this.registerOperation(VMOperation.LESS);
    this.registerOperation(VMOperation.GREATER);
    this.registerOperation(VMOperation.LESS_EQUAL);
    this.registerOperation(VMOperation.GREATER_EQUAL);
    this.registerOperation(VMOperation.EQUAL);
    this.registerOperation(VMOperation.NOT_EQUAL);
  }

  protected executeInstruction(
    instruction: VMInstruction,
    context: ExecutionContext
  ): ExecutionResult {
    const { operation, operands } = instruction;

    if (operands.length < 2) {
      return {
        success: false,
        error: `Comparison operation requires 2 operands, got ${operands.length}`,
      };
    }

    const left = this.getRuntimeValue(operands[0]);
    const right = this.getRuntimeValue(operands[1]);

    if (!left || !right) {
      return {
        success: false,
        error: 'Invalid operands for comparison operation',
      };
    }

    const leftVal = this.toNumber(left);
    const rightVal = this.toNumber(right);

    let result: boolean;

    switch (operation) {
      case VMOperation.LESS:
        result = leftVal < rightVal;
        break;
      case VMOperation.GREATER:
        result = leftVal > rightVal;
        break;
      case VMOperation.LESS_EQUAL:
        result = leftVal <= rightVal;
        break;
      case VMOperation.GREATER_EQUAL:
        result = leftVal >= rightVal;
        break;
      case VMOperation.EQUAL:
        result = leftVal === rightVal;
        break;
      case VMOperation.NOT_EQUAL:
        result = leftVal !== rightVal;
        break;
      default:
        return {
          success: false,
          error: `Unknown comparison operation: ${operation}`,
        };
    }

    return {
      success: true,
      value: {
        type: RuntimeType.INT,
        value: result ? 1 : 0,
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