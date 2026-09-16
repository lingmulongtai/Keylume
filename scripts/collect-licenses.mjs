import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';

// Collect license texts from the exact installed/locked dependency versions.
const metadata = JSON.parse(
  execFileSync(
    'cargo',
    [
      'metadata',
      '--manifest-path',
      'src-tauri/Cargo.toml',
      '--locked',
      '--format-version',
      '1',
      '--filter-platform',
      'x86_64-pc-windows-msvc',
    ],
    { encoding: 'utf8', maxBuffer: 32 * 1024 * 1024 },
  ),
);
const resolved = new Set(metadata.resolve.nodes.map((node) => node.id));
const overrides = JSON.parse(readFileSync('licenses/upstream/sources.json', 'utf8'));
const packages = [];
const missing = [];
function licenseFiles(root) {
  const result = [];
  function walk(directory, depth) {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const path = join(directory, entry.name);
      if (entry.isFile() && /^(licen[cs]e|notice|copying|copyright)([._-]|$)/i.test(entry.name)) {
        result.push(path);
      } else if (
        entry.isDirectory() &&
        depth < 2 &&
        /^(licenses?|legal|third.party.licenses?)$/i.test(entry.name)
      ) {
        walk(path, depth + 1);
      }
    }
  }
  walk(root, 0);
  return result.sort();
}
function add(kind, name, version, root, license, source) {
  const files = licenseFiles(root);
  const texts = files.map((file) => ({
    name: relative(root, file).replaceAll('\\', '/'),
    text: readFileSync(file, 'utf8'),
  }));
  for (const extra of overrides[`${kind}:${name}@${version}`] ?? []) {
    texts.push({
      name: extra.url,
      text: readFileSync(join('licenses/upstream', extra.file), 'utf8'),
    });
  }
  if (!texts.length) missing.push(`${kind}:${name}@${version} (${license ?? 'unknown license'})`);
  packages.push({ kind, name, version, license, source, texts });
}
for (const pkg of metadata.packages.filter(
  (pkg) => resolved.has(pkg.id) && pkg.name !== 'keylume',
)) {
  add(
    'cargo',
    pkg.name,
    pkg.version,
    dirname(pkg.manifest_path),
    pkg.license,
    pkg.repository ?? `https://crates.io/crates/${pkg.name}/${pkg.version}`,
  );
}
const lock = JSON.parse(readFileSync('package-lock.json', 'utf8'));
for (const [path, entry] of Object.entries(lock.packages)) {
  if (!path || entry.dev || !existsSync(join(path, 'package.json'))) continue;
  const pkg = JSON.parse(readFileSync(join(path, 'package.json'), 'utf8'));
  add(
    'npm',
    pkg.name,
    pkg.version,
    path,
    pkg.license,
    `https://www.npmjs.com/package/${pkg.name}/v/${pkg.version}`,
  );
}
if (missing.length) throw new Error(`Missing license texts:\n${missing.join('\n')}`);
packages.sort((a, b) =>
  `${a.kind}:${a.name}@${a.version}`.localeCompare(`${b.kind}:${b.name}@${b.version}`, 'en'),
);
const output = [
  'Keylume third-party license notices',
  'Generated from Cargo.lock (Windows dependency graph, including build tools) and production npm dependencies.',
  'Upstream packages are unmodified. Build dependencies are included conservatively.',
  'This document does not grant a license to Keylume application code.',
  '',
];
for (const pkg of packages) {
  output.push(
    '='.repeat(80),
    `${pkg.kind}: ${pkg.name} ${pkg.version}`,
    `License: ${pkg.license ?? 'See license text'}`,
    `Source: ${pkg.source}`,
    '',
  );
  for (const text of pkg.texts)
    output.push(`--- ${text.name} ---`, text.text.replaceAll('\r\n', '\n').trim(), '');
}
writeFileSync(
  'licenses/THIRD-PARTY-LICENSES.txt',
  output.join('\n').replace(/[ \t]+$/gm, '').trimEnd() + '\n',
);
console.log(`Collected license texts for ${packages.length} packages.`);
