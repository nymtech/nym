import SignalCellularAltIcon from "@mui/icons-material/SignalCellularAlt";
import SignalCellularAlt1Bar from "@mui/icons-material/SignalCellularAlt1Bar";
import SignalCellularAlt2Bar from "@mui/icons-material/SignalCellularAlt2Bar";
import SignalCellularConnectedNoInternet0BarIcon from "@mui/icons-material/SignalCellularConnectedNoInternet0Bar";

export const ScoreIcon = ({ score }: { score?: string }) => {
  if (!score) {
    return <SignalCellularAltIcon color="error" />;
  }
  if (score.toLowerCase() === "offline") {
    return <SignalCellularConnectedNoInternet0BarIcon color="disabled" />;
  }
  if (score.toLowerCase() === "high") {
    return <SignalCellularAltIcon color="success" />;
  }
  if (score.toLowerCase() === "medium") {
    return <SignalCellularAlt2Bar color="warning" />;
  }
  return <SignalCellularAlt1Bar color="disabled" />;
};

export const ReverseScoreIcon = ({ score }: { score?: string }) => {
  if (!score) {
    return <SignalCellularConnectedNoInternet0BarIcon color="disabled" />;
  }
  if (score.toLowerCase() === "offline") {
    return <SignalCellularConnectedNoInternet0BarIcon color="disabled" />;
  }
  if (score.toLowerCase() === "low") {
    return <SignalCellularAlt1Bar color="success" />;
  }
  if (score.toLowerCase() === "medium") {
    return <SignalCellularAlt2Bar color="warning" />;
  }
  return <SignalCellularAltIcon color="error" />;
};
