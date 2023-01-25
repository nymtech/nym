import React from 'react';
import { ArrowBack, Close, HelpOutline, Minimize } from '@mui/icons-material';
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
    padding: '16px',
    paddingBottom: '0px',
    borderTopLeftRadius: '12px',
    borderTopRightRadius: '12px',
  },
};

const CustomButton = ({ Icon, onClick }: { Icon: React.JSXElementConstructor<any>; onClick: () => void }) => (
  <IconButton size="small" onClick={onClick} sx={{ padding: 0 }}>
    <Icon style={{ fontSize: 16 }} />
  </IconButton>
);

export const CustomTitleBar = () => {
  const { showHelp, handleShowHelp } = useClientContext();
  return (
    <Box data-tauri-drag-region style={customTitleBarStyles.titlebar}>
      {/* set width to keep logo centered */}
      <Box sx={{ width: '40px' }}>
        <CustomButton
          Icon={!showHelp ? HelpOutline : ArrowBack}
          onClick={() => {
            handleShowHelp();
          }}
        />
      </Box>

      <NymWordmark width={36} />
    </Box>
  );
};
