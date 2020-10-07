package nym

import (
	sdk "github.com/cosmos/cosmos-sdk/types"
	sdkerrors "github.com/cosmos/cosmos-sdk/types/errors"

	"github.com/nymtech/nym/validator/nym/x/nym/types"
	"github.com/nymtech/nym/validator/nym/x/nym/keeper"
)

// Handle a message to delete name
func handleMsgDeleteMixnode(ctx sdk.Context, k keeper.Keeper, msg types.MsgDeleteMixnode) (*sdk.Result, error) {
	if !k.MixnodeExists(ctx, msg.ID) {
		// replace with ErrKeyNotFound for 0.39+
		return nil, sdkerrors.Wrap(sdkerrors.ErrInvalidRequest, msg.ID)
	}
	if !msg.Creator.Equals(k.GetMixnodeOwner(ctx, msg.ID)) {
		return nil, sdkerrors.Wrap(sdkerrors.ErrUnauthorized, "Incorrect Owner")
	}

	k.DeleteMixnode(ctx, msg.ID)
	return &sdk.Result{}, nil
}
