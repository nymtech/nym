import React from 'react';
import { ComponentMeta } from '@storybook/react';

import { Button, Paper } from '@mui/material';
import { SimpleModal } from './SimpleModal';
import { ModalDivider } from './ModalDivider';

export default {
  title: 'Modals/Simple Modal',
  component: SimpleModal,
} as ComponentMeta<typeof SimpleModal>;

const BasePage: React.FC<{ children: React.ReactElement<any, any>; handleClick: () => void }> = ({
  children,
  handleClick,
}) => (
  <>
    <Paper elevation={0} sx={{ px: 4, pt: 2, pb: 4 }}>
      <h2>Lorem ipsum</h2>
      <Button variant="contained" onClick={handleClick}>
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
        est irure sint magna sunt. Pariatur dolore commodo labore quis incididunt proident duis voluptate exercitation
        in duis. Occaecat aliqua laboris reprehenderit nostrud est aute pariatur fugiat anim. Dolore sunt cillum ea
        aliquip consectetur laborum ipsum qui veniam Lorem consectetur adipisicing velit magna aute. Amet tempor quis
        excepteur minim culpa velit Lorem enim ad.
      </p>
      <p>
        Mollit laborum exercitation excepteur laboris adipisicing ipsum veniam cillum mollit voluptate do. Amet et anim
        Lorem mollit minim duis cupidatat non. Consectetur sit deserunt nisi nisi non excepteur dolor eiusmod aute aute
        irure anim dolore ipsum et veniam.
      </p>
    </Paper>
    {children}
  </>
);

export const Default = () => {
  const [open, setOpen] = React.useState<boolean>(true);
  const handleClick = () => setOpen(true);

  return (
    <BasePage handleClick={handleClick}>
      <SimpleModal
        open={open}
        onClose={() => setOpen(false)}
        onOk={async () => setOpen(false)}
        header="This is a modal"
        subHeader="This is a sub header"
        okLabel="Click to continue"
      >
        <p>Lorem mollit minim duis cupidatat non. Consectetur sit deserunt</p>
        <p>
          Veniam dolor laborum labore sit reprehenderit enim mollit magna nulla adipisicing fugiat. Est ex irure quis.
        </p>
        <ModalDivider />
        <p>Occaecat commodo excepteur anim ut officia dolor laboris dolore id occaecat enim qui eius</p>
        <p>
          Tempor culpa est magna. Sit tempor cillum culpa sint ipsum nostrud ullamco voluptate exercitation dolore magna
          elit ut mollit.
        </p>
      </SimpleModal>
    </BasePage>
  );
};

export const NoSubheader = () => {
  const [open, setOpen] = React.useState<boolean>(true);
  const handleClick = () => setOpen(true);

  return (
    <BasePage handleClick={handleClick}>
      <SimpleModal
        open={open}
        onClose={() => setOpen(false)}
        onOk={async () => setOpen(false)}
        header="This is a modal"
        okLabel="Kaplow!"
      >
        <p>
          Tempor culpa est magna. Sit tempor cillum culpa sint ipsum nostrud ullamco voluptate exercitation dolore magna
          elit ut mollit.
        </p>
        <ModalDivider />
        <p>
          Veniam dolor laborum labore sit reprehenderit enim mollit magna nulla adipisicing fugiat. Est ex irure quis.
        </p>
      </SimpleModal>
    </BasePage>
  );
};

export const hideCloseIcon = () => {
  const [open, setOpen] = React.useState<boolean>(true);
  const handleClick = () => setOpen(true);

  return (
    <BasePage handleClick={handleClick}>
      <SimpleModal
        open={open}
        hideCloseIcon
        onClose={() => setOpen(false)}
        onOk={async () => setOpen(false)}
        header="This is a modal"
        okLabel="Kaplow!"
      >
        <p>
          Tempor culpa est magna. Sit tempor cillum culpa sint ipsum nostrud ullamco voluptate exercitation dolore magna
          elit ut mollit.
        </p>
        <ModalDivider />
        <p>
          Veniam dolor laborum labore sit reprehenderit enim mollit magna nulla adipisicing fugiat. Est ex irure quis.
        </p>
      </SimpleModal>
    </BasePage>
  );
};

export const hideCloseIconAndDisplayErrorIcon = () => {
  const [open, setOpen] = React.useState<boolean>(true);
  const handleClick = () => setOpen(true);

  return (
    <BasePage handleClick={handleClick}>
      <SimpleModal
        open={open}
        hideCloseIcon
        displayErrorIcon
        onClose={() => setOpen(false)}
        onOk={async () => setOpen(false)}
        header="This modal announces an error !"
        okLabel="Kaplow!"
        sx={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center' }}
        headerStyles={{
          width: '100%',
          mb: 3,
          textAlign: 'center',
          color: 'error.main',
        }}
        subHeaderStyles={{ textAlign: 'center', color: 'text.primary', fontSize: 14, fontWeight: 400 }}
      >
        <p>
          Tempor culpa est magna. Sit tempor cillum culpa sint ipsum nostrud ullamco voluptate exercitation dolore magna
          elit ut mollit.
        </p>
        <ModalDivider />
        <p>
          Veniam dolor laborum labore sit reprehenderit enim mollit magna nulla adipisicing fugiat. Est ex irure quis.
        </p>
      </SimpleModal>
    </BasePage>
  );
};

export const withBackButton = () => {
  const [open, setOpen] = React.useState<boolean>(true);
  const handleClick = () => setOpen(true);

  return (
    <BasePage handleClick={handleClick}>
      <SimpleModal
        open={open}
        hideCloseIcon
        onClose={() => setOpen(false)}
        onOk={async () => setOpen(false)}
        header="This is a modal"
        okLabel="Primary action"
        onBack={() => setOpen(false)}
      >
        <p>
          Tempor culpa est magna. Sit tempor cillum culpa sint ipsum nostrud ullamco voluptate exercitation dolore magna
          elit ut mollit.
        </p>
        <ModalDivider />
        <p>
          Veniam dolor laborum labore sit reprehenderit enim mollit magna nulla adipisicing fugiat. Est ex irure quis.
        </p>
      </SimpleModal>
    </BasePage>
  );
};
