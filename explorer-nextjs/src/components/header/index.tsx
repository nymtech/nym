import { DesktopHeader } from "./DesktopHeader";
import { MobileHeader } from "./MobileHeader";

export const Header = async () => {
  return (
    <header>
      <DesktopHeader />
      <MobileHeader />
      {/* Mobile header will go here */}
    </header>
  );
};
