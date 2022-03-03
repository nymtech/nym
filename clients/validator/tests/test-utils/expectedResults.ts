import { ExecuteResult } from "@cosmjs/cosmwasm-stargate";
import { DeliverTxResponse, logs } from "@cosmjs/stargate";

export const promiseExecuteResult = (): Promise<ExecuteResult> => {
  let log = <logs.Log>{
    msg_index: 0,
    log: "test",
    events: [],
  };
  return Promise.resolve(<ExecuteResult>{
    logs: [log],
    transactionHash:
      "9C7BF465AB5CAB0D62446CBB251CF89CD173A640C5DE8DBC14A4BB950916114E",
  });
};

export const promiseTxResult = (): Promise<DeliverTxResponse> => {
  return Promise.resolve(<DeliverTxResponse>{
    code: 0,
    height: 1208302,
    rawLog: "[]",
    transactionHash:
      "9C7BF465AB5CAB0D62446CBB251CF89CD173A640C5DE8DBC14A4BB950916114E",
    gasUsed: 65042,
    gasWanted: 67977,
  });
};
