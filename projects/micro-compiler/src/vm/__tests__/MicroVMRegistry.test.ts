import { MicroVMRegistry } from '../MicroVMRegistry';
import { MicroVM, ASTNode, ExecutionContext, ExecutionResult } from '../../types/MicroVM';
import { LiteralExpression } from '../../types/AST';
import { ValueType } from '../../types/Value';

// Mock micro-VM for testing
class MockMicroVM implements MicroVM {
  constructor(private name: string, private handledTypes: string[] = []) {}

  getName(): string {
    return this.name;
  }

  canHandle(node: ASTNode): boolean {
    return this.handledTypes.includes(node.type);
  }

  execute(node: ASTNode, context: ExecutionContext): ExecutionResult {
    return { value: { type: ValueType.NUMBER, data: 42 } };
  }
}

describe('MicroVMRegistry', () => {
  let registry: MicroVMRegistry;

  beforeEach(() => {
    registry = new MicroVMRegistry();
  });

  describe('Registration', () => {
    it('should register a micro-VM', () => {
      const vm = new MockMicroVM('test-vm');
      registry.register(vm);

      expect(registry.has('test-vm')).toBe(true);
    });

    it('should unregister a micro-VM', () => {
      const vm = new MockMicroVM('test-vm');
      registry.register(vm);
      registry.unregister('test-vm');

      expect(registry.has('test-vm')).toBe(false);
    });

    it('should get a registered micro-VM', () => {
      const vm = new MockMicroVM('test-vm');
      registry.register(vm);

      const retrieved = registry.get('test-vm');
      expect(retrieved).toBe(vm);
    });

    it('should return undefined for non-existent micro-VM', () => {
      const retrieved = registry.get('non-existent');
      expect(retrieved).toBeUndefined();
    });
  });

  describe('Handler finding', () => {
    it('should find handler for supported node type', () => {
      const vm = new MockMicroVM('literal-vm', ['LiteralExpression']);
      registry.register(vm);

      const node: LiteralExpression = {
        type: 'LiteralExpression',
        value: { type: ValueType.NUMBER, data: 42 }
      };

      const handler = registry.findHandler(node);
      expect(handler).toBe(vm);
    });

    it('should return undefined for unsupported node type', () => {
      const vm = new MockMicroVM('literal-vm', ['LiteralExpression']);
      registry.register(vm);

      const node: any = { type: 'UnsupportedType' };

      const handler = registry.findHandler(node);
      expect(handler).toBeUndefined();
    });

    it('should find first matching handler when multiple registered', () => {
      const vm1 = new MockMicroVM('vm1', ['LiteralExpression', 'BinaryExpression']);
      const vm2 = new MockMicroVM('vm2', ['LiteralExpression']);
      registry.register(vm1);
      registry.register(vm2);

      const node: LiteralExpression = {
        type: 'LiteralExpression',
        value: { type: ValueType.NUMBER, data: 42 }
      };

      const handler = registry.findHandler(node);
      expect(handler).toBe(vm1); // First registered
    });
  });

  describe('Execution', () => {
    it('should execute node with appropriate handler', () => {
      const vm = new MockMicroVM('literal-vm', ['LiteralExpression']);
      registry.register(vm);

      const node: LiteralExpression = {
        type: 'LiteralExpression',
        value: { type: ValueType.NUMBER, data: 42 }
      };

      const context: ExecutionContext = {
        scope: { parent: null, variables: new Map() },
        output: []
      };

      const result = registry.execute(node, context);
      expect(result.value).toBeDefined();
      expect(result.error).toBeUndefined();
    });

    it('should return error when no handler found', () => {
      const node: any = { type: 'UnsupportedType' };
      const context: ExecutionContext = {
        scope: { parent: null, variables: new Map() },
        output: []
      };

      const result = registry.execute(node, context);
      expect(result.error).toBeDefined();
      expect(result.error?.message).toContain('No micro-VM found');
    });
  });

  describe('Utility methods', () => {
    it('should get all registered micro-VMs', () => {
      const vm1 = new MockMicroVM('vm1');
      const vm2 = new MockMicroVM('vm2');
      registry.register(vm1);
      registry.register(vm2);

      const all = registry.getAll();
      expect(all).toHaveLength(2);
      expect(all).toContain(vm1);
      expect(all).toContain(vm2);
    });

    it('should clear all registered micro-VMs', () => {
      const vm1 = new MockMicroVM('vm1');
      const vm2 = new MockMicroVM('vm2');
      registry.register(vm1);
      registry.register(vm2);

      registry.clear();

      expect(registry.getAll()).toHaveLength(0);
      expect(registry.has('vm1')).toBe(false);
      expect(registry.has('vm2')).toBe(false);
    });
  });
});