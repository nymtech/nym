import { MoreVert } from "@mui/icons-material";
import { IconButton, Menu, MenuItem } from "@mui/material";
import { useState } from "react";

type StakeAction = "unstake";

const StakeActions = ({
  nodeId,
  onActionSelect,
}: {
  nodeId?: number;
  onActionSelect: (action: StakeAction) => void;
}) => {
  const [anchorEl, setAnchorEl] = useState<null | HTMLElement>(null);
  const open = Boolean(anchorEl);

  const handleShowMenu = (event: React.MouseEvent<HTMLButtonElement>) => {
    event.stopPropagation();
    setAnchorEl(event.currentTarget);
  };

  const handleClose = () => {
    setAnchorEl(null);
  };

  const handleActionSelect = (action: StakeAction) => {
    onActionSelect(action);
    handleClose();
  };

  return (
    <>
      <IconButton onClick={handleShowMenu} disabled={!nodeId}>
        <MoreVert />
      </IconButton>
      <Menu
        elevation={0}
        anchorEl={anchorEl}
        open={open}
        onClose={() => {
          handleClose();
        }}
        onClick={(e) => {
          e.stopPropagation();
          handleClose();
        }}
        hideBackdrop
      >
        <StakeAction
          actionName="Unstake"
          onSelect={() => handleActionSelect("unstake")}
        />
      </Menu>
    </>
  );
};

const StakeAction = ({
  actionName,
  disabled,
  onSelect,
}: {
  actionName: string;
  disabled?: boolean;
  onSelect: () => void;
}) => {
  return (
    <MenuItem
      disabled={disabled}
      onClick={(e) => {
        e.stopPropagation();
        onSelect();
      }}
    >
      {actionName}
    </MenuItem>
  );
};

export default StakeActions;
