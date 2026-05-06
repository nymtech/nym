/* eslint-disable no-console -- build-time diagnostic */
const path = require('path');
const fs = require('fs');

const walletRoot = path.resolve(__dirname, '..');

const PKGS = [
  '@emotion/react',
  '@emotion/styled',
  '@emotion/cache',
  '@mui/material',
  '@mui/system',
  '@mui/styled-engine',
  '@mui/private-theming',
  '@mui/utils',
  '@mui/lab',
  '@mui/icons-material',
  'react',
  'react-dom',
];

const probes = [
  walletRoot,
  path.join(walletRoot, 'node_modules', '@nymproject', 'react'),
  path.join(walletRoot, 'node_modules', '@mui', 'material'),
];

let bad = 0;
PKGS.forEach((pkg) => {
  const seen = new Set();
  probes.forEach((probe) => {
    try {
      const p = require.resolve(`${pkg}/package.json`, { paths: [probe] });
      seen.add(fs.realpathSync(p));
    } catch {
      /* probe may not resolve this package */
    }
  });
  if (seen.size > 1) {
    bad += 1;
    console.error(`[singleton] DUPLICATE ${pkg}:\n  ${[...seen].join('\n  ')}`);
  }
});

process.exit(bad ? 1 : 0);
