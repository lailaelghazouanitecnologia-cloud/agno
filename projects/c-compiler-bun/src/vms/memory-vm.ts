/**
 * MemoryVM
 * Handles variables, arrays, pointers, and scoping with stack frames
 */

import { BaseMicroVM } from './interfaces.js';
import { ASTNode, NodeType, ExecutionContext, RuntimeValue, ValueType, TypeInfo, CType } from './types.js';

export class MemoryVM extends BaseMicroVM {
  readonly name = 'MemoryVM';

  canHandle(node: ASTNode): boolean {
    return (
      node.type === NodeType.VAR_DECL ||
      node.type === NodeType.ARRAY_DECL ||
      node.type === NodeType.ASSIGN_EXPR ||
      node.type === NodeType.IDENTIFIER_EXPR ||
      node.type === NodeType.ARRAY_ACCESS_EXPR ||
      node.type === NodeType.ADDRESS_OF_EXPR ||
      node.type === NodeType.POINTER_DEREF_EXPR
    );
  }

  execute(node: ASTNode, context: ExecutionContext): RuntimeValue | void {
    switch (node.type) {
      case NodeType.VAR_DECL:
        return this.executeVarDecl(node as any, context);
      
      case NodeType.ARRAY_DECL:
        return this.executeArrayDecl(node as any, context);
      
      case NodeType.ASSIGN_EXPR:
        return this.executeAssign(node as any, context);
      
      case NodeType.IDENTIFIER_EXPR:
        return this.executeIdentifier(node as any, context);
      
      case NodeType.ARRAY_ACCESS_EXPR:
        return this.executeArrayAccess(node as any, context);
      
      case NodeType.ADDRESS_OF_EXPR:
        return this.executeAddressOf(node as any, context);
      
      case NodeType.POINTER_DEREF_EXPR:
        return this.executePointerDeref(node as any, context);
      
      default:
        throw new Error(`MemoryVM cannot handle node type: ${node.type}`);
    }
  }

  /**
   * Execute a variable declaration
   */
  private executeVarDecl(node: any, context: ExecutionContext): void {
    let value: RuntimeValue;

    if (node.init) {
      value = this.evaluateExpression(node.init, context);
    } else {
      // Default initialization based on type
      value = this.getDefaultValue(node.varType);
    }

    this.setVariable(node.name, value, context);
  }

  /**
   * Execute an array declaration
   */
  private executeArrayDecl(node: any, context: ExecutionContext): void {
    // Evaluate array size
    const sizeValue = this.evaluateExpression(node.size, context);
    const size = this.toNumber(sizeValue);

    // Create array value
    const arrayValue: RuntimeValue = {
      type: ValueType.ARRAY,
      value: new Array(size).fill(this.getDefaultValue(node.elementType))
    };

    this.setVariable(node.name, arrayValue, context);
  }

  /**
   * Execute an assignment
   */
  private executeAssign(node: any, context: ExecutionContext): RuntimeValue {
    const value = this.evaluateExpression(node.value, context);

    if (node.target.type === NodeType.IDENTIFIER_EXPR) {
      this.setVariable(node.target.name, value, context);
    } else if (node.target.type === NodeType.ARRAY_ACCESS_EXPR) {
      const array = this.evaluateExpression(node.target.array, context);
      const index = this.evaluateExpression(node.target.index, context);
      
      if (array.type !== ValueType.ARRAY) {
        throw new Error('Cannot assign to non-array');
      }
      
      const idx = this.toNumber(index);
      if (idx < 0 || idx >= array.value.length) {
        throw new Error('Array index out of bounds');
      }
      
      array.value[idx] = value;
    } else if (node.target.type === NodeType.POINTER_DEREF_EXPR) {
      const pointer = this.evaluateExpression(node.target.operand, context);
      
      if (pointer.type !== ValueType.POINTER) {
        throw new Error('Cannot dereference non-pointer');
      }
      
      const address = pointer.value as number;
      const heapValue = context.heap.get(address);
      
      if (!heapValue) {
        throw new Error('Invalid pointer address');
      }
      
      // Update heap value
      heapValue.value = value.value;
      context.heap.set(address, heapValue);
    } else {
      throw new Error(`Cannot assign to target type: ${node.target.type}`);
    }

    return value;
  }

  /**
   * Execute an identifier expression
   */
  private executeIdentifier(node: any, context: ExecutionContext): RuntimeValue {
    const value = this.getVariable(node.name, context);
    
    if (!value) {
      throw new Error(`Undefined variable: ${node.name}`);
    }
    
    return value;
  }

  /**
   * Execute an array access
   */
  private executeArrayAccess(node: any, context: ExecutionContext): RuntimeValue {
    const array = this.evaluateExpression(node.array, context);
    const index = this.evaluateExpression(node.index, context);
    
    if (array.type !== ValueType.ARRAY) {
      throw new Error('Cannot index non-array');
    }
    
    const idx = this.toNumber(index);
    if (idx < 0 || idx >= array.value.length) {
      throw new Error('Array index out of bounds');
    }
    
    return array.value[idx];
  }

  /**
   * Execute address-of (&)
   */
  private executeAddressOf(node: any, context: ExecutionContext): RuntimeValue {
    if (node.operand.type !== NodeType.IDENTIFIER_EXPR) {
      throw new Error('Can only take address of identifier');
    }
    
    const varName = node.operand.name;
    
    // Create a fake address based on variable name
    // In a real implementation, this would be the actual memory address
    const address = this.hashString(varName);
    
    // Store the variable in the heap if not already there
    const varValue = this.getVariable(varName, context);
    if (varValue && !context.heap.has(address)) {
      context.heap.set(address, { ...varValue });
    }
    
    return this.createValue(ValueType.POINTER, address);
  }

  /**
   * Execute pointer dereference (*)
   */
  private executePointerDeref(node: any, context: ExecutionContext): RuntimeValue {
    const pointer = this.evaluateExpression(node.operand, context);
    
    if (pointer.type !== ValueType.POINTER) {
      throw new Error('Cannot dereference non-pointer');
    }
    
    const address = pointer.value as number;
    const heapValue = context.heap.get(address);
    
    if (!heapValue) {
      throw new Error('Invalid pointer address');
    }
    
    return heapValue;
  }

  /**
   * Evaluate an expression
   */
  private evaluateExpression(expr: any, context: ExecutionContext): RuntimeValue {
    switch (expr.type) {
      case NodeType.INT_LITERAL:
        return this.createValue(ValueType.INT, expr.value);
      
      case NodeType.FLOAT_LITERAL:
        return this.createValue(ValueType.FLOAT, expr.value);
      
      case NodeType.CHAR_LITERAL:
        return this.createValue(ValueType.CHAR, expr.value.charCodeAt(0));
      
      case NodeType.STRING_LITERAL:
        return this.createValue(ValueType.POINTER, expr.value);
      
      case NodeType.IDENTIFIER_EXPR:
        return this.executeIdentifier(expr, context);
      
      case NodeType.ARRAY_ACCESS_EXPR:
        return this.executeArrayAccess(expr, context);
      
      case NodeType.ADDRESS_OF_EXPR:
        return this.executeAddressOf(expr, context);
      
      case NodeType.POINTER_DEREF_EXPR:
        return this.executePointerDeref(expr, context);
      
      case NodeType.BINARY_EXPR:
        return this.evaluateBinaryExpr(expr, context);
      
      case NodeType.UNARY_EXPR:
        return this.evaluateUnaryExpr(expr, context);
      
      case NodeType.CALL_EXPR:
        // This would be handled by FunctionVM
        throw new Error('Function calls not supported in this context');
      
      default:
        throw new Error(`Cannot evaluate expression of type: ${expr.type}`);
    }
  }

  /**
   * Evaluate a binary expression
   */
  private evaluateBinaryExpr(expr: any, context: ExecutionContext): RuntimeValue {
    const left = this.evaluateExpression(expr.left, context);
    const right = this.evaluateExpression(expr.right, context);

    // This would be handled by ArithmeticVM or ComparisonVM
    // For simplicity, we'll do basic evaluation
    const leftNum = this.toNumber(left);
    const rightNum = this.toNumber(right);

    // Check if it's a comparison operator
    const isComparison = 
      expr.operator === '<' ||
      expr.operator === '>' ||
      expr.operator === '<=' ||
      expr.operator === '>=' ||
      expr.operator === '==' ||
      expr.operator === '!=';

    if (isComparison) {
      let result: boolean;
      switch (expr.operator) {
        case '<': result = leftNum < rightNum; break;
        case '>': result = leftNum > rightNum; break;
        case '<=': result = leftNum <= rightNum; break;
        case '>=': result = leftNum >= rightNum; break;
        case '==': result = leftNum === rightNum; break;
        case '!=': result = leftNum !== rightNum; break;
        default: result = false;
      }
      return this.createValue(ValueType.INT, result ? 1 : 0);
    }

    // Arithmetic operation
    const useFloat = left.type === ValueType.FLOAT || right.type === ValueType.FLOAT;
    let result: number;

    switch (expr.operator) {
      case '+': result = leftNum + rightNum; break;
      case '-': result = leftNum - rightNum; break;
      case '*': result = leftNum * rightNum; break;
      case '/': result = leftNum / rightNum; break;
      case '%': result = leftNum % rightNum; break;
      default: throw new Error(`Unknown operator: ${expr.operator}`);
    }

    if (useFloat) {
      return this.createValue(ValueType.FLOAT, result);
    } else {
      return this.createValue(ValueType.INT, Math.trunc(result));
    }
  }

  /**
   * Evaluate a unary expression
   */
  private evaluateUnaryExpr(expr: any, context: ExecutionContext): RuntimeValue {
    const operand = this.evaluateExpression(expr.operand, context);

    if (expr.operator === '-') {
      if (operand.type === ValueType.INT) {
        return this.createValue(ValueType.INT, -operand.value);
      } else if (operand.type === ValueType.FLOAT) {
        return this.createValue(ValueType.FLOAT, -operand.value);
      }
    }

    throw new Error(`Unsupported unary operator: ${expr.operator}`);
  }

  /**
   * Get default value for a type
   */
  private getDefaultValue(typeInfo: TypeInfo): RuntimeValue {
    switch (typeInfo.base) {
      case CType.INT:
        return this.createValue(ValueType.INT, 0);
      
      case CType.FLOAT:
        return this.createValue(ValueType.FLOAT, 0.0);
      
      case CType.CHAR:
        return this.createValue(ValueType.CHAR, 0);
      
      case VOID:
        return this.createValue(ValueType.VOID, undefined);
      
      default:
        return this.createValue(ValueType.INT, 0);
    }
  }

  /**
   * Convert a runtime value to a number
   */
  private toNumber(value: RuntimeValue): number {
    switch (value.type) {
      case ValueType.INT:
      case ValueType.FLOAT:
      case ValueType.CHAR:
        return Number(value.value);
      default:
        throw new Error(`Cannot convert ${value.type} to number`);
    }
  }

  /**
   * Hash a string to create a fake address
   */
  private hashString(str: string): number {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      const char = str.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash; // Convert to 32bit integer
    }
    return Math.abs(hash);
  }
}