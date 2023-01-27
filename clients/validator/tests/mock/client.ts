import { INyxdQuery } from "../../src/query-client";
import { Mock, Times } from "moq.ts";

export class TestHelper {
    // beforeEach(mock: Mock<any>) {
    //     client = new Mock<INyxdQuery>();
    //     mock..reset();
    // }

    tests(mock: Mock<any>, methodName: string, args: any[], expectedResult: any) {
        mock.setup(nym => nym[methodName](...args)).returns(Promise.resolve(expectedResult));
        let obj = mock.object();
        let execute = obj[methodName](...args);
        mock.verify(nym => nym[methodName](...args), Times.Exactly(1));
        // expect(execute).toBeTruthy();
        // expect(execute).toStrictEqual(expectedResult);
        return execute;
    }
}
