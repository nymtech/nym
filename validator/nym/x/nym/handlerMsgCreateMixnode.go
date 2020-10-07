package nym

import (
	sdk "github.com/cosmos/cosmos-sdk/types"

	"github.com/nymtech/nym/validator/nym/x/nym/keeper"
	"github.com/nymtech/nym/validator/nym/x/nym/types"
)

func handleMsgCreateMixnode(ctx sdk.Context, k keeper.Keeper, msg types.MsgCreateMixnode) (*sdk.Result, error) {
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
	k.CreateMixnode(ctx, mixnode)

	return &sdk.Result{Events: ctx.EventManager().Events()}, nil
}
