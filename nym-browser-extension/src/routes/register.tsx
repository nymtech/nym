import React from 'react';
import { Route, Routes, useNavigate } from 'react-router-dom';
import { CreatePassword, ImportAccount, SeedPhrase, SetupComplete } from 'src/pages/register';

export const RegisterRoutes = () => {
  const navigate = useNavigate();
  return (
    <Routes>
      <Route path="seed-phrase" element={<SeedPhrase />} />
      <Route path="create-password" element={<CreatePassword onNext={() => navigate('/register/seed-phrase')} />} />
      <Route path="import-account" element={<ImportAccount />} />
      <Route
        path="import-account/create-password"
        element={<CreatePassword onNext={() => navigate('/register/complete')} />}
      />
      <Route path="complete" element={<SetupComplete onDone={() => navigate('/login')} />} />
    </Routes>
  );
};
