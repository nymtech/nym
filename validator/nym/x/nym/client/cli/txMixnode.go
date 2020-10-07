package cli

import (
	"bufio"
	"github.com/spf13/cobra"
	"strconv"

	"github.com/cosmos/cosmos-sdk/client/context"
	"github.com/cosmos/cosmos-sdk/codec"
	sdk "github.com/cosmos/cosmos-sdk/types"
	"github.com/cosmos/cosmos-sdk/x/auth"
	"github.com/cosmos/cosmos-sdk/x/auth/client/utils"
	"github.com/nymtech/nym/validator/nym/x/nym/types"
)

func GetCmdCreateMixnode(cdc *codec.Codec) *cobra.Command {
	return &cobra.Command{
		Use:   "create-mixnode [pubKey] [layer] [version] [host] [location] [stake]",
		Short: "Creates a new mixnode",
		Args:  cobra.MinimumNArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			argsPubKey := string(args[0])
			argsLayer, _ := strconv.ParseInt(args[1], 10, 64)
			argsVersion := string(args[2])
			argsHost := string(args[3])
			argsLocation := string(args[4])
			argsStake, _ := strconv.ParseInt(args[5], 10, 64)

			cliCtx := context.NewCLIContext().WithCodec(cdc)
			inBuf := bufio.NewReader(cmd.InOrStdin())
			txBldr := auth.NewTxBuilderFromCLI(inBuf).WithTxEncoder(utils.GetTxEncoder(cdc))
			msg := types.NewMsgCreateMixnode(cliCtx.GetFromAddress(), string(argsPubKey), int32(argsLayer), string(argsVersion), string(argsHost), string(argsLocation), int32(argsStake))
			err := msg.ValidateBasic()
			if err != nil {
				return err
			}
			return utils.GenerateOrBroadcastMsgs(cliCtx, txBldr, []sdk.Msg{msg})
		},
	}
}

func GetCmdSetMixnode(cdc *codec.Codec) *cobra.Command {
	return &cobra.Command{
		Use:   "set-mixnode [id]  [pubKey] [layer] [version] [host] [location] [stake]",
		Short: "Set a new mixnode",
		Args:  cobra.MinimumNArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			id := args[0]
			argsPubKey := string(args[1])
			argsLayer, _ := strconv.ParseInt(args[2], 10, 64)
			argsVersion := string(args[3])
			argsHost := string(args[4])
			argsLocation := string(args[5])
			argsStake, _ := strconv.ParseInt(args[6], 10, 64)

			cliCtx := context.NewCLIContext().WithCodec(cdc)
			inBuf := bufio.NewReader(cmd.InOrStdin())
			txBldr := auth.NewTxBuilderFromCLI(inBuf).WithTxEncoder(utils.GetTxEncoder(cdc))
			msg := types.NewMsgSetMixnode(cliCtx.GetFromAddress(), id, string(argsPubKey), int32(argsLayer), string(argsVersion), string(argsHost), string(argsLocation), int32(argsStake))
			err := msg.ValidateBasic()
			if err != nil {
				return err
			}
			return utils.GenerateOrBroadcastMsgs(cliCtx, txBldr, []sdk.Msg{msg})
		},
	}
}

func GetCmdDeleteMixnode(cdc *codec.Codec) *cobra.Command {
	return &cobra.Command{
		Use:   "delete-mixnode [id]",
		Short: "Delete a new mixnode by ID",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {

			cliCtx := context.NewCLIContext().WithCodec(cdc)
			inBuf := bufio.NewReader(cmd.InOrStdin())
			txBldr := auth.NewTxBuilderFromCLI(inBuf).WithTxEncoder(utils.GetTxEncoder(cdc))

			msg := types.NewMsgDeleteMixnode(args[0], cliCtx.GetFromAddress())
			err := msg.ValidateBasic()
			if err != nil {
				return err
			}
			return utils.GenerateOrBroadcastMsgs(cliCtx, txBldr, []sdk.Msg{msg})
		},
	}
}
