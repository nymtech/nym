import Image from "next/image";
import { icons } from "@/utils/getIconByName";

const ChevronMenu = () => {
  return (
    <Image
      src={icons.chevronMenu}
      alt="Chevron Menu"
      width={24}
      height={24}
    />
  );
};

export default ChevronMenu;
