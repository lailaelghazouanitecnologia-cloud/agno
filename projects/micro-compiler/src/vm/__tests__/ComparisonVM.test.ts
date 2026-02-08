import { ComparisonVM } from '../ComparisonVM';
import { createScope, defineVariable } from '../../types/Scope';
import { createValue, createExecutionContext } from '../../types/MicroVM';
import { BinaryExpression, LiteralExpression, IdentifierExpression } from '../../types/AST';

// Helper function to create a literal expression
function createLiteralExpression(value: any): LiteralExpression {
  return {
    type: 'LiteralExpression',
    value: createValue(value)
  };
}

// Helper function to create an identifier expression
function createIdentifierExpression(name: string): IdentifierExpression {
  return {
    type: 'IdentifierExpression',
    name
  };
}

describe('ComparisonVM', () => {
  let vm: ComparisonVM;

  beforeEach(() => {
    vm = new ComparisonVM();
  });

  describe('Less than', () => {
    it('should compare numbers with <', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '<',
        left: createLiteralExpression(5),
        right: createLiteralExpression(10)
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.error).toBeUndefined();
      expect(result.value?.data).toBe(true);
    });

    it('should return false when left is greater', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '<',
        left: createLiteralExpression(10),
        right: createLiteralExpression(5)
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.value?.data).toBe(false);
    });
  });

  describe('Greater than', () => {
    it('should compare numbers with >', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '>',
        left: createLiteralExpression(10),
        right: createLiteralExpression(5)
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.value?.data).toBe(true);
    });
  });

  describe('Less than or equal', () => {
    it('should return true when equal', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '<=',
        left: createLiteralExpression(5),
        right: createLiteralExpression(5)
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.value?.data).toBe(true);
    });

    it('should return true when less', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '<=',
        left: createLiteralExpression(3),
        right: createLiteralExpression(5)
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.value?.data).toBe(true);
    });

    it('should return false when greater', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '<=',
        left: createLiteralExpression(10),
        right: createLiteralExpression(5)
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.value?.data).toBe(false);
    });
  });

  describe('Greater than or equal', () => {
    it('should return true when equal', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '>=',
        left: createLiteralExpression(5),
        right: createLiteralExpression(5)
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.value?.data).toBe(true);
    });

    it('should return true when greater', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '>=',
        left: createLiteralExpression(10),
        right: createLiteralExpression(5)
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.value?.data).toBe(true);
    });
  });

  describe('Equality', () => {
    it('should return true for equal numbers', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '==',
        left: createLiteralExpression(5),
        right: createLiteralExpression(5)
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.value?.data).toBe(true);
    });

    it('should return false for different numbers', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '==',
        left: createLiteralExpression(5),
        right: createLiteralExpression(10)
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.value?.data).toBe(false);
    });
  });

  describe('Inequality', () => {
    it('should return true for different numbers', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '!=',
        left: createLiteralExpression(5),
        right: createLiteralExpression(10)
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.value?.data).toBe(true);
    });

    it('should return false for equal numbers', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '!=',
        left: createLiteralExpression(5),
        right: createLiteralExpression(5)
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.value?.data).toBe(false);
    });
  });

  describe('Variable operands', () => {
    it('should use variables as operands', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '>',
        left: createIdentifierExpression('x'),
        right: createIdentifierExpression('y')
      };

      const scope = createScope();
      defineVariable(scope, 'x', createValue(10));
      defineVariable(scope, 'y', createValue(5));
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.value?.data).toBe(true);
    });

    it('should throw error for undefined variable', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '==',
        left: createIdentifierExpression('undefined_var'),
        right: createLiteralExpression(5)
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.error).toBeDefined();
      expect(result.error?.message).toContain('Undefined variable');
    });
  });

  describe('String comparisons', () => {
    it('should compare strings', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '<',
        left: createLiteralExpression('apple'),
        right: createLiteralExpression('banana')
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.value?.data).toBe(true);
    });
  });

  describe('Type checking', () => {
    it('should throw error when comparing number with string using <', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '<',
        left: createLiteralExpression(5),
        right: createLiteralExpression('hello')
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.error).toBeDefined();
      expect(result.error?.message).toContain('Cannot compare');
    });
  });

  describe('canHandle', () => {
    it('should handle comparison operators', () => {
      const operators = ['<', '>', '<=', '>=', '==', '!='];

      operators.forEach(op => {
        const node: BinaryExpression = {
          type: 'BinaryExpression',
          operator: op,
          left: createLiteralExpression(1),
          right: createLiteralExpression(2)
        };

        expect(vm.canHandle(node)).toBe(true);
      });
    });

    it('should not handle arithmetic operators', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '+',
        left: createLiteralExpression(1),
        right: createLiteralExpression(2)
      };

      expect(vm.canHandle(node)).toBe(false);
    });
  });

  describe('getName', () => {
    it('should return correct name', () => {
      expect(vm.getName()).toBe('ComparisonVM');
    });
  });
});