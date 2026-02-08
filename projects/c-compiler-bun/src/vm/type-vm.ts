/**
 * TypeVM - Handles type checking and casting
 */

import { BaseVM } from './base-vm.js';
import { ASTNode, ExecutionContext, ExecutionResult, ValueType } from '../core/interfaces.js';
import { NodeType, CType, TypeInfo } from '../core/types.js';

/**
 * TypeVM - handles type checking and casting
 */
export class TypeVM extends BaseVM {
  readonly name = 'type';

  private typeCache: Map<string, TypeInfo> = new Map();

  execute(node: ASTNode, context: ExecutionContext): ExecutionResult {
    try {
      // Type checking is typically done during compilation
      // At runtime, we mainly handle type casting
      return {
        success: true,
        value: null,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  canHandle(node: ASTNode): boolean {
    // TypeVM is mainly for type checking during compilation
    // At runtime, it doesn't handle specific node types
    return false;
  }

  /**
   * Check type of an expression
   */
  checkType(node: ASTNode, context: ExecutionContext): string {
    switch (node.type) {
      case NodeType.INTEGER_LITERAL:
        return 'int';

      case NodeType.FLOAT_LITERAL:
        return 'float';

      case NodeType.CHAR_LITERAL:
        return 'char';

      case NodeType.STRING_LITERAL:
        return 'char*';

      case NodeType.IDENTIFIER_EXPR:
        const name = (node as { name: string }).name;
        const value = context.memory.get(name);
        if (value === undefined) {
          return 'unknown';
        }
        return this.inferType(value);

      case NodeType.BINARY_EXPR:
        // For binary expressions, we'd need to check operand types
        // This is a simplified version
        return 'int';

      case NodeType.UNARY_EXPR:
        // For unary expressions, we'd need to check operand type
        return 'int';

      case NodeType.CALL_EXPR:
        // For function calls, we'd need to look up the function signature
        return 'int';

      case NodeType.ARRAY_ACCESS_EXPR:
        // Array access returns the element type
        return 'int';

      default:
        return 'unknown';
    }
  }

  /**
   * Check if types are compatible
   */
  areCompatible(type1: string, type2: string): boolean {
    // Same types are compatible
    if (type1 === type2) {
      return true;
    }

    // Numeric types are compatible
    const numericTypes = ['int', 'float', 'char'];
    if (numericTypes.includes(type1) && numericTypes.includes(type2)) {
      return true;
    }

    // Pointer compatibility
    if (type1.endsWith('*') && type2 === 'void') {
      return true;
    }
    if (type2.endsWith('*') && type1 === 'void') {
      return true;
    }

    return false;
  }

  /**
   * Perform type cast
   */
  cast(value: ValueType, fromType: string, toType: string): ValueType {
    // Same type, no cast needed
    if (fromType === toType) {
      return value;
    }

    // Cast to/from numeric types
    if (this.isNumericType(toType)) {
      return this.castToNumber(value, fromType);
    }

    if (this.isNumericType(fromType) && toType === 'char') {
      const num = this.castToNumber(value, fromType);
      return String.fromCharCode(Math.floor(num));
    }

    // Pointer casts
    if (toType.endsWith('*') && fromType.endsWith('*')) {
      return value; // Just pass through for pointer casts
    }

    throw new Error(`Cannot cast from ${fromType} to ${toType}`);
  }

  /**
   * Infer type from value
   */
  inferType(value: ValueType): string {
    if (value === null) {
      return 'void';
    }
    if (typeof value === 'number') {
      return Number.isInteger(value) ? 'int' : 'float';
    }
    if (typeof value === 'string') {
      return 'char';
    }
    if (typeof value === 'boolean') {
      return 'int';
    }
    if (Array.isArray(value)) {
      return 'int[]';
    }
    return 'unknown';
  }

  /**
   * Parse type string into TypeInfo
   */
  parseType(typeStr: string): TypeInfo {
    // Check cache
    if (this.typeCache.has(typeStr)) {
      return this.typeCache.get(typeStr)!;
    }

    const info: TypeInfo = {
      base: this.getBaseType(typeStr),
      isPointer: typeStr.includes('*'),
      isArray: typeStr.endsWith('[]'),
    };

    if (info.isArray) {
      // Could parse array dimensions here
      info.arrayDimensions = [];
    }

    if (info.isPointer) {
      info.pointerDepth = (typeStr.match(/\*/g) || []).length;
    }

    // Cache the result
    this.typeCache.set(typeStr, info);

    return info;
  }

  /**
   * Get common type for binary operations
   */
  getCommonType(type1: string, type2: string): string {
    // If types are the same, return that type
    if (type1 === type2) {
      return type1;
    }

    // If one is float, result is float
    if (type1 === 'float' || type2 === 'float') {
      return 'float';
    }

    // If one is int, result is int
    if (type1 === 'int' || type2 === 'int') {
      return 'int';
    }

    // Default to int
    return 'int';
  }

  private isNumericType(type: string): boolean {
    return ['int', 'float', 'char', 'long', 'short', 'double'].includes(type);
  }

  private castToNumber(value: ValueType, fromType: string): number {
    if (typeof value === 'number') {
      return value;
    }

    if (typeof value === 'string') {
      if (fromType === 'char') {
        return value.charCodeAt(0);
      }
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

    throw new Error(`Cannot cast to number from type: ${typeof value}`);
  }

  private getBaseType(typeStr: string): CType {
    // Remove pointer and array qualifiers
    const base = typeStr.replace(/\*/g, '').replace(/\[\]/g, '').trim();

    switch (base) {
      case 'int':
        return CType.INT;
      case 'char':
        return CType.CHAR;
      case 'float':
        return CType.FLOAT;
      case 'void':
        return CType.VOID;
      case 'double':
        return CType.FLOAT;
      case 'long':
        return CType.INT;
      case 'short':
        return CType.INT;
      default:
        return CType.VOID;
    }
  }

  /**
   * Clear type cache
   */
  clearCache(): void {
    this.typeCache.clear();
  }
}