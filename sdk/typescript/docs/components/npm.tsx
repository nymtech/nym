import React, { FC } from 'react';
import { Chip, Link } from '@mui/material';

export const NPMLink: FC<{ packageName: string; kind: 'esm' | 'cjs'; preBundled?: boolean }> = ({
  packageName,
  kind,
  preBundled,
}) => (
  <Link
    href={`https://www.npmjs.com/package/${packageName}`}
    target="_blank"
    sx={{ whiteSpace: 'nowrap', textDecoration: 'none' }}
  >
    {packageName} <Chip label={kind === 'cjs' ? 'CommonJS' : 'ESM'} size="small" />{' '}
    {preBundled && <Chip label="pre-bundled" size="small" color="info" className="chipContained" />}
  </Link>
);
