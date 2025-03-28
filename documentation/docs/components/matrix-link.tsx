import Image from "next/image";
import Link from "next/link";
import matrixLogo from "../public/images/matrix-logo.png";

export const Matrix = () => {
  return (
    <Link
      href={"https://matrix.to/#/#dev:nymtech.chat"}
      target="_blank"
      rel="noopener noreferrer"
    >
      <Image src={matrixLogo}
        style = {{
          marginRight: "0.6rem"
        }}
        alt={"Matrix Logo"}
        width={20}
        height={24} 
      />
    </Link>
  );
};
