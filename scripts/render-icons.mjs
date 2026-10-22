import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { Resvg } from '@resvg/resvg-js';

const render = (source, target, width) => {
  const png = new Resvg(readFileSync(source), { fitTo: { mode: 'width', value: width } }).render().asPng();
  writeFileSync(target, png);
};

if (process.argv.includes('--prune')) {
  // Desktop only: `tauri icon` also writes the Android and iOS sets.
  rmSync('src-tauri/icons/android', { recursive: true, force: true });
  rmSync('src-tauri/icons/ios', { recursive: true, force: true });
  process.exit(0);
}

mkdirSync('icons-build', { recursive: true });
render('assets/icon.svg', 'icons-build/icon.png', 1024);
render('assets/tray-template.svg', 'src-tauri/icons/tray-template.png', 64);
