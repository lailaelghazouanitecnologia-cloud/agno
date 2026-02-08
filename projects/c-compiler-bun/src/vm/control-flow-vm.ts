/**
 * ControlFlowVM - Handles control flow: if/else, while, for
 */

import { BaseMicroVM } from './base-vm.js';
import { ExecutionContext, ExecutionResult, VMInstruction } from '../core/interfaces.js';
import { VMOperation, RuntimeValue, RuntimeType } from '../core/types.js';

export class ControlFlowVM extends BaseMicroVM {
  readonly name = 'ControlFlowVM';

  constructor() {
    super();
    this.registerOperation(VMOperation.JUMP);
    this.registerOperation(VMOperation.JUMP_IF_TRUE);
    this.registerOperation(VMOperation.JUMP_IF_FALSE);
  }

  protected executeInstruction(
    instruction: VMInstruction,
    context: ExecutionContext
  ): ExecutionResult {
    const { operation, operands } = instruction;

    switch (operation) {
      case VMOperation.JUMP:
        return this.executeJump(operands, context);

      case VMOperation.JUMP_IF_TRUE:
        return this.executeJumpIfTrue(operands, context);

      case VMOperation.JUMP_IF_FALSE:
        return this.executeJumpIfFalse(operands, context);

      default:
        return {
          success: false,
          error: `Unknown control flow operation: ${operation}`,
        };
    }
  }

  private executeJump(operands: unknown[], context: ExecutionContext): ExecutionResult {
    if (operands.length < 1) {
      return {
        success: false,
        error: 'JUMP operation requires a target address',
      };
    }

    const target = typeof operands[0] === 'number' ? operands[0] : parseInt(operands[0] as string);

    return {
      success: true,
      newPc: target,
    };
  }

  private executeJumpIfTrue(operands: unknown[], context: ExecutionContext): ExecutionResult {
    if (operands.length < 2) {
      return {
        success: false,
        error: 'JUMP_IF_TRUE operation requires a condition and target address',
      };
    }

    const condition = this.getRuntimeValue(operands[0]);
    const target = typeof operands[1] === 'number' ? operands[1] : parseInt(operands[1] as string);

    if (!condition) {
      return {
        success: false,
        error: 'Invalid condition for JUMP_IF_TRUE',
      };
    }

    const isTrue = this.toBoolean(condition);

    if (isTrue) {
      return {
        success: true,
        newPc: target,
      };
    }

    return {
      success: true,
    };
  }

  private executeJumpIfFalse(operands: unknown[], context: ExecutionContext): ExecutionResult {
    if (operands.length < 2) {
      return {
        success: false,
        error: 'JUMP_IF_FALSE operation requires a condition and target address',
      };
    }

    const condition = this.getRuntimeValue(operands[0]);
    const target = typeof operands[1] === 'number' ? operands[1] : parseInt(operands[1] as string);

    if (!condition) {
      return {
        success: false,
        error: 'Invalid condition for JUMP_IF_FALSE',
      };
    }

    const isTrue = this.toBoolean(condition);

    if (!isTrue) {
      return {
        success: true,
        newPc: target,
      };
    }

    return {
      success: true,
    };
  }

  private getRuntimeValue(operand: unknown): RuntimeValue | undefined {
    if (typeof operand === 'object' && operand !== null && 'type' in operand && 'value' in operand) {
      return operand as RuntimeValue;
    }
    return undefined;
  }

  private toBoolean(value: RuntimeValue): boolean {
    if (typeof value.value === 'boolean') {
      return value.value;
    }
    if (typeof value.value === 'number') {
      return value.value !== 0;
    }
    return false;
  }
}