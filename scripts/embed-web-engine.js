const fs = require('fs');
const path = require('path');

const repo = path.resolve(__dirname, '..');
const web = path.join(repo, 'web');
const wasm = path.join(repo, 'target', 'wasm32-unknown-unknown', 'release', 'previus_wasm.wasm');
const fixture = path.join(repo, 'crates', 'previus-rgps', 'tests', 'fixtures', 'synthetic-300-months.json');

if (!fs.existsSync(wasm)) {
  console.error('wasm not found: ' + wasm);
  console.error('run: cargo build -p previus-wasm --target wasm32-unknown-unknown --release');
  process.exit(1);
}
if (!fs.existsSync(fixture)) {
  console.error('fixture not found: ' + fixture);
  process.exit(1);
}

fs.copyFileSync(wasm, path.join(web, 'engine.wasm'));
const base64 = fs.readFileSync(wasm).toString('base64');
fs.writeFileSync(path.join(web, 'engine.data.js'), 'window.PIO_WASM_BASE64 = "' + base64 + '";\n', 'utf8');

const exampleText = fs.readFileSync(fixture, 'utf8');
const example = JSON.parse(exampleText);
fs.writeFileSync(path.join(web, 'engine.example.json'), exampleText, 'utf8');
fs.writeFileSync(path.join(web, 'engine.example.js'), 'window.PIO_ENGINE_EXAMPLE = ' + JSON.stringify(example) + ';\n', 'utf8');

console.log('engine.wasm ' + fs.statSync(wasm).size + ' bytes');
console.log('base64 ' + base64.length + ' chars');
console.log('example ' + exampleText.length + ' bytes');
