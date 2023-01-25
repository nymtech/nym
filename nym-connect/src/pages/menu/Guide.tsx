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
        description="Select your service provider from the dropdown menu."
        img={Image1}
        onNext={() => setStep(2)}
      />
    );

  if (step === 2)
    return (
      <HelpPage
        step={step}
        totalSteps={TOTAL_STEPS}
        description="Click yellow button and connect to a service provider."
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
        description="Go to settings in your app, select running via SOCKS5 proxy and paste the IP and Port values given by NymConnect."
        img={Image4}
        onPrev={() => setStep(3)}
      />
    );

  return null;
};
