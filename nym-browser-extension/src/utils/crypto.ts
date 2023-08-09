import cryptojs from 'crypto-js';

const encrypt = (mnemonic: string, password: string) => cryptojs.AES.encrypt(mnemonic, password).toString();

const decrypt = (cipher: string, password: string) =>
  cryptojs.AES.decrypt(cipher, password).toString(cryptojs.enc.Utf8);

export { encrypt, decrypt };
