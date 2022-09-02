import ValidatorClient from '@nymproject/nym-validator-client';

describe.skip('Creating valid account', () => {
  it('create mnemonic', async () => {
    const benny = "giraffe note order sun cradle bottom crime humble able antique rural donkey guess parent potato tongue truly way disagree exile zebra someone else typical";
    const mnemonic = ValidatorClient.randomMnemonic();
    console.log(ValidatorClient);
    const newAccountClient = await ValidatorClient.connect(mnemonic,
      'https://qa-validator.nymtech.net', 'https://qa-validator-api.nymtech.net/api', 'n', 'n1suhgf5svhu4usrurvxzlgn54ksxmn8gljarjtxqnapv8kjnp4nrsd3qaep', 'n1xr3rq8yvd7qplsw5yx90ftsr2zdhg4e9z60h5duusgxpv72hud3sjkxkav', 'nym');
    const address = newAccountClient.address;
    console.log({ address, mnemonic });

    const client = await ValidatorClient.connect(
      benny, 'https://qa-validator.nymtech.net', 'https://qa-validator-api.nymtech.net/api', 'n', 'n1suhgf5svhu4usrurvxzlgn54ksxmn8gljarjtxqnapv8kjnp4nrsd3qaep', 'n1xr3rq8yvd7qplsw5yx90ftsr2zdhg4e9z60h5duusgxpv72hud3sjkxkav', 'nym');

    await client.send(address, [{ amount: '10000000', denom: 'unym' }]);
    const balance = await client.getBalance(address);
    console.log({ balance });
    expect(Number.parseFloat(balance.amount)).toBe(10000000);
  }).timeout(5000);
})


// the newly created address from the test above:

// address: 'n13l7rwrygs0m3kx3en2eh55dtmwlzm0vskw0hxq',
// mnemonic: 'tree upset require kitten inquiry truck emotion ladder reject elbow page ability spot win board frog child much credit pizza picture hover medal zoo'

// always make sure it's on QA, unless youre on debug branch (~look in nym_path wdio.config.ts to check)
// ENABLE_QA_MODE=true target/release/nym-wallet 
