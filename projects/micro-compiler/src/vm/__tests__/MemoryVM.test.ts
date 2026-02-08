import { MemoryVM } from '../MemoryVM';
import { createScope, defineVariable } from '../../types/Scope';
import { createValue, createExecutionContext } from '../../types/MicroVM';
import { VariableDeclaration, AssignmentExpression, IdentifierExpression, LiteralExpression } from '../../types/AST';

describe('MemoryVM', () => {
  let vm: MemoryVM;

  beforeEach(() => {
    vm = new MemoryVM();
  });

  describe('Variable declarations', () => {
    it('should declare a variable with initializer', () => {
      const literalValue = createValue(42);
      const initializer: LiteralExpression = {
        type: 'LiteralExpression',
        value: literalValue
      };
      
      const node: VariableDeclaration = {
        type: 'VariableDeclaration',
        name: 'x',
        initializer
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.error).toBeUndefined();
      expect(result.value).toBeDefined();
      expect(scope.variables.get('x')).toEqual(createValue(42));
    });

    it('should declare a variable without initializer', () => {
      const node: VariableDeclaration = {
        type: 'VariableDeclaration',
        name: 'y',
        initializer: null
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.error).toBeUndefined();
      expect(scope.variables.get('y')).toEqual(createValue(null));
    });

    it('should throw error when redefining variable', () => {
      const literalValue = createValue(42);
      const initializer: LiteralExpression = {
        type: 'LiteralExpression',
        value: literalValue
      };
      
      const node: VariableDeclaration = {
        type: 'VariableDeclaration',
        name: 'x',
        initializer
      };

      const scope = createScope();
      defineVariable(scope, 'x', createValue(10));
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.error).toBeDefined();
      expect(result.error?.message).toContain('already defined');
    });
  });

  describe('Assignment expressions', () => {
    it('should assign to existing variable', () => {
      const literalValue = createValue(100);
      const valueExpr: LiteralExpression = {
        type: 'LiteralExpression',
        value: literalValue
      };
      
      const node: AssignmentExpression = {
        type: 'AssignmentExpression',
        name: 'x',
        value: valueExpr
      };

      const scope = createScope();
      defineVariable(scope, 'x', createValue(42));
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.error).toBeUndefined();
      expect(scope.variables.get('x')).toEqual(createValue(100));
    });

    it('should throw error when assigning to undefined variable', () => {
      const literalValue = createValue(100);
      const valueExpr: LiteralExpression = {
        type: 'LiteralExpression',
        value: literalValue
      };
      
      const node: AssignmentExpression = {
        type: 'AssignmentExpression',
        name: 'undefined_var',
        value: valueExpr
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.error).toBeDefined();
      expect(result.error?.message).toContain('Cannot assign to undefined variable');
    });
  });

  describe('Identifier expressions', () => {
    it('should look up existing variable', () => {
      const node: IdentifierExpression = {
        type: 'IdentifierExpression',
        name: 'x'
      };

      const scope = createScope();
      defineVariable(scope, 'x', createValue(42));
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.error).toBeUndefined();
      expect(result.value).toEqual(createValue(42));
    });

    it('should look up variable in parent scope', () => {
      const node: IdentifierExpression = {
        type: 'IdentifierExpression',
        name: 'x'
      };

      const parentScope = createScope();
      defineVariable(parentScope, 'x', createValue(42));

      const childScope = createScope(parentScope);
      const context = createExecutionContext(childScope);

      const result = vm.execute(node, context);

      expect(result.error).toBeUndefined();
      expect(result.value).toEqual(createValue(42));
    });

    it('should throw error for undefined variable', () => {
      const node: IdentifierExpression = {
        type: 'IdentifierExpression',
        name: 'undefined_var'
      };

      const scope = createScope();
      const context = createExecutionContext(scope);

      const result = vm.execute(node, context);

      expect(result.error).toBeDefined();
      expect(result.error?.message).toContain('Undefined variable');
    });

    it('should prefer local scope over parent scope', () => {
      const node: IdentifierExpression = {
        type: 'IdentifierExpression',
        name: 'x'
      };

      const parentScope = createScope();
      defineVariable(parentScope, 'x', createValue(42));

      const childScope = createScope(parentScope);
      defineVariable(childScope, 'x', createValue(100));

      const context = createExecutionContext(childScope);

      const result = vm.execute(node, context);

      expect(result.error).toBeUndefined();
      expect(result.value).toEqual(createValue(100));
    });
  });

  describe('canHandle', () => {
    it('should handle VariableDeclaration', () => {
      const node: VariableDeclaration = {
        type: 'VariableDeclaration',
        name: 'x',
        initializer: null
      };

      expect(vm.canHandle(node)).toBe(true);
    });

    it('should handle AssignmentExpression', () => {
      const literalValue = createValue(42);
      const valueExpr: LiteralExpression = {
        type: 'LiteralExpression',
        value: literalValue
      };
      
      const node: AssignmentExpression = {
        type: 'AssignmentExpression',
        name: 'x',
        value: valueExpr
      };

      expect(vm.canHandle(node)).toBe(true);
    });

    it('should handle IdentifierExpression', () => {
      const node: IdentifierExpression = {
        type: 'IdentifierExpression',
        name: 'x'
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
      expect(vm.getName()).toBe('MemoryVM');
    });
  });
});