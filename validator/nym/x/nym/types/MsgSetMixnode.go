package types

import (
	sdk "github.com/cosmos/cosmos-sdk/types"
	sdkerrors "github.com/cosmos/cosmos-sdk/types/errors"
)

var _ sdk.Msg = &MsgSetMixnode{}

type MsgSetMixnode struct {
  ID      string      `json:"id" yaml:"id"`
  Creator sdk.AccAddress `json:"creator" yaml:"creator"`
  PubKey string `json:"pubKey" yaml:"pubKey"`
  Layer int32 `json:"layer" yaml:"layer"`
  Version string `json:"version" yaml:"version"`
  Host string `json:"host" yaml:"host"`
  Location string `json:"location" yaml:"location"`
  Stake int32 `json:"stake" yaml:"stake"`
}

func NewMsgSetMixnode(creator sdk.AccAddress, id string, pubKey string, layer int32, version string, host string, location string, stake int32) MsgSetMixnode {
  return MsgSetMixnode{
    ID: id,
		Creator: creator,
    PubKey: pubKey,
    Layer: layer,
    Version: version,
    Host: host,
    Location: location,
    Stake: stake,
	}
}

func (msg MsgSetMixnode) Route() string {
  return RouterKey
}

func (msg MsgSetMixnode) Type() string {
  return "SetMixnode"
}

func (msg MsgSetMixnode) GetSigners() []sdk.AccAddress {
  return []sdk.AccAddress{sdk.AccAddress(msg.Creator)}
}

func (msg MsgSetMixnode) GetSignBytes() []byte {
  bz := ModuleCdc.MustMarshalJSON(msg)
  return sdk.MustSortJSON(bz)
}

func (msg MsgSetMixnode) ValidateBasic() error {
  if msg.Creator.Empty() {
    return sdkerrors.Wrap(sdkerrors.ErrInvalidAddress, "creator can't be empty")
  }
  return nil
}