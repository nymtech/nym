import ExplorerCard from "../cards/ExplorerCard";
import { Remark42Comments } from "../comments";

export const NodeChatCard = () => {
  return (
    <ExplorerCard label="Chat" sx={{ height: "100%" }}>
      <Remark42Comments />
    </ExplorerCard>
  );
};
