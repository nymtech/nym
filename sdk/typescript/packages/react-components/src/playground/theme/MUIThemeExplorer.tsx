import * as React from 'react';
import type { Theme } from '@mui/material/styles';
import { SimpleTreeView, TreeItem } from '@mui/x-tree-view';
import { Box } from '@mui/material';

const MUIThemeExplorerItem = ({
  theme,
  item,
  parentKey,
  path,
  isArrayItem,
}: {
  path: string;
  parentKey: string;
  theme: Theme;
  item: any;
  isArrayItem?: boolean;
  children: React.ReactNode;
}) => {
  if (!item) {
    return null;
  }

  if (typeof item === 'string') {
    return (
      <TreeItem
        itemId={`${path}`}
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
      <TreeItem itemId={`${path}-array`} label={`${parentKey}[]`}>
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
      <TreeItem itemId={`${path}`} label={`${parentKey}`}>
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

export const MUIThemeExplorer: FCWithChildren<{
  theme: Theme;
}> = ({ theme }) => (
  <SimpleTreeView
    defaultExpandedItems={['theme.palette', 'theme.palette.nym', 'theme.palette.nym.highlight']}
    sx={{ height: 500, flexGrow: 1, width: '100%', overflowY: 'auto' }}
  >
    <MUIThemeExplorerItem theme={theme} item={theme.palette} parentKey="palette" path="theme.palette" />
    <TreeItem itemId="test">Test</TreeItem>
  </SimpleTreeView>
);
