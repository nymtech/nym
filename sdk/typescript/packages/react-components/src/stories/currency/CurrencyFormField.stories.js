"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.ReadOnly = exports.EmptyWithAutofocus = exports.Empty = exports.Zero = exports.MicroNym = exports.ErrorNegative = exports.ErrorToSmall = exports.ErrorToBig = exports.HideCoinMark = exports.FullWidth = exports.Testnet = exports.Mainnet = void 0;
var jsx_runtime_1 = require("react/jsx-runtime");
var material_1 = require("@mui/material");
var currency_1 = require("@lib/components/currency");
exports.default = {
    title: 'Currency/Currency form field',
    component: currency_1.CurrencyFormField,
};
var Mainnet = function () { return (0, jsx_runtime_1.jsx)(currency_1.CurrencyFormField, { initialValue: "42.123456", denom: "nymt" }); };
exports.Mainnet = Mainnet;
var Testnet = function () { return (0, jsx_runtime_1.jsx)(currency_1.CurrencyFormField, { initialValue: "42.123456", denom: "nymt" }); };
exports.Testnet = Testnet;
var FullWidth = function () { return (0, jsx_runtime_1.jsx)(currency_1.CurrencyFormField, { initialValue: "42.123456", denom: "nym", fullWidth: true }); };
exports.FullWidth = FullWidth;
var HideCoinMark = function () { return (0, jsx_runtime_1.jsx)(currency_1.CurrencyFormField, { initialValue: "42.123456", denom: "nym", showCoinMark: false }); };
exports.HideCoinMark = HideCoinMark;
var ErrorToBig = function () { return (0, jsx_runtime_1.jsx)(currency_1.CurrencyFormField, { initialValue: "1_000_000_000_000_001", denom: "nym" }); };
exports.ErrorToBig = ErrorToBig;
var ErrorToSmall = function () { return (0, jsx_runtime_1.jsx)(currency_1.CurrencyFormField, { initialValue: "0.0000001", denom: "nym" }); };
exports.ErrorToSmall = ErrorToSmall;
var ErrorNegative = function () { return (0, jsx_runtime_1.jsx)(currency_1.CurrencyFormField, { initialValue: "-1", denom: "nym" }); };
exports.ErrorNegative = ErrorNegative;
var MicroNym = function () { return (0, jsx_runtime_1.jsx)(currency_1.CurrencyFormField, { initialValue: "0.000001", denom: "nym" }); };
exports.MicroNym = MicroNym;
var Zero = function () { return (0, jsx_runtime_1.jsx)(currency_1.CurrencyFormField, { initialValue: "0", denom: "nym" }); };
exports.Zero = Zero;
var Empty = function () { return (0, jsx_runtime_1.jsx)(currency_1.CurrencyFormField, {}); };
exports.Empty = Empty;
var EmptyWithAutofocus = function () { return (0, jsx_runtime_1.jsx)(currency_1.CurrencyFormField, { autoFocus: true }); };
exports.EmptyWithAutofocus = EmptyWithAutofocus;
var ReadOnly = function () { return ((0, jsx_runtime_1.jsxs)(material_1.Stack, { direction: "column", spacing: 2, children: [(0, jsx_runtime_1.jsx)(currency_1.CurrencyFormField, { initialValue: "42.123456", denom: "nym", readOnly: true }), (0, jsx_runtime_1.jsx)(currency_1.CurrencyFormField, { initialValue: "42.123456", denom: "nymt", readOnly: true })] })); };
exports.ReadOnly = ReadOnly;
