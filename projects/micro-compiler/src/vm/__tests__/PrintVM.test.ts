import { PrintVM } from '../PrintVM';
import { createScope, defineVariable } from '../../types/Scope';
import { createValue, createExecutionContext } from '../../types/MicroVM';
import { PrintStatement, LiteralExpression, IdentifierExpression } from '../../types/AST';

describe('PrintVM', () => {
  let vm: PrintVM;

  beforeEach(() => {
    vm = new PrintVM();
  });

  describe('Printing literals', () => {
    it('should print a number', () => {
      const literalValue = createValue(42);
      const expression: LiteralExpression = {
        type: 'LiteralExpression',
        value: literalValue
      };
      
      const node: PrintStatement = {
        type: 'PrintStatement',
        expression
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.error).toBeUndefined();
      expect(context.output).toHaveLength(1);
      expect(context.output[0]).toBe('42');
    });

    it('should print a string', () => {
      const literalValue = createValue('hello world');
      const expression: LiteralExpression = {
        type: 'LiteralExpression',
        value: literalValue
      };
      
      const node: PrintStatement = {
        type: 'PrintStatement',
        expression
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.error).toBeUndefined();
      expect(context.output[0]).toBe('hello world');
    });

    it('should print a boolean', () => {
      const literalValue = createValue(true);
      const expression: LiteralExpression = {
        type: 'LiteralExpression',
        value: literalValue
      };
      
      const node: PrintStatement = {
        type: 'PrintStatement',
        expression
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.error).toBeUndefined();
      expect(context.output[0]).toBe('true');
    });

    it('should print false boolean', () => {
      const literalValue = createValue(false);
      const expression: LiteralExpression = {
        type: 'LiteralExpression',
        value: literalValue
      };
      
      const node: PrintStatement = {
        type: 'PrintStatement',
        expression
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.error).toBeUndefined();
      expect(context.output[0]).toBe('false');
    });

    it('should print null', () => {
      const literalValue = createValue(null);
      const expression: LiteralExpression = {
        type: 'LiteralExpression',
        value: literalValue
      };
      
      const node: PrintStatement = {
        type: 'PrintStatement',
        expression
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.error).toBeUndefined();
      expect(context.output[0]).toBe('null');
    });
  });

  describe('Printing variables', () => {
    it('should print a variable value', () => {
      const expression: IdentifierExpression = {
        type: 'IdentifierExpression',
        name: 'x'
      };
      
      const node: PrintStatement = {
        type: 'PrintStatement',
        expression
      };

      const scope = createScope();
      defineVariable(scope, 'x', createValue(42));
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.error).toBeUndefined();
      expect(context.output[0]).toBe('42');
    });

    it('should print a string variable', () => {
      const expression: IdentifierExpression = {
        type: 'IdentifierExpression',
        name: 'message'
      };
      
      const node: PrintStatement = {
        type: 'PrintStatement',
        expression
      };

      const scope = createScope();
      defineVariable(scope, 'message', createValue('Hello, Volt!'));
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.error).toBeUndefined();
      expect(context.output[0]).toBe('Hello, Volt!');
    });

    it('should throw error for undefined variable', () => {
      const expression: IdentifierExpression = {
        type: 'IdentifierExpression',
        name: 'undefined_var'
      };
      
      const node: PrintStatement = {
        type: 'PrintStatement',
        expression
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.error).toBeDefined();
      expect(result.error?.message).toContain('Undefined variable');
    });
  });

  describe('Multiple prints', () => {
    it('should accumulate multiple outputs', () => {
      const scope = createScope();
      const context = createExecutionContext(scope);

      const literalValue1 = createValue('Hello');
      const expression1: LiteralExpression = {
        type: 'LiteralExpression',
        value: literalValue1
      };
      
      const node1: PrintStatement = {
        type: 'PrintStatement',
        expression: expression1
      };

      const literalValue2 = createValue('World');
      const expression2: LiteralExpression = {
        type: 'LiteralExpression',
        value: literalValue2
      };
      
      const node2: PrintStatement = {
        type: 'PrintStatement',
        expression: expression2
      };

      vm.execute(node1, context);
      vm.execute(node2, context);

      expect(context.output).toHaveLength(2);
      expect(context.output[0]).toBe('Hello');
      expect(context.output[1]).toBe('World');
    });
  });

  describe('canHandle', () => {
    it('should handle PrintStatement', () => {
      const literalValue = createValue(42);
      const expression: LiteralExpression = {
        type: 'LiteralExpression',
        value: literalValue
      };
      
      const node: PrintStatement = {
        type: 'PrintStatement',
        expression
      };

      expect(vm.canHandle(node)).toBe(true);
    });

    it('should not handle other node types', () => {
      const node: any = { type: 'BinaryExpression' };

      expect(vm.canHandle(node)).toBe(false);
    });
  });

  describe('getName', () => {
    it('should return correct name', () => {
      expect(vm.getName()).toBe('PrintVM');
    });
  });
});