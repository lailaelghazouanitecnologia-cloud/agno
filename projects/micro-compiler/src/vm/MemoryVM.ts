import {
  MicroVM,
  ASTNode,
  ExecutionContext,
  ExecutionResult
} from '../types/MicroVM';
import {
  VariableDeclaration,
  AssignmentExpression,
  IdentifierExpression
} from '../types/AST';
import { createValue, Value } from '../types/Value';
import { defineVariable, assignVariable, getVariable } from '../types/Scope';

/**
 * MemoryVM - Handles variable declarations, assignments, and identifier lookups
 */
export class MemoryVM implements MicroVM {
  getName(): string {
    return 'MemoryVM';
  }

  canHandle(node: ASTNode): boolean {
    return (
      node.type === 'VariableDeclaration' ||
      node.type === 'AssignmentExpression' ||
      node.type === 'IdentifierExpression'
    );
  }

  execute(node: ASTNode, context: ExecutionContext): ExecutionResult {
    switch (node.type) {
      case 'VariableDeclaration':
        return this.executeVariableDeclaration(node as VariableDeclaration, context);

      case 'AssignmentExpression':
        return this.executeAssignment(node as AssignmentExpression, context);

      case 'IdentifierExpression':
        return this.executeIdentifier(node as IdentifierExpression, context);

      default:
        return {
          error: new Error(`MemoryVM cannot handle node type: ${node.type}`)
        };
    }
  }

  /**
   * Execute a variable declaration
   */
  private executeVariableDeclaration(
    node: VariableDeclaration,
    context: ExecutionContext
  ): ExecutionResult {
    let value: Value;

    if (node.initializer) {
      // Evaluate the initializer
      // Note: This would typically be delegated to another VM
      // For now, we'll handle simple literals
      if (node.initializer.type === 'LiteralExpression') {
        value = (node.initializer as any).value;
      } else {
        // For complex expressions, we'd need to evaluate them
        // This is a simplified version
        value = createValue(null);
      }
    } else {
      value = createValue(null);
    }

    try {
      defineVariable(context.scope, node.name, value);
      return { value };
    } catch (error) {
      return {
        error: error instanceof Error ? error : new Error(String(error))
      };
    }
  }

  /**
   * Execute an assignment expression
   */
  private executeAssignment(
    node: AssignmentExpression,
    context: ExecutionContext
  ): ExecutionResult {
    // Evaluate the value expression
    let value: Value;

    if (node.value.type === 'LiteralExpression') {
      value = (node.value as any).value;
    } else {
      // For complex expressions, we'd need to evaluate them
      // This is a simplified version
      value = createValue(null);
    }

    try {
      assignVariable(context.scope, node.name, value);
      return { value };
    } catch (error) {
      return {
        error: error instanceof Error ? error : new Error(String(error))
      };
    }
  }

  /**
   * Execute an identifier expression (variable lookup)
   */
  private executeIdentifier(
    node: IdentifierExpression,
    context: ExecutionContext
  ): ExecutionResult {
    const value = getVariable(context.scope, node.name);

    if (value === undefined) {
      return {
        error: new Error(`Undefined variable: '${node.name}'`)
      };
    }

    // Ensure we return a Value object
    if (typeof value === 'object' && value !== null && 'type' in value) {
      return { value: value as Value };
    }

    return { value: createValue(value) };
  }
}