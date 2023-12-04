import { Outlet } from 'react-router-dom';
import { TopBar } from '../ui';

function NavLayout() {
  return (
    <div className="h-full flex flex-col bg-blanc-nacre dark:bg-baltic-sea text-baltic-sea dark:text-white">
      <TopBar />
      <Outlet />
    </div>
  );
}

export default NavLayout;
