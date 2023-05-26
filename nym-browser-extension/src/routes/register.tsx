import React from 'react';
import { Route, Routes, useNavigate } from 'react-router-dom';
import { RegisterContextProvider } from 'src/context/register';
import { ImportAccount, SeedPhrase, SetupComplete } from 'src/pages/register';
import { CreatePasswordOnExistingAccount } from 'src/pages/register/CreatePasswordOnExistingAccount';
import { CreatePasswordOnNewAccount } from 'src/pages/register/CreatePasswordOnNewAccount';

export const RegisterRoutes = () => {
  const navigate = useNavigate();

  const handleSetUpComplete = () => {
    navigate('/login');
  };

  return (
    <RegisterContextProvider>
      <Routes>
        <Route
          path="create-password"
          element={<CreatePasswordOnNewAccount onNext={() => navigate('/register/seed-phrase')} />}
        />
        <Route path="seed-phrase" element={<SeedPhrase />} />
        <Route path="import-account" element={<ImportAccount />} />
        <Route
          path="import-account/create-password"
          element={
            <CreatePasswordOnExistingAccount
              onComplete={() => {
                navigate('/register/complete');
              }}
            />
          }
        />
        <Route path="complete" element={<SetupComplete onDone={handleSetUpComplete} />} />
      </Routes>
    </RegisterContextProvider>
  );
};
