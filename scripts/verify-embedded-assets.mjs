import fs from 'node:fs';
import path from 'node:path';

const exePath = process.argv[2] || 'target/release/previus-next-desktop.exe';
const outputPath = process.argv[3];
if (!fs.existsSync(exePath)) throw Error('Executable not found: ' + exePath);
const binary = fs.readFileSync(exePath);
const assets = fs.readdirSync('apps/desktop/dist/assets').filter(name => /\.(js|css)$/.test(name));
if (!assets.some(name => name.endsWith('.js')) || !assets.some(name => name.endsWith('.css'))) {
  throw Error('apps/desktop/dist/assets has no built JS/CSS');
}
const checks = assets.map(name => ({ name, embedded: binary.includes(Buffer.from(name)) }));
const missing = checks.filter(c => !c.embedded);
if (missing.length > 0) {
  throw Error('Release binary does not contain built frontend assets: ' + missing.map(m => m.name).join(', '));
}
const result = {
  status: 'passed',
  executable: exePath,
  bytes: binary.length,
  assets: checks,
  verifiedAt: new Date().toISOString()
};
if (outputPath) {
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, JSON.stringify(result, null, 2));
}
console.log(JSON.stringify(result));
