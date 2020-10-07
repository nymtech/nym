package types

import (
	sdk "github.com/cosmos/cosmos-sdk/types"
)

type Mixnode struct {
	Creator sdk.AccAddress `json:"creator" yaml:"creator"`
	ID      string         `json:"id" yaml:"id"`
    PubKey string `json:"pubKey" yaml:"pubKey"`
    Layer int32 `json:"layer" yaml:"layer"`
    Version string `json:"version" yaml:"version"`
    Host string `json:"host" yaml:"host"`
    Location string `json:"location" yaml:"location"`
    Stake int32 `json:"stake" yaml:"stake"`
}