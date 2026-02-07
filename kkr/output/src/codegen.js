/**
 * Generador de código
 * Convierte el AST de C a JavaScript equivalente
 */

class CodeGenerator {
  constructor() {
    this.indent = 0;
    this.output = '';
    this.structs = new Map(); // Registro de structs definidos
  }

  /**
   * Genera código desde un nodo AST
   */
  generate(node) {
    if (!node) return '';
    
    switch (node.type) {
      case 'Program':
        return this.generateProgram(node);
      case 'StructDeclaration':
        return this.generateStructDeclaration(node);
      case 'FunctionDeclaration':
        return this.generateFunctionDeclaration(node);
      case 'VariableDeclaration':
        return this.generateVariableDeclaration(node);
      case 'CompoundStatement':
        return this.generateCompoundStatement(node);
      case 'IfStatement':
        return this.generateIfStatement(node);
      case 'WhileStatement':
        return this.generateWhileStatement(node);
      case 'ForStatement':
        return this.generateForStatement(node);
      case 'ReturnStatement':
        return this.generateReturnStatement(node);
      case 'ExpressionStatement':
        return this.generateExpressionStatement(node);
      case 'AssignmentExpression':
        return this.generateAssignmentExpression(node);
      case 'BinaryExpression':
        return this.generateBinaryExpression(node);
      case 'UnaryExpression':
        return this.generateUnaryExpression(node);
      case 'ConditionalExpression':
        return this.generateConditionalExpression(node);
      case 'CallExpression':
        return this.generateCallExpression(node);
      case 'MemberExpression':
        return this.generateMemberExpression(node);
      case 'Identifier':
        return this.generateIdentifier(node);
      case 'IntLiteral':
        return this.generateIntLiteral(node);
      case 'FloatLiteral':
        return this.generateFloatLiteral(node);
      case 'StringLiteral':
        return this.generateStringLiteral(node);
      case 'CharLiteral':
        return this.generateCharLiteral(node);
      default:
        throw new Error('Unknown node type: ' + node.type);
    }
  }

  /**
   * Genera código para el programa completo
   */
  generateProgram(node) {
    let code = '// Generated C to JavaScript code\n';
    code += '(function() {\n';
    code += '  "use strict";\n\n';

    // Primero registrar structs
    for (const decl of node.declarations) {
      if (decl.type === 'StructDeclaration') {
        this.structs.set(decl.name, decl.fields);
      }
    }

    // Generar código de structs como clases
    for (const decl of node.declarations) {
      if (decl.type === 'StructDeclaration') {
        code += this.generate(decl) + '\n';
      }
    }

    // Generar resto de declaraciones
    for (const decl of node.declarations) {
      if (decl.type !== 'StructDeclaration') {
        code += this.generate(decl) + '\n';
      }
    }

    code += '})();\n';
    return code;
  }

  /**
   * Genera código para declaración de struct
   */
  generateStructDeclaration(node) {
    let code = `  class ${node.name} {\n`;
    code += `    constructor() {\n`;
    
    for (const field of node.fields) {
      const jsType = this.cTypeToJS(field.type);
      code += `      this.${field.name} = ${jsType};\n`;
    }
    
    code += `    }\n`;
    code += `  }\n\n`;
    
    return code;
  }

  /**
   * Genera código para declaración de función
   */
  generateFunctionDeclaration(node) {
    let code = '';
    
    // Si es main, usar nombre especial
    const funcName = node.name === 'main' ? '__main' : node.name;
    
    code += `  function ${funcName}(`;
    
    // Parámetros
    const params = node.params.map(p => p.name);
    code += params.join(', ');
    
    code += ') {\n';
    this.indent++;
    
    // Cuerpo
    code += this.generate(node.body);
    
    this.indent--;
    code += '  }\n\n';
    
    return code;
  }

  /**
   * Genera código para declaración de variable
   */
  generateVariableDeclaration(node) {
    let code = this.getIndent();
    
    // Usar let/const según si hay inicialización
    const keyword = node.init ? 'let' : 'let';
    code += `${keyword} ${node.name}`;
    
    if (node.init) {
      code += ' = ' + this.generate(node.init);
    }
    
    code += ';\n';
    return code;
  }

  /**
   * Genera código para bloque compuesto
   */
  generateCompoundStatement(node) {
    let code = this.getIndent() + '{\n';
    this.indent++;
    
    for (const stmt of node.statements) {
      code += this.generate(stmt);
    }
    
    this.indent--;
    code += this.getIndent() + '}\n';
    
    return code;
  }

  /**
   * Genera código para if/else
   */
  generateIfStatement(node) {
    let code = this.getIndent();
    code += 'if (' + this.generate(node.condition) + ') ';
    
    if (node.then.type === 'CompoundStatement') {
      code += this.generate(node.then);
    } else {
      code += '{\n';
      this.indent++;
      code += this.generate(node.then);
      this.indent--;
      code += this.getIndent() + '}\n';
    }
    
    if (node.else) {
      code += this.getIndent() + 'else ';
      
      if (node.else.type === 'CompoundStatement') {
        code += this.generate(node.else);
      } else if (node.else.type === 'IfStatement') {
        code += this.generate(node.else);
      } else {
        code += '{\n';
        this.indent++;
        code += this.generate(node.else);
        this.indent--;
        code += this.getIndent() + '}\n';
      }
    }
    
    return code;
  }

  /**
   * Genera código para while
   */
  generateWhileStatement(node) {
    let code = this.getIndent();
    code += 'while (' + this.generate(node.condition) + ') ';
    
    if (node.body.type === 'CompoundStatement') {
      code += this.generate(node.body);
    } else {
      code += '{\n';
      this.indent++;
      code += this.generate(node.body);
      this.indent--;
      code += this.getIndent() + '}\n';
    }
    
    return code;
  }

  /**
   * Genera código para for
   */
  generateForStatement(node) {
    let code = this.getIndent();
    code += 'for (';
    
    if (node.init) {
      if (node.init.type === 'VariableDeclaration') {
        // Declaración dentro del for
        code += 'let ' + node.init.name;
        if (node.init.init) {
          code += ' = ' + this.generate(node.init.init);
        }
      } else {
        code += this.generate(node.init);
      }
    }
    
    code += '; ';
    
    if (node.condition) {
      code += this.generate(node.condition);
    }
    
    code += '; ';
    
    if (node.update) {
      code += this.generate(node.update);
    }
    
    code += ') ';
    
    if (node.body.type === 'CompoundStatement') {
      code += this.generate(node.body);
    } else {
      code += '{\n';
      this.indent++;
      code += this.generate(node.body);
      this.indent--;
      code += this.getIndent() + '}\n';
    }
    
    return code;
  }

  /**
   * Genera código para return
   */
  generateReturnStatement(node) {
    let code = this.getIndent();
    code += 'return';
    
    if (node.value) {
      code += ' ' + this.generate(node.value);
    }
    
    code += ';\n';
    return code;
  }

  /**
   * Genera código para instrucción de expresión
   */
  generateExpressionStatement(node) {
    return this.getIndent() + this.generate(node.expression) + ';\n';
  }

  /**
   * Genera código para asignación
   */
  generateAssignmentExpression(node) {
    let left = this.generate(node.left);
    let right = this.generate(node.right);
    
    // Manejar operadores compuestos
    switch (node.operator) {
      case '+=':
        return `${left} += ${right}`;
      case '-=':
        return `${left} -= ${right}`;
      case '*=':
        return `${left} *= ${right}`;
      case '/=':
        return `${left} /= ${right}`;
      default:
        return `${left} = ${right}`;
    }
  }

  /**
   * Genera código para expresión binaria
   */
  generateBinaryExpression(node) {
    const left = this.generate(node.left);
    const right = this.generate(node.right);
    
    // Operadores bit a bit necesitan manejo especial en JS
    switch (node.operator) {
      case '<<':
        return `(${left} << ${right})`;
      case '>>':
        return `(${left} >> ${right})`;
      case '&&':
        return `(${left} && ${right})`;
      case '||':
        return `(${left} || ${right})`;
      default:
        return `(${left} ${node.operator} ${right})`;
    }
  }

  /**
   * Genera código para expresión unaria
   */
  generateUnaryExpression(node) {
    const operand = this.generate(node.operand);
    
    if (node.postfix) {
      return `${operand}${node.operator}`;
    }
    
    switch (node.operator) {
      case '*':
        // Desreferenciación de puntero (simplificado)
        return operand;
      case '&':
        // Dirección de (simplificado - ignoramos)
        return operand;
      default:
        return `${node.operator}${operand}`;
    }
  }

  /**
   * Genera código para expresión condicional
   */
  generateConditionalExpression(node) {
    const condition = this.generate(node.condition);
    const thenExpr = this.generate(node.then);
    const elseExpr = this.generate(node.else);
    
    return `(${condition} ? ${thenExpr} : ${elseExpr})`;
  }

  /**
   * Genera código para llamada a función
   */
  generateCallExpression(node) {
    const callee = this.generate(node.callee);
    const args = node.arguments.map(arg => this.generate(arg));
    
    // Manejo especial para printf
    if (node.callee.type === 'Identifier' && node.callee.name === 'printf') {
      return this.generatePrintf(node.arguments);
    }
    
    // Manejo especial para main
    if (node.callee.type === 'Identifier' && node.callee.name === 'main') {
      return `__main(${args.join(', ')})`;
    }
    
    return `${callee}(${args.join(', ')})`;
  }

  /**
   * Genera código para printf
   */
  generatePrintf(args) {
    if (args.length === 0) {
      return 'console.log()';
    }
    
    const format = args[0];
    let jsCode = 'console.log(';
    
    if (format.type === 'StringLiteral') {
      let formatStr = format.value;
      let argIndex = 1;
      const replacements = [];
      
      // Reemplazar especificadores de formato
      formatStr = formatStr.replace(/%[sdifc]/g, (match) => {
        if (argIndex < args.length) {
          const arg = this.generate(args[argIndex++]);
          replacements.push(arg);
          return '${' + (replacements.length - 1) + '}';
        }
        return match;
      });
      
      // Escapar comillas dobles y backticks
      formatStr = formatStr.replace(/`/g, '\\`').replace(/\$/g, '\\$');
      jsCode += '`' + formatStr + '`';
      
      // Agregar argumentos adicionales
      for (let i = argIndex; i < args.length; i++) {
        jsCode += ', ' + this.generate(args[i]);
      }
    } else {
      jsCode += args.map(arg => this.generate(arg)).join(', ');
    }
    
    jsCode += ')';
    return jsCode;
  }

  /**
   * Genera código para acceso a miembro
   */
  generateMemberExpression(node) {
    const object = this.generate(node.object);
    return `${object}.${node.property}`;
  }

  /**
   * Genera código para identificador
   */
  generateIdentifier(node) {
    return node.name;
  }

  /**
   * Genera código para literal entero
   */
  generateIntLiteral(node) {
    return node.value.toString();
  }

  /**
   * Genera código para literal flotante
   */
  generateFloatLiteral(node) {
    return node.value.toString();
  }

  /**
   * Genera código para literal string
   */
  generateStringLiteral(node) {
    // Escapar caracteres especiales
    let escaped = node.value
      .replace(/\\/g, '\\\\')
      .replace(/"/g, '\\"')
      .replace(/\n/g, '\\n')
      .replace(/\t/g, '\\t')
      .replace(/\r/g, '\\r');
    
    return `"${escaped}"`;
  }

  /**
   * Genera código para literal char
   */
  generateCharLiteral(node) {
    // En JS, los chars son strings de longitud 1
    let escaped = node.value
      .replace(/\\/g, '\\\\')
      .replace(/'/g, "\\'")
      .replace(/\n/g, '\\n')
      .replace(/\t/g, '\\t')
      .replace(/\r/g, '\\r');
    
    return `"${escaped}"`;
  }

  /**
   * Convierte tipo C a valor inicial JS
   */
  cTypeToJS(type) {
    switch (type.baseType) {
      case 'int':
        return '0';
      case 'float':
        return '0.0';
      case 'char':
        return `'\\0'`;
      case 'void':
        return 'null';
      default:
        if (this.structs.has(type.baseType)) {
          return 'null';
        }
        return 'null';
    }
  }

  /**
   * Obtiene indentación actual
   */
  getIndent() {
    return '  '.repeat(this.indent);
  }
}

module.exports = CodeGenerator;