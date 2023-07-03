import React from 'react';
import { SetupCompleteTemplate } from '../templates/Complete';

export const SetupComplete = ({ onDone }: { onDone: () => void }) => (
  <SetupCompleteTemplate
    title="You're all set!"
    description="Open the extension and sign in to begin your interchain journey"
    onDone={onDone}
  />
);
