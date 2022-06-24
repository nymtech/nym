import React from 'react';

import { Button, Paper, Typography } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { DelegateModal } from './DelegateModal';
import { UndelegateModal } from './UndelegateModal';

export default {
  title: 'Delegation/Components/Action Modals',
};

const Background: React.FC<{ onOpen: () => void }> = ({ onOpen }) => {
  const theme = useTheme();
  return (
    <Paper elevation={0} sx={{ px: 4, pt: 2, pb: 4 }}>
      <h2>Lorem ipsum</h2>
      <Button variant="contained" onClick={onOpen}>
        Show modal
      </Button>
      <Typography sx={{ color: theme.palette.text.primary }}>
        Veniam dolor laborum labore sit reprehenderit enim mollit magna nulla adipisicing fugiat. Est ex irure quis sunt
        velit elit do minim mollit non duis reprehenderit. Eiusmod dolore adipisicing ex nostrud consectetur culpa
        exercitation do. Ad elit esse ipsum aliqua labore irure laborum qui culpa.
      </Typography>
      <Typography sx={{ color: theme.palette.text.primary }}>
        Occaecat commodo excepteur anim ut officia dolor laboris dolore id occaecat enim qui eiusmod occaecat aliquip ad
        tempor. Labore amet laborum magna amet consequat dolor cupidatat in consequat sunt aliquip magna laboris tempor
        culpa est magna. Sit tempor cillum culpa sint ipsum nostrud ullamco voluptate exercitation dolore magna elit ut
        mollit.
      </Typography>
      <Typography sx={{ color: theme.palette.text.primary }}>
        Labore voluptate elit amet ipsum qui officia duis in et occaecat culpa ex do non labore mollit. Cillum cupidatat
        duis ea dolore laboris laboris sunt duis anim consectetur cupidatat nulla ad minim sunt ea. Aliqua amet commodo
        est irure sint magna sunt. Pariatur dolore commodo labore quis incididunt proident duis voluptate exercitation
        in duis. Occaecat aliqua laboris reprehenderit nostrud est aute pariatur fugiat anim. Dolore sunt cillum ea
        aliquip consectetur laborum ipsum qui veniam Lorem consectetur adipisicing velit magna aute. Amet tempor quis
        excepteur minim culpa velit Lorem enim ad.
      </Typography>
      <Typography sx={{ color: theme.palette.text.primary }}>
        Mollit laborum exercitation excepteur laboris adipisicing ipsum veniam cillum mollit voluptate do. Amet et anim
        Lorem mollit minim duis cupidatat non. Consectetur sit deserunt nisi nisi non excepteur dolor eiusmod aute aute
        irure anim dolore ipsum et veniam.
      </Typography>
    </Paper>
  );
};

export const Delegate = () => {
  const [open, setOpen] = React.useState<boolean>(true);
  const theme = useTheme();
  return (
    <>
      <Background onOpen={() => setOpen(true)} />
      <DelegateModal
        open={open}
        onClose={() => setOpen(false)}
        onOk={async () => setOpen(false)}
        currency="NYM"
        estimatedReward={50.423}
        accountBalance="425.2345053"
        nodeUptimePercentage={99.28394}
        profitMarginPercentage={11.12334234}
        rewardInterval="weekly"
        hasVestingContract={false}
        sx={{ left: theme.palette.mode === 'light' ? '25%' : '75%' }}
      />
    </>
  );
};

export const DelegateBelowMinimum = () => {
  const [open, setOpen] = React.useState<boolean>(true);
  const theme = useTheme();
  return (
    <>
      <Background onOpen={() => setOpen(true)} />
      <DelegateModal
        open={open}
        onClose={() => setOpen(false)}
        onOk={async () => setOpen(false)}
        currency="NYM"
        estimatedReward={425.2345053}
        nodeUptimePercentage={99.28394}
        profitMarginPercentage={11.12334234}
        rewardInterval="weekly"
        initialAmount="0.1"
        hasVestingContract={false}
        sx={{ left: theme.palette.mode === 'light' ? '25%' : '75%' }}
      />
    </>
  );
};

export const DelegateMore = () => {
  const [open, setOpen] = React.useState<boolean>(true);
  const theme = useTheme();
  return (
    <>
      <Background onOpen={() => setOpen(true)} />
      <DelegateModal
        open={open}
        onClose={() => setOpen(false)}
        onOk={async () => setOpen(false)}
        header="Delegate more"
        buttonText="Delegate more"
        currency="NYM"
        estimatedReward={50.423}
        accountBalance="425.2345053"
        nodeUptimePercentage={99.28394}
        profitMarginPercentage={11.12334234}
        rewardInterval="weekly"
        hasVestingContract={false}
        sx={{ left: theme.palette.mode === 'light' ? '25%' : '75%' }}
      />
    </>
  );
};

export const Undelegate = () => {
  const [open, setOpen] = React.useState<boolean>(true);
  const theme = useTheme();
  return (
    <>
      <Background onOpen={() => setOpen(true)} />
      <UndelegateModal
        open={open}
        onClose={() => setOpen(false)}
        onOk={async () => setOpen(false)}
        currency="NYM"
        amount={150}
        identityKey="AA6RfeY8DttMD3CQKoayV6mss5a5FC3RoH75Kmcujyxx"
        usesVestingContractTokens={false}
        sx={{ left: theme.palette.mode === 'light' ? '25%' : '75%' }}
      />
    </>
  );
};
