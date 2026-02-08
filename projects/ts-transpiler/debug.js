const { transpile } = require("./dist/index");

const tsCode = `let x: number = 5;`;

try {
  const jsCode = transpile(tsCode);
  console.log("Input:", tsCode);
  console.log("Output:", jsCode);
} catch (error) {
  console.error("Error:", error.message);
  console.error(error.stack);
}