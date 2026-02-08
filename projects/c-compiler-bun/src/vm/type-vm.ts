/**
 * TypeVM - Handles type checking and casting operations
 */

import { BaseMicroVM } from './base-vm.js';
import { ExecutionContext, ExecutionResult, VMInstruction, TypeChecker } from '../core/interfaces.js';
import { VMOperation, RuntimeValue, RuntimeType, CType, ASTNode } from '../core/types.js';

export class TypeVM extends BaseMicroVM implements TypeChecker {
  readonly name = 'TypeVM';

  constructor() {
    super();
    this.registerOperation(VMOperation.CAST);
  }

  protected executeInstruction(
    instruction: VMInstruction,
    context: ExecutionContext
  ): ExecutionResult {
    const { operation, operands } = instruction;

    switch (operation) {
      case VMOperation.CAST:
        return this.executeCast(operands, context);

      default:
        return {
          success: false,
          error: `Unknown type operation: ${operation}`,
        };
    }
  }

  private executeCast(operands: unknown[], context: ExecutionContext): ExecutionResult {
    if (operands.length < 2) {
      return {
        success: false,
        error: 'CAST operation requires a value and target type',
      };
    }

    const value = this.getRuntimeValue(operands[0]);
    const targetType = operands[1] as string;

    if (!value) {
      return {
        success: false,
        error: 'Invalid value for CAST operation',
      };
    }

    const castedValue = this.castValue(value, targetType);

    return {
      success: true,
      value: castedValue,
    };
  }

  // TypeChecker interface implementation

  check(node: ASTNode): boolean {
    // TODO: Implement type checking
    return true;
  }

  getType(node: ASTNode): string | undefined {
    // TODO: Implement type inference
    return undefined;
  }

  isCompatible(type1: string, type2: string): boolean {
    // Same types are always compatible
    if (type1 === type2) return true;

    // Numeric types are compatible
    const numericTypes = [CType.INT, CType.FLOAT, CType.CHAR];
    if (numericTypes.includes(type1 as CType) && numericTypes.includes(type2 as CType)) {
      return true;
    }

    // Void is compatible with nothing
    if (type1 === CType.VOID || type2 === CType.VOID) {
      return false;
    }

    return false;
  }

  reset(): void {
    // No state to reset
  }

  private castValue(value: RuntimeValue, targetType: string): RuntimeValue {
    const targetRuntimeType = this.mapCTypeToRuntimeType(targetType);
    const numericValue = this.toNumber(value);

    switch (targetRuntimeType) {
      case RuntimeType.INT:
        return {
          type: RuntimeType.INT,
          value: Math.trunc(numericValue),
        };
      case RuntimeType.FLOAT:
        return {
          type: RuntimeType.FLOAT,
          value: numericValue,
        };
      case RuntimeType.CHAR:
        return {
          type: RuntimeType.CHAR,
          value: String.fromCharCode(Math.trunc(numericValue) % 256),
        };
      default:
        return value;
    }
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

  private mapCTypeToRuntimeType(cType: string): RuntimeType {
    switch (cType) {
      case CType.INT:
        return RuntimeType.INT;
      case CType.FLOAT:
        return RuntimeType.FLOAT;
      case CType.CHAR:
        return RuntimeType.CHAR;
      case CType.VOID:
        return RuntimeType.VOID;
      case CType.POINTER:
        return RuntimeType.POINTER;
      case CType.ARRAY:
        return RuntimeType.ARRAY;
      default:
        return RuntimeType.VOID;
    }
  }
}