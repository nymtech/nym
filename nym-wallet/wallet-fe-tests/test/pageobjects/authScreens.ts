import Balance from '../pageobjects/balanceScreen'

class Auth {
  //Welcome landing page
  get signInButton(): Promise<WebdriverIO.Element> { return $("[data-testid='signIn']") }
  get createAccount(): Promise<WebdriverIO.Element> { return $("[data-testid='createAccount']") }

  // Existing account sign in option page
  get signInMnemonic(): Promise<WebdriverIO.Element> { return $("[data-testid='signInWithMnemonic']") }
  get signInPassword(): Promise<WebdriverIO.Element> { return $("[data-testid='signInWithPassword']") }
  get backToWelcomePage(): Promise<WebdriverIO.Element> { return $("[data-testid='backToWelcomePage']") }
  get forgotPassword(): Promise<WebdriverIO.Element> { return $("[data-testid='forgotPassword']") }

  // Sign in with mnemonic page 
  get mnemonicLoginScreenHeader(): Promise<WebdriverIO.Element> { return $("[data-testid='Enter a mnemonic to sign in']") }
  get mnemonicInput(): Promise<WebdriverIO.Element> { return $("[data-testid='inputMnemonic']") }
  get signIn(): Promise<WebdriverIO.Element> { return $("[data-testid='signInWithMnemonicButton']") }
  get backToSignInOptions(): Promise<WebdriverIO.Element> { return $("[data-testid='backToSignInOptions']") }
  get revealMnemonic(): Promise<WebdriverIO.Element> { return $("[data-testid='Reveal Mnemonic']") }
  get createPassword(): Promise<WebdriverIO.Element> { return $("[data-testid='goToCreatePassword']") }

  // Create password step 1/2
  get backToMnemonicSignIn(): Promise<WebdriverIO.Element> { return $("[data-testid='backToMnemonicSignIn']") }
  get nextToPasswordCreation(): Promise<WebdriverIO.Element> { return $("[data-testid='nextToPasswordCreation']") }

  // Create password step 2/2
  get password(): Promise<WebdriverIO.Element> { return $("[data-testid='Password']") }
  get confirmPassword(): Promise<WebdriverIO.Element> { return $("[data-testid='Confirm password']") }
  get createPasswordButton(): Promise<WebdriverIO.Element> { return $("[data-testid='createPasswordButton']") }
  get backToStep1PasswordCreation(): Promise<WebdriverIO.Element> { return $("[data-testid='backToStep1PasswordCreation']") }

  // Create account step 1/3
  get copyMnemonic(): Promise<WebdriverIO.Element> { return $("[data-testid='copyMnemonic']") }
  get iSavedMnemonic(): Promise<WebdriverIO.Element> { return $("[data-testid='iSavedMnemonic']") }
  get mnemonicPhrase(): Promise<WebdriverIO.Element> { return $("mnemonicPhrase") }
  // get mnemonicPhrase(): Promise<WebdriverIO.Element> { return $("[data-testid='mnemonicPhrase']") }
  get backToWelcomePageFromCreate(): Promise<WebdriverIO.Element> { return $("[data-testid='backToWelcome']") }

  // Create account step 2/3
  get wordIndex(): Promise<WebdriverIO.ElementArray> { return $$("[data-testid='wordIndex']") }
  get mnemonicWordTile(): Promise<WebdriverIO.ElementArray> { return $$("[data-testid='mnemonicWordTile']") }
  // get wordIndex(): Promise<WebdriverIO.Element> { return $("[data-testid='wordIndex']") }
  // get mnemonicWordTile(): Promise<WebdriverIO.Element> { return $("[data-testid='mnemonicWordTile']") }
  get nextToStep3(): Promise<WebdriverIO.Element> { return $("[data-testid='nextToStep3']") }
  get backToStep1(): Promise<WebdriverIO.Element> { return $("[data-testid='backToStep1']") }

  // Create account step 3/3
  get nextStorePassword(): Promise<WebdriverIO.Element> { return $("[data-testid='nextStorePassword']") }
  get skipPasswordAndSignInWithMnemonic(): Promise<WebdriverIO.Element> { return $("[data-testid='skipPasswordAndSignInWithMnemonic']") }

  // Enter password to sign in
  get passwordLoginScreenHeader(): Promise<WebdriverIO.Element> { return $("[data-testid='Enter a password to sign in']") }
  get enterPassword(): Promise<WebdriverIO.Element> { return $("[data-testid='Enter password']") }
  get signInPasswordButton(): Promise<WebdriverIO.Element> { return $("[data-testid='signInPasswordButton']") }
  get backToSignInOptionsFromPassword(): Promise<WebdriverIO.Element> { return $("[data-testid='skipAndSignInWithMnemonic']") }
  get forgotPasswordButton(): Promise<WebdriverIO.Element> { return $("[data-testid='forgotPasswordButton']") }

  // Errors
  get error(): Promise<WebdriverIO.Element> { return $("[data-testid='error']") } //check
  //TO-DO get this bit below working 
  getErrorMessage = async () => {
    await (await this.error).waitForDisplayed({ timeout: 1500 })
    await (await this.error).getText()
  }

  //login to the application
  loginWithMnemonic = async (mnemonic) => {
    await (await this.signInButton).click()
    await (await this.signInMnemonic).click()
    await (await this.mnemonicInput).waitForDisplayed()
    await (await this.revealMnemonic).click()
    console.log("--------------- " + mnemonic)
    await (await this.mnemonicInput).addValue(mnemonic);
    await (await this.signIn).click();
    await (await Balance.nymBalance).waitForDisplayed({ timeout: 4000 });
  };

}

export default new Auth()