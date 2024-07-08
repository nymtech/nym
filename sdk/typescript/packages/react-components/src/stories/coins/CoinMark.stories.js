"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.Sizes = exports.DarkMode = exports.LightMode = exports.Auto = void 0;
var jsx_runtime_1 = require("react/jsx-runtime");
var material_1 = require("@mui/material");
var coins_1 = require("@lib/components/coins");
exports.default = {
    title: 'Branding/Coin Mark',
    component: coins_1.CoinMark,
};
var Auto = function () { return (0, jsx_runtime_1.jsx)(coins_1.CoinMark, { height: 250 }); };
exports.Auto = Auto;
var LightMode = function () { return (0, jsx_runtime_1.jsx)(coins_1.CoinMark, { mode: "light", height: 250 }); };
exports.LightMode = LightMode;
var DarkMode = function () { return (0, jsx_runtime_1.jsx)(coins_1.CoinMark, { mode: "dark", height: 250 }); };
exports.DarkMode = DarkMode;
var sizes = [8, 10, 12, 16, 20, 32, 40, 64];
var Sizes = function () { return ((0, jsx_runtime_1.jsx)(material_1.Stack, { direction: "column", spacing: 2, children: sizes.map(function (size) { return ((0, jsx_runtime_1.jsxs)(material_1.Stack, { direction: "row", spacing: 4, p: 1, alignItems: "center", borderBottom: "1px solid #444", children: [(0, jsx_runtime_1.jsxs)(material_1.Typography, { sx: { opacity: 0.5 }, width: "40px", children: [size, "px"] }), (0, jsx_runtime_1.jsx)(coins_1.CoinMark, { height: size }, size)] })); }) })); };
exports.Sizes = Sizes;
