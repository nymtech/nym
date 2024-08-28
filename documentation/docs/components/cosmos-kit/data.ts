import { AminoMsg, makeSignDoc, serializeSignDoc } from '@cosmjs/amino';
import { MsgSend } from 'cosmjs-types/cosmos/bank/v1beta1/tx';

export const getDoc = (address: string) => {
  const chainId = 'nyx';

  const msg: AminoMsg = {
    type: '/cosmos.bank.v1beta1.MsgSend',
    value: MsgSend.fromPartial({
      fromAddress: address,
      toAddress: 'n1nn8tghp94n8utsgyg3kfttlxm0exgjrsqkuwu9',
      amount: [{ amount: '1000', denom: 'unym' }],
    }),
  };
  const fee = {
    amount: [{ amount: '2000', denom: 'ucosm' }],
    gas: '180000', // 180k
  };
  const memo = 'Use your power wisely';
  const accountNumber = 15;
  const sequence = 16;

  return makeSignDoc([msg], fee, chainId, memo, accountNumber, sequence);
};
export const aminoDoc = (address: string) => {
  const signDoc = getDoc(address);
  return serializeSignDoc(signDoc);
};
