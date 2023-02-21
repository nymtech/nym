import React from 'react';
import { Box, Typography } from '@mui/material';
import prettyBytes from 'pretty-bytes';

export interface ConnectionStatsItem {
  label: string;
  rateBytesPerSecond: number;
  totalBytes: number;
}

const FONT_SIZE = '14px';

export const ConnectionStats: React.FC<{
  stats: ConnectionStatsItem[];
}> = ({ stats }) => (
  <Box color="rgba(255,255,255,0.6)" width="100%" display="flex" justifyContent="space-between">
    <div>
      {stats.map((stat) => (
        <Typography key={`stat-${stat.label}-label`} fontSize={FONT_SIZE}>
          {stat.label}
        </Typography>
      ))}
    </div>
    <div>
      {stats.map((stat) => (
        <Typography key={`stat-${stat.label}-rate`} textAlign="center" fontSize={FONT_SIZE}>
          {formatRate(stat.rateBytesPerSecond)}
        </Typography>
      ))}
    </div>
    <div>
      {stats.map((stat) => (
        <Typography key={`stat-${stat.label}-total`} textAlign="right" fontSize={FONT_SIZE}>
          {formatTotal(stat.totalBytes)}
        </Typography>
      ))}
    </div>
  </Box>
);

export function formatRate(bytesPerSecond: number): string {
  return `${prettyBytes(bytesPerSecond)}/s`;
}

export function formatTotal(totalBytes: number): string {
  return prettyBytes(totalBytes);
}
