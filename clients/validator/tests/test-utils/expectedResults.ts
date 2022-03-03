import { ExecuteResult } from "@cosmjs/cosmwasm-stargate";
import { logs } from "@cosmjs/stargate";

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