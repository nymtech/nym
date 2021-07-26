import {Alert} from '@material-ui/lab';
import { getDisplayExecGasFee } from "../common/helpers";


const ExecFeeNotice = ({name}: {name: string}) => {
    return (
        <Alert severity="info">
            The gas fee for
            <strong> {name} </strong>
            {`is ${getDisplayExecGasFee()}`}
        </Alert>
    )
}

export default ExecFeeNotice