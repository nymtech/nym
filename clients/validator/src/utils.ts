// eslint-disable-next-line @typescript-eslint/no-explicit-any,@typescript-eslint/explicit-module-boundary-types
import { Coin } from '@cosmjs/stargate';
import { EncodeObject } from '@cosmjs/proto-signing';

export function makeBankMsgSend(
  senderAddress: string,
  recipientAddress: string,
  transferAmount: readonly Coin[],
): EncodeObject {
  return {
    typeUrl: '/cosmos.bank.v1beta1.MsgSend',
    value: {
      fromAddress: senderAddress,
      toAddress: recipientAddress,
      amount: transferAmount,
    },
  };
}
