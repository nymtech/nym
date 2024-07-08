"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.WithValue = exports.Default = void 0;
var jsx_runtime_1 = require("react/jsx-runtime");
var react_1 = require("react");
var networks_1 = require("@lib/components/networks");
exports.default = {
    title: 'Networks/Network Selector',
    component: networks_1.NetworkSelector,
    argTypes: {
        network: {
            options: ['MAINNET', 'SANDBOX', 'QA'],
            control: { type: 'radio' },
        },
        onSwitchNetwork: { type: 'function' },
    },
};
var Template = function (_a) {
    var networkArg = _a.network, onSwitchNetwork = _a.onSwitchNetwork;
    var _b = react_1.default.useState(networkArg), network = _b[0], setNetwork = _b[1];
    var handleClick = function (newNetwork) {
        setNetwork(newNetwork);
        if (onSwitchNetwork && newNetwork) {
            onSwitchNetwork(newNetwork);
        }
    };
    return (0, jsx_runtime_1.jsx)(networks_1.NetworkSelector, { network: network || networkArg, onSwitchNetwork: handleClick });
};
exports.Default = Template.bind({});
exports.WithValue = Template.bind({});
exports.WithValue.args = { network: 'MAINNET' };
