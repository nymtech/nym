import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import { Login, Register } from 'src/pages/auth';
import { Delegation, Balance, Home, Receive, Send, Settings } from 'src/pages/';

export const AppRoutes = () => (
  <Router>
    <Routes>
      <Route path="/" element={<Home />} />
      <Route path="/auth">
        <Route path="/auth/register" element={<Register />} />
        <Route path="/auth/login" element={<Login />} />
      </Route>
      <Route path="/balance" element={<Balance />} />
      <Route path="/delegation" element={<Delegation />} />
      <Route path="/receive" element={<Receive />} />
      <Route path="/send" element={<Send />} />
      <Route path="/settings" element={<Settings />} />
    </Routes>
  </Router>
);
