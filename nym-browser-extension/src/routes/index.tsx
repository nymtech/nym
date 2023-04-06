import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import { Delegation, BalancePage, Home, Receive, Send, Settings, Login } from 'src/pages/';
import { RegisterRoutes } from './register';

export const AppRoutes = () => (
  <Router>
    <Routes>
      <Route path="/" element={<Home />} />
      <Route path="/register/*" element={<RegisterRoutes />} />
      <Route path="/login" element={<Login />} />
      <Route path="/balance" element={<BalancePage />} />
      <Route path="/delegation" element={<Delegation />} />
      <Route path="/receive" element={<Receive />} />
      <Route path="/send" element={<Send />} />
      <Route path="/settings" element={<Settings />} />
    </Routes>
  </Router>
);
