import React from 'react';
import { ComponentMeta } from '@storybook/react';
import { Button, Paper } from '@mui/material';
import { useTheme, Theme } from '@mui/material/styles';
import { RedeemModal } from './RedeemModal';
import { backDropStyles, modalStyles } from '../../../.storybook/storiesStyles';

const storybookStyles = (theme: Theme) => ({
  backdropProps: backDropStyles(theme),
  sx: modalStyles(theme),
});

export default {
  title: 'Rewards/Components/Redeem Modals',
  component: RedeemModal,
} as ComponentMeta<typeof RedeemModal>;

const Content: FCWithChildren<{
  setOpen: (value: boolean) => void;
}> = ({ setOpen }) => (
  <Paper elevation={0} sx={{ px: 4, pt: 2, pb: 4 }}>
    <h2>Lorem ipsum</h2>
    <Button variant="contained" onClick={() => setOpen(true)}>
      Show modal
    </Button>
    <p>
      Veniam dolor laborum labore sit reprehenderit enim mollit magna nulla adipisicing fugiat. Est ex irure quis sunt
      velit elit do minim mollit non duis reprehenderit. Eiusmod dolore adipisicing ex nostrud consectetur culpa
      exercitation do. Ad elit esse ipsum aliqua labore irure laborum qui culpa.
    </p>
    <p>
      Occaecat commodo excepteur anim ut officia dolor laboris dolore id occaecat enim qui eiusmod occaecat aliquip ad
      tempor. Labore amet laborum magna amet consequat dolor cupidatat in consequat sunt aliquip magna laboris tempor
      culpa est magna. Sit tempor cillum culpa sint ipsum nostrud ullamco voluptate exercitation dolore magna elit ut
      mollit.
    </p>
    <p>
      Labore voluptate elit amet ipsum qui officia duis in et occaecat culpa ex do non labore mollit. Cillum cupidatat
      duis ea dolore laboris laboris sunt duis anim consectetur cupidatat nulla ad minim sunt ea. Aliqua amet commodo
      est irure sint magna sunt. Pariatur dolore commodo labore quis incididunt proident duis voluptate exercitation in
      duis. Occaecat aliqua laboris reprehenderit nostrud est aute pariatur fugiat anim. Dolore sunt cillum ea aliquip
      consectetur laborum ipsum qui veniam Lorem consectetur adipisicing velit magna aute. Amet tempor quis excepteur
      minim culpa velit Lorem enim ad.
    </p>
    <p>
      Mollit laborum exercitation excepteur laboris adipisicing ipsum veniam cillum mollit voluptate do. Amet et anim
      Lorem mollit minim duis cupidatat non. Consectetur sit deserunt nisi nisi non excepteur dolor eiusmod aute aute
      irure anim dolore ipsum et veniam.
    </p>
  </Paper>
);

export const RedeemAllRewards = () => {
  const [open, setOpen] = React.useState<boolean>(false);
  const theme = useTheme();
  return (
    <>
      <Content setOpen={setOpen} />
      <RedeemModal
        open={open}
        onClose={() => setOpen(false)}
        onOk={async () => setOpen(false)}
        message="Redeem all rewards"
        denom="nym"
        mixId={1234}
        identityKey="D88RfeY8DttMD3CQKoayV6mss5a5FC3RoH75Kmcujaaa"
        amount={425.65843}
        {...storybookStyles(theme)}
        usesVestingTokens={false}
      />
    </>
  );
};

export const RedeemRewardForMixnode = () => {
  const [open, setOpen] = React.useState<boolean>(false);
  const theme = useTheme();
  return (
    <>
      <Content setOpen={setOpen} />
      <RedeemModal
        open={open}
        onClose={() => setOpen(false)}
        onOk={async () => setOpen(false)}
        message="Redeem rewards"
        denom="nym"
        mixId={1234}
        identityKey="D88RfeY8DttMD3CQKoayV6mss5a5FC3RoH75Kmcujaaa"
        amount={425.65843}
        {...storybookStyles(theme)}
        usesVestingTokens={false}
      />
    </>
  );
};

export const FeeIsMoreThanAllRewards = () => {
  const [open, setOpen] = React.useState<boolean>(false);
  const theme = useTheme();
  return (
    <>
      <Content setOpen={setOpen} />
      <RedeemModal
        open={open}
        onClose={() => setOpen(false)}
        onOk={() => setOpen(false)}
        message="Redeem all rewards"
        denom="nym"
        mixId={1234}
        identityKey="D88RfeY8DttMD3CQKoayV6mss5a5FC3RoH75Kmcujaaa"
        amount={0.001}
        {...storybookStyles(theme)}
        usesVestingTokens={false}
      />
    </>
  );
};

export const FeeIsMoreThanMixnodeReward = () => {
  const [open, setOpen] = React.useState<boolean>(false);
  const theme = useTheme();
  return (
    <>
      <Content setOpen={setOpen} />
      <RedeemModal
        open={open}
        onClose={() => setOpen(false)}
        onOk={async () => setOpen(false)}
        mixId={1234}
        identityKey="D88RfeY8DttMD3CQKoayV6mss5a5FC3RoH75Kmcujaaa"
        message="Redeem rewards"
        denom="nym"
        amount={0.001}
        {...storybookStyles(theme)}
        usesVestingTokens={false}
      />
    </>
  );
};
