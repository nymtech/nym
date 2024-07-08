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
exports.WithSmallIcons = exports.EmptyWithLabelAndCopy = exports.Empty = exports.ShowEntireAddress = exports.WithLabel = exports.WithCopy = exports.Default = void 0;
var jsx_runtime_1 = require("react/jsx-runtime");
var material_1 = require("@mui/material");
var client_address_1 = require("@lib/components/client-address");
exports.default = {
    title: 'Wallet / Client Address',
    component: client_address_1.ClientAddressDisplay,
};
var Template = function (args) { return ((0, jsx_runtime_1.jsx)(material_1.Box, { display: "flex", alignContent: "center", children: (0, jsx_runtime_1.jsx)(client_address_1.ClientAddressDisplay, __assign({}, args)) })); };
exports.Default = Template.bind({});
exports.Default.args = {
    address: 'n222gnd9k6rytn6tz7pf8d2d4dawl7e9cr26111',
};
exports.WithCopy = Template.bind({});
exports.WithCopy.args = {
    address: 'n222gnd9k6rytn6tz7pf8d2d4dawl7e9cr26111',
    withCopy: true,
    smallIcons: true,
};
exports.WithLabel = Template.bind({});
exports.WithLabel.args = {
    withLabel: true,
    address: 'n222gnd9k6rytn6tz7pf8d2d4dawl7e9cr26111',
};
exports.ShowEntireAddress = Template.bind({});
exports.ShowEntireAddress.args = {
    withLabel: true,
    showEntireAddress: true,
    address: 'n222gnd9k6rytn6tz7pf8d2d4dawl7e9cr26111',
};
exports.Empty = Template.bind({});
exports.Empty.args = {};
exports.EmptyWithLabelAndCopy = Template.bind({});
exports.EmptyWithLabelAndCopy.args = {
    withLabel: true,
    withCopy: true,
};
exports.WithSmallIcons = Template.bind({});
exports.WithSmallIcons.args = {
    address: 'n222gnd9k6rytn6tz7pf8d2d4dawl7e9cr26111',
    withCopy: true,
    smallIcons: true,
};
