import Image from "next/image";
import { icons } from "@/utils/getIconByName";

const CrossDark = () => (
  <Image src={icons.crossDark} alt="cross" width={12.5} height={12.5} />
);

export default CrossDark;
