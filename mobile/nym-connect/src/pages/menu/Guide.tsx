import React, { useState } from 'react';
import { HelpPage } from 'src/components/HelpPage';
import Image1 from '../../assets/help-step-one.png';
import Image2 from '../../assets/help-step-two.png';
import Image4 from '../../assets/help-step-four.png';

export const HelpGuide = () => {
  const [step, setStep] = useState(1);
  const TOTAL_STEPS = 3;

  if (step === 1)
    return (
      <HelpPage
        step={step}
        totalSteps={TOTAL_STEPS}
        description="Click button and connect 
        to a Nym mixnet."
        img={Image1}
        onNext={() => setStep(2)}
      />
    );

  if (step === 2)
    return (
      <HelpPage
        step={step}
        totalSteps={TOTAL_STEPS}
        description="Click on IP and Port to copy their values to the clipboard."
        img={Image2}
        onPrev={() => setStep(1)}
        onNext={() => setStep(3)}
      />
    );

  if (step === 3)
    return (
      <HelpPage
        step={step}
        totalSteps={TOTAL_STEPS}
        description="Go to settings in your app, select run via SOCKS5 proxy and paste the IP and Port values given by NymConnect."
        img={Image4}
        onPrev={() => setStep(2)}
      />
    );

  return null;
};
