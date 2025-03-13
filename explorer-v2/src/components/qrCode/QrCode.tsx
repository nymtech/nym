import { Box } from "@mui/material";
import { QRCodeCanvas } from "qrcode.react";

export interface ICardQRCodeProps {
  url: string;
}

export const CardQRCode = (props: ICardQRCodeProps) => {
  const { url } = props;
  return (
    <Box
      border={"1px solid #C3D7D7"}
      display={"flex"}
      justifyContent={"center"}
      alignItems={"center"}
      width={144}
      height={144}
    >
      <QRCodeCanvas value={url} />
    </Box>
  );
};
