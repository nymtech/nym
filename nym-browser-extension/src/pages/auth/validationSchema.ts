import * as z from 'zod';

export const validationSchema = z.object({
  password: z.string().min(1, { message: 'Required' }),
});
