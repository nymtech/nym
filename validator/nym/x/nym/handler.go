package nym

import (
	"fmt"

	sdk "github.com/cosmos/cosmos-sdk/types"
	sdkerrors "github.com/cosmos/cosmos-sdk/types/errors"
	"github.com/nymtech/nym/validator/nym/x/nym/keeper"
	"github.com/nymtech/nym/validator/nym/x/nym/types"
)

// NewHandler ...
func NewHandler(k keeper.Keeper) sdk.Handler {
	return func(ctx sdk.Context, msg sdk.Msg) (*sdk.Result, error) {
		ctx = ctx.WithEventManager(sdk.NewEventManager())
		switch msg := msg.(type) {
		// this line is used by starport scaffolding # 1
		case types.MsgCreateMixnode:
			return handleMsgCreateMixnode(ctx, k, msg)
		case types.MsgSetMixnode:
			return handleMsgSetMixnode(ctx, k, msg)
		case types.MsgDeleteMixnode:
			return handleMsgDeleteMixnode(ctx, k, msg)
		default:
			errMsg := fmt.Sprintf("unrecognized %s message type: %T", types.ModuleName, msg)
			return nil, sdkerrors.Wrap(sdkerrors.ErrUnknownRequest, errMsg)
		}
	}
}
