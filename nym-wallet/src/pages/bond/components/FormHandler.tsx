import React, { useState } from 'react';
import { Box } from '@mui/material';
import { TNodeType } from '@nymproject/types';
import { NodeTypeSelector } from 'src/components';
import { GatewayForm } from './GatewayForm';
import { MixnodeForm } from './MixnodeForm';

export const FormHandler = ({
  onSuccess,
  onError,
}: {
  onSuccess: (details: { address: string; amount: string }) => void;
  onError: (msg?: string) => void;
}) => {
  const [nodeType, setNodeType] = useState<TNodeType>('mixnode');

  return (
    <Box sx={{ p: 3 }}>
      <NodeTypeSelector disabled={false} nodeType={nodeType} setNodeType={setNodeType} />
      {nodeType === 'mixnode' && <MixnodeForm disabled={false} onError={onError} onSuccess={onSuccess} />}
      {nodeType === 'gateway' && <GatewayForm disabled={false} onError={onError} onSuccess={onSuccess} />}
    </Box>
  );
};
