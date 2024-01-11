import { Outlet } from 'react-router-dom';
import clsx from 'clsx';
import { TopBar } from '../ui';

function MainLayout() {
  return (
    <div
      className={clsx([
        'h-full flex flex-col',
        'bg-blanc-nacre text-baltic-sea',
        'dark:bg-baltic-sea dark:text-white',
      ])}
    >
      <TopBar />
      <div className="h-full flex flex-col overflow-auto overscroll-auto p-4">
        <div className="grow">
          <Outlet />
        </div>
      </div>
    </div>
  );
}

export default MainLayout;
