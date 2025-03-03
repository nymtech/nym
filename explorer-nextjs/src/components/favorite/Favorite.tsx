import {
  FavoriteBorder as FavoriteBorderIcon,
  Favorite as FavoriteIcon,
} from "@mui/icons-material";
import { IconButton } from "@mui/material";
import { useLocalStorage } from "@uidotdev/usehooks";

const Favorite = ({ address }: { address: string }) => {
  const [favorites, saveFavorites] = useLocalStorage<string[]>(
    "nym-node-favorites",
    [],
  );

  const onFavorite = (address: string) => {
    saveFavorites([...favorites, address]);
  };

  if (favorites.includes(address)) {
    return <UnFavorite address={address} />;
  }

  return (
    <IconButton
      onClick={(e) => {
        e.stopPropagation();
        onFavorite(address);
      }}
    >
      <FavoriteBorderIcon sx={{ color: "accent.main" }} />
    </IconButton>
  );
};

const UnFavorite = ({ address }: { address: string }) => {
  const [favorites, saveFavorites] = useLocalStorage<string[]>(
    "nym-node-favorites",
    [],
  );

  const handleUnfavorite = (address: string) => {
    saveFavorites(favorites.filter((favorite) => favorite !== address));
  };

  return (
    <IconButton
      onClick={(e) => {
        e.stopPropagation();
        handleUnfavorite(address);
      }}
    >
      <FavoriteIcon sx={{ color: "accent.main" }} />
    </IconButton>
  );
};

export { Favorite, UnFavorite };
