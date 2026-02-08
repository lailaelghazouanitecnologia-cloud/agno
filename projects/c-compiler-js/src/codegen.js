/**
 * Code Generator for C to JavaScript transpilation
 * Converts AST to valid JavaScript code
 */

class CodeGenerator {
  constructor() {
    this.indent = 0;
    this.output = '';
  }

  /**
   * Generate JavaScript code from AST
   */
  generate(ast) {
    this.output = '';
    this.indent = 0;
    
    this.visitProgram(ast);
    
    return this.output;
  }

  emit(str) {
    this.output += str;
  }

  emitLine(str = '') {
    this.output += '  '.repeat(this.indent) + str + '\n';
  }

  indentIn() {
    this.indent++;
  }

  indentOut() {
    this.indent--;
  }

  visitProgram(node) {
    this.emitLine('// Transpiled from C');
    this.emitLine('');
    
    // Generate all declarations
    for (const decl of node.declarations) {
      this.visitDeclaration(decl);
      this.emitLine('');
    }
  }

  visitDeclaration(node) {
    switch (node.type) {
      case 'FunctionDeclaration':
        this.visitFunctionDeclaration(node);
        break;
      case 'VariableDeclaration':
        this.visitVariableDeclaration(node);
        break;
      default:
        throw new Error(`Unknown declaration type: ${node.type}`);
    }
  }

  visitFunctionDeclaration(node) {
    // Generate function
    this.emit('function ');
    this.emit(node.name);
    this.emit('(');
    
    // Parameters
    for (let i = 0; i < node.params.length; i++) {
      if (i > 0) this.emit(', ');
      this.emit(node.params[i].name);
    }
    
    this.emit(') ');
    this.emitLine('{');
    this.indentIn();
    
    // Function body
    this.visitCompoundStatement(node.body);
    
    this.indentOut();
    this.emitLine('}');
  }

  visitVariableDeclaration(node) {
    if (node.isArray) {
      // Array declaration
      this.emit('let ');
      this.emit(node.name);
      this.emit(' = ');
      
      if (node.arraySize) {
        this.emit('new Array(');
        this.visitExpression(node.arraySize);
        this.emit(').fill(0)');
      } else {
        this.emit('[]');
      }
      
      if (node.init) {
        this.emit('; ');
        this.emit(node.name);
        this.emit(' = ');
        this.visitExpression(node.init);
      }
    } else {
      // Regular variable declaration
      this.emit('let ');
      this.emit(node.name);
      
      if (node.init) {
        this.emit(' = ');
        this.visitExpression(node.init);
      } else {
        this.emit(' = 0');
      }
    }
    
    this.emitLine(';');
  }

  visitCompoundStatement(node) {
    for (const stmt of node.statements) {
      this.visitStatement(stmt);
    }
  }

  visitStatement(node) {
    switch (node.type) {
      case 'VariableDeclaration':
        this.visitVariableDeclaration(node);
        break;
      case 'ExpressionStatement':
        this.visitExpressionStatement(node);
        break;
      case 'IfStatement':
        this.visitIfStatement(node);
        break;
      case 'WhileStatement':
        this.visitWhileStatement(node);
        break;
      case 'ForStatement':
        this.visitForStatement(node);
        break;
      case 'ReturnStatement':
        this.visitReturnStatement(node);
        break;
      case 'CompoundStatement':
        this.emitLine('{');
        this.indentIn();
        this.visitCompoundStatement(node);
        this.indentOut();
        this.emitLine('}');
        break;
      default:
        throw new Error(`Unknown statement type: ${node.type}`);
    }
  }

  visitExpressionStatement(node) {
    this.visitExpression(node.expression);
    this.emitLine(';');
  }

  visitIfStatement(node) {
    this.emit('if (');
    this.visitExpression(node.condition);
    this.emit(') ');
    
    if (node.thenBranch.type === 'CompoundStatement') {
      this.visitStatement(node.thenBranch);
    } else {
      this.emitLine('{');
      this.indentIn();
      this.visitStatement(node.thenBranch);
      this.indentOut();
      this.emitLine('}');
    }
    
    if (node.elseBranch) {
      this.emit('else ');
      
      if (node.elseBranch.type === 'CompoundStatement') {
        this.visitStatement(node.elseBranch);
      } else {
        this.emitLine('{');
        this.indentIn();
        this.visitStatement(node.elseBranch);
        this.indentOut();
        this.emitLine('}');
      }
    }
  }

  visitWhileStatement(node) {
    this.emit('while (');
    this.visitExpression(node.condition);
    this.emit(') ');
    
    if (node.body.type === 'CompoundStatement') {
      this.visitStatement(node.body);
    } else {
      this.emitLine('{');
      this.indentIn();
      this.visitStatement(node.body);
      this.indentOut();
      this.emitLine('}');
    }
  }

  visitForStatement(node) {
    this.emit('for (');
    
    // Init
    if (node.init) {
      if (node.init.type === 'VariableDeclaration') {
        this.emit('let ');
        this.emit(node.init.name);
        if (node.init.init) {
          this.emit(' = ');
          this.visitExpression(node.init.init);
        }
      } else {
        this.visitExpression(node.init);
      }
    }
    this.emit('; ');
    
    // Condition
    if (node.condition) {
      this.visitExpression(node.condition);
    }
    this.emit('; ');
    
    // Update
    if (node.update) {
      this.visitExpression(node.update);
    }
    
    this.emit(') ');
    
    if (node.body.type === 'CompoundStatement') {
      this.visitStatement(node.body);
    } else {
      this.emitLine('{');
      this.indentIn();
      this.visitStatement(node.body);
      this.indentOut();
      this.emitLine('}');
    }
  }

  visitReturnStatement(node) {
    this.emit('return');
    if (node.value) {
      this.emit(' ');
      this.visitExpression(node.value);
    }
    this.emitLine(';');
  }

  visitExpression(node) {
    switch (node.type) {
      case 'Number':
        this.emit(String(node.value));
        break;
      case 'String':
        this.emit(JSON.stringify(node.value));
        break;
      case 'Char':
        // Convert char to its ASCII value
        this.emit(String(node.value.charCodeAt(0)));
        break;
      case 'Identifier':
        this.emit(node.name);
        break;
      case 'Binary':
        this.visitBinary(node);
        break;
      case 'Unary':
        this.visitUnary(node);
        break;
      case 'Assignment':
        this.visitAssignment(node);
        break;
      case 'Call':
        this.visitCall(node);
        break;
      case 'ArrayAccess':
        this.visitArrayAccess(node);
        break;
      case 'Postfix':
        this.visitPostfix(node);
        break;
      default:
        throw new Error(`Unknown expression type: ${node.type}`);
    }
  }

  visitBinary(node) {
    this.emit('(');
    this.visitExpression(node.left);
    this.emit(' ');
    this.emit(node.operator);
    this.emit(' ');
    this.visitExpression(node.right);
    this.emit(')');
  }

  visitUnary(node) {
    if (node.operator === '&') {
      // Address-of operator - in JS we just use the variable name
      this.visitExpression(node.operand);
    } else if (node.operator === '*') {
      // Dereference operator - in JS we just use the variable name
      this.visitExpression(node.operand);
    } else {
      this.emit('(');
      this.emit(node.operator);
      this.visitExpression(node.operand);
      this.emit(')');
    }
  }

  visitAssignment(node) {
    this.visitExpression(node.target);
    this.emit(' = ');
    this.visitExpression(node.value);
  }

  visitCall(node) {
    // Check if it's printf
    if (node.callee.type === 'Identifier' && node.callee.name === 'printf') {
      this.emit('print(');
      
      // Handle format string and arguments
      if (node.arguments.length > 0) {
        const formatArg = node.arguments[0];
        
        if (formatArg.type === 'String') {
          let format = formatArg.value;
          let argIndex = 1;
          
          // Convert C format specifiers to JS
          format = format.replace(/%d/g, () => {
            if (argIndex < node.arguments.length) {
              const arg = node.arguments[argIndex++];
              return '${' + this.expressionToString(arg) + '}';
            }
            return '%d';
          });
          
          format = format.replace(/%s/g, () => {
            if (argIndex < node.arguments.length) {
              const arg = node.arguments[argIndex++];
              return '${String(' + this.expressionToString(arg) + ')}';
            }
            return '%s';
          });
          
          format = format.replace(/%c/g, () => {
            if (argIndex < node.arguments.length) {
              const arg = node.arguments[argIndex++];
              return '${String.fromCharCode(' + this.expressionToString(arg) + ')}';
            }
            return '%c';
          });
          
          format = format.replace(/%%/g, '%');
          
          this.emit('`' + format + '`');
        } else {
          this.visitExpression(formatArg);
        }
      }
      
      this.emit(')');
    } else {
      // Regular function call
      this.visitExpression(node.callee);
      this.emit('(');
      
      for (let i = 0; i < node.arguments.length; i++) {
        if (i > 0) this.emit(', ');
        this.visitExpression(node.arguments[i]);
      }
      
      this.emit(')');
    }
  }

  visitArrayAccess(node) {
    this.visitExpression(node.array);
    this.emit('[');
    this.visitExpression(node.index);
    this.emit(']');
  }

  visitPostfix(node) {
    this.visitExpression(node.operand);
    this.emit(node.operator);
  }

  // Helper to convert expression to string representation
  expressionToString(node) {
    switch (node.type) {
      case 'Number':
        return String(node.value);
      case 'String':
        return JSON.stringify(node.value);
      case 'Char':
        return String(node.value.charCodeAt(0));
      case 'Identifier':
        return node.name;
      case 'Binary':
        return `(${this.expressionToString(node.left)} ${node.operator} ${this.expressionToString(node.right)})`;
      case 'Unary':
        return `${node.operator}${this.expressionToString(node.operand)}`;
      case 'ArrayAccess':
        return `${this.expressionToString(node.array)}[${this.expressionToString(node.index)}]`;
      case 'Call':
        let args = node.arguments.map(a => this.expressionToString(a)).join(', ');
        return `${this.expressionToString(node.callee)}(${args})`;
      case 'Assignment':
        return `${this.expressionToString(node.target)} = ${this.expressionToString(node.value)}`;
      case 'Postfix':
        return `${this.expressionToString(node.operand)}${node.operator}`;
      default:
        throw new Error(`Unknown expression type: ${node.type}`);
    }
  }
}

module.exports = CodeGenerator;