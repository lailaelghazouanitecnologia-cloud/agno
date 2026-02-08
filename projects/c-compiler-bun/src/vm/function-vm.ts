/**
 * FunctionVM - Handles function definitions, calls, returns, and call stack
 */

import { BaseMicroVM } from './base-vm.js';
import { ExecutionContext, ExecutionResult, VMInstruction, CallStackFrame } from '../core/interfaces.js';
import { VMOperation, RuntimeValue, RuntimeType } from '../core/types.js';

export class FunctionVM extends BaseMicroVM {
  readonly name = 'FunctionVM';

  constructor() {
    super();
    this.registerOperation(VMOperation.CALL);
    this.registerOperation(VMOperation.RET);
  }

  protected executeInstruction(
    instruction: VMInstruction,
    context: ExecutionContext
  ): ExecutionResult {
    const { operation, operands } = instruction;

    switch (operation) {
      case VMOperation.CALL:
        return this.executeCall(operands, context);

      case VMOperation.RET:
        return this.executeReturn(operands, context);

      default:
        return {
          success: false,
          error: `Unknown function operation: ${operation}`,
        };
    }
  }

  private executeCall(operands: unknown[], context: ExecutionContext): ExecutionResult {
    if (operands.length < 1) {
      return {
        success: false,
        error: 'CALL operation requires a function name',
      };
    }

    const functionName = operands[0] as string;
    const args = operands.slice(1) as RuntimeValue[];

    // Create new stack frame
    const frame: CallStackFrame = {
      functionName,
      returnAddress: context.pc + 1,
      locals: new Map(),
      params: new Map(),
      baseAddress: context.callStack.length * 1000, // Simple address calculation
    };

    // Store arguments as parameters
    args.forEach((arg, index) => {
      frame.params.set(`arg${index}`, arg);
    });

    // Push frame onto call stack
    context.callStack.push(frame);
    context.currentFunction = functionName;

    // For built-in functions, handle them specially
    if (functionName === 'main') {
      // main function - just continue execution
      return {
        success: true,
      };
    }

    // For other functions, we would need to look up their address
    // For now, just continue
    return {
      success: true,
    };
  }

  private executeReturn(operands: unknown[], context: ExecutionContext): ExecutionResult {
    // Get return value if provided
    let returnValue: RuntimeValue | undefined;
    if (operands.length > 0) {
      returnValue = this.getRuntimeValue(operands[0]);
    }

    // Pop current stack frame
    const currentFrame = context.callStack.pop();
    if (!currentFrame) {
      // Returning from main - stop execution
      context.shouldStop = true;
      context.returnValue = returnValue;

      return {
        success: true,
        shouldStop: true,
        value: returnValue,
      };
    }

    // Set return value in context
    context.returnValue = returnValue;

    // Get previous frame
    const previousFrame = context.callStack[context.callStack.length - 1];
    if (previousFrame) {
      context.currentFunction = previousFrame.functionName;
    } else {
      context.currentFunction = undefined;
    }

    // Jump back to return address
    return {
      success: true,
      newPc: currentFrame.returnAddress,
      value: returnValue,
    };
  }

  private getRuntimeValue(operand: unknown): RuntimeValue | undefined {
    if (typeof operand === 'object' && operand !== null && 'type' in operand && 'value' in operand) {
      return operand as RuntimeValue;
    }
    return undefined;
  }
}