import Balance from '../pageobjects/balanceScreen'

class Welcome {
  //Welcome landing page
  get signInButton() { return $("[data-testid='signIn']") }
  get createAccount() { return $("[data-testid='createAccount']") }

  // Existing account sign in option page
  get signInMnemonic() { return $("[data-testid='signInWithMnemonic']") }
  get signInPassword() { return $("[data-testid='signInWithPassword']") }
  get backToWelcomePage() { return $("[data-testid='backToWelcomePage']") }
  get forgotPassword() { return $("[data-testid='forgotPassword']") }

  // Sign in with mnemonic page 
  get mnemonicInput() { return $("[data-testid='mnemonicInput']") }
  get signIn() { return $("[data-testid='signInSubmitButton']") }
  get backToSignInOptions() { return $("[data-testid='backToSignInOptions']") }
  get createPassword() { return $("[data-testid='goToCreatePassword']") }

  // Create password step 1/2
  get backToMnemonicSignIn() { return $("[data-testid='backToMnemonicSignIn']") }
  get nextToPasswordCreation() { return $("[data-testid='nextToPasswordCreation']") }

  // Create password step 2/2
  get password() { return $("[data-testid='Password']") }
  get confirmPassword() { return $("[data-testid='Confirm password']") }
  get createPasswordButton() { return $("[data-testid='createPasswordButton']") }
  get backToStep1PasswordCreation() { return $("[data-testid='backToStep1PasswordCreation']") }

  // Create account step 1/3
  get copyMnemonic() { return $("[data-testid='copyMnemonic']") }
  get iSavedMnemonic() { return $("[data-testid='iSavedMnemonic']") }
  get mnemonicPhrase() { return $("[data-testid='mnemonicPhrase']") }
  get backToWelcomePageFromCreate() { return $("[data-testid='backToWelcome']") }

  // Create account step 2/3
  get wordIndex() { return $$("[data-testid='wordIndex']") }
  get mnemonicWordTile() { return $$("[data-testid='mnemonicWordTile']") }
  get nextToStep3() { return $("[data-testid='nextToStep3']") }
  get backToStep1() { return $("[data-testid='backToStep1']") }

  // Create account step 3/3
  get next() { return $("[data-testid='next']") } // TO-DO possibly rename this?
  get skipPasswordAndSignInWithMnemonic() { return $("[data-testid='skipPasswordAndSignInWithMnemonic']") }

  // Enter password to sign in
  get enterPassword() { return $("[data-testid='Enter password']") }
  get signInPasswordButton() { return $("[data-testid='signInPasswordButton']") }
  get backToSignInOptionsFromPassword() { return $("[data-testid='backToSignInOptionsFromPassword']") }
  get forgotPasswordButton() { return $("[data-testid='forgotPasswordButton']") }

  // Errors
  get error() { return $("[data-testid='error']") }
  //TO-DO get this bit below working 
  getErrorMessage = async () => {
    await (await this.error).waitForDisplayed({ timeout: 1500 })
    await this.error.getText()
  }


  //login to the application
  loginWithMnemonic = async (mnemonic) => {
    await this.signInButton.click()
    await this.signInMnemonic.click()
    await this.mnemonicInput.waitForDisplayed()
    await this.mnemonicInput.addValue(mnemonic);
    await this.signIn.click();
    await Balance.nymBalance.isExisting();
  };

}

export default new Welcome()