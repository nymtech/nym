package keeper

import (
	sdk "github.com/cosmos/cosmos-sdk/types"
	sdkerrors "github.com/cosmos/cosmos-sdk/types/errors"

	"github.com/cosmos/cosmos-sdk/codec"
	"github.com/nymtech/nym/validator/nym/x/nym/types"
)

// CreateMixnode creates a mixnode
func (k Keeper) CreateMixnode(ctx sdk.Context, mixnode types.Mixnode) {
	store := ctx.KVStore(k.storeKey)
	key := []byte(types.MixnodePrefix + mixnode.ID)
	value := k.cdc.MustMarshalBinaryLengthPrefixed(mixnode)
	store.Set(key, value)
}

// GetMixnode returns the mixnode information
func (k Keeper) GetMixnode(ctx sdk.Context, key string) (types.Mixnode, error) {
	store := ctx.KVStore(k.storeKey)
	var mixnode types.Mixnode
	byteKey := []byte(types.MixnodePrefix + key)
	err := k.cdc.UnmarshalBinaryLengthPrefixed(store.Get(byteKey), &mixnode)
	if err != nil {
		return mixnode, err
	}
	return mixnode, nil
}

// SetMixnode sets a mixnode
func (k Keeper) SetMixnode(ctx sdk.Context, mixnode types.Mixnode) {
	mixnodeKey := mixnode.ID
	store := ctx.KVStore(k.storeKey)
	bz := k.cdc.MustMarshalBinaryLengthPrefixed(mixnode)
	key := []byte(types.MixnodePrefix + mixnodeKey)
	store.Set(key, bz)
}

// DeleteMixnode deletes a mixnode
func (k Keeper) DeleteMixnode(ctx sdk.Context, key string) {
	store := ctx.KVStore(k.storeKey)
	store.Delete([]byte(types.MixnodePrefix + key))
}

//
// Functions used by querier
//

func listMixnode(ctx sdk.Context, k Keeper) ([]byte, error) {
	var mixnodeList []types.Mixnode
	store := ctx.KVStore(k.storeKey)
	iterator := sdk.KVStorePrefixIterator(store, []byte(types.MixnodePrefix))
	for ; iterator.Valid(); iterator.Next() {
		var mixnode types.Mixnode
		k.cdc.MustUnmarshalBinaryLengthPrefixed(store.Get(iterator.Key()), &mixnode)
		mixnodeList = append(mixnodeList, mixnode)
	}
	res := codec.MustMarshalJSONIndent(k.cdc, mixnodeList)
	return res, nil
}

func getMixnode(ctx sdk.Context, path []string, k Keeper) (res []byte, sdkError error) {
	key := path[0]
	mixnode, err := k.GetMixnode(ctx, key)
	if err != nil {
		return nil, err
	}

	res, err = codec.MarshalJSONIndent(k.cdc, mixnode)
	if err != nil {
		return nil, sdkerrors.Wrap(sdkerrors.ErrJSONMarshal, err.Error())
	}

	return res, nil
}

// Get creator of the item
func (k Keeper) GetMixnodeOwner(ctx sdk.Context, key string) sdk.AccAddress {
	mixnode, err := k.GetMixnode(ctx, key)
	if err != nil {
		return nil
	}
	return mixnode.Creator
}

// Check if the key exists in the store
func (k Keeper) MixnodeExists(ctx sdk.Context, key string) bool {
	store := ctx.KVStore(k.storeKey)
	return store.Has([]byte(types.MixnodePrefix + key))
}
