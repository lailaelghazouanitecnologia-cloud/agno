/**
 * MicroVM tests
 */

import { describe, it, expect } from "bun:test";
import { ArithmeticVM } from "../src/vm/arithmetic.js";
import { ComparisonVM } from "../src/vm/comparison.js";
import { TypeVM } from "../src/vm/type.js";
import { MicroVMRegistry } from "../src/vm/registry.js";
import { Type } from "../src/core/interfaces.js";

describe("ArithmeticVM", () => {
  it("should handle addition", () => {
    const vm = new ArithmeticVM();
    expect(vm.canHandle("BinaryOp")).toBe(true);
  });

  it("should handle subtraction", () => {
    const vm = new ArithmeticVM();
    expect(vm.canHandle("UnaryOp")).toBe(true);
  });

  it("should not handle other node types", () => {
    const vm = new ArithmeticVM();
    expect(vm.canHandle("IfStatement")).toBe(false);
    expect(vm.canHandle("FunctionCall")).toBe(false);
  });

  it("should reset without errors", () => {
    const vm = new ArithmeticVM();
    expect(() => vm.reset()).not.toThrow();
  });
});

describe("ComparisonVM", () => {
  it("should handle comparison operations", () => {
    const vm = new ComparisonVM();
    expect(vm.canHandle("BinaryOp")).toBe(true);
  });

  it("should not handle other node types", () => {
    const vm = new ComparisonVM();
    expect(vm.canHandle("IfStatement")).toBe(false);
    expect(vm.canHandle("WhileStatement")).toBe(false);
  });

  it("should reset without errors", () => {
    const vm = new ComparisonVM();
    expect(() => vm.reset()).not.toThrow();
  });
});

describe("TypeVM", () => {
  it("should handle type checking nodes", () => {
    const vm = new TypeVM();
    expect(vm.canHandle("BinaryOp")).toBe(true);
    expect(vm.canHandle("UnaryOp")).toBe(true);
    expect(vm.canHandle("Assignment")).toBe(true);
    expect(vm.canHandle("Declaration")).toBe(true);
  });

  it("should not handle control flow nodes", () => {
    const vm = new TypeVM();
    expect(vm.canHandle("IfStatement")).toBe(false);
    expect(vm.canHandle("WhileStatement")).toBe(false);
  });

  it("should reset without errors", () => {
    const vm = new TypeVM();
    expect(() => vm.reset()).not.toThrow();
  });
});

describe("MicroVMRegistry", () => {
  it("should register and retrieve VMs", () => {
    const registry = new MicroVMRegistry();
    const vm = new ArithmeticVM();

    registry.register(vm);
    expect(registry.hasVM("ArithmeticVM")).toBe(true);
    expect(registry.getVM("ArithmeticVM")).toBe(vm);
  });

  it("should unregister VMs", () => {
    const registry = new MicroVMRegistry();
    const vm = new ArithmeticVM();

    registry.register(vm);
    registry.unregister("ArithmeticVM");
    expect(registry.hasVM("ArithmeticVM")).toBe(false);
  });

  it("should find VM by node type", () => {
    const registry = new MicroVMRegistry();
    registry.register(new ArithmeticVM());
    registry.register(new ComparisonVM());

    const vm = registry.findVM("BinaryOp");
    expect(vm).toBeDefined();
    expect(vm?.name).toBe("ArithmeticVM"); // First registered
  });

  it("should return undefined for unknown node type", () => {
    const registry = new MicroVMRegistry();
    registry.register(new ArithmeticVM());

    const vm = registry.findVM("UnknownNode");
    expect(vm).toBeUndefined();
  });

  it("should get all registered VMs", () => {
    const registry = new MicroVMRegistry();
    registry.register(new ArithmeticVM());
    registry.register(new ComparisonVM());

    const vms = registry.getAllVMs();
    expect(vms).toHaveLength(2);
  });

  it("should reset all VMs", () => {
    const registry = new MicroVMRegistry();
    registry.register(new ArithmeticVM());
    registry.register(new ComparisonVM());

    expect(() => registry.resetAll()).not.toThrow();
  });
});

describe("VM Integration", () => {
  it("should handle multiple VMs for different node types", () => {
    const registry = new MicroVMRegistry();
    registry.register(new ArithmeticVM());
    registry.register(new ComparisonVM());
    registry.register(new TypeVM());

    expect(registry.findVM("BinaryOp")).toBeDefined();
    expect(registry.findVM("UnaryOp")).toBeDefined();
    expect(registry.findVM("Assignment")).toBeDefined();
  });

  it("should allow VMs to be replaced", () => {
    const registry = new MicroVMRegistry();
    const vm1 = new ArithmeticVM();
    const vm2 = new ArithmeticVM();

    registry.register(vm1);
    registry.register(vm2);

    expect(registry.getVM("ArithmeticVM")).toBe(vm2);
  });
});