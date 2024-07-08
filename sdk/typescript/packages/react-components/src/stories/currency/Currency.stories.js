"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.WithSX = exports.Empty = exports.Testnet = exports.Mainnet = void 0;
var jsx_runtime_1 = require("react/jsx-runtime");
var material_1 = require("@mui/material");
var currency_1 = require("@lib/components/currency");
var CurrencyAmount_stories_1 = require("./CurrencyAmount.stories");
exports.default = {
    title: 'Currency/Currency display',
    component: currency_1.Currency,
};
var Mainnet = function () { return ((0, jsx_runtime_1.jsxs)(material_1.Stack, { direction: "column", children: [(0, jsx_runtime_1.jsx)(currency_1.Currency, { majorAmount: { amount: '42.123456', denom: 'nym' } }), (0, jsx_runtime_1.jsx)(currency_1.Currency, { majorAmount: { amount: '42.123456', denom: 'nym' }, showDenom: false }), (0, jsx_runtime_1.jsx)(currency_1.Currency, { majorAmount: { amount: '42.123456', denom: 'nym' }, showCoinMark: true }), (0, jsx_runtime_1.jsx)(currency_1.Currency, { majorAmount: { amount: '42.123456', denom: 'nym' }, showCoinMark: true, coinMarkPrefix: true }), CurrencyAmount_stories_1.amounts.map(function (amount) { return ((0, jsx_runtime_1.jsx)(currency_1.Currency, { majorAmount: { amount: amount, denom: 'nym' }, showCoinMark: true, coinMarkPrefix: true }, amount)); })] })); };
exports.Mainnet = Mainnet;
var Testnet = function () { return ((0, jsx_runtime_1.jsxs)(material_1.Stack, { direction: "column", children: [(0, jsx_runtime_1.jsx)(currency_1.Currency, { majorAmount: { amount: '42.123456', denom: 'nymt' } }), (0, jsx_runtime_1.jsx)(currency_1.Currency, { majorAmount: { amount: '42.123456', denom: 'nymt' }, showDenom: false }), (0, jsx_runtime_1.jsx)(currency_1.Currency, { majorAmount: { amount: '42.123456', denom: 'nymt' }, showCoinMark: true }), (0, jsx_runtime_1.jsx)(currency_1.Currency, { majorAmount: { amount: '42.123456', denom: 'nymt' }, showCoinMark: true, coinMarkPrefix: true }), CurrencyAmount_stories_1.amounts.map(function (amount) { return ((0, jsx_runtime_1.jsx)(currency_1.Currency, { majorAmount: { amount: amount, denom: 'nymt' }, showCoinMark: true, coinMarkPrefix: true }, amount)); })] })); };
exports.Testnet = Testnet;
var Empty = function () { return (0, jsx_runtime_1.jsx)(currency_1.Currency, {}); };
exports.Empty = Empty;
var WithSX = function () { return ((0, jsx_runtime_1.jsxs)(material_1.Stack, { direction: "column", children: [CurrencyAmount_stories_1.amounts.map(function (amount) { return ((0, jsx_runtime_1.jsx)(currency_1.Currency, { majorAmount: { amount: amount, denom: 'nym' }, showCoinMark: true, sx: { fontSize: 14, color: 'red', fontWeight: 'bold', m: 1 } }, amount)); }), CurrencyAmount_stories_1.amounts.map(function (amount) { return ((0, jsx_runtime_1.jsx)(currency_1.Currency, { majorAmount: { amount: amount, denom: 'nym' }, sx: { fontSize: 14, color: 'red', fontWeight: 'bold', m: 1 } }, amount)); })] })); };
exports.WithSX = WithSX;
