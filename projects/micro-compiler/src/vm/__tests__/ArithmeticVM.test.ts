import { ArithmeticVM } from '../ArithmeticVM';
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

describe('ArithmeticVM', () => {
  let vm: ArithmeticVM;

  beforeEach(() => {
    vm = new ArithmeticVM();
  });

  describe('Addition', () => {
    it('should add two numbers', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '+',
        left: createLiteralExpression(5),
        right: createLiteralExpression(3)
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.error).toBeUndefined();
      expect(result.value?.data).toBe(8);
    });

    it('should add negative numbers', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '+',
        left: createLiteralExpression(-5),
        right: createLiteralExpression(3)
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.value?.data).toBe(-2);
    });
  });

  describe('Subtraction', () => {
    it('should subtract two numbers', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '-',
        left: createLiteralExpression(10),
        right: createLiteralExpression(4)
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.value?.data).toBe(6);
    });
  });

  describe('Multiplication', () => {
    it('should multiply two numbers', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '*',
        left: createLiteralExpression(6),
        right: createLiteralExpression(7)
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.value?.data).toBe(42);
    });
  });

  describe('Division', () => {
    it('should divide two numbers', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '/',
        left: createLiteralExpression(20),
        right: createLiteralExpression(4)
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.value?.data).toBe(5);
    });

    it('should handle decimal division', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '/',
        left: createLiteralExpression(7),
        right: createLiteralExpression(2)
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.value?.data).toBe(3.5);
    });

    it('should throw error on division by zero', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '/',
        left: createLiteralExpression(10),
        right: createLiteralExpression(0)
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.error).toBeDefined();
      expect(result.error?.message).toContain('Division by zero');
    });
  });

  describe('Modulo', () => {
    it('should compute modulo', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '%',
        left: createLiteralExpression(10),
        right: createLiteralExpression(3)
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.value?.data).toBe(1);
    });

    it('should throw error on modulo by zero', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '%',
        left: createLiteralExpression(10),
        right: createLiteralExpression(0)
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.error).toBeDefined();
      expect(result.error?.message).toContain('Modulo by zero');
    });
  });

  describe('Variable operands', () => {
    it('should use variables as operands', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '+',
        left: createIdentifierExpression('x'),
        right: createIdentifierExpression('y')
      };

      const scope = createScope();
      defineVariable(scope, 'x', createValue(5));
      defineVariable(scope, 'y', createValue(3));
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.value?.data).toBe(8);
    });

    it('should throw error for undefined variable', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '+',
        left: createIdentifierExpression('undefined_var'),
        right: createLiteralExpression(3)
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.error).toBeDefined();
      expect(result.error?.message).toContain('Undefined variable');
    });
  });

  describe('Nested expressions', () => {
    it('should evaluate nested expressions', () => {
      const leftExpr: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '*',
        left: createLiteralExpression(2),
        right: createLiteralExpression(3)
      };
      
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '+',
        left: leftExpr,
        right: createLiteralExpression(4)
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.value?.data).toBe(10); // 2 * 3 + 4 = 10
    });
  });

  describe('Literal expressions', () => {
    it('should handle numeric literals', () => {
      const literalValue = createValue(42);
      const node: LiteralExpression = {
        type: 'LiteralExpression',
        value: literalValue
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.value?.data).toBe(42);
    });
  });

  describe('canHandle', () => {
    it('should handle arithmetic operators', () => {
      const operators = ['+', '-', '*', '/', '%'];

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

    it('should not handle comparison operators', () => {
      const node: BinaryExpression = {
        type: 'BinaryExpression',
        operator: '==',
        left: createLiteralExpression(1),
        right: createLiteralExpression(2)
      };

      expect(vm.canHandle(node)).toBe(false);
    });

    it('should handle numeric literals', () => {
      const literalValue = createValue(42);
      const node: LiteralExpression = {
        type: 'LiteralExpression',
        value: literalValue
      };

      expect(vm.canHandle(node)).toBe(true);
    });
  });

  describe('getName', () => {
    it('should return correct name', () => {
      expect(vm.getName()).toBe('ArithmeticVM');
    });
  });
});