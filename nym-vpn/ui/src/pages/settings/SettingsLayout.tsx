import { Outlet } from 'react-router-dom';

function SettingsLayout() {
    return (
        <div className="what you need as custom style for all settings pages">
            <Outlet />
        </div>
    );
}

export default SettingsLayout;