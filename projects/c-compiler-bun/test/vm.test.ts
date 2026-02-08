/**
 * Test suite for MicroVMs
 */

import { describe, it, expect, beforeEach } from 'bun:test';
import {
  ArithmeticVM,
  ComparisonVM,
  ControlFlowVM,
  FunctionVM,
  MemoryVM,
  IOVM,
  TypeVM,
} from '../src/vm/index.js';
import { VMType, ValueType, type VMContext, type Value, type CallFrame } from '../src/core/types.js';

describe('ArithmeticVM', () => {
  let vm: ArithmeticVM;
  let context: VMContext;

  beforeEach(() => {
    vm = new ArithmeticVM();
    context = {
      programCounter: 0,
      callStack: [],
      globalMemory: new Map(),
      output: [],
      input: [],
    };
    vm.initialize(context);
  });

  it('should have correct VM type', () => {
    expect(vm.vmType).toBe(VMType.ARITHMETIC);
  });

  it('should add integers', () => {
    const left: Value = { type: ValueType.INT, value: 5 };
    const right: Value = { type: ValueType.INT, value: 3 };
    const result = vm.add(left, right);

    expect(result.type).toBe(ValueType.INT);
    expect(result.value).toBe(8);
  });

  it('should add floats', () => {
    const left: Value = { type: ValueType.FLOAT, value: 5.5 };
    const right: Value = { type: ValueType.FLOAT, value: 3.2 };
    const result = vm.add(left, right);

    expect(result.type).toBe(ValueType.FLOAT);
    expect(result.value).toBeCloseTo(8.7);
  });

  it('should subtract integers', () => {
    const left: Value = { type: ValueType.INT, value: 5 };
    const right: Value = { type: ValueType.INT, value: 3 };
    const result = vm.subtract(left, right);

    expect(result.type).toBe(ValueType.INT);
    expect(result.value).toBe(2);
  });

  it('should multiply integers', () => {
    const left: Value = { type: ValueType.INT, value: 5 };
    const right: Value = { type: ValueType.INT, value: 3 };
    const result = vm.multiply(left, right);

    expect(result.type).toBe(ValueType.INT);
    expect(result.value).toBe(15);
  });

  it('should divide integers', () => {
    const left: Value = { type: ValueType.INT, value: 6 };
    const right: Value = { type: ValueType.INT, value: 3 };
    const result = vm.divide(left, right);

    expect(result.type).toBe(ValueType.INT);
    expect(result.value).toBe(2);
  });

  it('should throw on division by zero', () => {
    const left: Value = { type: ValueType.INT, value: 5 };
    const right: Value = { type: ValueType.INT, value: 0 };

    expect(() => vm.divide(left, right)).toThrow('Division by zero');
  });

  it('should modulo integers', () => {
    const left: Value = { type: ValueType.INT, value: 7 };
    const right: Value = { type: ValueType.INT, value: 3 };
    const result = vm.modulo(left, right);

    expect(result.type).toBe(ValueType.INT);
    expect(result.value).toBe(1);
  });

  it('should throw on modulo by zero', () => {
    const left: Value = { type: ValueType.INT, value: 5 };
    const right: Value = { type: ValueType.INT, value: 0 };

    expect(() => vm.modulo(left, right)).toThrow('Modulo by zero');
  });

  it('should reset state', () => {
    vm.reset();
    const state = vm.getState();

    expect(state.context).toBe('null');
  });
});

describe('ComparisonVM', () => {
  let vm: ComparisonVM;
  let context: VMContext;

  beforeEach(() => {
    vm = new ComparisonVM();
    context = {
      programCounter: 0,
      callStack: [],
      globalMemory: new Map(),
      output: [],
      input: [],
    };
    vm.initialize(context);
  });

  it('should have correct VM type', () => {
    expect(vm.vmType).toBe(VMType.COMPARISON);
  });

  it('should compare less than', () => {
    const left: Value = { type: ValueType.INT, value: 3 };
    const right: Value = { type: ValueType.INT, value: 5 };
    const result = vm.lessThan(left, right);

    expect(result.type).toBe(ValueType.INT);
    expect(result.value).toBe(1);
  });

  it('should compare greater than', () => {
    const left: Value = { type: ValueType.INT, value: 5 };
    const right: Value = { type: ValueType.INT, value: 3 };
    const result = vm.greaterThan(left, right);

    expect(result.type).toBe(ValueType.INT);
    expect(result.value).toBe(1);
  });

  it('should compare less equal', () => {
    const left: Value = { type: ValueType.INT, value: 3 };
    const right: Value = { type: ValueType.INT, value: 3 };
    const result = vm.lessEqual(left, right);

    expect(result.type).toBe(ValueType.INT);
    expect(result.value).toBe(1);
  });

  it('should compare greater equal', () => {
    const left: Value = { type: ValueType.INT, value: 5 };
    const right: Value = { type: ValueType.INT, value: 5 };
    const result = vm.greaterEqual(left, right);

    expect(result.type).toBe(ValueType.INT);
    expect(result.value).toBe(1);
  });

  it('should compare equal', () => {
    const left: Value = { type: ValueType.INT, value: 5 };
    const right: Value = { type: ValueType.INT, value: 5 };
    const result = vm.equal(left, right);

    expect(result.type).toBe(ValueType.INT);
    expect(result.value).toBe(1);
  });

  it('should compare not equal', () => {
    const left: Value = { type: ValueType.INT, value: 5 };
    const right: Value = { type: ValueType.INT, value: 3 };
    const result = vm.notEqual(left, right);

    expect(result.type).toBe(ValueType.INT);
    expect(result.value).toBe(1);
  });

  it('should return 0 for false comparisons', () => {
    const left: Value = { type: ValueType.INT, value: 5 };
    const right: Value = { type: ValueType.INT, value: 3 };
    const result = vm.lessThan(left, right);

    expect(result.value).toBe(0);
  });
});

describe('TypeVM', () => {
  let vm: TypeVM;
  let context: VMContext;

  beforeEach(() => {
    vm = new TypeVM();
    context = {
      programCounter: 0,
      callStack: [],
      globalMemory: new Map(),
      output: [],
      input: [],
    };
    vm.initialize(context);
  });

  it('should have correct VM type', () => {
    expect(vm.vmType).toBe(VMType.TYPE);
  });

  it('should check compatible types', () => {
    expect(vm.isCompatible(ValueType.INT, ValueType.FLOAT)).toBe(true);
    expect(vm.isCompatible(ValueType.FLOAT, ValueType.INT)).toBe(true);
    expect(vm.isCompatible(ValueType.INT, ValueType.INT)).toBe(true);
    expect(vm.isCompatible(ValueType.INT, ValueType.CHAR)).toBe(true);
    expect(vm.isCompatible(ValueType.INT, ValueType.POINTER)).toBe(true);
  });

  it('should check incompatible types', () => {
    expect(vm.isCompatible(ValueType.VOID, ValueType.INT)).toBe(false);
  });

  it('should cast int to float', () => {
    const value: Value = { type: ValueType.INT, value: 42 };
    const result = vm.castValue(value, ValueType.FLOAT);

    expect(result.type).toBe(ValueType.FLOAT);
    expect(result.value).toBe(42.0);
  });

  it('should cast float to int', () => {
    const value: Value = { type: ValueType.FLOAT, value: 3.7 };
    const result = vm.castValue(value, ValueType.INT);

    expect(result.type).toBe(ValueType.INT);
    expect(result.value).toBe(3);
  });

  it('should cast int to char', () => {
    const value: Value = { type: ValueType.INT, value: 65 };
    const result = vm.castValue(value, ValueType.CHAR);

    expect(result.type).toBe(ValueType.CHAR);
    expect(result.value).toBe('A');
  });

  it('should cast int to pointer', () => {
    const value: Value = { type: ValueType.INT, value: 0x1000 };
    const result = vm.castValue(value, ValueType.POINTER);

    expect(result.type).toBe(ValueType.POINTER);
    expect(result.value).toBe(0x1000);
  });
});

describe('MemoryVM', () => {
  let vm: MemoryVM;
  let context: VMContext;

  beforeEach(() => {
    vm = new MemoryVM();
    context = {
      programCounter: 0,
      callStack: [],
      globalMemory: new Map(),
      output: [],
      input: [],
    };
    vm.initialize(context);
  });

  it('should have correct VM type', () => {
    expect(vm.vmType).toBe(VMType.MEMORY);
  });

  it('should allocate and get variables', () => {
    vm.allocateVariable('x', ValueType.INT, { type: ValueType.INT, value: 42 });
    const value = vm.getVariable('x');

    expect(value).toBeDefined();
    expect(value?.value).toBe(42);
  });

  it('should set variables', () => {
    vm.allocateVariable('x', ValueType.INT);
    vm.setVariable('x', { type: ValueType.INT, value: 100 });

    const value = vm.getVariable('x');
    expect(value?.value).toBe(100);
  });

  it('should return undefined for non-existent variables', () => {
    const value = vm.getVariable('nonexistent');
    expect(value).toBeUndefined();
  });

  it('should allocate arrays', () => {
    vm.allocateArray('arr', ValueType.INT, 5);
    const value = vm.getVariable('arr');

    expect(value).toBeDefined();
    expect(value?.type).toBe(ValueType.ARRAY);
  });

  it('should get and set array elements', () => {
    vm.allocateArray('arr', ValueType.INT, 5);
    vm.setArrayElement('arr', 2, { type: ValueType.INT, value: 42 });

    const value = vm.getArrayElement('arr', 2);
    expect(value?.value).toBe(42);
  });

  it('should handle scopes', () => {
    vm.allocateVariable('x', ValueType.INT, { type: ValueType.INT, value: 1 });

    vm.pushScope();
    vm.allocateVariable('x', ValueType.INT, { type: ValueType.INT, value: 2 });

    let value = vm.getVariable('x');
    expect(value?.value).toBe(2);

    vm.popScope();
    value = vm.getVariable('x');
    expect(value?.value).toBe(1);
  });

  it('should get address of variable', () => {
    vm.allocateVariable('x', ValueType.INT);
    const address = vm.getAddress('x');

    expect(address).toBeDefined();
    expect(typeof address).toBe('number');
  });
});

describe('FunctionVM', () => {
  let vm: FunctionVM;
  let context: VMContext;

  beforeEach(() => {
    vm = new FunctionVM();
    context = {
      programCounter: 0,
      callStack: [],
      globalMemory: new Map(),
      output: [],
      input: [],
    };
    vm.initialize(context);
  });

  it('should have correct VM type', () => {
    expect(vm.vmType).toBe(VMType.FUNCTION);
  });

  it('should define functions', () => {
    const node = { type: 'FUNCTION_DECL', name: 'foo' };
    vm.defineFunction('foo', node as any);

    const state = vm.getState();
    expect(state.functions).toContain('foo');
  });

  it('should push and pop call frames', () => {
    const frame: CallFrame = {
      functionName: 'main',
      locals: new Map(),
      parameters: [],
    };

    vm.pushCallFrame(frame);
    expect(vm.getCurrentCallFrame()).toBe(frame);

    const popped = vm.popCallFrame();
    expect(popped).toBe(frame);
    expect(vm.getCurrentCallFrame()).toBeUndefined();
  });

  it('should return error for undefined function', () => {
    const result = vm.callFunction('nonexistent', [], context);

    expect(result.success).toBe(false);
    expect(result.error).toContain('Undefined function');
  });
});

describe('IOVM', () => {
  let vm: IOVM;
  let context: VMContext;

  beforeEach(() => {
    vm = new IOVM();
    context = {
      programCounter: 0,
      callStack: [],
      globalMemory: new Map(),
      output: [],
      input: [],
    };
    vm.initialize(context);
  });

  it('should have correct VM type', () => {
    expect(vm.vmType).toBe(VMType.IO);
  });

  it('should format printf with integers', () => {
    const args: Value[] = [
      { type: ValueType.INT, value: 42 },
    ];
    const result = vm.printf('Value: %d', args);

    expect(result).toBe('Value: 42');
  });

  it('should format printf with floats', () => {
    const args: Value[] = [
      { type: ValueType.FLOAT, value: 3.14 },
    ];
    const result = vm.printf('Pi: %f', args);

    expect(result).toBe('Pi: 3.14');
  });

  it('should format printf with characters', () => {
    const args: Value[] = [
      { type: ValueType.CHAR, value: 'A' },
    ];
    const result = vm.printf('Char: %c', args);

    expect(result).toBe('Char: A');
  });

  it('should format printf with strings', () => {
    const args: Value[] = [
      { type: ValueType.INT, value: 0 },
      { type: ValueType.INT, value: 'Hello' },
    ];
    const result = vm.printf('String: %s', args);

    expect(result).toContain('String:');
  });

  it('should format printf with pointers', () => {
    const args: Value[] = [
      { type: ValueType.POINTER, value: 0x1000, baseType: ValueType.INT },
    ];
    const result = vm.printf('Pointer: %p', args);

    expect(result).toContain('0x1000');
  });

  it('should handle multiple format specifiers', () => {
    const args: Value[] = [
      { type: ValueType.INT, value: 42 },
      { type: ValueType.FLOAT, value: 3.14 },
    ];
    const result = vm.printf('Int: %d, Float: %f', args);

    expect(result).toBe('Int: 42, Float: 3.14');
  });

  it('should handle literal %%', () => {
    const result = vm.printf('100%% complete', []);

    expect(result).toBe('100% complete');
  });

  it('should collect output', () => {
    vm.printf('Line 1', []);
    vm.printf('Line 2', []);

    const output = vm.getOutput();
    expect(output.length).toBe(2);
    expect(output[0]).toBe('Line 1');
    expect(output[1]).toBe('Line 2');
  });

  it('should clear output', () => {
    vm.printf('Line 1', []);
    vm.clearOutput();

    const output = vm.getOutput();
    expect(output.length).toBe(0);
  });

  it('should return values from scanf', () => {
    const result = vm.scanf('%d %f');

    expect(result.length).toBe(2);
    expect(result[0].type).toBe(ValueType.INT);
    expect(result[1].type).toBe(ValueType.FLOAT);
  });
});

describe('ControlFlowVM', () => {
  let vm: ControlFlowVM;
  let context: VMContext;

  beforeEach(() => {
    vm = new ControlFlowVM();
    context = {
      programCounter: 0,
      callStack: [],
      globalMemory: new Map(),
      output: [],
      input: [],
    };
    vm.initialize(context);
  });

  it('should have correct VM type', () => {
    expect(vm.vmType).toBe(VMType.CONTROL_FLOW);
  });

  it('should evaluate conditions', () => {
    const result = vm.evaluateCondition({ type: 'BINARY_EXPR' } as any, context);

    expect(result.success).toBe(true);
    expect(result.value).toBeDefined();
  });
});