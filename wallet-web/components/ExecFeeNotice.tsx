import {Alert} from '@material-ui/lab';
import { getDisplayExecGasFee } from "../common/helpers";

type ExecFeeNoticeProps = {
    name: string
}

const ExecFeeNotice = (props: ExecFeeNoticeProps) => {
    return (
        <Alert severity="info">
            The gas fee for
            <strong> {props.name} </strong>
            {`is ${getDisplayExecGasFee()}`}
        </Alert>
    )
}

export default ExecFeeNotice