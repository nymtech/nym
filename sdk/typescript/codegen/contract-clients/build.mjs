import codegen from '@cosmwasm/ts-codegen';

codegen
  .default({
    contracts: [
      { name: 'coconut-bandwidth', dir: '../../../../contracts/coconut-bandwidth' },
      { name: 'coconut-dkg', dir: '../../../../contracts/coconut-dkg' },
      { name: 'mixnet', dir: '../../../../contracts/mixnet' },
      { name: 'cw3-flex-multisig', dir: '../../../../contracts/multisig/cw3-flex-multisig' },
      { name: 'cw4-group', dir: '../../../../contracts/multisig/cw4-group' },
      { name: 'name-service', dir: '../../../../contracts/name-service' },
      { name: 'service-provider-directory', dir: '../../../../contracts/service-provider-directory' },
      { name: 'vesting', dir: '../../../../contracts/vesting' },
    ],
    outPath: './src',

    // options are completely optional ;)
    options: {
      bundle: {
        bundleFile: 'index.ts',
        scope: 'contracts',
      },
      types: {
        enabled: true,
      },
      client: {
        enabled: true,
      },
      // useContractsHooks: {
      //   enabled: false,
      // },
    },
  })
  .then(() => {
    console.log('✨ all done!');
  });
