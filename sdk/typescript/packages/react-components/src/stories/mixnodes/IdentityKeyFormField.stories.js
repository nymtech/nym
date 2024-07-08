"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.HideValidTick = exports.FullWidth = exports.WithPlaceholder = exports.WithLabel = exports.ReadOnlyErrorValue = exports.ReadOnlyValidValue = exports.ValidValue = exports.ErrorValue = exports.Empty = void 0;
var jsx_runtime_1 = require("react/jsx-runtime");
var material_1 = require("@mui/material");
var components_1 = require("@lib/components");
exports.default = {
    title: 'Mixnodes/Identity Key',
    component: components_1.IdentityKeyFormField,
};
var Empty = function () { return (0, jsx_runtime_1.jsx)(components_1.IdentityKeyFormField, {}); };
exports.Empty = Empty;
var ErrorValue = function () { return (0, jsx_runtime_1.jsx)(components_1.IdentityKeyFormField, { initialValue: "this is a bad value" }); };
exports.ErrorValue = ErrorValue;
var ValidValue = function () { return (0, jsx_runtime_1.jsx)(components_1.IdentityKeyFormField, { initialValue: "DZ6RfeY8DttMD3CQKoayV6mss5a5FC3RoH75Kmcujyhu" }); };
exports.ValidValue = ValidValue;
var ReadOnlyValidValue = function () { return ((0, jsx_runtime_1.jsx)(components_1.IdentityKeyFormField, { readOnly: true, initialValue: "DZ6RfeY8DttMD3CQKoayV6mss5a5FC3RoH75Kmcujyhu" })); };
exports.ReadOnlyValidValue = ReadOnlyValidValue;
var ReadOnlyErrorValue = function () { return (0, jsx_runtime_1.jsx)(components_1.IdentityKeyFormField, { readOnly: true, initialValue: "this is a bad value" }); };
exports.ReadOnlyErrorValue = ReadOnlyErrorValue;
var WithLabel = function () { return ((0, jsx_runtime_1.jsx)(material_1.Box, { p: 2, children: (0, jsx_runtime_1.jsx)(components_1.IdentityKeyFormField, { initialValue: "DZ6RfeY8DttMD3CQKoayV6mss5a5FC3RoH75Kmcujyhu", textFieldProps: { label: 'Identity Key' } }) })); };
exports.WithLabel = WithLabel;
var WithPlaceholder = function () { return ((0, jsx_runtime_1.jsx)(components_1.IdentityKeyFormField, { textFieldProps: { placeholder: 'Please enter an Identity Key' } })); };
exports.WithPlaceholder = WithPlaceholder;
var FullWidth = function () { return ((0, jsx_runtime_1.jsx)(components_1.IdentityKeyFormField, { fullWidth: true, initialValue: "DZ6RfeY8DttMD3CQKoayV6mss5a5FC3RoH75Kmcujyhu" })); };
exports.FullWidth = FullWidth;
var HideValidTick = function () { return ((0, jsx_runtime_1.jsx)(components_1.IdentityKeyFormField, { showTickOnValid: false, fullWidth: true, initialValue: "DZ6RfeY8DttMD3CQKoayV6mss5a5FC3RoH75Kmcujyhu" })); };
exports.HideValidTick = HideValidTick;
