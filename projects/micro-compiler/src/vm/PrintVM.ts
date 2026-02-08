import {
  MicroVM,
  ASTNode,
  ExecutionContext,
  ExecutionResult
} from '../types/MicroVM';
import { PrintStatement } from '../types/AST';
import { createValue, Value } from '../types/Value';
import { getVariable } from '../types/Scope';

/**
 * PrintVM - Handles print() statements for output
 */
export class PrintVM implements MicroVM {
  getName(): string {
    return 'PrintVM';
  }

  canHandle(node: ASTNode): boolean {
    return node.type === 'PrintStatement';
  }

  execute(node: ASTNode, context: ExecutionContext): ExecutionResult {
    if (node.type !== 'PrintStatement') {
      return {
        error: new Error(`PrintVM cannot handle node type: ${node.type}`)
      };
    }

    return this.executePrintStatement(node as PrintStatement, context);
  }

  /**
   * Execute a print statement
   */
  private executePrintStatement(
    node: PrintStatement,
    context: ExecutionContext
  ): ExecutionResult {
    // Evaluate the expression to print
    const valueResult = this.evaluateExpression(node.expression, context);

    if (valueResult.error) {
      return valueResult;
    }

    const value = valueResult.value;

    if (value === null || value === undefined) {
      return {
        error: new Error('Failed to evaluate print expression')
      };
    }

    // Convert value to string for output
    let output: string;
    switch (value.type) {
      case 'NUMBER':
        output = String(value.data);
        break;
      case 'STRING':
        output = value.data;
        break;
      case 'BOOLEAN':
        output = value.data ? 'true' : 'false';
        break;
      case 'NULL':
        output = 'null';
        break;
      case 'UNDEFINED':
        output = 'undefined';
        break;
      default:
        output = String(value.data);
    }

    // Add to output
    context.output.push(output);

    return { value };
  }

  /**
   * Evaluate an expression
   */
  private evaluateExpression(expr: any, context: ExecutionContext): ExecutionResult {
    if (expr.type === 'LiteralExpression') {
      return { value: expr.value };
    }

    if (expr.type === 'IdentifierExpression') {
      const value = getVariable(context.scope, expr.name);

      if (value === undefined) {
        return {
          error: new Error(`Undefined variable: '${expr.name}'`)
        };
      }

      // Ensure we return a Value object
      if (typeof value === 'object' && value !== null && 'type' in value) {
        return { value: value as Value };
      }

      return { value: createValue(value) };
    }

    if (expr.type === 'BinaryExpression') {
      // For binary expressions, we'd need to delegate to the appropriate VM
      // This is a simplified version that only handles literals
      return {
        error: new Error('Complex expressions in print() not yet supported')
      };
    }

    return {
      error: new Error(`Cannot evaluate expression of type: ${expr.type}`)
    };
  }
}