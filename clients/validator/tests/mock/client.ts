import { INyxdQuery } from "../../src/query-client";
import { Mock, Times } from "moq.ts";
import expect from 'expect';

export class TestHelper {

    tests = async <T>(la, methodName: string, args: any[], expectedResult: any): Promise<T> => {
    const client = new Mock<INyxdQuery>().setup((nym) => nym[methodName](...args)).returns(Promise.resolve(expectedResult));
    const obj = client.object();
    const actualDetails = await obj[methodName](...args);
    client.verify((nym) => nym[methodName](...args), Times.Exactly(1));
    expect(actualDetails).toBeDefined();
    return actualDetails;
};
};

