import { useState } from 'react';

export const useCreatePassword = () => {
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [isSafePassword, setIsSafePassword] = useState(false);
  const [hasReadTerms, setHasReadTerms] = useState(false);

  const canProceed = isSafePassword && hasReadTerms && password === confirmPassword;

  return {
    password,
    setPassword,
    confirmPassword,
    setConfirmPassword,
    setIsSafePassword,
    canProceed,
    setHasReadTerms,
    hasReadTerms,
  };
};
