package nym

import (
	sdk "github.com/cosmos/cosmos-sdk/types"
	sdkerrors "github.com/cosmos/cosmos-sdk/types/errors"

	"github.com/nymtech/nym/validator/nym/x/nym/keeper"
	"github.com/nymtech/nym/validator/nym/x/nym/types"
)

func handleMsgSetMixnode(ctx sdk.Context, k keeper.Keeper, msg types.MsgSetMixnode) (*sdk.Result, error) {
	var mixnode = types.Mixnode{
		Creator:  msg.Creator,
		ID:       msg.ID,
		PubKey:   msg.PubKey,
		Layer:    msg.Layer,
		Version:  msg.Version,
		Host:     msg.Host,
		Location: msg.Location,
		Stake:    msg.Stake,
	}
	if !msg.Creator.Equals(k.GetMixnodeOwner(ctx, msg.ID)) { // Checks if the the msg sender is the same as the current owner
		return nil, sdkerrors.Wrap(sdkerrors.ErrUnauthorized, "Incorrect Owner") // If not, throw an error
	}

	k.SetMixnode(ctx, mixnode)

	return &sdk.Result{Events: ctx.EventManager().Events()}, nil
}
