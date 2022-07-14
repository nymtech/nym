import ValidatorClient from '../../dist';
import expect from 'expect';

// TODO: implement for QA with .env for mnemonics
// describe('Sign: send', () => {
//   it('can send tokens', async () => {
//     const client = await ValidatorClient.connect(
//       '<ADD MNEMONIC HERE>',
//       'https://rpc.nyx.nodes.guru/', 'https://validator.nymtech.net/api/', 'n', 'n14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sjyvg3g', 'n1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrq73f2nw', 'nym');
//     await client.send('<ADD ADDRESS HERE>')
//     const balance = await client.getBalance('n10yyd98e2tuwu0f7ypz9dy3hhjw7v772q6287gy');
//     expect(Number.parseFloat(balance.amount)).toBeGreaterThan(0);
//   }).timeout(5000);
// })