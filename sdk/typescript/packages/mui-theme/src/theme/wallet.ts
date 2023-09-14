import { NymPaletteVariant } from './common';

/**
 * This interface defines a palette used by the Nym wallet
 */
export interface NymWalletPalette {
  wallet: {
    fee: string;
  };
}

export const nymWalletPallete = (_variant: NymPaletteVariant): NymWalletPalette => ({
  wallet: {
    fee: '#967FF0',
  },
});
