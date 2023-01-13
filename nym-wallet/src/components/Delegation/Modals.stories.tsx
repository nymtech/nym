import React from 'react';

import { Button, Paper, Typography } from '@mui/material';
import { useTheme, Theme } from '@mui/material/styles';
import { DelegateModal } from './DelegateModal';
import { UndelegateModal } from './UndelegateModal';
import { backDropStyles, modalStyles } from '../../../.storybook/storiesStyles';

const storybookStyles = (theme: Theme) => ({
  backdropProps: backDropStyles(theme),
  sx: modalStyles(theme),
});

export default {
  title: 'Delegation/Components/Action Modals',
};

const Background: FCWithChildren<{ onOpen: () => void }> = ({ onOpen }) => {
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
  const [open, setOpen] = React.useState<boolean>(false);
  const theme = useTheme();
  return (
    <>
      <Background onOpen={() => setOpen(true)} />
      <DelegateModal
        open={open}
        onClose={() => setOpen(false)}
        onOk={async () => setOpen(false)}
        denom="nym"
        estimatedReward={50.423}
        accountBalance="425.2345053"
        nodeUptimePercentage={99.28394}
        profitMarginPercentage="11.12334234"
        rewardInterval="monthlyish"
        hasVestingContract={false}
        {...storybookStyles(theme)}
      />
    </>
  );
};

export const DelegateBelowMinimum = () => {
  const [open, setOpen] = React.useState<boolean>(false);
  const theme = useTheme();
  return (
    <>
      <Background onOpen={() => setOpen(true)} />
      <DelegateModal
        open={open}
        onClose={() => setOpen(false)}
        onOk={async () => setOpen(false)}
        denom="nym"
        estimatedReward={425.2345053}
        nodeUptimePercentage={99.28394}
        profitMarginPercentage="11.12334234"
        rewardInterval="monthlyish"
        initialAmount="0.1"
        hasVestingContract={false}
        {...storybookStyles(theme)}
      />
    </>
  );
};

export const DelegateMore = () => {
  const [open, setOpen] = React.useState<boolean>(false);
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
        denom="nym"
        estimatedReward={50.423}
        accountBalance="425.2345053"
        nodeUptimePercentage={99.28394}
        profitMarginPercentage="11.12334234"
        rewardInterval="monthlyish"
        hasVestingContract={false}
        {...storybookStyles(theme)}
      />
    </>
  );
};

export const Undelegate = () => {
  const [open, setOpen] = React.useState<boolean>(false);
  const theme = useTheme();
  return (
    <>
      <Background onOpen={() => setOpen(true)} />
      <UndelegateModal
        open={open}
        onClose={() => setOpen(false)}
        onOk={async () => setOpen(false)}
        currency="nym"
        amount={150}
        mixId={1234}
        identityKey="AA6RfeY8DttMD3CQKoayV6mss5a5FC3RoH75Kmcujyxx"
        usesVestingContractTokens={false}
        {...storybookStyles(theme)}
      />
    </>
  );
};
