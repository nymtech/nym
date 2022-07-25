class Helpers {
  //helper to decode mnemonic so plain 24 character passphrase isn't in sight albeit it is presented when ruunning the scripts
  // TO=-DO weork on this, figure out what's going on with the decoding bit
  decodeBase = async (input) => {
    var m = Buffer.from(input, "base64").toString();
    return m;
  };

  navigateAndClick = async (element) => {
    await element.click();
  };

  scrollIntoView = async (element) => {
    await element.scrollIntoView();
  };
}

module.exports = new Helpers();
