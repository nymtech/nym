import ValidatorClient from '@nymproject/nym-validator-client';
import * as z from 'zod';

export const validationSchema = z.object({
  password: z
    .string()
    .min(1, { message: 'Required' })
    .refine(
      async (password) => {
        try {
          await ValidatorClient.mnemonicToAddress(password, 'n');
          return true;
        } catch (e) {
          return false;
        }
      },
      { message: 'Incorrect password. Please try again' },
    ),
});
