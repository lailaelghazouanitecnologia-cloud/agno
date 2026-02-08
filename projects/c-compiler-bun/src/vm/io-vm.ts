/**
 * IOVM - Handles printf/scanf operations
 */

import { BaseVM } from './base-vm.js';
import { ASTNode, ExecutionContext, ExecutionResult, ValueType, IOHandler } from '../core/interfaces.js';
import { NodeType } from '../core/types.js';

/**
 * IOVM - handles input/output operations
 */
export class IOVM extends BaseVM {
  readonly name = 'io';

  private ioHandler: IOHandler;

  constructor() {
    super();
    this.ioHandler = this.createIOHandler();
  }

  initialize?(context?: unknown): void {
    this.ioHandler = this.createIOHandler();
  }

  execute(node: ASTNode, context: ExecutionContext): ExecutionResult {
    try {
      // Handle function calls to printf/scanf
      if (node.type === NodeType.CALL_EXPR) {
        const call = node as { callee: ASTNode; arguments: ASTNode[] };

        if (call.callee.type === NodeType.IDENTIFIER_EXPR) {
          const funcName = (call.callee as { name: string }).name;

          if (funcName === 'printf') {
            return this.handlePrintf(call.arguments, context);
          }

          if (funcName === 'scanf') {
            return this.handleScanf(call.arguments, context);
          }
        }
      }

      return {
        success: false,
        error: `IOVM cannot handle node type: ${node.type}`,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  canHandle(node: ASTNode): boolean {
    if (node.type !== NodeType.CALL_EXPR) {
      return false;
    }

    const call = node as { callee: ASTNode };
    if (call.callee.type !== NodeType.IDENTIFIER_EXPR) {
      return false;
    }

    const funcName = (call.callee as { name: string }).name;
    return funcName === 'printf' || funcName === 'scanf';
  }

  private handlePrintf(args: ASTNode[], context: ExecutionContext): ExecutionResult {
    if (args.length === 0) {
      return {
        success: false,
        error: 'printf requires at least one argument',
      };
    }

    // Get format string
    const formatResult = this.evaluateArgument(args[0], context);
    if (!formatResult.success) {
      return formatResult;
    }

    const format = formatResult.value! as string;

    // Evaluate remaining arguments
    const values: ValueType[] = [];
    for (let i = 1; i < args.length; i++) {
      const argResult = this.evaluateArgument(args[i], context);
      if (!argResult.success) {
        return argResult;
      }
      values.push(argResult.value!);
    }

    // Format the output
    const output = this.formatString(format, values);

    // Write to context output
    context.output.push(output);

    return {
      success: true,
      value: output.length, // Return number of characters written
    };
  }

  private handleScanf(args: ASTNode[], context: ExecutionContext): ExecutionResult {
    if (args.length === 0) {
      return {
        success: false,
        error: 'scanf requires at least one argument',
      };
    }

    // Get format string
    const formatResult = this.evaluateArgument(args[0], context);
    if (!formatResult.success) {
      return formatResult;
    }

    const format = formatResult.value! as string;

    // Get input from context
    if (context.input.length === 0) {
      return {
        success: false,
        error: 'No input available for scanf',
      };
    }

    const inputLine = context.input.shift()!;

    // Parse input according to format
    const values = this.parseInput(format, inputLine);

    // Assign values to variables
    for (let i = 0; i < Math.min(values.length, args.length - 1); i++) {
      const arg = args[i + 1];

      // Handle address expressions (e.g., &x)
      if (arg.type === NodeType.ADDRESS_EXPR) {
        const addrExpr = arg as { operand: ASTNode };
        if (addrExpr.operand.type === NodeType.IDENTIFIER_EXPR) {
          const name = (addrExpr.operand as { name: string }).name;
          context.memory.set(name, values[i]);
        }
      }
      // Handle identifier expressions (for simplicity)
      else if (arg.type === NodeType.IDENTIFIER_EXPR) {
        const name = (arg as { name: string }).name;
        context.memory.set(name, values[i]);
      }
    }

    return {
      success: true,
      value: values.length, // Return number of items read
    };
  }

  private evaluateArgument(node: ASTNode, context: ExecutionContext): ExecutionResult {
    // Handle literals
    if (node.type === NodeType.INTEGER_LITERAL) {
      return { success: true, value: (node as { value: number }).value };
    }
    if (node.type === NodeType.FLOAT_LITERAL) {
      return { success: true, value: (node as { value: number }).value };
    }
    if (node.type === NodeType.CHAR_LITERAL) {
      return { success: true, value: (node as { value: string }).value };
    }
    if (node.type === NodeType.STRING_LITERAL) {
      return { success: true, value: (node as { value: string }).value };
    }

    // Handle identifiers
    if (node.type === NodeType.IDENTIFIER_EXPR) {
      const name = (node as { name: string }).name;
      const value = context.memory.get(name);
      if (value === undefined) {
        return {
          success: false,
          error: `Undefined variable: ${name}`,
        };
      }
      return { success: true, value };
    }

    // For other expressions, we'd need to delegate to appropriate VMs
    return {
      success: false,
      error: `Cannot evaluate argument node type: ${node.type}`,
    };
  }

  private formatString(format: string, values: ValueType[]): string {
    let result = '';
    let valueIndex = 0;

    for (let i = 0; i < format.length; i++) {
      const char = format[i];

      if (char === '%' && i + 1 < format.length) {
        const specifier = format[i + 1];
        i++; // Skip the specifier

        if (valueIndex >= values.length) {
          result += '%' + specifier;
          continue;
        }

        const value = values[valueIndex++];

        switch (specifier) {
          case 'd':
          case 'i':
            result += Math.floor(Number(value));
            break;
          case 'f':
            result += Number(value).toFixed(6);
            break;
          case 'c':
            if (typeof value === 'number') {
              result += String.fromCharCode(value);
            } else {
              result += String(value)[0] || '';
            }
            break;
          case 's':
            result += String(value);
            break;
          case 'x':
            result += Math.floor(Number(value)).toString(16);
            break;
          case 'X':
            result += Math.floor(Number(value)).toString(16).toUpperCase();
            break;
          case '%':
            result += '%';
            break;
          default:
            result += '%' + specifier;
        }
      } else {
        result += char;
      }
    }

    return result;
  }

  private parseInput(format: string, input: string): ValueType[] {
    const values: ValueType[] = [];
    const tokens = input.trim().split(/\s+/);
    let tokenIndex = 0;

    for (let i = 0; i < format.length; i++) {
      const char = format[i];

      if (char === '%' && i + 1 < format.length) {
        const specifier = format[i + 1];
        i++; // Skip the specifier

        if (specifier === '%') {
          continue;
        }

        if (tokenIndex >= tokens.length) {
          break;
        }

        const token = tokens[tokenIndex++];

        switch (specifier) {
          case 'd':
          case 'i':
            values.push(parseInt(token, 10));
            break;
          case 'f':
            values.push(parseFloat(token));
            break;
          case 'c':
            values.push(token[0] || '');
            break;
          case 's':
            values.push(token);
            break;
          case 'x':
            values.push(parseInt(token, 16));
            break;
        }
      }
    }

    return values;
  }

  private createIOHandler(): IOHandler {
    const output: string[] = [];
    let input: string[] = [];

    return {
      write(format: string, args: ValueType[]): void {
        output.push(this.formatString(format, args));
      },

      read(format: string): ValueType[] {
        if (input.length === 0) {
          return [];
        }
        const line = input.shift()!;
        return this.parseInput(format, line);
      },

      getOutput(): string[] {
        return [...output];
      },

      clearOutput(): void {
        output.length = 0;
      },

      setInput(newInput: string[]): void {
        input = [...newInput];
      },

      formatString(format: string, args: ValueType[]): string {
        // Same implementation as formatString method
        let result = '';
        let argIndex = 0;

        for (let i = 0; i < format.length; i++) {
          const char = format[i];

          if (char === '%' && i + 1 < format.length) {
            const specifier = format[i + 1];
            i++;

            if (argIndex >= args.length) {
              result += '%' + specifier;
              continue;
            }

            const value = args[argIndex++];

            switch (specifier) {
              case 'd':
              case 'i':
                result += Math.floor(Number(value));
                break;
              case 'f':
                result += Number(value).toFixed(6);
                break;
              case 'c':
                result += String(value)[0] || '';
                break;
              case 's':
                result += String(value);
                break;
              case '%':
                result += '%';
                break;
              default:
                result += '%' + specifier;
            }
          } else {
            result += char;
          }
        }

        return result;
      },

      parseInput(format: string, input: string): ValueType[] {
        // Same implementation as parseInput method
        const values: ValueType[] = [];
        const tokens = input.trim().split(/\s+/);
        let tokenIndex = 0;

        for (let i = 0; i < format.length; i++) {
          const char = format[i];

          if (char === '%' && i + 1 < format.length) {
            const specifier = format[i + 1];
            i++;

            if (specifier === '%') {
              continue;
            }

            if (tokenIndex >= tokens.length) {
              break;
            }

            const token = tokens[tokenIndex++];

            switch (specifier) {
              case 'd':
              case 'i':
                values.push(parseInt(token, 10));
                break;
              case 'f':
                values.push(parseFloat(token));
                break;
              case 'c':
                values.push(token[0] || '');
                break;
              case 's':
                values.push(token);
                break;
            }
          }
        }

        return values;
      },
    };
  }

  /**
   * Get the IO handler
   */
  getIOHandler(): IOHandler {
    return this.ioHandler;
  }

  /**
   * Set the IO handler
   */
  setIOHandler(handler: IOHandler): void {
    this.ioHandler = handler;
  }
}