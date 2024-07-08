"use strict";
var __assign = (this && this.__assign) || function () {
    __assign = Object.assign || function(t) {
        for (var s, i = 1, n = arguments.length; i < n; i++) {
            s = arguments[i];
            for (var p in s) if (Object.prototype.hasOwnProperty.call(s, p))
                t[p] = s[p];
        }
        return t;
    };
    return __assign.apply(this, arguments);
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.SmallIcon = exports.Default = void 0;
var jsx_runtime_1 = require("react/jsx-runtime");
var material_1 = require("@mui/material");
var clipboard_1 = require("@lib/components/clipboard");
exports.default = {
    title: 'Decorators / Copy to clipboard',
    component: clipboard_1.CopyToClipboard,
};
var Template = function (args) {
    var value = args.value;
    return ((0, jsx_runtime_1.jsxs)(material_1.Box, { display: "flex", alignContent: "center", children: [(0, jsx_runtime_1.jsx)(clipboard_1.CopyToClipboard, __assign({}, args)), (0, jsx_runtime_1.jsx)(material_1.Typography, { ml: 1, children: value })] }));
};
exports.Default = Template.bind({});
exports.Default.args = {
    tooltip: 'Copy identity key to clipboard',
    value: '123456',
};
exports.SmallIcon = Template.bind({});
exports.SmallIcon.args = {
    tooltip: 'Copy identity key to clipboard',
    value: '123456',
    smallIcons: true,
};
