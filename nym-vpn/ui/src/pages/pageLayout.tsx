import { Outlet } from 'react-router-dom';

function PageLayout() {
  return (
    <div className="h-full bg-blanc-nacre dark:bg-baltic-sea text-baltic-sea dark:text-white">
      <Outlet />
    </div>
  );
}

export default PageLayout;
