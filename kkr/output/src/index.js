#!/usr/bin/env node

/**
 * Entry point del compilador de C a JavaScript
 * Conecta: Lexer -> Parser -> CodeGenerator
 */

const fs = require('fs');
const path = require('path');
const Lexer = require('./lexer');
const Parser = require('./parser');
const CodeGenerator = require('./codegen');

/**
 * Compila código C a JavaScript
 */
function compile(sourceCode, options = {}) {
  try {
    // 1. Lexer: Tokenizar el código C
    if (options.verbose) {
      console.error('=== LEXER ===');
    }
    const lexer = new Lexer(sourceCode);
    const tokens = lexer.tokenize();
    
    if (options.verbose) {
      console.error(`Tokens generados: ${tokens.length}`);
      tokens.slice(0, 20).forEach(t => {
        console.error(`  ${t.type.padEnd(20)} ${t.value}`);
      });
      if (tokens.length > 20) {
        console.error('  ...');
      }
      console.error('');
    }

    // 2. Parser: Construir el AST
    if (options.verbose) {
      console.error('=== PARSER ===');
    }
    const parser = new Parser(tokens);
    const ast = parser.parse();
    
    if (options.verbose) {
      console.error('AST generado:');
      console.error(JSON.stringify(ast, null, 2));
      console.error('');
    }

    // 3. CodeGenerator: Generar JavaScript
    if (options.verbose) {
      console.error('=== CODE GENERATOR ===');
    }
    const codegen = new CodeGenerator();
    const jsCode = codegen.generate(ast);
    
    if (options.verbose) {
      console.error('Código JavaScript generado:');
      console.error('---');
    }
    
    return jsCode;
  } catch (error) {
    throw new Error(`Error de compilación: ${error.message}`);
  }
}

/**
 * Ejecuta código JavaScript generado
 */
function execute(jsCode) {
  try {
    // Envolver en una función para capturar el resultado
    const wrapped = `
      ${jsCode}
      return typeof __main !== 'undefined' ? __main() : undefined;
    `;
    
    const fn = new Function(wrapped);
    return fn();
  } catch (error) {
    throw new Error(`Error de ejecución: ${error.message}`);
  }
}

/**
 * Lee código desde stdin
 */
function readFromStdin() {
  return new Promise((resolve, reject) => {
    let data = '';
    
    process.stdin.setEncoding('utf8');
    process.stdin.on('data', chunk => {
      data += chunk;
    });
    
    process.stdin.on('end', () => {
      resolve(data);
    });
    
    process.stdin.on('error', reject);
  });
}

/**
 * Lee código desde archivo
 */
function readFromFile(filePath) {
  try {
    return fs.readFileSync(filePath, 'utf8');
  } catch (error) {
    throw new Error(`No se puede leer el archivo: ${filePath}`);
  }
}

/**
 * Main
 */
async function main() {
  const args = process.argv.slice(2);
  const options = {
    verbose: false,
    execute: true,
    output: null
  };

  // Parsear argumentos
  let inputFile = null;
  
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    
    if (arg === '-v' || arg === '--verbose') {
      options.verbose = true;
    } else if (arg === '-o' || arg === '--output') {
      options.output = args[++i];
    } else if (arg === '-c' || arg === '--compile-only') {
      options.execute = false;
    } else if (arg === '-h' || arg === '--help') {
      printHelp();
      process.exit(0);
    } else if (arg.startsWith('-')) {
      console.error(`Opción desconocida: ${arg}`);
      process.exit(1);
    } else {
      inputFile = arg;
    }
  }

  // Leer código fuente
  let sourceCode;
  if (inputFile) {
    sourceCode = readFromFile(inputFile);
  } else if (!process.stdin.isTTY) {
    sourceCode = await readFromStdin();
  } else {
    console.error('Uso: c-compiler-js [opciones] <archivo.c>');
    console.error('       echo "int main() { return 0; }" | c-compiler-js');
    console.error('');
    console.error('Opciones:');
    console.error('  -v, --verbose      Mostrar información detallada');
    console.error('  -o, --output FILE  Guardar código JS en archivo');
    console.error('  -c, --compile-only Solo compilar, no ejecutar');
    console.error('  -h, --help         Mostrar ayuda');
    process.exit(1);
  }

  // Compilar
  try {
    const jsCode = compile(sourceCode, options);
    
    // Output
    if (options.output) {
      fs.writeFileSync(options.output, jsCode, 'utf8');
      console.error(`Código guardado en: ${options.output}`);
    } else {
      console.log(jsCode);
    }
    
    // Ejecutar si se solicita
    if (options.execute && !options.output) {
      console.error('\n=== EJECUTANDO ===');
      const result = execute(jsCode);
      if (result !== undefined) {
        console.log(`\nResultado: ${result}`);
      }
    }
    
  } catch (error) {
    console.error(error.message);
    process.exit(1);
  }
}

/**
 * Imprime ayuda
 */
function printHelp() {
  console.log('Compilador de C a JavaScript');
  console.log('');
  console.log('Uso:');
  console.log('  c-compiler-js [opciones] <archivo.c>');
  console.log('  cat archivo.c | c-compiler-js [opciones]');
  console.log('');
  console.log('Opciones:');
  console.log('  -v, --verbose      Mostrar información detallada del proceso');
  console.log('  -o, --output FILE  Guardar código JavaScript en archivo');
  console.log('  -c, --compile-only Solo compilar, no ejecutar el código');
  console.log('  -h, --help         Mostrar este mensaje de ayuda');
  console.log('');
  console.log('Ejemplos:');
  console.log('  c-compiler-js examples/hello.c');
  console.log('  c-compiler-js -v examples/fibonacci.c');
  console.log('  c-compiler-js -o output.js examples/hello.c');
  console.log('  echo "int main() { return 42; }" | c-compiler-js');
}

// Exportar para uso como módulo
module.exports = {
  compile,
  execute
};

// Ejecutar si se llama directamente
if (require.main === module) {
  main();
}