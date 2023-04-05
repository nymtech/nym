import { MemoryRouter as Router, Routes, Route } from 'react-router-dom';
import { Delegation, Balance, Home, Receive, Send, Settings, Login } from 'src/pages/';
import { RegisterRoutes } from './register';

export const AppRoutes = () => (
  <Router>
    <Routes>
      {RegisterRoutes}
      <Route path="/" element={<Home />} />
      <Route path="/login" element={<Login />} />
      <Route path="/balance" element={<Balance />} />
      <Route path="/delegation" element={<Delegation />} />
      <Route path="/receive" element={<Receive />} />
      <Route path="/send" element={<Send />} />
      <Route path="/settings" element={<Settings />} />
    </Routes>
  </Router>
);
