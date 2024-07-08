"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.WithSX = exports.NoSeparatorsWithSX = exports.Empty = exports.Weird = exports.MaxRange = exports.NoSeparators = exports.WithSeparators = exports.amounts = void 0;
var jsx_runtime_1 = require("react/jsx-runtime");
var material_1 = require("@mui/material");
var currency_1 = require("@lib/components/currency");
exports.default = {
    title: 'Currency/Currency amount',
    component: currency_1.CurrencyAmount,
};
exports.amounts = [
    '0',
    '0.1',
    '0.01',
    '0.001',
    '0.0001',
    '0.00001',
    '1.000001',
    '10.000001',
    '100.000001',
    '1000.000001',
    '10000.000001',
    '100000.000001',
    '1000000.000001',
    '10000000.000001',
    '100000000.000001',
    '1000000000.000001',
    '10000000000.000001',
    '100000000000.000001',
    '1000000000000.000001',
];
var WithSeparators = function () { return ((0, jsx_runtime_1.jsx)(material_1.Stack, { direction: "column", children: exports.amounts.map(function (amount) { return ((0, jsx_runtime_1.jsx)(currency_1.CurrencyAmount, { majorAmount: { amount: amount, denom: 'nym' } }, amount)); }) })); };
exports.WithSeparators = WithSeparators;
var NoSeparators = function () { return ((0, jsx_runtime_1.jsx)(material_1.Stack, { direction: "column", children: exports.amounts.map(function (amount) { return ((0, jsx_runtime_1.jsx)(currency_1.CurrencyAmount, { majorAmount: { amount: amount, denom: 'nym' }, showSeparators: false }, amount)); }) })); };
exports.NoSeparators = NoSeparators;
var MaxRange = function () { return (0, jsx_runtime_1.jsx)(currency_1.CurrencyAmount, { majorAmount: { amount: '1000000000000.000001', denom: 'nym' } }); };
exports.MaxRange = MaxRange;
var Weird = function () { return ((0, jsx_runtime_1.jsxs)(material_1.Stack, { direction: "column", children: [(0, jsx_runtime_1.jsx)(currency_1.CurrencyAmount, { majorAmount: { amount: '0000000000000.000000', denom: 'nym' } }), (0, jsx_runtime_1.jsx)(currency_1.CurrencyAmount, { majorAmount: { amount: '0000000000000.00', denom: 'nym' } }), (0, jsx_runtime_1.jsx)(currency_1.CurrencyAmount, { majorAmount: { amount: '0000.0000', denom: 'nym' } }), (0, jsx_runtime_1.jsx)(currency_1.CurrencyAmount, { majorAmount: { amount: '0000.000', denom: 'nym' } }), (0, jsx_runtime_1.jsx)(currency_1.CurrencyAmount, { majorAmount: { amount: '0.00', denom: 'nym' } })] })); };
exports.Weird = Weird;
var Empty = function () { return (0, jsx_runtime_1.jsx)(currency_1.CurrencyAmount, {}); };
exports.Empty = Empty;
var NoSeparatorsWithSX = function () { return ((0, jsx_runtime_1.jsx)(material_1.Stack, { direction: "column", children: exports.amounts.map(function (amount) { return ((0, jsx_runtime_1.jsx)(currency_1.CurrencyAmount, { majorAmount: { amount: amount, denom: 'nym' }, showSeparators: false, sx: { fontSize: 14, color: 'red', fontWeight: 'bold', m: 1 } }, amount)); }) })); };
exports.NoSeparatorsWithSX = NoSeparatorsWithSX;
var WithSX = function () { return ((0, jsx_runtime_1.jsx)(material_1.Stack, { direction: "column", children: exports.amounts.map(function (amount) { return ((0, jsx_runtime_1.jsx)(currency_1.CurrencyAmount, { majorAmount: { amount: amount, denom: 'nym' }, sx: { fontSize: 14, color: 'red', fontWeight: 'bold', m: 1 } }, amount)); }) })); };
exports.WithSX = WithSX;
