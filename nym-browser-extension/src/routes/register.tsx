import React from 'react';
import { Route, Routes, useNavigate } from 'react-router-dom';
import { CreatePassword, ImportAccount, SeedPhrase, SetupComplete } from 'src/pages/register';

export const RegisterRoutes = () => {
  const navigate = useNavigate();

  // hack to work on redirect until password capability is set up
  const handleSetUpComplete = () => {
    localStorage.setItem('nym-browser-extension', 'true');
    navigate('/login');
  };
  return (
    <Routes>
      <Route path="seed-phrase" element={<SeedPhrase />} />
      <Route path="create-password" element={<CreatePassword onNext={() => navigate('/register/seed-phrase')} />} />
      <Route path="import-account" element={<ImportAccount />} />
      <Route
        path="import-account/create-password"
        element={<CreatePassword onNext={() => navigate('/register/complete')} />}
      />
      <Route path="complete" element={<SetupComplete onDone={handleSetUpComplete} />} />
    </Routes>
  );
};
