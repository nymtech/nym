import walletLogin from '../pageobjects/wallet.login'

describe('My Login application', () => {
  it('should login with valid credentials', async () => {
    let test = 'enter new mnemonic for the test'

    await walletLogin.enterMnemonic(test)
  })
})
