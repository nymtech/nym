import { Mock, Times } from 'moq.ts';
import expect from 'expect';
import { INyxdQuery } from '../../query-client';

export class TestHelper {
  buildMethod = async <T>(methodName: string, args: any[], expectedResult: any): Promise<T> => {
    const client = new Mock<INyxdQuery>()
      .setup((nym) => nym[methodName](...args))
      .returns(Promise.resolve(expectedResult));
    const obj = client.object();
    const actualDetails = await obj[methodName](...args);

    client.verify((nym) => nym[methodName](...args), Times.Exactly(1));
    expect(Object.keys([actualDetails])).toEqual(Object.keys(expectedResult));
    expect(actualDetails).toBeDefined();

    return actualDetails;
  };
}
