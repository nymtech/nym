"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.NEStyle = exports.Default = void 0;
var jsx_runtime_1 = require("react/jsx-runtime");
var tooltip_1 = require("@lib/components/tooltip");
exports.default = {
    title: 'Basics/Tooltip',
    component: tooltip_1.Tooltip,
};
var Default = function () { return (0, jsx_runtime_1.jsx)(tooltip_1.Tooltip, { title: "tooltip", id: "field-name", placement: "top-start", arrow: true }); };
exports.Default = Default;
var NEStyle = function () { return ((0, jsx_runtime_1.jsx)(tooltip_1.Tooltip, { title: "Level of stake saturation for this node. Nodes receive more rewards the higher their saturation level, up to 100%. Beyond 100% no additional rewards are granted. The current stake saturation level is: 1 million NYM, computed as S/K where S is  total amount of tokens available to stakeholders and K is the number of nodes in the reward set.", id: "field-name", placement: "top-start", textColor: "#111826", bgColor: "#A0AED1", maxWidth: 230, arrow: true })); };
exports.NEStyle = NEStyle;
