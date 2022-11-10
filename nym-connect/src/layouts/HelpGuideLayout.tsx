import React, { useState } from 'react';
import { HelpPage } from 'src/components/HelpPage';
import { Button, Link, Stack } from '@mui/material';
import Image1 from '../assets/help-step-one.png';
import Image2 from '../assets/help-step-two.png';
import Image3 from '../assets/help-step-three.png';
import Image4 from '../assets/help-step-four.png';

export const HelpGuideLayout = () => {
  const [step, setStep] = useState(0);

  if (step === 1)
    return (
      <HelpPage
        step={step}
        description="Select your service provider from the dropdown menu"
        img={Image1}
        onNext={() => setStep(2)}
      />
    );

  if (step === 2)
    return (
      <HelpPage
        step={step}
        description="Click yellow button and connect to a service provider"
        img={Image2}
        onPrev={() => setStep(1)}
        onNext={() => setStep(3)}
      />
    );

  if (step === 3)
    return (
      <HelpPage
        step={step}
        description="Click on IP and Port to copy their values to the clipboard"
        img={Image3}
        onPrev={() => setStep(2)}
        onNext={() => setStep(4)}
      />
    );

  if (step === 4)
    return (
      <HelpPage
        step={step}
        description="Go to settings in your app, select running via SOCKS5 proxy and paste the IP and Port values given by NymConnect."
        img={Image4}
        onPrev={() => setStep(3)}
      />
    );

  return (
    <Stack gap={1}>
      <Button variant="text" color="inherit" onClick={() => setStep(1)}>
        How to connect guide
      </Button>
      <Button
        LinkComponent={Link}
        variant="text"
        color="inherit"
        href="https://shipyard.nymtech.net/nym-connect"
        target="_blank"
      >
        Docs
      </Button>
    </Stack>
  );
};
