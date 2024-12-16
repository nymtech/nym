import {
  FavoriteBorder as FavoriteBorderIcon,
  Favorite as FavoriteIcon,
} from "@mui/icons-material";
import { IconButton } from "@mui/material";

const Favorite = ({ onFavorite }: { onFavorite: () => void }) => {
  return (
    <IconButton onClick={onFavorite}>
      <FavoriteBorderIcon sx={{ color: "accent.main" }} />
    </IconButton>
  );
};

const UnFavorite = ({ onUnfavorite }: { onUnfavorite: () => void }) => {
  return (
    <IconButton onClick={onUnfavorite}>
      <FavoriteIcon sx={{ color: "accent.main" }} />
    </IconButton>
  );
};

export { Favorite, UnFavorite };
