import { coinGeckoPriceAPI } from 'src/urls';

export type Currency = 'gbp' | 'usd';
export type TokenId = 'nym';

type ResponseMap<T extends TokenId, C extends Currency> = { [token in T]: { [currency in C]: number } };

const constructUrl = (tokenId: TokenId, currency: Currency) =>
  `${coinGeckoPriceAPI}ids=${tokenId}&vs_currencies=${currency}`;

export async function getTokenPrice<T extends TokenId, C extends Currency>(
  tokenId: T,
  currency: C,
): Promise<ResponseMap<T, C>> {
  const response = await fetch(constructUrl(tokenId, currency));
  const json = await response.json();
  return json;
}
