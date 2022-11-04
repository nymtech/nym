import React from 'react';
import { ArrowBack, Close, HelpOutline } from '@mui/icons-material';
import { Box, IconButton } from '@mui/material';
import { NymWordmark } from '@nymproject/react/logo/NymWordmark';
import { appWindow } from '@tauri-apps/api/window';
import { useClientContext } from 'src/context/main';

const customTitleBarStyles = {
  titlebar: {
    background: '#1D2125',
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: '24px',
    padding: '16px',
    paddingBottom: '0px',
    borderTopLeftRadius: '12px',
    borderTopRightRadius: '12px',
  },
};

const CustomButton = ({ Icon, onClick }: { Icon: React.JSXElementConstructor<any>; onClick: () => void }) => (
  <IconButton size="small" style={{ padding: 0 }} onClick={onClick}>
    <Icon style={{ fontSize: 16 }} />
  </IconButton>
);

export const CustomTitleBar = () => {
  const { showHelp, handleShowHelp } = useClientContext();
  return (
    <Box data-tauri-drag-region style={customTitleBarStyles.titlebar}>
      <CustomButton
        Icon={!showHelp ? HelpOutline : ArrowBack}
        onClick={() => {
          handleShowHelp();
        }}
      />
      <NymWordmark width={36} />
      <CustomButton Icon={Close} onClick={() => appWindow.close()} />
    </Box>
  );
};
