export type CmdErrorSource = 'InternalError' | 'CallerError' | 'Unknown';

export interface CmdError {
  source: CmdErrorSource;
  message: string;
}
