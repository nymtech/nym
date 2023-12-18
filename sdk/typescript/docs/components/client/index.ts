export const mainnetSettings = {
  url: 'wss://rpc.nymtech.net:443',
  mixnetContractAddress: 'n17srjznxl9dvzdkpwpw24gg668wc73val88a6m5ajg6ankwvz9wtst0cznr',
  mnemonic: process.env.MAINNET_MNEMONIC,
  address: 'n1c7y676pe3av76r5usala759xgj0yplmvngu8u8',
};

export const qaSettings = {
  url: 'wss://rpc.sandbox.nymtech.net',
  mixnetContractAddress: 'n1xr3rq8yvd7qplsw5yx90ftsr2zdhg4e9z60h5duusgxpv72hud3sjkxkav',
  mnemonic: process.env.QA_MNEMONIC,
  address: 'n13uryxldwdllpakevsmt6n0uyfn3kgr2wvj5dnf',
};

export const settings = qaSettings;
