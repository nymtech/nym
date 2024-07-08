"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.HideValidTick = exports.FullWidth = exports.WithPlaceholder = exports.WithLabel = exports.ReadOnlyErrorValue = exports.ReadOnlyValidValue = exports.ValidValue = exports.ErrorValue = exports.Empty = void 0;
var jsx_runtime_1 = require("react/jsx-runtime");
var material_1 = require("@mui/material");
var account_1 = require("@lib/components/account");
exports.default = {
    title: 'Accounts/Wallet Address',
    component: account_1.WalletAddressFormField,
};
var Empty = function () { return (0, jsx_runtime_1.jsx)(account_1.WalletAddressFormField, {}); };
exports.Empty = Empty;
var ErrorValue = function () { return (0, jsx_runtime_1.jsx)(account_1.WalletAddressFormField, { initialValue: "this is a bad value" }); };
exports.ErrorValue = ErrorValue;
var ValidValue = function () { return (0, jsx_runtime_1.jsx)(account_1.WalletAddressFormField, { initialValue: "n1xr4w0kddak8d8zlfmu8sl6dk2r4p9uhhzzlaec" }); };
exports.ValidValue = ValidValue;
var ReadOnlyValidValue = function () { return ((0, jsx_runtime_1.jsx)(account_1.WalletAddressFormField, { readOnly: true, initialValue: "n1xr4w0kddak8d8zlfmu8sl6dk2r4p9uhhzzlaec" })); };
exports.ReadOnlyValidValue = ReadOnlyValidValue;
var ReadOnlyErrorValue = function () { return (0, jsx_runtime_1.jsx)(account_1.WalletAddressFormField, { readOnly: true, initialValue: "this is a bad value" }); };
exports.ReadOnlyErrorValue = ReadOnlyErrorValue;
var WithLabel = function () { return ((0, jsx_runtime_1.jsx)(material_1.Box, { p: 2, children: (0, jsx_runtime_1.jsx)(account_1.WalletAddressFormField, { initialValue: "n1xr4w0kddak8d8zlfmu8sl6dk2r4p9uhhzzlaec", textFieldProps: { label: 'Identity Key' } }) })); };
exports.WithLabel = WithLabel;
var WithPlaceholder = function () { return ((0, jsx_runtime_1.jsx)(account_1.WalletAddressFormField, { textFieldProps: { placeholder: 'Please enter an wallet address' } })); };
exports.WithPlaceholder = WithPlaceholder;
var FullWidth = function () { return ((0, jsx_runtime_1.jsx)(account_1.WalletAddressFormField, { fullWidth: true, initialValue: "n1xr4w0kddak8d8zlfmu8sl6dk2r4p9uhhzzlaec" })); };
exports.FullWidth = FullWidth;
var HideValidTick = function () { return ((0, jsx_runtime_1.jsx)(account_1.WalletAddressFormField, { showTickOnValid: false, fullWidth: true, initialValue: "n1xr4w0kddak8d8zlfmu8sl6dk2r4p9uhhzzlaec" })); };
exports.HideValidTick = HideValidTick;
