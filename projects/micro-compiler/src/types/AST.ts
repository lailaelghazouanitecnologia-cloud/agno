import { Value } from './Value';

/**
 * Base interface for all AST nodes
 */
export interface ASTNode {
  type: string;
}

/**
 * Expression nodes
 */
export interface Expression extends ASTNode {}

export interface BinaryExpression extends Expression {
  type: 'BinaryExpression';
  operator: string;
  left: Expression;
  right: Expression;
}

export interface UnaryExpression extends Expression {
  type: 'UnaryExpression';
  operator: string;
  operand: Expression;
}

export interface LiteralExpression extends Expression {
  type: 'LiteralExpression';
  value: Value;
}

export interface IdentifierExpression extends Expression {
  type: 'IdentifierExpression';
  name: string;
}

export interface CallExpression extends Expression {
  type: 'CallExpression';
  callee: Expression;
  arguments: Expression[];
}

export interface AssignmentExpression extends Expression {
  type: 'AssignmentExpression';
  name: string;
  value: Expression;
}

/**
 * Statement nodes
 */
export interface Statement extends ASTNode {}

export interface ExpressionStatement extends Statement {
  type: 'ExpressionStatement';
  expression: Expression;
}

export interface VariableDeclaration extends Statement {
  type: 'VariableDeclaration';
  name: string;
  initializer: Expression | null;
}

export interface BlockStatement extends Statement {
  type: 'BlockStatement';
  statements: Statement[];
}

export interface IfStatement extends Statement {
  type: 'IfStatement';
  condition: Expression;
  thenBranch: Statement;
  elseBranch: Statement | null;
}

export interface WhileStatement extends Statement {
  type: 'WhileStatement';
  condition: Expression;
  body: Statement;
}

export interface FunctionDeclaration extends Statement {
  type: 'FunctionDeclaration';
  name: string;
  parameters: string[];
  body: BlockStatement;
}

export interface PrintStatement extends Statement {
  type: 'PrintStatement';
  expression: Expression;
}

export interface Program extends ASTNode {
  type: 'Program';
  statements: Statement[];
}

/**
 * Type guards for AST nodes
 */
export function isBinaryExpression(node: ASTNode): node is BinaryExpression {
  return node.type === 'BinaryExpression';
}

export function isUnaryExpression(node: ASTNode): node is UnaryExpression {
  return node.type === 'UnaryExpression';
}

export function isIdentifierExpression(node: ASTNode): node is IdentifierExpression {
  return node.type === 'IdentifierExpression';
}

export function isLiteralExpression(node: ASTNode): node is LiteralExpression {
  return node.type === 'LiteralExpression';
}

export function isCallExpression(node: ASTNode): node is CallExpression {
  return node.type === 'CallExpression';
}

export function isStatement(node: ASTNode): node is Statement {
  return [
    'ExpressionStatement',
    'VariableDeclaration',
    'BlockStatement',
    'IfStatement',
    'WhileStatement',
    'FunctionDeclaration',
    'PrintStatement'
  ].includes(node.type);
}