import { validateAmount } from "@/utils/currency";
import { z } from "zod";

const MIN_AMOUNT_TO_DELEGATE = "10";
const fee = { gas: "1000000", amount: [{ amount: "1000000", denom: "unym" }] };

const stakingSchema = z
  .object({
    mixId: z.number(),
    balance: z.string().refine(
      async (val) => {
        const num = Number.parseFloat(val);
        return num > 0;
      },
      {
        message: "Balance must be greater than 0",
      },
    ),
    amount: z.string().refine(
      async (val) => {
        let isValid = false;

        isValid = await validateAmount(val, MIN_AMOUNT_TO_DELEGATE.toString());
        isValid =
          isValid &&
          Number.parseFloat(val) >= Number.parseFloat(MIN_AMOUNT_TO_DELEGATE);
        return isValid;
      },
      {
        message: "Amount must be greater than 10 NYM",
      },
    ),
  })
  .extend({})
  .refine(
    (data) => {
      const balance = Number.parseFloat(data.balance);
      const amount = Number.parseFloat(data.amount);
      console.log(balance);
      return balance - amount >= 0;
    },
    {
      message: "Not enough funds",
    },
  );

export default stakingSchema;
export { MIN_AMOUNT_TO_DELEGATE, fee };
