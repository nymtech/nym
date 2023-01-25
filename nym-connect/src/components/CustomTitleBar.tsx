import React from 'react';
import { ArrowBack, Close, Menu, Minimize } from '@mui/icons-material';
import { Box, IconButton, Typography } from '@mui/material';
import { NymWordmark } from '@nymproject/react/logo/NymWordmark';
import { appWindow } from '@tauri-apps/api/window';
import { useNavigate } from 'react-router-dom';

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

const MenuIcon = () => {
  const navigate = useNavigate();
  return <CustomButton Icon={Menu} onClick={() => navigate('/menu')} />;
};

const ArrowBackIcon = () => {
  const navigate = useNavigate();
  return <CustomButton Icon={ArrowBack} onClick={() => navigate(-1)} />;
};

const getTitleIcon = (path: string) => {
  if (path !== '/') {
    const title = path.split('/').slice(-1);
    return (
      <Typography textTransform="capitalize" fontWeight={700}>
        {title}
      </Typography>
    );
  }
  return <NymWordmark width={36} />;
};

export const CustomTitleBar = ({ path = '/' }: { path?: string }) => {
  console.log(path);

  return (
    <Box data-tauri-drag-region style={customTitleBarStyles.titlebar}>
      {/* set width to keep logo centered */}
      <Box sx={{ width: '40px' }}>{path === '/' ? <MenuIcon /> : <ArrowBackIcon />}</Box>
      {getTitleIcon(path)}
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
        <CustomButton Icon={Minimize} onClick={() => appWindow.minimize()} />
        <CustomButton Icon={Close} onClick={() => appWindow.close()} />
      </Box>
    </Box>
  );
};
