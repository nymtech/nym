import React from "react";
import { FormControl, FormControlLabel, FormLabel, RadioGroup } from "@material-ui/core";
import { NodeType } from "../common/node";
import Radio from "@material-ui/core/Radio";
import { Dispatch, SetStateAction } from "react";

type NodeTypeChooserProps = {
    nodeType: NodeType,
    setNodeType: Dispatch<SetStateAction<NodeType>>
}

const NodeTypeChooser = (props: NodeTypeChooserProps) => {
    const handleNodeTypeChange = (event: React.ChangeEvent<HTMLInputElement>) => {
        let eventValue = (event.target as HTMLInputElement).value
        let type = NodeType[eventValue as keyof typeof NodeType]
        props.setNodeType(type);
    };

    return (
        <FormControl component="fieldset">
            <FormLabel component="legend">Select node type</FormLabel>
            <RadioGroup aria-label="nodeType" name="nodeTypeRadio" value={props.nodeType} onChange={handleNodeTypeChange}>
                <FormControlLabel value={NodeType.Mixnode} control={<Radio />} label="Mixnode" />
                <FormControlLabel value={NodeType.Gateway} control={<Radio />} label="Gateway" />
            </RadioGroup>
        </FormControl>
    )
}

export default NodeTypeChooser