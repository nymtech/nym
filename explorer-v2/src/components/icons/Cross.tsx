import Image from "next/image";
import { icons } from "@/utils/getIconByName";

const Cross = () => (
  <Image src={icons.cross} alt="cross" width={12.5} height={12.5} />
);

export default Cross;
