import React from 'react';
import { ArrowBack, Menu } from '@mui/icons-material';
import { Box, IconButton, Typography } from '@mui/material';
// TODO since the structure refactor for NC this import fails
// import { NymWordmark } from '@nymproject/react/logo/NymWordmark';
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
    <Icon style={{ fontSize: 24 }} />
  </IconButton>
);

const MenuIcon = () => {
  const navigate = useNavigate();
  return <CustomButton Icon={Menu} onClick={() => navigate('/menu')} />;
};

const ArrowBackIcon = ({ onBack }: { onBack?: () => void }) => {
  const navigate = useNavigate();
  const handleBack = () => {
    onBack?.();
    navigate(-1);
  };
  return <CustomButton Icon={ArrowBack} onClick={handleBack} />;
};

const getTitle = (path: string) => {
  if (path.includes('settings')) return 'Settings';
  if (path !== '/') {
    const title = path.split('/').slice(-1);
    return (
      <Typography textTransform="capitalize" fontSize="16px" fontWeight={700}>
        {title}
      </Typography>
    );
  }

  // TODO return <NymWordmark width={36} />;
  return (
    <Typography fontSize="18px" fontWeight={700}>
      NYM
    </Typography>
  );
};

export const CustomTitleBar = ({ path = '/', onBack }: { path?: string; onBack?: () => void }) => (
  <Box style={customTitleBarStyles.titlebar}>
    {/* set width to keep logo centered */}
    <Box sx={{ width: '40px' }}>{path === '/' ? <MenuIcon /> : <ArrowBackIcon onBack={onBack} />}</Box>
    {getTitle(path)}
    <Box sx={{ width: '40px' }} />
  </Box>
);
