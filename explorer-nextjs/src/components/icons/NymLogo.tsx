import Image from "next/image";
import { icons } from "@/utils/getIconByName";

const NymLogo = () => {
  return (
    <Image src={icons.nymLogo} alt="Nym Logo" width={90} height={25} />
  );
};
export default NymLogo;
