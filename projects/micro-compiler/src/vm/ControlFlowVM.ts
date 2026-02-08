import {
  MicroVM,
  ASTNode,
  ExecutionContext,
  ExecutionResult,
  createChildContext
} from '../types/MicroVM';
import {
  IfStatement,
  WhileStatement,
  BlockStatement,
  ExpressionStatement
} from '../types/AST';
import { valueToBoolean, Value } from '../types/Value';
import { getVariable } from '../types/Scope';

/**
 * ControlFlowVM - Handles if/else statements and while loops
 */
export class ControlFlowVM implements MicroVM {
  private registry: any; // Will be set by the compiler

  constructor(registry?: any) {
    this.registry = registry;
  }

  public setRegistry(registry: any): void {
    this.registry = registry;
  }

  getName(): string {
    return 'ControlFlowVM';
  }

  canHandle(node: ASTNode): boolean {
    return (
      node.type === 'IfStatement' ||
      node.type === 'WhileStatement' ||
      node.type === 'BlockStatement'
    );
  }

  execute(node: ASTNode, context: ExecutionContext): ExecutionResult {
    switch (node.type) {
      case 'IfStatement':
        return this.executeIfStatement(node as IfStatement, context);

      case 'WhileStatement':
        return this.executeWhileStatement(node as WhileStatement, context);

      case 'BlockStatement':
        return this.executeBlockStatement(node as BlockStatement, context);

      default:
        return {
          error: new Error(`ControlFlowVM cannot handle node type: ${node.type}`)
        };
    }
  }

  /**
   * Execute an if statement
   */
  private executeIfStatement(
    node: IfStatement,
    context: ExecutionContext
  ): ExecutionResult {
    // Evaluate the condition
    const conditionValue = this.evaluateExpression(node.condition, context);

    if (conditionValue === null) {
      return {
        error: new Error('Failed to evaluate if condition')
      };
    }

    const condition = valueToBoolean(conditionValue);

    if (condition) {
      // Execute then branch
      return this.executeStatement(node.thenBranch, context);
    } else if (node.elseBranch) {
      // Execute else branch
      return this.executeStatement(node.elseBranch, context);
    }

    return { value: conditionValue };
  }

  /**
   * Execute a while statement
   */
  private executeWhileStatement(
    node: WhileStatement,
    context: ExecutionContext
  ): ExecutionResult {
    let lastResult: ExecutionResult = { value: { type: 'NULL', data: null } };

    while (true) {
      // Evaluate the condition
      const conditionValue = this.evaluateExpression(node.condition, context);

      if (conditionValue === null) {
        return {
          error: new Error('Failed to evaluate while condition')
        };
      }

      const condition = valueToBoolean(conditionValue);

      if (!condition) {
        break;
      }

      // Execute the body
      const result = this.executeStatement(node.body, context);

      if (result.error) {
        return result;
      }

      lastResult = result;

      // Check for break/continue (not yet implemented)
      if (context.shouldBreak) {
        context.shouldBreak = false;
        break;
      }

      if (context.shouldContinue) {
        context.shouldContinue = false;
        continue;
      }
    }

    return lastResult;
  }

  /**
   * Execute a block statement
   */
  private executeBlockStatement(
    node: BlockStatement,
    context: ExecutionContext
  ): ExecutionResult {
    // Create a new scope for the block
    const blockContext = createChildContext(context);

    let lastResult: ExecutionResult = { value: { type: 'NULL', data: null } };

    for (const stmt of node.statements) {
      const result = this.executeStatement(stmt, blockContext);

      if (result.error) {
        return result;
      }

      lastResult = result;

      // Check for return
      if (blockContext.shouldReturn) {
        context.shouldReturn = true;
        context.returnValue = blockContext.returnValue;
        return lastResult;
      }
    }

    // Copy output back to parent context
    context.output.push(...blockContext.output);

    return lastResult;
  }

  /**
   * Execute a statement
   */
  private executeStatement(stmt: any, context: ExecutionContext): ExecutionResult {
    if (this.registry) {
      return this.registry.execute(stmt, context);
    }

    // Fallback: handle basic statement types
    switch (stmt.type) {
      case 'BlockStatement':
        return this.executeBlockStatement(stmt, context);

      case 'ExpressionStatement':
        return this.executeExpressionStatement(stmt, context);

      default:
        return {
          error: new Error(`Cannot execute statement of type: ${stmt.type}`)
        };
    }
  }

  /**
   * Execute an expression statement
   */
  private executeExpressionStatement(
    stmt: ExpressionStatement,
    context: ExecutionContext
  ): ExecutionResult {
    return this.evaluateExpression(stmt.expression, context);
  }

  /**
   * Evaluate an expression
   */
  private evaluateExpression(expr: any, context: ExecutionContext): Value | null {
    if (expr.type === 'LiteralExpression') {
      return expr.value;
    }

    if (expr.type === 'IdentifierExpression') {
      const value = getVariable(context.scope, expr.name);

      if (value === undefined) {
        throw new Error(`Undefined variable: '${expr.name}'`);
      }

      // Ensure we return a Value object
      if (typeof value === 'object' && value !== null && 'type' in value) {
        return value as Value;
      }

      return { type: 'UNDEFINED', data: value };
    }

    if (expr.type === 'BinaryExpression') {
      // For binary expressions, we'd need to delegate to the appropriate VM
      // This is a simplified version
      throw new Error('Complex expressions not yet supported in ControlFlowVM');
    }

    throw new Error(`Cannot evaluate expression of type: ${expr.type}`);
  }
}