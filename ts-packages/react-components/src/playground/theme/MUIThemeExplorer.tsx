/* eslint-disable react/no-array-index-key */
import type { Theme } from '@mui/material/styles';
import * as React from 'react';
import TreeView from '@mui/lab/TreeView';
import TreeItem from '@mui/lab/TreeItem';
import ArrowDropDownIcon from '@mui/icons-material/ArrowDropDown';
import ArrowRightIcon from '@mui/icons-material/ArrowRight';
import { Box } from '@mui/material';

const MUIThemeExplorerItem: React.FC<{
  path: string;
  parentKey: string;
  theme: Theme;
  item: any;
  isArrayItem?: boolean;
}> = ({ theme, item, parentKey, path, isArrayItem }) => {
  if (!item) {
    return null;
  }

  if (typeof item === 'string') {
    return (
      <TreeItem
        nodeId={`${path}`}
        label={
          <Box sx={{ display: 'flex', alignItems: 'center', p: 0.5, pr: 0 }}>
            <Box
              sx={{
                mr: 2,
              }}
            >
              <svg height="25px" width="25px">
                <rect width="100%" height="100%" fill={item} stroke={theme.palette.text.primary} strokeWidth="1px" />
              </svg>
            </Box>
            {!isArrayItem && <Box mr={2}>{parentKey}</Box>}
            {/* <Box mr={2}> */}
            {/*  <code>{item}</code> */}
            {/* </Box> */}
            <Box color={theme.palette.text.disabled}>
              <code>{path}</code>
            </Box>
          </Box>
        }
      />
    );
  }

  if (Array.isArray(item)) {
    return (
      <TreeItem nodeId={`${path}-array`} label={`${parentKey}[]`}>
        {item.map((i, idx) => (
          <MUIThemeExplorerItem
            key={`${parentKey}-${idx}`}
            parentKey={parentKey}
            item={i}
            theme={theme}
            path={`${path}[${idx}]`}
            isArrayItem
          />
        ))}
      </TreeItem>
    );
  }

  if (typeof item === 'object' && Object.keys(item).length) {
    return (
      <TreeItem nodeId={`${path}`} label={`${parentKey}`}>
        {Object.keys(item).map((key) => (
          <MUIThemeExplorerItem
            key={`${parentKey}-${key}`}
            parentKey={key}
            item={item[key]}
            theme={theme}
            path={`${path}.${key}`}
          />
        ))}
      </TreeItem>
    );
  }

  return null;
};

export const MUIThemeExplorer: React.FC<{
  theme: Theme;
}> = ({ theme }) => (
  <TreeView
    defaultExpanded={['theme.palette', 'theme.palette.nym', 'theme.palette.nym.highlight']}
    defaultCollapseIcon={<ArrowDropDownIcon />}
    defaultExpandIcon={<ArrowRightIcon />}
    defaultEndIcon={<div style={{ width: 24 }} />}
    sx={{ height: 500, flexGrow: 1, width: '100%', overflowY: 'auto' }}
  >
    <MUIThemeExplorerItem theme={theme} item={theme.palette} parentKey="palette" path="theme.palette" />
    <TreeItem nodeId="test">Test</TreeItem>
  </TreeView>
);
