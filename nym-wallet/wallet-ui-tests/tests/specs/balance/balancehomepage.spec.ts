import Balance from '../../pageobjects/balanceScreen'
import Auth from '../../pageobjects/authScreens'

describe('Balance screen displays correctly', () => {
  const mne = "giraffe note order sun cradle bottom crime humble able antique rural donkey guess parent potato tongue truly way disagree exile zebra someone else typical";

  it('selecting qa network', async () => {
    await (await Auth.signInButton).click()
    await (await Auth.signInMnemonic).click()
    await (await Auth.mnemonicInput).waitForDisplayed()
    await (await Auth.mnemonicInput).addValue(mne)
    await (await Auth.signIn).click()

    await (await Balance.networkDropdown).waitForDisplayed({timeout: 1500})
    await (await Balance.networkDropdown).click()
    // TO-DO fix the selector below, it's an odd one
    await (await Balance.networkSelectQa).waitForDisplayed({timeout: 1500})
    await (await Balance.networkSelectQa).click()
    let network = await (await Balance.networkDropdown).getText()
    expect(network).toStrictEqual('QA')

  })

  it('copy the account id', async () => {
    // await (await Auth.signInButton).click()
    // await (await Auth.signInMnemonic).click()
    // await (await Auth.mnemonicInput).waitForDisplayed()
    // await (await Auth.mnemonicInput).addValue(mne)
    // await (await Auth.signIn).click()

    // await (await Balance.copyAccountId).click()

  })

})