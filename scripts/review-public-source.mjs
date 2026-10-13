import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
const root = process.cwd();
const files = execFileSync('git', ['ls-files', '-z'], { encoding: 'utf8' }).split('\0').filter(Boolean);
const blocked = [];
const prohibited = /\.(?:pdf|pvr|db|sqlite3?|bak|exe|dll|dump|log|fic|ndx|mmo|wdd|wdl|7z|zip)$/i;
const secret = /(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{40,}|-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----)/;
for (const file of files) {
  if (prohibited.test(file) || /(?:^|\/)(?:verification|private|node_modules|target|output|test-results)(?:\/|$)/.test(file) || /(?:^|\/)\.env(?:$|\.)/.test(file)) {
    blocked.push({ file, reason: 'private/generated/binary path is not public source' }); continue;
  }
  const bytes = fs.readFileSync(path.join(root, file));
  if (/\.(?:ttf|woff2|ico|png)$/.test(file)) continue;
  const text = bytes.toString('utf8');
  if (secret.test(text)) blocked.push({ file, reason: 'credential/private-key pattern' });
  if (/C:\\Users\\|C:\/Users\/|OneDrive[\\/]|Downloads[\\/]previus[\\/]/i.test(text)) blocked.push({ file, reason: 'personal local path' });
}
console.log(JSON.stringify({ trackedFiles: files.length, findings: blocked, status: blocked.length ? 'blocked' : 'passed' }, null, 2));
if (!files.length || blocked.length) process.exitCode = 1;
